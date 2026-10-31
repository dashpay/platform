import Foundation
import DashSDKFFI

extension PlatformWalletManager {
    /// Start fetching Platform's recorded shielded-anchor set for an
    /// upcoming spend, in the background. Call it when a shielded send
    /// screen (transfer, unshield, withdraw, identity top-up/create from
    /// the pool) opens: a send confirmed within ~30 s — before the next
    /// shielded sync grows the commitment tree — then skips that network
    /// round trip. Returns immediately and is cheap to repeat; a failed
    /// prefetch only means the send fetches for itself.
    ///
    /// Throws when the manager isn't configured or has no shielded
    /// support configured.
    public func prefetchShieldedSpendAnchors() throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        try platform_wallet_manager_shielded_prefetch_spend_anchors(handle).check()
    }
}
