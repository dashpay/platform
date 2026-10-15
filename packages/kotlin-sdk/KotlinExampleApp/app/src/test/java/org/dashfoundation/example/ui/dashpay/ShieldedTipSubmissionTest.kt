package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.ExperimentalCoroutinesApi
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ShieldedTipSubmissionTest {
    @Test
    fun dismissAndReopenCannotSubmitAgainWhileNativeSendRuns() = runTest {
        val submissions = ShieldedTipSubmissions(backgroundScope)
        val payment = submissions.forWallet(1, "wallet")
        val nativeFinished = CompletableDeferred<Unit>()
        var sends = 0
        payment.submit { sends++; nativeFinished.await() }
        assertTrue(payment.busy)
        // Even a second click before the first coroutine starts is ignored.
        payment.submit { sends++ }
        runCurrent()
        val sheetScope = CoroutineScope(coroutineContext + SupervisorJob())
        sheetScope.cancel()
        val reopened = submissions.forWallet(1, "wallet")
        assertSame(payment, reopened)
        reopened.submit { sends++ }
        assertEquals(1, sends)
        nativeFinished.complete(Unit)
        runCurrent()
        assertEquals(ShieldedTipSubmission.Status.Sent, reopened.status)
        assertFalse(reopened.busy)
        reopened.submit { sends++ }
        runCurrent()
        assertEquals(1, sends)
        reopened.startNewTip()
        reopened.submit { sends++ }
        runCurrent()
        assertEquals(2, sends)
    }

    @Test
    fun uncertainOutcomeStaysLockedAcrossReopeningAndNewTipAction() = runTest {
        val submissions = ShieldedTipSubmissions(backgroundScope)
        val payment = submissions.forWallet(1, "wallet")
        payment.submit { throw IllegalStateException("JNI outcome lost") }
        runCurrent()
        val reopened = submissions.forWallet(1, "wallet")
        assertEquals(ShieldedTipSubmission.Status.Uncertain, reopened.status)
        reopened.startNewTip()
        var retried = false
        reopened.submit { retried = true }
        runCurrent()
        assertFalse(retried)
        assertTrue(reopened.submitted)
        assertFalse(reopened.busy)
    }

    @Test
    fun definitivePreflightFailureAllowsFreshReview() = runTest {
        val payment = ShieldedTipSubmission(backgroundScope)
        payment.submit { throw IllegalArgumentException("invalid amount") }
        runCurrent()
        assertEquals(ShieldedTipSubmission.Status.Ready, payment.status)
        assertEquals("invalid amount", payment.message)
        var retried = false
        payment.submit { retried = true }
        runCurrent()
        assertTrue(retried)
    }

    @Test
    fun walletAndNetworkGuardsAreIndependent() = runTest {
        val submissions = ShieldedTipSubmissions(backgroundScope)
        submissions.forWallet(1, "a").submit { }
        assertFalse(submissions.forWallet(2, "a").submitted)
        assertFalse(submissions.forWallet(1, "b").submitted)
    }
}
