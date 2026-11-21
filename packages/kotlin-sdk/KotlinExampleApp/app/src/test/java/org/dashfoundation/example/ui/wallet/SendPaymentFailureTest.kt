package org.dashfoundation.example.ui.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.example.util.PaymentFailure
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The Send form's hold after a failed payment (← Swift
 * `test_paymentFailureHoldsTheFormUnlessNothingWasSent`): a stored failure
 * that may have left the wallet, holds a lock, or has no class refuses
 * another submission; any NOT_SENT one does not.
 */
class SendPaymentFailureTest {

    private val heldRaw = ByteArray(32) { 7 } + byteArrayOf(1, 0, 0, 0)

    private fun walletError(delivery: Int, retryable: Boolean, held: ByteArray? = null) =
        DashSdkError.fromNative(
            DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 6, "boom", 0, 0, delivery, retryable, held),
        )

    @Test
    fun `submission is refused while a stored failure holds the form`() {
        assertTrue("no failure stored", sendPermittedAfter(null))
        listOf<Pair<Throwable, Boolean>>(
            walletError(delivery = 1, retryable = true) to true,
            walletError(delivery = 1, retryable = false) to true,
            walletError(delivery = 3, retryable = false, held = heldRaw) to false,
            walletError(delivery = 2, retryable = false) to false,
            RuntimeException("unclassified") to false,
        ).forEach { (error, permitted) ->
            assertEquals(error.toString(), permitted, sendPermittedAfter(PaymentFailure.of(error)))
        }
    }
}
