package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for buying a token from the shielded credit pool (the
 * Type 28 `TokenPurchaseFromShieldedPool` transition), mirrors
 * `rs-unified-sdk-jni/src/shielded_token_purchase.rs`. Each function is one
 * call into `platform-wallet-ffi`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedTokenPurchaseFromPool]
 * and
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedTokenPurchaseAvailability].
 * Available only when the native library was built with shielded support
 * ([org.dashfoundation.dashsdk.Sdk.hasShielded]); calling these on a
 * non-shielded build throws [UnsatisfiedLinkError].
 */
internal object ShieldedTokenPurchaseNative {

    /**
     * Buy [tokenCount] tokens of the token at [tokenPosition] of
     * [dataContractId] from the notes of [account] (bridges
     * `platform_wallet_manager_shielded_token_purchase_from_pool`).
     * [tokenCount] and [totalAgreedPrice] are unsigned 64-bit values passed
     * as their two's-complement `Long`. [resolverHandle] supplies the
     * transient spend authority as for the other note-spending calls.
     *
     * Returning means SUBMITTED, not confirmed: Platform accepted the
     * purchase, but its result proof does not show that it executed, so it
     * stays pending until a shielded sync finds it. Throws as the other
     * note-spending calls do; the spend-unconfirmed error means the purchase
     * may have reached a node and must not be retried.
     */
    @Suppress("LongParameterList")
    external fun purchaseFromPool(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        dataContractId: ByteArray,
        tokenPosition: Int,
        tokenCount: Long,
        totalAgreedPrice: Long,
    )

    /**
     * Whether the token at [tokenPosition] of [dataContractId] can be bought
     * from the shielded pool (bridges
     * `platform_wallet_manager_shielded_token_purchase_availability`):
     * 0 = available, 1 = the token has no shielded pool, 2 = its
     * configuration could not be read. Throws when the question itself could
     * not be asked.
     */
    external fun availability(
        managerHandle: Long,
        dataContractId: ByteArray,
        tokenPosition: Int,
    ): Int
}
