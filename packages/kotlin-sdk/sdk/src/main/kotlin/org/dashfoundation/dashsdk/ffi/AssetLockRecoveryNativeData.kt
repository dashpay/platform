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

/** One copied `TrackedAssetLockEntryFFI`; constructor descriptor `([BIIBIZI)V`. */
class TrackedAssetLockNativeData(
    @JvmField val outpointTxid: ByteArray,
    @JvmField val outpointVout: Int,
    @JvmField val fundingType: Int,
    @JvmField val status: Byte,
    @JvmField val registrationIndex: Int,
    @JvmField val instantLockPresent: Boolean,
    @JvmField val chainLockHeight: Int,
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
