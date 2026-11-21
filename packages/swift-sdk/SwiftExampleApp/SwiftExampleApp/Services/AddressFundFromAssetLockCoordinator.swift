import Combine
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
final class AddressFundFromAssetLockCoordinator: ObservableObject {
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
    /// the "Pending Platform Top Ups" row on the Wallet Detail
    /// screen can observe map mutations via `objectWillChange`.
    @Published private(set) var controllers: [SlotKey: AddressFundFromAssetLockController] = [:]

    /// Per-slot subscriptions that forward each controller's
    /// `objectWillChange` (fired on a `phase` transition) to this
    /// coordinator's own `objectWillChange`.
    ///
    /// The `@Published controllers` map only re-publishes on
    /// insert/remove, NOT when a controller already in the map flips
    /// phase (`.inFlight → .failed`/`.completed`, or a `.failed`-slot
    /// retry that reuses the controller). A view that observes only the
    /// coordinator — e.g. the wallet-detail "Pending Top Ups" list, whose
    /// Resume gate reads controller `phase` — would otherwise render from
    /// stale phase state across those transitions. Keyed by slot so the
    /// subscription is torn down with the controller it tracks.
    private var phaseObservers: [SlotKey: AnyCancellable] = [:]

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
    ) -> AddressFundFromAssetLockController? {
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
    func activeControllers() -> [AddressFundFromAssetLockController] {
        controllers.values.sorted { lhs, rhs in
            (lhs.lastSubmittedAt ?? .distantPast) > (rhs.lastSubmittedAt ?? .distantPast)
        }
    }

    /// Start a funding for the slot, or reuse an existing
    /// controller if one is already in flight for it. Returns the
    /// controller for `FundFromAssetLockPlatformAddressView` to bind a
    /// `AddressFundFromAssetLockProgressView` against.
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
        recipientType: UInt8,
        resuming: AssetLockBuiltOutPoint? = nil,
        body: @escaping () async throws -> UInt64
    ) -> AddressFundFromAssetLockController {
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
            case .failed where !existing.phase.permitsSubmit(resuming: resuming):
                // The failure's class forbids this operation (the payment
                // may have left, or it holds a lock this call does not
                // resume): keep the failed state instead of paying again.
                return existing
            case .idle, .failed:
                // Legitimate restart paths: a retry the failure's class
                // allows, or the resume of the lock it holds.
                existing.submit(resuming: resuming, body: body)
                // No retention sweep here — the slot is sticky on
                // .failed (we want the user to see + dismiss the
                // error) and a duplicate sweep on retry would just
                // spawn a second 30s poll Task against the same
                // controller. Sweep was already scheduled when the
                // controller was first created.
                return existing
            }
        }
        let controller = AddressFundFromAssetLockController(
            walletId: walletId,
            platformAccountIndex: platformAccountIndex,
            recipientHash: recipientHash,
            recipientType: recipientType
        )
        controllers[key] = controller
        // Forward this controller's phase transitions to observers of the
        // coordinator (see `phaseObservers`). Registered before `submit()`
        // so the initial `.idle → .inFlight` flip re-publishes too.
        phaseObservers[key] = controller.objectWillChange.sink { [weak self] _ in
            self?.objectWillChange.send()
        }
        controller.submit(resuming: resuming, body: body)
        scheduleRetentionSweep(key: key, controller: controller)
        return controller
    }

    /// Manually drop a controller from the map. Used by the UI's
    /// "Dismiss" action on a `.failed` row (failures stay
    /// indefinitely until acknowledged so the user can read the
    /// error) and its "Done" after a completion.
    ///
    /// A failed slot is dropped only when its failure releases it
    /// (`PaymentFailure.holdsSlot`, any `.notSent`): `startFunding` gates a
    /// restart only on a slot it still holds, so dropping a failure that may
    /// have left, holds a lock, or has no class would let a fresh lock be
    /// funded for the same recipient. Such a slot stays until its lock is
    /// resumed or the app restarts. Returns whether the controller was
    /// dropped (← `RegistrationCoordinator.dismiss`).
    @discardableResult
    func dismiss(
        walletId: Data,
        platformAccountIndex: UInt32,
        recipientHash: Data
    ) -> Bool {
        let key = SlotKey(
            walletId: walletId,
            platformAccountIndex: platformAccountIndex,
            recipientHash: recipientHash
        )
        if let controller = controllers[key], case .failed(let failure) = controller.phase,
           failure.holdsSlot {
            return false
        }
        controllers.removeValue(forKey: key)
        phaseObservers.removeValue(forKey: key)
        return true
    }

    // MARK: - Retention sweep

    /// Auto-purge `.completed` controllers ~30s after the success
    /// transition so the wallet's Pending list doesn't accumulate
    /// stale rows. `.failed` controllers stay indefinitely until
    /// the user dismisses them. Same shape as
    /// `RegistrationCoordinator`.
    private func scheduleRetentionSweep(
        key: SlotKey,
        controller: AddressFundFromAssetLockController
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
                            self?.phaseObservers.removeValue(forKey: key)
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
