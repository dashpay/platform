package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Regression test for the DashPay payment dispose-mid-send double-send guard
 * ([performDashPaySend]'s `withContext(NonCancellable)` block).
 *
 * The hazard: the JNI broadcast is uncancellable, so once `sendPayment`
 * returns the coin has left the wallet. If the coroutine is cancelled between
 * the broadcast and the durability bookkeeping (`onSent` →
 * `refreshDashPayPayments`), the app never records the payment and a retry
 * double-sends. The fake [PaymentSender] records the broadcast BEFORE
 * suspending on a gate, modelling exactly "broadcast completed, bookkeeping
 * still pending" — then the test cancels the hosting Job before releasing it.
 *
 * Red→green: remove the `withContext(NonCancellable)` wrapper and
 * [disposeMidSend_stillFiresOnSent] fails — the cancellation observed on resume
 * skips `onSent`, so its count is 0 instead of 1.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class PerformDashPaySendDoubleSendGuardTest {

    @Test
    fun disposeMidSend_stillFiresOnSent() = runTest {
        val broadcastEntered = CompletableDeferred<Unit>()
        val releaseBroadcast = CompletableDeferred<Unit>()
        var sendCount = 0
        var onSentCount = 0

        val sender = PaymentSender {
            sendCount++
            broadcastEntered.complete(Unit) // record the broadcast BEFORE the gate
            releaseBroadcast.await()        // suspension window: bookkeeping still pending
            ByteArray(32) { 0x11 }          // txid — the coin has left the wallet
        }

        val job = launch {
            performDashPaySend(
                sender = sender,
                onSuccessTxid = {},
                onError = {},
                onSent = { onSentCount++ },
                settle = {},
                onClose = {},
                onSendingDone = {},
            )
        }

        broadcastEntered.await() // broadcast has happened
        job.cancel()             // dispose the sheet mid-send
        releaseBroadcast.complete(Unit)
        job.join()

        assertEquals("broadcast must run exactly once", 1, sendCount)
        assertEquals(
            "onSent (durability refresh) must still fire despite dispose-mid-send",
            1,
            onSentCount,
        )
    }

    @Test
    fun happyPath_firesOnSentAndCloses() = runTest {
        var onSentCount = 0
        var closed = false
        var doneCount = 0

        performDashPaySend(
            sender = { ByteArray(32) { 0x22 } },
            onSuccessTxid = {},
            onError = {},
            onSent = { onSentCount++ },
            settle = {},
            onClose = { closed = true },
            onSendingDone = { doneCount++ },
        )

        assertEquals(1, onSentCount)
        assertTrue("sheet auto-closes after a successful send", closed)
        assertEquals("sending state is cleared exactly once", 1, doneCount)
    }
}
