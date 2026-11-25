package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** Codes 63-66 from the DashPay invitation claim (`rs-platform-wallet-ffi/src/error.rs`). */
@RunWith(RobolectricTestRunner::class)
class InvitationClaimErrorTest {
    @Test
    fun shouldPreserveInvitationClaimOutcomes() {
        val id = "8eTDkBhpQjHeqgbVeriwLeZr1tCa6yBGw76SckvD1cwc"
        // Each code with the class Rust gives it (`payment_delivery`, `is_retryable`).
        fun map(code: Int, message: String, delivery: Int = NOT_SENT, retryable: Boolean = false) =
            DashSdkError.fromNative(
                DashSDKException(
                    DashSdkError.PLATFORM_WALLET_CODE_OFFSET + code, message, 0, 0, delivery, retryable,
                ),
            )

        val taken = map(63, "{\"identityId\":\"$id\"}")
        assertTrue(taken is DashSdkError.PlatformWallet.InvitationKeysAlreadyRegistered)
        assertEquals(id, (taken as DashSdkError.PlatformWallet.InvitationKeysAlreadyRegistered).identityId)
        assertFalse(taken.isRetryable)

        val takenByUnknown = map(63, "{\"identityId\":null}")
        assertTrue(takenByUnknown is DashSdkError.PlatformWallet.InvitationKeysAlreadyRegistered)
        assertNull((takenByUnknown as DashSdkError.PlatformWallet.InvitationKeysAlreadyRegistered).identityId)

        val ours = map(64, "{\"identityId\":\"$id\"}", MAYBE_SENT)
        assertTrue(ours is DashSdkError.PlatformWallet.InvitationClaimedByThisWallet)
        assertEquals(id, (ours as DashSdkError.PlatformWallet.InvitationClaimedByThisWallet).identityId)

        // 64 always names its identity; a payload without one stays generic.
        assertTrue(map(64, "{\"identityId\":null}") is DashSdkError.PlatformWallet.Generic)
        assertTrue(map(64, "{}") is DashSdkError.PlatformWallet.Generic)
        assertTrue(map(63, "not json") is DashSdkError.PlatformWallet.Generic)

        val notReady = map(65, "quorum not verified", NOT_SENT, true)
        assertTrue(notReady is DashSdkError.PlatformWallet.InvitationNotReady)
        assertTrue(notReady.isRetryable)
        // Rust's class, not the code's: the claim may already have been broadcast.
        val notChainLocked = map(66, "retry later", MAYBE_SENT)
        assertTrue(notChainLocked is DashSdkError.PlatformWallet.AssetLockNotChainLocked)
        assertFalse(notChainLocked.isRetryable)
    }

    private companion object {
        const val NOT_SENT = 1
        const val MAYBE_SENT = 2
    }
}
