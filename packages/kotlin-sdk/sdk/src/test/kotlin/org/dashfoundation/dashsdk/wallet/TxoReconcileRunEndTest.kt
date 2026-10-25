package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The end of a TXO-store reconcile run ([endTxoReconcileRun]): the held
 * verdicts are freed first, then the native claim is released, and a
 * failure of either never skips the other. Robolectric, for the warning
 * each failure logs.
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
}
