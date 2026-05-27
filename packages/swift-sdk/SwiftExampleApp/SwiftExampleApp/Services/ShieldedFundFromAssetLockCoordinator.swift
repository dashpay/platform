import Foundation
import SwiftDashSDK

/// Singleton hub for in-flight shielded fund-from-asset-lock attempts,
/// hosted on `PlatformWalletManager` so funds survive view dismissal
/// and network-toggle pressure.
///
/// Mirrors [`AddressFundFromAssetLockCoordinator`] for Type 18
/// (`ShieldFromAssetLockTransition`). Keyed by
/// `(walletId, recipientRaw43)` — a wallet can shield to many
/// distinct Orchard recipients concurrently, but the single-flight
/// invariant prevents a user from double-tapping the same recipient
/// during the asset-lock + Halo 2 proof window (~30s).
@MainActor
final class ShieldedFundFromAssetLockCoordinator: ObservableObject {
    /// Composite key — `walletId` is 32 raw bytes; `recipientRaw43`
    /// is the 43-byte raw Orchard payment address (11-byte
    /// diversifier + 32-byte pk_d).
    struct SlotKey: Hashable {
        let walletId: Data
        let recipientRaw43: Data
    }

    /// Active controllers keyed by slot. Stored as `@Published` so
    /// the "Pending Shielded Top Ups" row on the Wallet Detail
    /// screen can observe map mutations via `objectWillChange`.
    @Published private(set) var controllers: [SlotKey: ShieldedFundFromAssetLockController] = [:]

    /// True when at least one slot is currently in flight (phase
    /// `.inFlight`). Used by the network toggle's `.disabled(_:)`
    /// modifier — switching testnet ↔ mainnet mid-flight tears
    /// down the FFI manager and would abort the in-flight call.
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
        recipientRaw43: Data
    ) -> ShieldedFundFromAssetLockController? {
        controllers[SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)]
    }

    /// Snapshot of every active controller, sorted by recency of
    /// last submit (most recent first). Used by the "Pending
    /// Shielded Funding" row so dismissed-but-still-running flows
    /// remain reachable.
    func activeControllers() -> [ShieldedFundFromAssetLockController] {
        controllers.values.sorted { lhs, rhs in
            (lhs.lastSubmittedAt ?? .distantPast) > (rhs.lastSubmittedAt ?? .distantPast)
        }
    }

    /// Start a funding for the slot, or reuse an existing controller
    /// if one is already in flight. Returns the controller for
    /// `ShieldedFundFromAssetLockView` to bind a progress section
    /// against.
    ///
    /// Single-flighting is enforced here at the coordinator level —
    /// the controller's `submit()` only guards within its own phase
    /// machine, so without a phase check before fresh-slot creation
    /// a second tap during the FFI window would race two FFI calls
    /// for the same recipient + asset lock.
    func startFunding(
        walletId: Data,
        recipientRaw43: Data,
        shieldAmountCredits: UInt64,
        body: @escaping () async throws -> Void
    ) -> ShieldedFundFromAssetLockController {
        let key = SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)
        if let existing = controllers[key] {
            switch existing.phase {
            case .inFlight, .completed:
                // Active or just-completed — don't re-enter.
                // Returning the existing controller lets the caller
                // bind to its progress / terminal state without
                // disrupting it.
                return existing
            case .idle, .failed:
                // Legitimate restart paths.
                existing.submit(body: body)
                // No retention sweep here — the slot is sticky on
                // .failed (we want the user to see + dismiss the
                // error) and a duplicate sweep on retry would
                // spawn a second 30s poll Task against the same
                // controller. Sweep was already scheduled when the
                // controller was first created.
                return existing
            }
        }
        let controller = ShieldedFundFromAssetLockController(
            walletId: walletId,
            recipientRaw43: recipientRaw43,
            shieldAmountCredits: shieldAmountCredits
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
    func dismiss(walletId: Data, recipientRaw43: Data) {
        let key = SlotKey(walletId: walletId, recipientRaw43: recipientRaw43)
        controllers.removeValue(forKey: key)
    }

    // MARK: - Retention sweep

    /// Auto-purge `.completed` controllers ~30s after the success
    /// transition so the wallet's Pending list doesn't accumulate
    /// stale rows. `.failed` controllers stay indefinitely until
    /// the user dismisses them. Same shape as the address-funding
    /// sibling.
    private func scheduleRetentionSweep(
        key: SlotKey,
        controller: ShieldedFundFromAssetLockController
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
