import Foundation
import ObjectiveC
import SwiftDashSDK

/// Per-manager `AddressTopUpCoordinator` accessor. Mirrors the
/// [`registrationCoordinator`](PlatformWalletManager.registrationCoordinator)
/// shape — lazy-initialized on first access and lifetime-tied to
/// the `PlatformWalletManager` instance via an
/// `objc_getAssociatedObject` slot.
///
/// Why this shape: the coordinator is example-app-only state (it
/// stores `AddressTopUpController` instances, which live in the
/// app, not the SDK). The associated-object hook keeps the call
/// site clean while leaving the SDK module untouched.
@MainActor
extension PlatformWalletManager {
    private static var addressTopUpCoordinatorKey: UInt8 = 0

    /// Per-manager address-funding coordinator. Created on first
    /// access; subsequent reads return the same instance.
    var addressTopUpCoordinator: AddressTopUpCoordinator {
        if let existing = objc_getAssociatedObject(
            self,
            &PlatformWalletManager.addressTopUpCoordinatorKey
        ) as? AddressTopUpCoordinator {
            return existing
        }
        let fresh = AddressTopUpCoordinator()
        objc_setAssociatedObject(
            self,
            &PlatformWalletManager.addressTopUpCoordinatorKey,
            fresh,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC
        )
        return fresh
    }
}
