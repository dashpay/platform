package org.dashfoundation.dashsdk.ffi

/**
 * JNI result for identity registration resumed from an existing asset lock.
 * Constructor descriptor: `([BJ)V`.
 *
 * Native must destroy `managedIdentityHandle` if constructing or returning
 * this object fails. Kotlin adopts the handle immediately and destroys it
 * after validating/copying [identityId].
 */
class IdentityRegistrationNativeResult(
    @JvmField val identityId: ByteArray,
    @JvmField val managedIdentityHandle: Long,
)

/**
 * One copied `TrackedAssetLockEntryFFI`; constructor descriptor
 * `([BIIBIZI[BZ)V`.
 *
 * [identityRecipient] is the 32-byte identity an unbound identity top-up pays
 * (`has_identity_recipient` / `identity_recipient`), null when the row records
 * none. [identityRecipientUnassigned] is `identity_recipient_unassigned`: an
 * unbound top-up recording no identity, in a live status (Built through
 * ChainLocked), which no resume pays until one is assigned. False for a
 * RecoveredFromChain lock, whose consumption is unknown.
 */
class TrackedAssetLockNativeData(
    @JvmField val outpointTxid: ByteArray,
    @JvmField val outpointVout: Int,
    @JvmField val fundingType: Int,
    @JvmField val status: Byte,
    @JvmField val registrationIndex: Int,
    @JvmField val instantLockPresent: Boolean,
    @JvmField val chainLockHeight: Int,
    @JvmField val identityRecipient: ByteArray?,
    @JvmField val identityRecipientUnassigned: Boolean,
)

/**
 * JNI result of `CreditsNative.resumeTopUpToRecordedRecipient`; constructor
 * descriptor `([BJ)V`: the 32-byte identity the resume paid and its proved
 * new credit balance.
 */
class IdentityTopUpNativeResult(
    @JvmField val identityId: ByteArray,
    @JvmField val newBalance: Long,
)

/**
 * Single JNI-owned snapshot result; constructor descriptor
 * `([Lorg/dashfoundation/dashsdk/ffi/TrackedAssetLockNativeData;)V`.
 *
 * The JNI implementation must call
 * `platform_wallet_tracked_asset_locks_free(entries, count)` on every path
 * after `platform_wallet_tracked_asset_locks_list`, including element/object
 * allocation failures. No native pointer escapes into Kotlin.
 */
class TrackedAssetLocksNativeResult(
    @JvmField val entries: Array<TrackedAssetLockNativeData>,
)
