import Foundation
import ObjectiveC
import SwiftDashSDK

/// Per-manager `ShieldedFundFromAssetLockCoordinator` accessor.
/// Mirrors the [`addressFundFromAssetLockCoordinator`]
/// (PlatformWalletManager.addressFundFromAssetLockCoordinator) shape
/// — lazy-initialized on first access and lifetime-tied to the
/// `PlatformWalletManager` instance via an `objc_getAssociatedObject`
/// slot.
///
/// Why this shape: the coordinator is example-app-only state (it
/// stores `ShieldedFundFromAssetLockController` instances, which live
/// in the app, not the SDK). The associated-object hook keeps the
/// call site clean while leaving the SDK module untouched.
@MainActor
extension PlatformWalletManager {
    private static var shieldedFundFromAssetLockCoordinatorKey: UInt8 = 0

    /// Per-manager shielded-funding coordinator. Created on first
    /// access; subsequent reads return the same instance.
    var shieldedFundFromAssetLockCoordinator: ShieldedFundFromAssetLockCoordinator {
        if let existing = objc_getAssociatedObject(
            self,
            &PlatformWalletManager.shieldedFundFromAssetLockCoordinatorKey
        ) as? ShieldedFundFromAssetLockCoordinator {
            return existing
        }
        let fresh = ShieldedFundFromAssetLockCoordinator()
        objc_setAssociatedObject(
            self,
            &PlatformWalletManager.shieldedFundFromAssetLockCoordinatorKey,
            fresh,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC
        )
        return fresh
    }
}
