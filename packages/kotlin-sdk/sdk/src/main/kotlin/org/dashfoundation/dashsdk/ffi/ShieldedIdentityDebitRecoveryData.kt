package org.dashfoundation.dashsdk.ffi

/** JNI transport only. Unsigned integers retain their native bit patterns. */
internal data class ShieldedIdentityDebitRecoveryData(
    val accountIndex: Int,
    val activityId: ByteArray,
    val identityId: ByteArray?,
    val hasNonce: Boolean,
    val nonce: Long,
    val hasAmount: Boolean,
    val amount: Long,
    val status: Int,
)
