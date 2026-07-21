package org.dashfoundation.example.services

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.dashfoundation.example.services.IdentityRegistrationController.Phase
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * State-machine tests for [RegistrationCoordinator] + [IdentityRegistrationController]
 * (← the phase model in `RegistrationCoordinator.swift` /
 * `IdentityRegistrationController.swift`).
 *
 * A fake registration backend (a [CompletableDeferred] resolved by the
 * test) stands in for the FFI body, so phase reduction, single-flight, and
 * the 30s retention sweep are exercised deterministically. Time is driven
 * off the test scheduler — the coordinator/controller read a `now` clock
 * bound to `testScheduler.currentTime`, so `advanceTimeBy` moves both the
 * `delay`-based poll loop AND the wall-clock comparison together.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RegistrationCoordinatorTest {

    private val walletA = ByteArray(32) { 0xAA.toByte() }
    private val walletB = ByteArray(32) { 0xBB.toByte() }

    /** Build a coordinator whose clock and scheduler are the test's virtual time. */
    private fun TestScope.coordinator(
        retentionMillis: Long = 30_000L,
        pollMillis: Long = 1_000L,
    ): RegistrationCoordinator =
        RegistrationCoordinator(
            scope = this,
            retentionMillis = retentionMillis,
            pollMillis = pollMillis,
            now = { testScheduler.currentTime },
        )

    @Test
    fun `happy path walks idle to preparingKeys to inFlight to completed`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()
        val id = ByteArray(32) { 1 }

        val controller = coordinator.startRegistration(
            walletId = walletA,
            identityIndex = 0,
            body = { gate.await() },
        )

        // startRegistration drives enterPreparingKeys() then submit() → InFlight.
        assertEquals(Phase.InFlight, controller.phase.value)
        assertTrue(controller.phase.value.isActive)

        gate.complete(id)
        advanceUntilIdle()

        assertEquals(Phase.Completed(id), controller.phase.value)
        assertFalse(controller.phase.value.isActive)
    }

    @Test
    fun `failure body transitions to failed and is not active`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()

        val controller = coordinator.startRegistration(
            walletId = walletA,
            identityIndex = 0,
            body = { gate.await() },
        )
        gate.completeExceptionally(IllegalStateException("asset lock rejected"))
        advanceUntilIdle()

        val phase = controller.phase.value
        assertTrue(phase is Phase.Failed)
        assertEquals("asset lock rejected", (phase as Phase.Failed).message)
        assertFalse(phase.isActive)
    }

    @Test
    fun `unconfirmed classifier yields active unconfirmed phase`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()
        val id = ByteArray(32) { 7 }

        val controller = coordinator.startRegistration(
            walletId = walletA,
            identityIndex = 3,
            fundingKind = IdentityRegistrationController.FundingKind.ShieldedPool,
            body = { gate.await() },
            isUnconfirmed = { id },
        )
        gate.completeExceptionally(RuntimeException("proof not confirmed"))
        advanceUntilIdle()

        val phase = controller.phase.value
        assertTrue(phase is Phase.Unconfirmed)
        assertEquals(id.toList(), (phase as Phase.Unconfirmed).identityId.toList())
        assertTrue(phase.isActive) // unconfirmed holds the slot
    }

    @Test
    fun `broadcast-rejected classifier sets failure stage`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()

        val controller = coordinator.startRegistration(
            walletId = walletA,
            identityIndex = 0,
            fundingKind = IdentityRegistrationController.FundingKind.ShieldedPool,
            body = { gate.await() },
            isBroadcastRejected = { true },
        )
        gate.completeExceptionally(RuntimeException("broadcast rejected"))
        advanceUntilIdle()

        assertTrue(controller.phase.value is Phase.Failed)
        assertEquals(
            IdentityRegistrationController.FailureStage.BroadcastRejected,
            controller.failureStage.value,
        )
    }

    @Test
    fun `single-flight returns the same controller for an in-flight slot`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()
        var bodyInvocations = 0

        val first = coordinator.startRegistration(
            walletId = walletA,
            identityIndex = 0,
            body = { bodyInvocations++; gate.await() },
        )
        // A second tap on the same slot while in flight returns the SAME
        // controller and does NOT fire the body again.
        val second = coordinator.startRegistration(
            walletId = walletA,
            identityIndex = 0,
            body = { bodyInvocations++; gate.await() },
        )

        // Single-flight is enforced synchronously: same controller, one slot.
        assertSame(first, second)
        assertEquals(1, coordinator.controllers.value.size)

        // Only ONE body ever runs for the slot (the second tap was a no-op).
        // runCurrent (not advanceUntilIdle — the InFlight sweep polls forever)
        // lets the single launched body reach its gate.await().
        runCurrent()
        assertEquals(1, bodyInvocations)

        gate.complete(ByteArray(32))
        advanceUntilIdle() // Completed → sweep terminates at the 30s window
    }

    @Test
    fun `distinct slots get distinct controllers`() = runTest {
        val coordinator = coordinator()
        val gateA = CompletableDeferred<ByteArray>()
        val gateB = CompletableDeferred<ByteArray>()

        val a = coordinator.startRegistration(walletA, 0, body = { gateA.await() })
        val b0 = coordinator.startRegistration(walletB, 0, body = { gateB.await() })
        val a1 = coordinator.startRegistration(walletA, 1, body = { gateA.await() })

        assertEquals(3, coordinator.controllers.value.size)
        assertTrue(a !== b0 && a !== a1 && b0 !== a1)
        assertTrue(coordinator.hasInFlightRegistrations)

        gateA.complete(ByteArray(32))
        gateB.complete(ByteArray(32))
        advanceUntilIdle()
    }

    @Test
    fun `retry after failure re-enters preparingKeys and re-fires the body`() = runTest {
        val coordinator = coordinator()
        val failGate = CompletableDeferred<ByteArray>()
        val retryGate = CompletableDeferred<ByteArray>()
        var invocations = 0

        val controller = coordinator.startRegistration(
            walletA, 0,
            body = { invocations++; failGate.await() },
        )
        failGate.completeExceptionally(RuntimeException("boom"))
        advanceUntilIdle()
        assertTrue(controller.phase.value is Phase.Failed)

        // Retry on the same slot: allowed from Failed → re-enters and re-fires.
        val retried = coordinator.startRegistration(
            walletA, 0,
            body = { invocations++; retryGate.await() },
        )
        // submit() flips the phase synchronously; the body launch is deferred.
        assertSame(controller, retried)
        assertEquals(Phase.InFlight, controller.phase.value)

        runCurrent() // second body reaches its gate.await() (InFlight sweep polls forever)
        assertEquals(2, invocations)

        retryGate.complete(ByteArray(32) { 9 })
        advanceUntilIdle() // Completed → sweep terminates
        assertTrue(controller.phase.value is Phase.Completed)
    }

    @Test
    fun `completed controller is swept ~30s after success`() = runTest {
        val coordinator = coordinator(retentionMillis = 30_000L, pollMillis = 1_000L)
        val gate = CompletableDeferred<ByteArray>()

        val controller = coordinator.startRegistration(walletA, 0, body = { gate.await() })
        gate.complete(ByteArray(32))
        // Step time deliberately (NOT advanceUntilIdle — the poll loop never
        // idles until it sweeps). runCurrent lets the body resolve → Completed.
        runCurrent()
        assertTrue(controller.phase.value is Phase.Completed)

        // Still present shortly after completion (retention window open).
        advanceTimeBy(5_000L)
        runCurrent()
        assertEquals(1, coordinator.controllers.value.size)

        // Cross the 30s retention window → swept out of the map on the next poll.
        advanceTimeBy(30_000L)
        runCurrent()
        assertTrue(coordinator.controllers.value.isEmpty())
    }

    @Test
    fun `failed controller is retained indefinitely and never swept`() = runTest {
        val coordinator = coordinator(retentionMillis = 30_000L, pollMillis = 1_000L)
        val gate = CompletableDeferred<ByteArray>()

        coordinator.startRegistration(walletA, 0, body = { gate.await() })
        gate.completeExceptionally(RuntimeException("nope"))
        advanceUntilIdle()

        advanceTimeBy(120_000L)
        advanceUntilIdle()
        // Failures stay until the user dismisses them.
        assertEquals(1, coordinator.controllers.value.size)
    }

    @Test
    fun `dismiss removes a failed controller`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()

        coordinator.startRegistration(walletA, 0, body = { gate.await() })
        gate.completeExceptionally(RuntimeException("nope"))
        advanceUntilIdle()
        assertEquals(1, coordinator.controllers.value.size)

        coordinator.dismiss(walletA, 0)
        assertTrue(coordinator.controllers.value.isEmpty())
        assertNull(coordinator.controller(walletA, 0))
    }

    @Test
    fun `activeControllers sorts newest submit first`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()

        coordinator.startRegistration(walletA, 0, body = { gate.await() })
        advanceTimeBy(1_000L)
        coordinator.startRegistration(walletB, 0, body = { gate.await() })

        val active = coordinator.activeControllers()
        assertEquals(2, active.size)
        // Most-recently-submitted (walletB) first.
        assertEquals(walletB.toList(), active.first().walletId.toList())

        gate.complete(ByteArray(32))
        advanceUntilIdle()
    }

    @Test
    fun `hasInFlightRegistrations is false once all slots reach a non-active terminal`() = runTest {
        val coordinator = coordinator()
        val gate = CompletableDeferred<ByteArray>()

        coordinator.startRegistration(walletA, 0, body = { gate.await() })
        assertTrue(coordinator.hasInFlightRegistrations)

        gate.complete(ByteArray(32))
        advanceUntilIdle()
        // Completed is not active → gate opens (before the 30s sweep removes it).
        assertFalse(coordinator.hasInFlightRegistrations)
    }
}
