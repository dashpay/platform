import Foundation
import SwiftDashSDK

/// Singleton hub for in-flight platform-address funding attempts,
/// hosted on `PlatformWalletManager` so funds survive view
/// dismissal and network-toggle pressure.
///
/// Mirrors [`RegistrationCoordinator`] for the
/// `AddressFundingFromAssetLockTransition` flow. Keyed by
/// `(walletId, platformAccountIndex, recipientHash)` — that's the
/// natural unit of work since a wallet can fund many addresses
/// concurrently and "next unused" address allocation happens
/// Rust-side per call. The single-flight invariant prevents a user
/// from double-tapping the same address-funding submission during
/// the asset-lock broadcast window.
@MainActor
final class AddressFundingCoordinator: ObservableObject {
    /// Composite key — needs `Hashable` so the map can index by it.
    /// `walletId` is 32 raw bytes; `recipientHash` is 20 raw bytes;
    /// `platformAccountIndex` is the DIP-17 account that owns the
    /// recipient address.
    struct SlotKey: Hashable {
        let walletId: Data
        let platformAccountIndex: UInt32
        let recipientHash: Data
    }

    /// Active controllers keyed by slot. Stored as `@Published` so
    /// the "Pending Platform Funding" row on the Wallet Detail
    /// screen can observe map mutations via `objectWillChange`.
    @Published private(set) var controllers: [SlotKey: AddressFundingController] = [:]

    /// True when at least one slot is currently in flight (phase
    /// `.inFlight`). Used by the network toggle's `.disabled(_:)`
    /// modifier — switching testnet ↔ mainnet mid-flight tears down
    /// the FFI manager and would abort the in-flight call.
    var hasInFlightFundings: Bool {
        controllers.contains { _, controller in
            if case .inFlight = controller.phase { return true }
            return false
        }
    }

    /// Look up the controller for a slot if one exists. Returns
    /// `nil` when there's no active funding for the slot — callers
    /// use that to decide whether to spawn a new controller or
    /// reuse the existing one.
    func controller(
        walletId: Data,
        platformAccountIndex: UInt32,
        recipientHash: Data
    ) -> AddressFundingController? {
        controllers[
            SlotKey(
                walletId: walletId,
                platformAccountIndex: platformAccountIndex,
                recipientHash: recipientHash
            )
        ]
    }

    /// Snapshot of every active controller, sorted by recency of
    /// last submit (most recent first). Used by the "Pending
    /// Platform Funding" row so dismissed-but-still-running flows
    /// remain reachable.
    func activeControllers() -> [AddressFundingController] {
        controllers.values.sorted { lhs, rhs in
            (lhs.lastSubmittedAt ?? .distantPast) > (rhs.lastSubmittedAt ?? .distantPast)
        }
    }

    /// Start a funding for the slot, or reuse an existing
    /// controller if one is already in flight for it. Returns the
    /// controller for `FundPlatformAddressView` to bind a
    /// `AddressFundingProgressView` against.
    ///
    /// Single-flighting is enforced here at the coordinator level
    /// because the controller's `submit()` only guards within its
    /// own phase machine — without a phase check before fresh-slot
    /// creation, a second tap during the FFI window would race two
    /// FFI calls for the same asset lock.
    func startFunding(
        walletId: Data,
        platformAccountIndex: UInt32,
        recipientHash: Data,
        body: @escaping () async throws -> UInt64
    ) -> AddressFundingController {
        let key = SlotKey(
            walletId: walletId,
            platformAccountIndex: platformAccountIndex,
            recipientHash: recipientHash
        )
        if let existing = controllers[key] {
            switch existing.phase {
            case .inFlight, .completed:
                // Active or just-completed — don't re-enter. Returning
                // the existing controller lets the caller bind to its
                // progress / terminal state without disrupting it.
                return existing
            case .idle, .failed:
                // Legitimate restart paths.
                existing.submit(body: body)
                scheduleRetentionSweep(key: key, controller: existing)
                return existing
            }
        }
        let controller = AddressFundingController(
            walletId: walletId,
            platformAccountIndex: platformAccountIndex,
            recipientHash: recipientHash
        )
        controllers[key] = controller
        controller.submit(body: body)
        scheduleRetentionSweep(key: key, controller: controller)
        return controller
    }

    /// Manually drop a controller from the map. Used by the UI's
    /// "Dismiss" action on a `.failed` row (failures stay
    /// indefinitely until acknowledged so the user can read the
    /// error).
    func dismiss(
        walletId: Data,
        platformAccountIndex: UInt32,
        recipientHash: Data
    ) {
        let key = SlotKey(
            walletId: walletId,
            platformAccountIndex: platformAccountIndex,
            recipientHash: recipientHash
        )
        controllers.removeValue(forKey: key)
    }

    // MARK: - Retention sweep

    /// Auto-purge `.completed` controllers ~30s after the success
    /// transition so the wallet's Pending list doesn't accumulate
    /// stale rows. `.failed` controllers stay indefinitely until
    /// the user dismisses them. Same shape as
    /// `RegistrationCoordinator`.
    private func scheduleRetentionSweep(
        key: SlotKey,
        controller: AddressFundingController
    ) {
        Task { [weak self, weak controller] in
            guard let controller = controller else { return }
            var completedAt: Date?
            while !Task.isCancelled {
                let phase = await MainActor.run { controller.phase }
                switch phase {
                case .completed:
                    if completedAt == nil {
                        completedAt = Date()
                    } else if let at = completedAt,
                              Date().timeIntervalSince(at) >= 30 {
                        await MainActor.run {
                            _ = self?.controllers.removeValue(forKey: key)
                        }
                        return
                    }
                case .failed:
                    return
                default:
                    completedAt = nil
                }
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }
}
