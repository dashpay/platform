package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PaymentRecoveryTest {

    private val held: ByteArray = ByteArray(32) { 7 } + byteArrayOf(2, 0, 0, 0)

    private fun native(code: Int, delivery: Int, retryable: Boolean, lock: ByteArray? = null) =
        DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + code,
                "detail",
                0,
                0,
                delivery,
                retryable,
                lock,
            ),
        )

    @Test
    fun shouldDecideTheRecoveryFromThePaymentClassAlone() {
        // The same code under each class: the class decides, not the type.
        assertEquals(PaymentRecovery.Retry, PaymentRecovery.of(native(6, 1, true)))
        assertEquals(PaymentRecovery.Wait, PaymentRecovery.of(native(6, 1, false)))
        assertEquals(PaymentRecovery.Reconcile, PaymentRecovery.of(native(6, 2, false)))
        assertEquals(
            PaymentRecovery.Resume(AssetLockBuiltOutPoint.fromRawBytes(held)),
            PaymentRecovery.of(native(6, 3, false, held)),
        )
        // Unknown class (4) and rs-sdk-ffi errors carry none.
        assertEquals(PaymentRecovery.Unconfirmed, PaymentRecovery.of(native(6, 4, false)))
        assertEquals(PaymentRecovery.Unconfirmed, PaymentRecovery.of(DashSdkError.Timeout("x")))
        assertEquals(PaymentRecovery.Unconfirmed, PaymentRecovery.of(IllegalStateException("x")))
    }

    @Test
    fun shouldShowTheClassSuggestionText() {
        assertEquals("Nothing was sent. You can try again.", PaymentRecovery.Retry.suggestion)
        assertEquals(
            "Nothing was sent. An earlier payment may still be settling; wait for it to finish.",
            PaymentRecovery.Wait.suggestion,
        )
        listOf(
            PaymentRecovery.Wait,
            PaymentRecovery.Reconcile,
            PaymentRecovery.Resume(AssetLockBuiltOutPoint.fromRawBytes(held)),
            PaymentRecovery.Unconfirmed,
        ).forEach {
            assertFalse("$it must not invite a retry", it.suggestion.lowercase().contains("try again"))
        }
    }

    /** A held invitation voucher is reclaimed; every other class reads as it always does. */
    @Test
    fun shouldTellAnInvitationVoucherToBeReclaimed() {
        val resume = PaymentRecovery.Resume(AssetLockBuiltOutPoint.fromRawBytes(held))
        assertTrue(resume.invitationVoucherSuggestion.contains("Reclaim the voucher"))
        assertFalse(resume.invitationVoucherSuggestion.contains("Resume"))
        listOf(PaymentRecovery.Retry, PaymentRecovery.Wait, PaymentRecovery.Reconcile, PaymentRecovery.Unconfirmed)
            .forEach { assertEquals(it.suggestion, it.invitationVoucherSuggestion) }
    }

    @Test
    fun shouldNotClaimAnOutcomeOrARetryWithoutTheClass() {
        val changed = DashSdkError.PlatformWallet.DocumentPriceChanged("d", 1u, 2u)
        assertFalse(changed.message!!.contains("Nothing was purchased"))

        assertEquals(
            "The wallet database is busy. Try again in a moment.",
            native(51, 1, true).userMessage,
        )
        assertEquals("The wallet database is busy.", native(51, 1, false).userMessage)

        assertFalse(DashSdkError.NetworkError("x").isRetryable)
        assertFalse(DashSdkError.Timeout("x").isRetryable)
        assertTrue(native(19, 1, true).isRetryable)
    }
}
