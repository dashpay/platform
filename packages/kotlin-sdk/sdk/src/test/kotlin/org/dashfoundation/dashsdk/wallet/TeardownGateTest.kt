package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CancellationException
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

    @Test
    fun cancelledCallerStillReceivesACompletedResult() = runBlocking {
        for (gate in listOf(TeardownGate(), null)) {
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            val received = CompletableDeferred<Int>()

            val op = launch(Dispatchers.Default) {
                val result = gate.opReturningCompletedResult {
                    entered.countDown()
                    // A blocking native call does not observe cancellation; it
                    // completes only after the caller has been cancelled.
                    release.await()
                    42
                }
                // No suspension between the call and here: the cancelled
                // caller still sees the completed result.
                received.complete(result)
            }
            assertTrue("blocking operation never started", entered.await(5, TimeUnit.SECONDS))
            op.cancel()
            release.countDown()
            withTimeout(5_000) { op.join() }

            assertTrue(op.isCancelled)
            assertTrue("completed result was dropped", received.isCompleted)
            assertEquals(42, received.await())
            // The gate's slot was released on that path too.
            gate?.let { withTimeout(5_000) { it.closeAndAwait() } }
        }
    }

    @Test
    fun cancellationBeforeTheBlockCompletesStillThrows() = runBlocking {
        val gate = TeardownGate()
        val entered = CompletableDeferred<Unit>()
        val outcome = CompletableDeferred<Throwable?>()

        val op = launch(Dispatchers.Default) {
            outcome.complete(
                runCatching {
                    gate.opReturningCompletedResult {
                        entered.complete(Unit)
                        // Park until cancelled: the block never completes.
                        CompletableDeferred<Int>().await()
                    }
                }.exceptionOrNull(),
            )
        }
        entered.await()
        op.cancelAndJoin()

        assertTrue(outcome.isCompleted)
        assertTrue(outcome.await() is CancellationException)
        withTimeout(5_000) { gate.closeAndAwait() }
    }

    @Test
    fun aFailedBlockThrowsItsOwnError() = runBlocking {
        val gate = TeardownGate()
        val error = runCatching {
            gate.opReturningCompletedResult<Int> { throw IllegalStateException("native failed") }
        }.exceptionOrNull()
        assertTrue(error is IllegalStateException)
        assertEquals("native failed", error?.message)
    }
}
