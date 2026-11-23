package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator.Key
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator.Kind
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator.State
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The coordinator owns a payment between an identity and the shielded pool
 * past the screen that started it. A screen recreated by a rotation or a tab
 * switch while the payment proves sees it still running, cannot start a
 * second one for the same wallet and identity, and finds the outcome when
 * the payment ends, even though the screen that started it is gone.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ShieldedIdentityPaymentCoordinatorTest {

    private val key = Key(Kind.TOP_UP_FROM_POOL, "aa", "bb")

    @Test
    fun `should refuse a second payment for the same identity while the first is proving`() = runTest {
        val coordinator = ShieldedIdentityPaymentCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        val proof = CompletableDeferred<ShieldedPaymentOutcome>()
        var calls = 0

        assertTrue(coordinator.start(key, 5) { calls++; proof.await() })
        assertEquals(State.InFlight(5), coordinator.states.value[key])

        // The recreated screen taps again: nothing new is paid.
        assertFalse(coordinator.start(key, 5) { calls++; ShieldedPaymentOutcome.Submitted })
        assertEquals(1, calls)

        // Another identity or kind is its own slot.
        assertTrue(coordinator.start(Key(Kind.SHIELD_FROM_IDENTITY, "aa", "bb"), 1) { ShieldedPaymentOutcome.Submitted })

        proof.complete(ShieldedPaymentOutcome.MayHaveGoneThrough)
        assertEquals(State.Done(5, ShieldedPaymentOutcome.MayHaveGoneThrough), coordinator.states.value[key])
    }

    @Test
    fun `should keep the outcome until a screen showed it, then free the slot`() = runTest {
        val coordinator = ShieldedIdentityPaymentCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        coordinator.start(key, 7) { ShieldedPaymentOutcome.Submitted }

        assertEquals(State.Done(7, ShieldedPaymentOutcome.Submitted), coordinator.states.value[key])
        assertFalse("an outcome not shown yet blocks a new payment", coordinator.start(key, 7) {
            ShieldedPaymentOutcome.Submitted
        })

        coordinator.acknowledge(key)
        assertNull(coordinator.states.value[key])
        assertTrue(coordinator.start(key, 7) { ShieldedPaymentOutcome.Submitted })
    }

    @Test
    fun `should not free a slot whose payment is still running`() = runTest {
        val coordinator = ShieldedIdentityPaymentCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        val proof = CompletableDeferred<ShieldedPaymentOutcome>()
        coordinator.start(key, 3) { proof.await() }

        coordinator.acknowledge(key)
        assertEquals(State.InFlight(3), coordinator.states.value[key])
        proof.complete(ShieldedPaymentOutcome.Submitted)
    }

    @Test
    fun `should record a payment that throws as failed instead of losing it`() = runTest {
        val coordinator = ShieldedIdentityPaymentCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        coordinator.start(key, 9) { throw IllegalStateException("store busy") }

        assertEquals(State.Done(9, ShieldedPaymentOutcome.Failed("store busy")), coordinator.states.value[key])
    }
}
