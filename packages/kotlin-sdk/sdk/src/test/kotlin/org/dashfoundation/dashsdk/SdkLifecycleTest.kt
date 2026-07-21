package org.dashfoundation.dashsdk

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.wallet.op
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Behavioral regression tests for the [Sdk] teardown lease — the wiring
 * the source-scanning `GateCoverageLintTest` cannot prove:
 *
 * 1. `closeSuspending()` waits for an in-flight leased query.
 * 2. A query starting after close began fails fast (before it could read
 *    the native handle).
 * 3. Cancelling the closer cannot strand cleanup (`closeSuspending` is
 *    NonCancellable once entered).
 * 4. The public `AutoCloseable.close()` follows the same fence.
 *
 * Uses [Sdk.forLifecycleTest] (handle 0 — the Cleaner skips the native
 * destroy), so the semantics run without the native library.
 */
class SdkLifecycleTest {

    @Test
    fun closeSuspendingAwaitsInFlightLease() = runBlocking {
        val sdk = Sdk.forLifecycleTest()
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        var leaseFinished = false

        val lease = async(Dispatchers.Default) {
            sdk.queryGate.op {
                entered.complete(Unit)
                release.await()
                leaseFinished = true
            }
        }
        entered.await()

        var closed = false
        val closer = launch(Dispatchers.Default) {
            sdk.closeSuspending()
            closed = true
        }
        delay(100)
        assertFalse("closeSuspending returned while a lease was active", closed)

        release.complete(Unit)
        withTimeout(5_000) {
            lease.await()
            closer.join()
        }
        assertTrue(leaseFinished)
        assertTrue(closed)
    }

    @Test
    fun queriesAfterCloseFailFast() = runBlocking {
        val sdk = Sdk.forLifecycleTest()
        sdk.closeSuspending()
        val rejected = runCatching { sdk.queryGate.op { 1 } }
        assertTrue(rejected.exceptionOrNull() is IllegalStateException)
    }

    @Test
    fun cancellingTheCloserDoesNotStrandCleanup() = runBlocking {
        val cleanupRuns = java.util.concurrent.atomic.AtomicInteger(0)
        val sdk = Sdk.forLifecycleTest(onCleanup = { cleanupRuns.incrementAndGet() })
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()

        val lease = CoroutineScope(Dispatchers.Default).launch {
            sdk.queryGate.op {
                entered.complete(Unit)
                release.await()
            }
        }
        entered.await()

        val closer = CoroutineScope(Dispatchers.Default).launch {
            sdk.closeSuspending()
        }
        delay(100)
        // Cancel the closer while it is awaiting the lease: NonCancellable
        // means the close still runs to completion once the lease ends.
        closer.cancel()
        release.complete(Unit)
        withTimeout(5_000) {
            lease.join()
            closer.join()
        }
        // Close completed despite the cancellation: cleanup ran exactly
        // once (the regression was cancellation skipping cleanable.clean())
        // and new ops are rejected.
        org.junit.Assert.assertEquals(1, cleanupRuns.get())
        val rejected = runCatching { sdk.queryGate.op { 1 } }
        assertTrue(rejected.exceptionOrNull() is IllegalStateException)
    }

    @Test
    fun autoCloseableCloseFollowsTheFence() = runBlocking {
        val sdk = Sdk.forLifecycleTest()
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()

        val lease = async(Dispatchers.Default) {
            sdk.queryGate.op {
                entered.complete(Unit)
                release.await()
                42
            }
        }
        entered.await()

        var closed = false
        val closer = launch(Dispatchers.Default) {
            @Suppress("BlockingMethodInNonBlockingContext")
            sdk.close() // blocking path — delegates through closeSuspending
            closed = true
        }
        delay(100)
        assertFalse("close() returned while a lease was active", closed)

        release.complete(Unit)
        withTimeout(5_000) {
            assertTrue(lease.await() == 42)
            closer.join()
        }
        assertTrue(closed)
        assertTrue(sdk.isClosed)
    }
}
