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

    /// True when at least one slot is currently in flight (phase
    /// `.preparingKeys` or `.inFlight`). Used by the network
    /// toggle's `.disabled(_:)` modifier — switching testnet ↔
    /// mainnet mid-flight tears down the FFI manager and would
    /// abort the in-flight call mid-stream. The UI guards against
    /// that race by reading this flag.
    var hasInFlightRegistrations: Bool {
        controllers.contains { _, controller in
            switch controller.phase {
            case .preparingKeys, .inFlight:
                return true
            default:
                return false
            }
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
    /// Single-flighting is handled inside
    /// `IdentityRegistrationController.submit` — a second call for
    /// the same slot while the first is in flight is silently
    /// ignored at the controller layer.
    func startRegistration(
        walletId: Data,
        identityIndex: UInt32,
        body: @escaping () async throws -> Data
    ) -> IdentityRegistrationController {
        let key = SlotKey(walletId: walletId, identityIndex: identityIndex)
        let controller: IdentityRegistrationController
        if let existing = controllers[key] {
            controller = existing
        } else {
            controller = IdentityRegistrationController(
                walletId: walletId,
                identityIndex: identityIndex
            )
            controllers[key] = controller
        }
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
                case .failed:
                    // Keep indefinitely; the user dismisses manually
                    // via the "Dismiss" action. Return so the poll
                    // loop doesn't spin.
                    return
                default:
                    completedAt = nil
                }
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }
}
