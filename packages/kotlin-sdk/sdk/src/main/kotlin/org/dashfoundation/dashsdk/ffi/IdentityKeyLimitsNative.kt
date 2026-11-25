package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for raising one identity key's limits from a reviewed preview: mirrors
 * `rs-unified-sdk-jni/src/identity_key_limits.rs`, one export per `platform-wallet-ffi`
 * function (the calls Swift's `ManagedPlatformWallet` makes for `UpdateKeyLimitsView`).
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.identity.IdentityUpdates]
 * (`previewKeyLimitsUpdate`, `updateKeyLimitsAsReviewed`, `refreshIdentity`). Key ids,
 * credits and milliseconds cross as their unsigned bit patterns. Errors throw
 * [DashSDKException] with the platform-wallet code shifted by `PLATFORM_WALLET_CODE_OFFSET`;
 * a value the C types cannot carry throws code 1 before any C call. Every call blocks for
 * Rust's work; call it off the main thread.
 */
internal object IdentityKeyLimitsNative {

    /**
     * `platform_wallet_preview_identity_key_limits_update`: what raising key [keyId] of
     * [identityId] would do, signing nothing. [addBudget] counts only when [hasAddBudget],
     * [expiresAt] only when [hasExpiresAt]. Returns six slots, decoded by
     * [org.dashfoundation.dashsdk.identity.IdentityKeyLimitsUpdatePreview.fromSlots]:
     * `[signingKeyId, flags, currentTotalBudget, currentExpiresAt, newTotalBudget,
     * newExpiresAt]`.
     */
    external fun previewKeyLimitsUpdate(
        walletHandle: Long,
        identityId: ByteArray,
        keyId: Int,
        hasAddBudget: Boolean,
        addBudget: Long,
        hasExpiresAt: Boolean,
        expiresAt: Long,
        signerHandle: Long,
    ): LongArray

    /**
     * `platform_wallet_update_identity_key_limits_as_reviewed_with_signer`: sign exactly the
     * limits [reviewed] (the slots [previewKeyLimitsUpdate] returned for key [keyId] of
     * [identityId]) shows, or nothing.
     */
    external fun updateKeyLimitsAsReviewed(
        walletHandle: Long,
        identityId: ByteArray,
        keyId: Int,
        reviewed: LongArray,
        signerHandle: Long,
    )

    /** `platform_wallet_refresh_identity`: fold a proved fetch of [identityId] into the wallet. */
    external fun refreshIdentity(walletHandle: Long, identityId: ByteArray)
}
