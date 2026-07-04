package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for identity credit movements — transfer, withdraw, and
 * address-input top-up — mirrors `rs-unified-sdk-jni/src/credits.rs`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.credits.IdentityCredits]. Handles are raw
 * Rust pointers as [Long] (wallet handle, `SignerHandle`); passing a stale
 * or foreign value is undefined behavior, so ownership is confined to the
 * SDK wrapper classes. Errors throw [DashSDKException].
 *
 * Each function is a thin marshaler over a SINGLE `platform-wallet-ffi`
 * entry point — no orchestration crosses this boundary (see
 * `packages/kotlin-sdk/CLAUDE.md`).
 */
internal object CreditsNative {

    /**
     * Transfer [amount] credits from [fromIdentityId] to [toIdentityId]
     * (both 32 bytes), signed via [signerHandle] (the identity's
     * transfer-purpose key). No balance is returned — the sender's row
     * refreshes through the persistence changeset.
     */
    external fun transferCredits(
        walletHandle: Long,
        fromIdentityId: ByteArray,
        toIdentityId: ByteArray,
        amount: Long,
        signerHandle: Long,
    )

    /**
     * Withdraw [amount] credits from [identityId] (32 bytes) to the
     * Base58Check Dash address [toAddress], signed via [signerHandle].
     * Rust validates the address against the wallet's network. The L1
     * payout is pooled + broadcast asynchronously (no txid returned).
     */
    external fun withdrawCredits(
        walletHandle: Long,
        identityId: ByteArray,
        amount: Long,
        toAddress: String,
        signerHandle: Long,
    )

    /**
     * Top up [identityId] (32 bytes) from Platform-address inputs, signed
     * per-address via [signerHandle] (the platform-address signer).
     * Returns the post-transition credit balance.
     *
     * @param inputsBlob big-endian: `u32 rowCount` then per row
     *   `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits`.
     *   Built by [org.dashfoundation.dashsdk.credits.FundingInput.encode].
     */
    external fun topUpFromAddresses(
        walletHandle: Long,
        identityId: ByteArray,
        inputsBlob: ByteArray,
        signerHandle: Long,
    ): Long
}
