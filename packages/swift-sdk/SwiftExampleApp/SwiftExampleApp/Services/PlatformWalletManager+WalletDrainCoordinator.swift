import Foundation
import ObjectiveC
import SwiftDashSDK

/// Per-manager `WalletDrainCoordinator`, held in an associated object like
/// `shieldedFundFromAssetLockCoordinator`: app-only state that keeps the
/// SDK module untouched.
@MainActor
extension PlatformWalletManager {
    private static var walletDrainCoordinatorKey: UInt8 = 0

    var walletDrainCoordinator: WalletDrainCoordinator {
        if let existing = objc_getAssociatedObject(
            self,
            &PlatformWalletManager.walletDrainCoordinatorKey
        ) as? WalletDrainCoordinator {
            return existing
        }
        let fresh = WalletDrainCoordinator()
        objc_setAssociatedObject(
            self,
            &PlatformWalletManager.walletDrainCoordinatorKey,
            fresh,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC
        )
        return fresh
    }
}
