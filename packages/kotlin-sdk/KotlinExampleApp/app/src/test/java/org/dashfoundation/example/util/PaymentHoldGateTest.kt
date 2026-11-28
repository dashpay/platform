package org.dashfoundation.example.util

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.example.ui.credits.topUpPermittedAfter
import org.dashfoundation.example.ui.dashpay.invitationCreatePermittedAfter
import org.dashfoundation.example.ui.identity.dpnsTradePermittedAfter
import org.dashfoundation.example.ui.wallet.sendPermittedAfter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Every screen that starts a payment refuses a fresh one while the failure it
 * kept holds its slot (MAYBE_SENT, LOCK_HELD, no class) and allows it after a
 * NOT_SENT one, through [PaymentFailure.permitsFreshPayment].
 */
class PaymentHoldGateTest {

    private val heldRaw = ByteArray(32) { 7 } + byteArrayOf(1, 0, 0, 0)

    private fun walletError(delivery: Int, retryable: Boolean = false, lock: ByteArray? = null) =
        DashSdkError.fromNative(
            DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 6, "detail", 0, 0, delivery, retryable, lock),
        )

    private val gates: Map<String, (PaymentFailure?) -> Boolean> = mapOf(
        "send" to ::sendPermittedAfter,
        "createInvitation" to ::invitationCreatePermittedAfter,
        "dpnsTrade" to ::dpnsTradePermittedAfter,
        "topUp" to ::topUpPermittedAfter,
    )

    private val cases: List<Pair<PaymentFailure, Boolean>> = listOf(
        PaymentFailure.of(walletError(1, retryable = true)) to true,
        PaymentFailure.of(walletError(1)) to true,
        PaymentFailure.of(walletError(2)) to false,
        PaymentFailure.of(walletError(3, lock = heldRaw)) to false,
        PaymentFailure.of(walletError(3)) to false,
        PaymentFailure.of(walletError(4)) to false,
        PaymentFailure.of(IllegalStateException("unclassified")) to false,
        PaymentFailure.of(walletError(3, lock = heldRaw), invitationVoucher = true) to false,
    )

    @Test
    fun shouldRefuseAFreshPaymentWhileTheKeptFailureHoldsItsSlot() {
        assertTrue(PaymentFailure.permitsFreshPayment(null))
        cases.forEach { (failure, permitted) ->
            assertEquals(failure.toString(), permitted, PaymentFailure.permitsFreshPayment(failure))
        }
    }

    @Test
    fun shouldGateEveryPaymentScreenTheSameWay() {
        gates.forEach { (site, gate) ->
            assertTrue("$site: no failure kept", gate(null))
            cases.forEach { (failure, permitted) ->
                assertEquals("$site: $failure", permitted, gate(failure))
            }
        }
    }

    /**
     * The hold clears only on a later NOT_SENT failure (or success / a
     * confirmed release, which the screens do by setting null): a non-SDK
     * throwable, such as the screen's own input check, keeps it.
     */
    @Test
    fun shouldReplaceAHoldingFailureOnlyWithALaterPaymentFailure() {
        val maybeSent = PaymentFailure.keptAfter(null, walletError(2))
        assertFalse(PaymentFailure.permitsFreshPayment(maybeSent))

        val afterInputCheck = PaymentFailure.keptAfter(maybeSent, IllegalArgumentException("Enter a price"))
        assertEquals(maybeSent, afterInputCheck)
        assertFalse(PaymentFailure.permitsFreshPayment(afterInputCheck))

        val afterNotSent = PaymentFailure.keptAfter(afterInputCheck, walletError(1))
        assertTrue(PaymentFailure.permitsFreshPayment(afterNotSent))
    }

    /** A held invitation voucher is kept as a voucher and tells the user to reclaim it. */
    @Test
    fun shouldKeepTheVoucherTextOfAKeptInvitationFailure() {
        val voucher = PaymentFailure.keptAfter(null, walletError(3, lock = heldRaw), invitationVoucher = true)!!
        assertTrue(voucher.invitationVoucher)
        assertTrue(voucher.message, voucher.message.contains("Sent invitations"))
        assertFalse(invitationCreatePermittedAfter(voucher))
        assertEquals(voucher, PaymentFailure.keptAfter(voucher, IllegalStateException("x"), invitationVoucher = true))
    }
}
