package org.dashfoundation.example.services.assetlock

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.dashfoundation.example.services.assetlock.AddressFundFromAssetLockController.Phase
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.example.util.PaymentFailure
import org.junit.Test

/**
 * State-machine tests for [AddressFundFromAssetLockCoordinator] +
 * [AddressFundFromAssetLockController] (← the phase model in
 * `AddressFundFromAssetLockCoordinator.swift` /
 * `AddressFundFromAssetLockController.swift`).
 *
 * A fake funding backend (a [CompletableDeferred] resolved by the test)
 * stands in for the FFI body, so phase reduction, single-flight, and the
 * 30s retention sweep are exercised deterministically. Time is driven off
 * the test scheduler — the coordinator/controller read a `now` clock bound
 * to `testScheduler.currentTime`, so `advanceTimeBy` moves both the
 * `delay`-based poll loop AND the wall-clock comparison together.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AddressFundFromAssetLockCoordinatorTest {

    /** A wallet error carrying the native class (delivery discriminant, retry, held lock). */
    private fun walletError(delivery: Int, retryable: Boolean, held: ByteArray? = null) =
        DashSdkError.fromNative(
            DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 6, "boom", 0, 0, delivery, retryable, held),
        )

    private val heldRaw = ByteArray(32) { 7 } + byteArrayOf(1, 0, 0, 0)


    private val walletA = ByteArray(32) { 0xAA.toByte() }
    private val walletB = ByteArray(32) { 0xBB.toByte() }
    private val recipient0 = ByteArray(20) { 0x10 }
    private val recipient1 = ByteArray(20) { 0x11 }

    private fun TestScope.coordinator(
        retentionMillis: Long = 30_000L,
        pollMillis: Long = 1_000L,
    ): AddressFundFromAssetLockCoordinator =
        AddressFundFromAssetLockCoordinator(
            scope = this,
            retentionMillis = retentionMillis,
            pollMillis = pollMillis,
            now = { testScheduler.currentTime },
        )

    @Test
    fun `happy path walks idle to inFlight to completed`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Long>()

        val controller = coordinator.startFunding(
            walletId = walletA,
            platformAccountIndex = 0,
            recipientHash = recipient0,
            recipientType = 0,
            body = { gate.await() },
        )
        assertEquals(Phase.InFlight, controller.phase.value)
        assertTrue(controller.phase.value.isActive)

        gate.complete(123_456L)
        advanceUntilIdle()

        assertEquals(Phase.Completed(123_456L), controller.phase.value)
        assertFalse(controller.phase.value.isActive)
    }

    @Test
    fun `failure body transitions to failed and is not active`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Long>()

        val controller = coordinator.startFunding(walletA, 0, recipient0, 0) { gate.await() }
        gate.completeExceptionally(IllegalStateException("asset lock rejected"))
        advanceUntilIdle()

        val phase = controller.phase.value
        assertTrue(phase is Phase.Failed)
        assertEquals("asset lock rejected", (phase as Phase.Failed).failure.detail)
        assertEquals(PaymentFailure.Action.Wait, phase.failure.action)
        assertFalse(phase.isActive)
    }

    @Test
    fun `single-flight returns the same controller for an in-flight slot`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Long>()
        var invocations = 0

        val first = coordinator.startFunding(walletA, 0, recipient0, 0) { invocations++; gate.await() }
        val second = coordinator.startFunding(walletA, 0, recipient0, 0) { invocations++; gate.await() }

        assertSame(first, second)
        assertEquals(1, coordinator.controllers.value.size)
        runCurrent()
        assertEquals(1, invocations)

        gate.complete(1L)
        advanceUntilIdle()
    }

    @Test
    fun `distinct slots on the same wallet get distinct controllers`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Long>()

        val a0 = coordinator.startFunding(walletA, 0, recipient0, 0) { gate.await() }
        val a0acct1 = coordinator.startFunding(walletA, 1, recipient0, 0) { gate.await() }
        val a1 = coordinator.startFunding(walletA, 0, recipient1, 0) { gate.await() }

        assertEquals(3, coordinator.controllers.value.size)
        assertTrue(a0 !== a0acct1 && a0 !== a1 && a0acct1 !== a1)
        assertTrue(coordinator.hasInFlightFundings)

        gate.complete(1L)
        advanceUntilIdle()
    }

    @Test
    fun `a failure that may have left or holds a lock runs no fresh funding`() = runTest {
        val coordinator = coordinator()
        var invocations = 0
        val maybe = coordinator.startFunding(walletA, 0, recipient0, 0) {
            invocations++; throw walletError(delivery = 2, retryable = false)
        }
        advanceUntilIdle()
        coordinator.startFunding(walletA, 0, recipient0, 0) { invocations++; 1L }
        advanceUntilIdle()
        assertEquals(1, invocations)
        assertTrue(maybe.phase.value is Phase.Failed)

        val held = coordinator.startFunding(walletA, 1, recipient0, 0) {
            invocations++; throw walletError(delivery = 3, retryable = false, held = heldRaw)
        }
        advanceUntilIdle()
        val outPoint = PaymentFailure.outPoint(ByteArray(32) { 7 }, 1)
        assertEquals(
            PaymentFailure.Action.Resume(outPoint),
            (held.phase.value as Phase.Failed).failure.action,
        )
        coordinator.startFunding(walletA, 1, recipient0, 0) { invocations++; 1L }
        advanceUntilIdle()
        assertEquals("no fresh funding while a lock is held", 2, invocations)
        coordinator.startFunding(walletA, 1, recipient0, 0, resuming = outPoint) { invocations++; 5L }
        advanceUntilIdle()
        assertEquals(3, invocations)
        assertEquals(Phase.Completed(5L), held.phase.value)
    }

    @Test
    fun `retry after failure re-fires the body`() = runTest {
        val coordinator = coordinator()
        val retryGate = CompletableDeferred<Long>()
        var invocations = 0

        // Any NOT_SENT allows running it again (here a retryable one). Thrown from the
        // body itself: a deferred's await would rethrow a recovered copy
        // without the native cause that carries the class.
        val controller = coordinator.startFunding(walletA, 0, recipient0, 0) {
            invocations++; throw walletError(delivery = 1, retryable = true)
        }
        advanceUntilIdle()
        assertTrue(controller.phase.value is Phase.Failed)

        val retried = coordinator.startFunding(walletA, 0, recipient0, 0) { invocations++; retryGate.await() }
        assertSame(controller, retried)
        assertEquals(Phase.InFlight, controller.phase.value)
        runCurrent()
        assertEquals(2, invocations)

        retryGate.complete(9L)
        advanceUntilIdle()
        assertTrue(controller.phase.value is Phase.Completed)
    }

    @Test
    fun `completed controller is swept ~30s after success`() = runTest {
        val coordinator = coordinator(retentionMillis = 30_000L, pollMillis = 1_000L)
        val gate = CompletableDeferred<Long>()

        val controller = coordinator.startFunding(walletA, 0, recipient0, 0) { gate.await() }
        gate.complete(1L)
        runCurrent()
        assertTrue(controller.phase.value is Phase.Completed)

        advanceTimeBy(5_000L)
        runCurrent()
        assertEquals(1, coordinator.controllers.value.size)

        advanceTimeBy(30_000L)
        runCurrent()
        assertTrue(coordinator.controllers.value.isEmpty())
    }

    @Test
    fun `failed controller is retained indefinitely`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Long>()

        coordinator.startFunding(walletA, 0, recipient0, 0) { gate.await() }
        gate.completeExceptionally(RuntimeException("nope"))
        advanceUntilIdle()

        advanceTimeBy(120_000L)
        advanceUntilIdle()
        assertEquals(1, coordinator.controllers.value.size)
    }

    /**
     * Dismiss drops a failed controller only when its failure releases the
     * slot (any NOT_SENT); a failure that may have left, holds a lock, or
     * has no class keeps it, and a fresh funding of the recipient does not
     * run (← Swift `testAddressFundingFailureIsDismissedOnlyWhenNothingWasSent`).
     */
    @Test
    fun `dismiss removes a failed controller only when its payment was not sent`() = runTest {
        val coordinator = coordinator()
        listOf<Pair<Throwable, Boolean>>(
            walletError(delivery = 1, retryable = true) to true,
            walletError(delivery = 1, retryable = false) to true,
            walletError(delivery = 3, retryable = false, held = heldRaw) to false,
            walletError(delivery = 2, retryable = false) to false,
            RuntimeException("unclassified") to false,
        ).forEachIndexed { index, (error, dismissable) ->
            val recipient = ByteArray(20) { index.toByte() }
            val controller = coordinator.startFunding(walletA, 0, recipient, 0) { throw error }
            advanceUntilIdle()
            assertTrue(controller.phase.value is Phase.Failed)

            assertEquals(error.toString(), dismissable, coordinator.dismiss(walletA, 0, recipient))
            assertEquals(dismissable, coordinator.controller(walletA, 0, recipient) == null)

            var ran = false
            val again = coordinator.startFunding(walletA, 0, recipient, 0) { ran = true; 1L }
            advanceUntilIdle()
            assertEquals("a fresh funding runs only after a released slot", dismissable, ran)
            if (!dismissable) assertSame(controller, again)
        }
    }

    /**
     * Release drops a failed controller only when its failure holds the slot
     * without a lock (may have left, or no class), after which a fresh
     * payment runs; it refuses a held lock (resumed instead) and a NOT_SENT
     * failure, keeping the controller (← Swift
     * `testCoordinatorsReleaseOnlyAHoldWithoutALock`).
     */
    @Test
    fun `release removes a failed controller only when it holds its slot without a lock`() = runTest {
        val coordinator = coordinator()
        listOf<Pair<Throwable, Boolean>>(
            walletError(delivery = 1, retryable = true) to false,
            walletError(delivery = 3, retryable = false, held = heldRaw) to false,
            walletError(delivery = 2, retryable = false) to true,
            RuntimeException("unclassified") to true,
        ).forEachIndexed { index, (error, releasable) ->
            val recipient = ByteArray(20) { index.toByte() }
            val controller = coordinator.startFunding(walletA, 0, recipient, 0) { throw error }
            advanceUntilIdle()
            assertTrue(controller.phase.value is Phase.Failed)

            assertEquals(error.toString(), releasable, coordinator.release(walletA, 0, recipient))
            if (!releasable) {
                assertSame(controller, coordinator.controller(walletA, 0, recipient))
                return@forEachIndexed
            }
            assertNull(coordinator.controller(walletA, 0, recipient))
            var ran = false
            coordinator.startFunding(walletA, 0, recipient, 0) { ran = true; 1L }
            advanceUntilIdle()
            assertTrue("a fresh funding runs after a release", ran)
        }
    }

    @Test
    fun `activeControllers sorts newest submit first`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Long>()

        coordinator.startFunding(walletA, 0, recipient0, 0) { gate.await() }
        advanceTimeBy(1_000L)
        coordinator.startFunding(walletB, 0, recipient0, 0) { gate.await() }

        val active = coordinator.activeControllers()
        assertEquals(2, active.size)
        assertEquals(walletB.toList(), active.first().walletId.toList())

        gate.complete(1L)
        advanceUntilIdle()
    }
}
