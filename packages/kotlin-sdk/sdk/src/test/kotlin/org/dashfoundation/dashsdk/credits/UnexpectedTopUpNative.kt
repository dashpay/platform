package org.dashfoundation.dashsdk.credits

import org.dashfoundation.dashsdk.ffi.IdentityTopUpNativeResult

/**
 * A [TopUpNative] that fails every call; tests override the calls they
 * expect, so any other JNI call fails the test.
 */
internal open class UnexpectedTopUpNative : TopUpNative {
    override fun topUpIdentityWithExistingAssetLock(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        identityId: ByteArray,
        coreSignerHandle: Long,
        consumeInvitationVoucher: Boolean,
    ): Long = throw AssertionError("unexpected topUpIdentityWithExistingAssetLock")

    override fun topUpExternalIdentityFromCore(
        walletHandle: Long,
        identityId: ByteArray,
        amountDuffs: Long,
        accountIndex: Int,
        coreSignerHandle: Long,
        outOutpoint: ByteArray,
    ): Long = throw AssertionError("unexpected topUpExternalIdentityFromCore")

    override fun resumeTopUpToRecordedRecipient(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        coreSignerHandle: Long,
    ): IdentityTopUpNativeResult = throw AssertionError("unexpected resumeTopUpToRecordedRecipient")

    override fun assignTopUpRecipient(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        identityId: ByteArray,
        acknowledgeUnknownConsumption: Boolean,
    ): Unit = throw AssertionError("unexpected assignTopUpRecipient")
}
