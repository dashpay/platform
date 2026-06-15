import Foundation
import SwiftDashSDK

/// Singleton hub for in-flight identity registrations, hosted on
/// `PlatformWalletManager` so registrations survive view dismissal
/// and network-toggle pressure.
///
/// Why on PlatformWalletManager (not AppState):
/// - `PlatformWalletManager` is the per-network operational hub
///   and outlives any individual view's lifetime.
/// - `AppState` is a bootstrap host whose lifetime is the whole
///   app, but it doesn't own the wallet / FFI handle the
///   registration call needs.
/// - Registration state has the same lifetime as the
///   wallet/network pairing: switching networks blows away the
///   manager and any in-flight registrations belong to the prior
///   network anyway.
///
/// Keyed by `(walletId, identityIndex)`. The slot model enforces
/// "one registration in flight per identity slot" naturally because
/// the wallet's `unusedIdentityIndices` invariant only ever
/// surfaces a slot to the UI once.
@MainActor
final class RegistrationCoordinator: ObservableObject {
    /// Composite key — needs `Hashable` so the map can index by it.
    /// The `walletId` is treated as 32 raw bytes; the `identityIndex`
    /// is the HD slot the caller is registering against.
    struct SlotKey: Hashable {
        let walletId: Data
        let identityIndex: UInt32
    }

    /// Active controllers keyed by slot. Stored as `@Published` so
    /// the "Pending registrations" row on the identities tab can
    /// observe map mutations via `objectWillChange`.
    @Published private(set) var controllers: [SlotKey: IdentityRegistrationController] = [:]

    /// True when at least one slot is still holding itself against a
    /// fresh registration — exactly `controller.phase.isActive`
    /// (`.preparingKeys`, `.inFlight`, or `.unconfirmed`). Used by the
    /// network toggle's `.disabled(_:)` modifier.
    ///
    /// Two reasons a slot must hold this gate:
    /// - `.preparingKeys` / `.inFlight`: switching testnet ↔ mainnet
    ///   mid-flight tears down the FFI manager and would abort the
    ///   in-flight call mid-stream.
    /// - `.unconfirmed`: the identity is probably live on chain. The
    ///   picker's `usedIdentityIndices` unions the persisted `isUsed`
    ///   reservation with the `PersistentIdentity` rows, but that
    ///   reservation write is best-effort (silent no-op when the slot row
    ///   is beyond the derived lookahead) — so the live controller remains
    ///   a load-bearing guard until the identity row lands via sync.
    ///   Switching networks tears down the `PlatformWalletManager` and
    ///   with it this coordinator, dropping the controller (and the
    ///   Rust-side note reservation); the same HD slot could become
    ///   selectable and a re-submission would be rejected by the
    ///   registered-key-hash stateful check and burn the funded spend.
    ///
    /// Reading `isActive` directly (rather than re-listing the cases)
    /// keeps this gate from drifting from the phase model, mirroring
    /// `PendingRegistrationsList.isDismissable`. UX trade-off, by design
    /// (same as the dismissal gate): an `.unconfirmed` row blocks network
    /// switching until it becomes dismissable (the identity row arrives
    /// via sync) or the app restarts.
    var hasInFlightRegistrations: Bool {
        controllers.contains { _, controller in
            controller.phase.isActive
        }
    }

    /// Look up the controller for a slot if one exists. Returns
    /// `nil` when there's no active registration for the slot —
    /// callers use that to decide whether to spawn a new controller
    /// or reuse the existing one.
    func controller(walletId: Data, identityIndex: UInt32) -> IdentityRegistrationController? {
        controllers[SlotKey(walletId: walletId, identityIndex: identityIndex)]
    }

    /// Snapshot of every active controller, sorted by recency of
    /// last submit (most recent first). Used by the "Pending
    /// registrations" row so dismissed-but-still-running flows
    /// remain reachable.
    func activeControllers() -> [IdentityRegistrationController] {
        controllers.values.sorted { lhs, rhs in
            (lhs.lastSubmittedAt ?? .distantPast) > (rhs.lastSubmittedAt ?? .distantPast)
        }
    }

    /// Start a registration for the slot, or reuse an existing
    /// controller if one is already in flight for it. Returns the
    /// controller for `CreateIdentityView` to bind a
    /// `RegistrationProgressView` against.
    ///
    /// Single-flighting is enforced here at the coordinator level
    /// (rather than just inside `IdentityRegistrationController.submit`)
    /// because the controller's `enterPreparingKeys()` unconditionally
    /// overwrites `phase`; without a phase check before that call, a
    /// second tap on the same slot during the FFI window would set
    /// `.inFlight → .preparingKeys → .inFlight`, racing two FFI calls
    /// for the same asset lock. The Resumable Registrations section
    /// surfaces orphan locks based on the absence of a
    /// `PersistentIdentity` row, which only lands after the FFI
    /// returns — so during the lock-broadcast-to-identity-write
    /// window, the same slot was visible in both Pending and
    /// Resumable surfaces and could be double-tapped.
    func startRegistration(
        walletId: Data,
        identityIndex: UInt32,
        fundingKind: IdentityRegistrationController.FundingKind = .assetLock,
        body: @escaping () async throws -> Data
    ) -> IdentityRegistrationController {
        let key = SlotKey(walletId: walletId, identityIndex: identityIndex)
        if let existing = controllers[key] {
            switch existing.phase {
            case .preparingKeys, .inFlight, .completed, .unconfirmed:
                // Active, just-completed, or unconfirmed — don't re-enter.
                // Returning the existing controller lets the caller bind to
                // its progress / terminal state without disrupting it. For
                // `.unconfirmed` in particular, re-submitting would race a
                // duplicate registration against an identity that's probably
                // already live on chain.
                return existing
            case .idle, .failed:
                // Legitimate restart paths: a brand-new idle
                // controller (shouldn't happen via the standard
                // entry but safe to allow), or a user-initiated
                // retry after a failure.
                existing.enterPreparingKeys()
                existing.submit(body: body)
                scheduleRetentionSweep(key: key, controller: existing)
                return existing
            }
        }
        let controller = IdentityRegistrationController(
            walletId: walletId,
            identityIndex: identityIndex,
            fundingKind: fundingKind
        )
        controllers[key] = controller
        controller.enterPreparingKeys()
        controller.submit(body: body)
        scheduleRetentionSweep(key: key, controller: controller)
        return controller
    }

    /// Manually drop a controller from the map. Used by the UI's
    /// "Dismiss" action on a `.failed` row (failures stay
    /// indefinitely until acknowledged so the user can read the
    /// error).
    func dismiss(walletId: Data, identityIndex: UInt32) {
        let key = SlotKey(walletId: walletId, identityIndex: identityIndex)
        controllers.removeValue(forKey: key)
    }

    // MARK: - Retention sweep

    /// Auto-purge `.completed` controllers ~30s after the success
    /// transition so the home tab's pending list doesn't accumulate
    /// stale rows. `.failed` controllers stay indefinitely until
    /// the user dismisses them (their error message is the only
    /// surface where the failure is reported).
    private func scheduleRetentionSweep(
        key: SlotKey,
        controller: IdentityRegistrationController
    ) {
        // Observe phase transitions on the controller and arm a
        // 30s sweep after a `.completed` flip. `Combine`'s sink is
        // overkill for a single observer — we poll via a Task that
        // re-checks every second until either:
        //   - the controller is gone (already dismissed), or
        //   - 30s have elapsed since the success transition.
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
                case .failed, .unconfirmed:
                    // Keep indefinitely; the user dismisses manually via the
                    // "Dismiss" action (for `.unconfirmed`, only after the
                    // identity row appears via sync). Return so the poll loop
                    // doesn't spin.
                    return
                default:
                    completedAt = nil
                }
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }
}
