package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * Pins the three properties the manager teardown depends on (each was, or
 * would have been, a real bug):
 *
 * 1. reject-after-close — a borrow starting after [TeardownGate.closeAndAwait]
 *    began must fail fast instead of touching freed native boxes.
 * 2. wait-for-in-flight — closeAndAwait must not return while a borrow is
 *    still running (the use-after-free window).
 * 3. decrement-always-runs — cancelling a borrower must still release its
 *    slot, or closeAndAwait hangs teardown forever.
 *
 * Real dispatchers ([Dispatchers.IO] inside `withOp`) rather than a
 * TestDispatcher: the gate's contract is about real cross-thread joins.
 */
class TeardownGateTest {

    @Test
    fun opRunsAndReturnsValue() = runBlocking {
        val gate = TeardownGate()
        assertEquals(42, gate.withOp { 42 })
    }

    @Test
    fun rejectsNewOpsAfterClose() = runBlocking {
        val gate = TeardownGate()
        gate.closeAndAwait()
        val rejected = runCatching { gate.withOp { 1 } }
        assertTrue(rejected.exceptionOrNull() is IllegalStateException)
    }

    @Test
    fun closeAwaitsInFlightOp() = runBlocking {
        val gate = TeardownGate()
        val opEntered = CompletableDeferred<Unit>()
        val opRelease = CompletableDeferred<Unit>()
        var opFinished = false

        val op = async(Dispatchers.Default) {
            gate.withOp {
                opEntered.complete(Unit)
                opRelease.await()
                opFinished = true
            }
        }
        opEntered.await()

        var closed = false
        val closer = launch(Dispatchers.Default) {
            gate.closeAndAwait()
            closed = true
        }
        // The op is still holding the gate: close must not have completed.
        // (Give the closer a real chance to run wrongly-early.)
        kotlinx.coroutines.delay(100)
        assertFalse("closeAndAwait returned while an op was in flight", closed)

        opRelease.complete(Unit)
        withTimeout(5_000) {
            op.await()
            closer.join()
        }
        assertTrue(opFinished)
        assertTrue(closed)
    }

    @Test
    fun cancelledOpStillReleasesItsSlot() = runBlocking {
        val gate = TeardownGate()
        val opEntered = CompletableDeferred<Unit>()

        val op = CoroutineScope(Dispatchers.Default).launch {
            gate.withOp {
                opEntered.complete(Unit)
                // Park until cancelled.
                CompletableDeferred<Unit>().await()
            }
        }
        opEntered.await()
        op.cancelAndJoin()

        // The cancelled op's finally-decrement must have run (it is
        // NonCancellable) — otherwise this hangs and times out.
        withTimeout(5_000) { gate.closeAndAwait() }
    }

    @Test
    fun cancelledOpScrubsACompletedSecretResultBeforeDiscardingIt() = runBlocking {
        val gate = TeardownGate()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val secret = ByteArray(32) { 0x5A }

        val op = launch(Dispatchers.Default) {
            gate.opWithCleanupOnCancellation(
                cleanup = { discarded: ByteArray -> discarded.fill(0) },
            ) {
                entered.countDown()
                // A blocking native call does not observe coroutine cancellation.
                // Return the secret only after the caller has cancelled, forcing
                // withContext's prompt-cancellation result-discard path.
                release.await()
                secret
            }
        }
        assertTrue("blocking operation never started", entered.await(5, TimeUnit.SECONDS))
        op.cancel()
        release.countDown()
        withTimeout(5_000) { op.join() }

        assertArrayEquals(ByteArray(32), secret)
    }
}
