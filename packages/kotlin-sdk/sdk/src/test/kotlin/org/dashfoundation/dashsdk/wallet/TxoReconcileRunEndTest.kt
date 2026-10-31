package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.util.concurrent.atomic.AtomicLong

/**
 * The edges of a TXO-store reconcile run: its start
 * ([withTxoReconcileEpoch]) claims nothing for a lifecycle that ended
 * while it waited, and its end ([endTxoReconcileRun]) frees the held
 * verdicts first, then releases the native claim, a failure of either
 * never skipping the other. Robolectric, for the warning each failure
 * logs.
 */
@RunWith(RobolectricTestRunner::class)
class TxoReconcileRunEndTest {

    @Test
    fun theHeldVerdictsAreFreedBeforeTheClaimIsReleased() {
        val steps = ArrayList<String>()
        endTxoReconcileRun(
            walletRef = "ref",
            freeHeldVerdicts = { steps += "free" },
            finish = { steps += "finish" },
        )
        assertEquals(listOf("free", "finish"), steps)
    }

    @Test
    fun aFailedFreeStillReleasesTheClaim() {
        val steps = ArrayList<String>()
        endTxoReconcileRun(
            walletRef = "ref",
            freeHeldVerdicts = {
                steps += "free"
                throw IllegalStateException("native free failed")
            },
            finish = { steps += "finish" },
        )
        assertEquals("finish runs after a failed free", listOf("free", "finish"), steps)
    }

    @Test
    fun aFailedFinishIsLoggedNotThrown() {
        var freed = 0
        endTxoReconcileRun(
            walletRef = "ref",
            freeHeldVerdicts = { freed++ },
            finish = { throw IllegalStateException("handle gone") },
        )
        assertEquals(1, freed)
    }

    @Test
    fun aRunScheduledBeforeTheWalletWasRemovedClaimsNothing() = runBlocking {
        // The coroutine read the epoch, then waited; meanwhile the wallet
        // was removed (and its id may be live again). Once it holds the
        // teardown borrow it must not reach the native begin.
        val epoch = AtomicLong(7)
        val captured = epoch.get()
        epoch.incrementAndGet()
        var begun = false
        val outcome = TeardownGate().withTxoReconcileEpoch(
            epoch = epoch,
            captured = captured,
            stale = { "stale" },
        ) {
            begun = true
            "claimed"
        }
        assertEquals("stale", outcome)
        assertTrue("the native begin was never reached", !begun)
    }

    @Test
    fun aRunOfTheCurrentLifecycleGoesAhead() = runBlocking {
        val epoch = AtomicLong(7)
        val outcome = TeardownGate().withTxoReconcileEpoch(
            epoch = epoch,
            captured = epoch.get(),
            stale = { "stale" },
        ) { "claimed" }
        assertEquals("claimed", outcome)
    }
}
