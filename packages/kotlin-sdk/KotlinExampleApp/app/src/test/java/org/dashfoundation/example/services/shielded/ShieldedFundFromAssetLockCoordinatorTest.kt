package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockController.Phase
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * State-machine tests for [ShieldedFundFromAssetLockCoordinator] +
 * [ShieldedFundFromAssetLockController] (← the phase model in
 * `ShieldedFundFromAssetLockCoordinator.swift` /
 * `ShieldedFundFromAssetLockController.swift`).
 *
 * Same fake-backend + virtual-clock harness as the registration and
 * address-funding tests. Adds coverage for the shielded coordinator's
 * per-wallet serialization gate (a different recipient in flight on the
 * same wallet is blocked, mirroring the Rust `shield_guard` mutex).
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ShieldedFundFromAssetLockCoordinatorTest {

    private val walletA = ByteArray(32) { 0xAA.toByte() }
    private val walletB = ByteArray(32) { 0xBB.toByte() }
    private val recipientX = ByteArray(43) { 0x20 }
    private val recipientY = ByteArray(43) { 0x21 }

    private fun TestScope.coordinator(
        retentionMillis: Long = 30_000L,
        pollMillis: Long = 1_000L,
    ): ShieldedFundFromAssetLockCoordinator =
        ShieldedFundFromAssetLockCoordinator(
            scope = this,
            retentionMillis = retentionMillis,
            pollMillis = pollMillis,
            now = { testScheduler.currentTime },
        )

    private fun StartFundingResult.controller(): ShieldedFundFromAssetLockController =
        (this as StartFundingResult.Started).controller

    @Test
    fun `happy path walks idle to inFlight to completed`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Unit>()

        val controller = coordinator.startFunding(walletA, recipientX) { gate.await() }.controller()
        assertEquals(Phase.InFlight, controller.phase.value)
        assertTrue(controller.phase.value.isActive)

        gate.complete(Unit)
        advanceUntilIdle()

        assertEquals(Phase.Completed, controller.phase.value)
        assertFalse(controller.phase.value.isActive)
    }

    @Test
    fun `failure body transitions to failed`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Unit>()

        val controller = coordinator.startFunding(walletA, recipientX) { gate.await() }.controller()
        gate.completeExceptionally(RuntimeException("proof build failed"))
        advanceUntilIdle()

        val phase = controller.phase.value
        assertTrue(phase is Phase.Failed)
        assertEquals("proof build failed", (phase as Phase.Failed).message)
        assertFalse(phase.isActive)
    }

    @Test
    fun `single-flight returns the same controller for an in-flight slot`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Unit>()
        var invocations = 0

        val first = coordinator.startFunding(walletA, recipientX) { invocations++; gate.await() }.controller()
        val second = coordinator.startFunding(walletA, recipientX) { invocations++; gate.await() }.controller()

        assertSame(first, second)
        assertEquals(1, coordinator.controllers.value.size)
        runCurrent()
        assertEquals(1, invocations)

        gate.complete(Unit)
        advanceUntilIdle()
    }

    @Test
    fun `different recipient on the same wallet is blocked while one is in flight`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Unit>()

        val started = coordinator.startFunding(walletA, recipientX) { gate.await() }
        assertTrue(started is StartFundingResult.Started)

        val blocked = coordinator.startFunding(walletA, recipientY) { gate.await() }
        assertTrue(blocked is StartFundingResult.BlockedByOtherWalletFunding)
        assertSame(
            started.controller(),
            (blocked as StartFundingResult.BlockedByOtherWalletFunding).blocker,
        )
        // The blocked start added no controller.
        assertEquals(1, coordinator.controllers.value.size)

        gate.complete(Unit)
        advanceUntilIdle()
    }

    @Test
    fun `different recipient on a different wallet is allowed concurrently`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Unit>()

        val a = coordinator.startFunding(walletA, recipientX) { gate.await() }
        val b = coordinator.startFunding(walletB, recipientY) { gate.await() }
        assertTrue(a is StartFundingResult.Started)
        assertTrue(b is StartFundingResult.Started)
        assertEquals(2, coordinator.controllers.value.size)

        gate.complete(Unit)
        advanceUntilIdle()
    }

    @Test
    fun `different recipient allowed once the first slot completes`() = runTest {
        val coordinator = coordinator()
        val firstGate = CompletableDeferred<Unit>()
        val secondGate = CompletableDeferred<Unit>()

        coordinator.startFunding(walletA, recipientX) { firstGate.await() }
        firstGate.complete(Unit)
        runCurrent() // first → Completed (not active)

        val second = coordinator.startFunding(walletA, recipientY) { secondGate.await() }
        assertTrue(second is StartFundingResult.Started)

        secondGate.complete(Unit)
        advanceUntilIdle()
    }

    @Test
    fun `completed controller is swept ~30s after success`() = runTest {
        val coordinator = coordinator(retentionMillis = 30_000L, pollMillis = 1_000L)
        val gate = CompletableDeferred<Unit>()

        val controller = coordinator.startFunding(walletA, recipientX) { gate.await() }.controller()
        gate.complete(Unit)
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
    fun `dismiss removes a failed controller`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<Unit>()

        coordinator.startFunding(walletA, recipientX) { gate.await() }
        gate.completeExceptionally(RuntimeException("nope"))
        advanceUntilIdle()
        assertEquals(1, coordinator.controllers.value.size)

        coordinator.dismiss(walletA, recipientX)
        assertTrue(coordinator.controllers.value.isEmpty())
        assertNull(coordinator.controller(walletA, recipientX))
    }
}
