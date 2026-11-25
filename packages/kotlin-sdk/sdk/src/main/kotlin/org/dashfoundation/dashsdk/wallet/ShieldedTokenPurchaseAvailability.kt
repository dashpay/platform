package org.dashfoundation.dashsdk.wallet

/**
 * Whether a token can be bought from the shielded pool, as Rust reads the
 * token's verified configuration
 * (`platform_wallet_manager_shielded_token_purchase_availability`). Port of
 * `PlatformWalletManager.ShieldedTokenPurchaseAvailability` in
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManagerShieldedSync.swift`.
 *
 * A purchase form offers the shielded payment on [Available] only.
 */
enum class ShieldedTokenPurchaseAvailability(val raw: Int) {
    /** The token has a shielded pool to mint the bought tokens into. */
    Available(0),

    /** The token's configuration has no shielded pool. */
    NoShieldedPool(1),

    /** The token's configuration could not be read; asking again may succeed. */
    ConfigurationUnreadable(2),
    ;

    internal companion object {
        /**
         * The availability for the byte the native call returned. A byte
         * this build does not know is read as unreadable, never as
         * available: the form then keeps the shielded payment off.
         */
        fun fromNative(raw: Int): ShieldedTokenPurchaseAvailability =
            entries.firstOrNull { it.raw == raw } ?: ConfigurationUnreadable
    }
}
