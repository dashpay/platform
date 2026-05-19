import Foundation
import ObjectiveC
import SwiftDashSDK

/// Per-manager `RegistrationCoordinator` accessor. Lazy-initialized
/// on first access and lifetime-tied to the
/// [`PlatformWalletManager`](SwiftDashSDK.PlatformWalletManager)
/// instance via an `objc_getAssociatedObject` slot.
///
/// Why this shape: the coordinator is example-app-only state (it
/// stores `IdentityRegistrationController` instances, which live in
/// the app, not the SDK), but the call site convention from the
/// plan reads as `walletManager.registrationCoordinator.startRegistration(...)`.
/// Storing it directly on the SDK type would push the controller
/// type into `SwiftDashSDK`, which violates the architectural rule
/// in `swift-sdk/CLAUDE.md` ("SDK does persist / load / bridge — no
/// business logic"). The associated-object hook keeps the call site
/// clean while leaving the SDK module untouched.
///
/// The coordinator's lifetime matches the manager's: when
/// `WalletManagerStore` deallocs an inactive manager (none does
/// today, but in principle), the associated object is released
/// alongside it. Switching networks at runtime tears down the
/// active manager and any in-flight registrations belong to the
/// prior network anyway — this matches the
/// `hasInFlightRegistrations` gate on the network toggle in
/// `CoreContentView`.
@MainActor
extension PlatformWalletManager {
    /// Backing key for the associated-object slot. A static address
    /// is required by the runtime; `let _key = UInt8(0)` produces a
    /// stable per-program-address that's unique to this extension.
    private static var coordinatorKey: UInt8 = 0

    /// Per-manager registration coordinator. Created on first
    /// access; subsequent reads return the same instance.
    var registrationCoordinator: RegistrationCoordinator {
        if let existing = objc_getAssociatedObject(
            self,
            &PlatformWalletManager.coordinatorKey
        ) as? RegistrationCoordinator {
            return existing
        }
        let fresh = RegistrationCoordinator()
        objc_setAssociatedObject(
            self,
            &PlatformWalletManager.coordinatorKey,
            fresh,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC
        )
        return fresh
    }
}
