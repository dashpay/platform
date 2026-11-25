package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for a wallet's tokens in token shielded pools: watching a
 * pool, listing the balances the local scan found, and spending those notes
 * with the Type 26 shielded token transfer and the Type 27 unshield to an
 * identity. Mirrors `rs-unified-sdk-jni/src/shielded_token_pools.rs`; each
 * function is one call into `platform-wallet-ffi`.
 *
 * Internal: the public API is the `PlatformWalletManager.shieldedTokenPool*`
 * and `shieldedToken{Transfer,Unshield,SpendPreview}` wrappers. Available
 * only when the native library was built with shielded support
 * ([org.dashfoundation.dashsdk.Sdk.hasShielded]); calling these on a
 * non-shielded build throws [UnsatisfiedLinkError].
 */
internal object ShieldedTokenPoolNative {

    /**
     * Watch the shielded pool of the token at [tokenPosition] of
     * [dataContractId] for [walletId] (bridges
     * `platform_wallet_manager_shielded_token_pool_watch`). Returns the
     * token's 32-byte id. Throws Rust's refusal for a token without a
     * shielded pool or whose configuration could not be read.
     */
    external fun watch(
        managerHandle: Long,
        walletId: ByteArray,
        dataContractId: ByteArray,
        tokenPosition: Int,
    ): ByteArray

    /**
     * Stop watching [tokenId]'s shielded pool for [walletId], forgetting the
     * wallet's notes there (bridges
     * `platform_wallet_manager_shielded_token_pool_unwatch`).
     */
    external fun unwatch(managerHandle: Long, walletId: ByteArray, tokenId: ByteArray)

    /**
     * One row per watched pool and bound account, from the local scan alone
     * (bridges `platform_wallet_manager_shielded_token_pool_balances`; the
     * native array is freed before this returns).
     */
    external fun balances(
        managerHandle: Long,
        walletId: ByteArray,
    ): Array<ShieldedTokenPoolBalanceData>

    /**
     * Type 26 shielded token transfer (bridges
     * `platform_wallet_manager_shielded_token_transfer`). [amount] is an
     * unsigned 64-bit value in the token's smallest unit passed as its
     * two's-complement `Long`. Returning means SUBMITTED, not confirmed.
     */
    @Suppress("LongParameterList")
    external fun transfer(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        dataContractId: ByteArray,
        tokenPosition: Int,
        recipientRaw43: ByteArray,
        amount: Long,
    )

    /**
     * Type 27 unshield to an identity's token balance (bridges
     * `platform_wallet_manager_shielded_token_unshield`). Submitted, never
     * confirmed, as for [transfer].
     */
    @Suppress("LongParameterList")
    external fun unshield(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        dataContractId: ByteArray,
        tokenPosition: Int,
        recipientIdentityId: ByteArray,
        amount: Long,
    )

    /**
     * Local fee preview of a token pool spend (bridges
     * `platform_wallet_manager_shielded_token_spend_preview`): returns
     * `[fee, tokenNotes, feeNotes]`, the fee's bits as a `Long`. [kind] is
     * the FFI byte (0 transfer, 1 unshield). Throws Rust's refusal.
     */
    @Suppress("LongParameterList")
    external fun spendPreview(
        managerHandle: Long,
        walletId: ByteArray,
        account: Int,
        dataContractId: ByteArray,
        tokenPosition: Int,
        kind: Byte,
        amount: Long,
    ): LongArray
}

/**
 * JNI transport for one `ShieldedTokenPoolBalanceFFI` row. Unsigned values
 * keep their native bit patterns; each optional value is a flag and the
 * value, meaningful only when the flag is set. The constructor descriptor is
 * hand-written in `rs-unified-sdk-jni` (`BALANCE_ROW_CTOR`): change both
 * together.
 */
@Suppress("LongParameterList")
internal class ShieldedTokenPoolBalanceData(
    val tokenId: ByteArray,
    val dataContractId: ByteArray,
    val tokenContractPosition: Int,
    val accountIndex: Int,
    val balance: Long,
    val unspentNoteCount: Int,
    val reservedBalance: Long,
    val hasScannedNoteCount: Boolean,
    val scannedNoteCount: Long,
    val hasPoolNoteCount: Boolean,
    val poolNoteCount: Long,
    val hasScannedHeight: Boolean,
    val scannedHeight: Long,
    val lastError: String?,
)
