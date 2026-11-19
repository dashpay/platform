package org.dashfoundation.example.util

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.PaymentRecovery
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.example.ui.identity.dpnsTradeErrorMessage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [PaymentFailure] is the app's one mapping from the native payment class to
 * the text and action a failed payment screen shows (← Swift
 * `PaymentFailureTests`).
 */
class PaymentFailureTest {

    private val heldRaw = ByteArray(32) { 7 } + byteArrayOf(1, 0, 0, 0)
    private val held = PaymentFailure.outPoint(ByteArray(32) { 7 }, 1)
    private val other = PaymentFailure.outPoint(ByteArray(32) { 9 }, 0)

    private fun walletError(code: Int, delivery: Int, retryable: Boolean, lock: ByteArray? = null) =
        DashSdkError.fromNative(
            DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + code, "detail", 0, 0, delivery, retryable, lock),
        )

    @Test
    fun shouldMapEachClassToOneActionAndText() {
        val cases = listOf(
            walletError(6, 1, true) to (PaymentFailure.Action.Retry to PaymentRecovery.Retry),
            walletError(6, 1, false) to (PaymentFailure.Action.Wait to PaymentRecovery.Wait),
            walletError(6, 2, false) to (PaymentFailure.Action.Wait to PaymentRecovery.Reconcile),
            walletError(6, 3, false, heldRaw) to
                (PaymentFailure.Action.Resume(held) to PaymentRecovery.Resume(held)),
            walletError(6, 4, false) to (PaymentFailure.Action.Wait to PaymentRecovery.Unconfirmed),
        )
        cases.forEach { (error, expected) ->
            val failure = PaymentFailure.of(error)
            assertEquals(expected.first, failure.action)
            assertTrue(failure.message, failure.message.startsWith(expected.second.suggestion))
            assertTrue(failure.message, failure.message.endsWith("detail"))
            if (failure.action != PaymentFailure.Action.Retry) {
                assertFalse(failure.message, failure.message.lowercase().contains("try again"))
            }
        }
        assertTrue(PaymentFailure.of(walletError(6, 3, false, heldRaw)).message.contains(held.outPointHex))
    }

    @Test
    fun shouldReadTheClassFromTheErrorNotItsType() {
        // A typically-retryable type without a class waits; an "unconfirmed"
        // type the native wallet classed retryable offers the retry.
        assertEquals(PaymentFailure.Action.Wait, PaymentFailure.of(walletError(16, 4, false)).action)
        assertEquals(PaymentFailure.Action.Retry, PaymentFailure.of(walletError(20, 1, true)).action)
        assertEquals(PaymentFailure.Action.Wait, PaymentFailure.of(DashSdkError.Timeout("t")).action)
        // A screen's own input check is not an SDK failure.
        assertNull(PaymentFailure.ofPayment(IllegalArgumentException("Enter a price")))
    }

    /**
     * A failure that may have left (MAYBE_SENT), holds a lock (LOCK_HELD) or
     * has no class holds its slot; any NOT_SENT one, retryable or not,
     * releases it (← Swift `testShouldHoldTheSlotUnlessNothingWasSent`).
     */
    @Test
    fun shouldHoldTheSlotUnlessNothingWasSent() {
        listOf<Pair<Throwable, Boolean>>(
            walletError(6, 1, true) to false,
            walletError(6, 1, false) to false,
            walletError(6, 3, false, heldRaw) to true,
            walletError(6, 2, false) to true,
            walletError(6, 4, false) to true,
            RuntimeException("unclassified") to true,
        ).forEach { (error, holds) ->
            assertEquals(error.toString(), holds, PaymentFailure.of(error).holdsSlot)
            assertEquals(holds, PaymentFailure.of(error, invitationVoucher = true).holdsSlot)
        }
    }

    @Test
    fun shouldRestartOnlyAsTheClassAllows() {
        // Nothing left the wallet: any new request, retryable or not.
        listOf(walletError(6, 1, true), walletError(6, 1, false)).forEach {
            val released = PaymentFailure.of(it)
            assertTrue(released.permitsRestart(null))
            assertTrue(released.permitsRestart(other))
        }

        val resume = PaymentFailure.of(walletError(6, 3, false, heldRaw))
        assertFalse("never fund a new payment", resume.permitsRestart(null))
        assertFalse(resume.permitsRestart(other))
        assertTrue(resume.permitsRestart(held))

        listOf(walletError(6, 2, false), walletError(6, 4, false)).forEach {
            val wait = PaymentFailure.of(it)
            assertFalse(wait.permitsRestart(null))
            assertFalse(wait.permitsRestart(held))
        }
    }

    /**
     * A failed invitation create holding its voucher lock tells the user to
     * reclaim the voucher, not to resume the lock: the operation says it is a
     * voucher, the class is unchanged (← Swift
     * `testShouldReclaimAnInvitationVoucherInsteadOfResumingIt`).
     */
    @Test
    fun shouldReclaimAnInvitationVoucherInsteadOfResumingIt() {
        val voucher = PaymentFailure.of(walletError(6, 3, false, heldRaw), invitationVoucher = true)
        assertEquals(PaymentFailure.Action.Resume(held), voucher.action)
        assertFalse("never fund a new invitation", voucher.permitsRestart(null))
        assertTrue(voucher.message, voucher.message.startsWith(PaymentRecovery.Resume(held).invitationVoucherSuggestion))
        assertTrue(voucher.message, voucher.message.contains("Reclaim the voucher"))
        assertFalse(voucher.message, voucher.message.contains("Resume that lock"))
        assertTrue(voucher.message.contains(held.outPointHex))
        assertEquals("Funds held in the invitation voucher", voucher.title)

        listOf(walletError(6, 1, true), walletError(6, 1, false), walletError(6, 2, false), walletError(6, 4, false))
            .forEach {
                assertEquals(PaymentFailure.of(it).message, PaymentFailure.of(it, invitationVoucher = true).message)
            }
        assertNull(PaymentFailure.ofPayment(IllegalStateException("x"), invitationVoucher = true))
    }

    /** The DPNS marketplace's trade failures go through the helper. */
    @Test
    fun dpnsTradeFailuresFollowThePaymentClass() {
        val changed = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 38,
                """{"documentId":"d","expected":10,"actual":12}""",
                0, 0, 2, false, null,
            ),
        )
        val message = dpnsTradeErrorMessage(changed)
        assertTrue(message, message.contains(PaymentRecovery.Reconcile.suggestion))
        assertFalse(message, message.contains("Nothing was purchased"))
        assertEquals("Enter a price in credits", dpnsTradeErrorMessage(IllegalArgumentException("Enter a price in credits")))
    }
}
