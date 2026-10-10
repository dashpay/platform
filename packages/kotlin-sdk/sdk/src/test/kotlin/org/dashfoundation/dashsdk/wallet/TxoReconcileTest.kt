package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The manager-side gates and trigger of the TXO-store reconcile — port of
 * the steady-state and scheduling cases of Swift's `CoreTxoReconcileTests`
 * / `CoreTxoReconcileShutdownTests` (dashpay/platform#4638). Pure: no
 * native manager is needed.
 */
class TxoReconcileTest {

    private fun progress(
        state: SpvSyncState,
        filters: Pair<Long, Long>?,
        headers: Long = 2_535_898,
    ) = SpvSyncProgressData(
        overallState = state,
        overallPercentage = 0.0,
        headers = SpvSubProgress(state, headers, headers, 1.0),
        filterHeaders = null,
        filters = filters?.let { (current, target) -> SpvSubProgress(state, current, target, 0.0) },
        masternodes = null,
    )

    private val steady = progress(SpvSyncState.WAIT_FOR_EVENTS, filters = 2_535_898L to 2_535_898L)
    private val syncing = progress(SpvSyncState.SYNCING, filters = 199_999L to 2_535_898L)

    @Test
    fun theSteadyStateGateRefusesAnUnfinishedScan() {
        assertFalse(TxoReconcileGates.isSteadySyncState(syncing))
        assertFalse(TxoReconcileGates.isSteadySyncState(progress(SpvSyncState.WAITING_FOR_CONNECTIONS, filters = null)))
        assertFalse(
            TxoReconcileGates.isSteadySyncState(progress(SpvSyncState.ERROR, filters = 2_535_898L to 2_535_898L)),
        )
        assertFalse(
            "waiting for events with the filter phase behind its target is a scan still running",
            TxoReconcileGates.isSteadySyncState(progress(SpvSyncState.WAIT_FOR_EVENTS, filters = 2_535_800L to 2_535_898L)),
        )
        assertFalse(
            "no filter phase at all proves nothing",
            TxoReconcileGates.isSteadySyncState(progress(SpvSyncState.WAIT_FOR_EVENTS, filters = null)),
        )
        assertFalse(
            "a filter phase with no target proves nothing",
            TxoReconcileGates.isSteadySyncState(progress(SpvSyncState.WAIT_FOR_EVENTS, filters = 0L to 0L)),
        )
        assertFalse(
            "the poll's not-running placeholder is not steady",
            TxoReconcileGates.isSteadySyncState(SpvSyncProgressData.EMPTY),
        )
        assertTrue(TxoReconcileGates.isSteadySyncState(progress(SpvSyncState.SYNCED, filters = 2_535_898L to 2_535_898L)))
        assertTrue("dash-spv's fully synced steady state", TxoReconcileGates.isSteadySyncState(steady))
    }

    @Test
    fun theTipIsTheFilterHeightFallingBackToTheHeaderTip() {
        assertEquals(2_535_898, TxoReconcileGates.scanTipHeight(steady))
        assertEquals(
            "falls back to the header tip",
            2_535_898,
            TxoReconcileGates.scanTipHeight(progress(SpvSyncState.SYNCED, filters = null)),
        )
        assertEquals(
            "a filter phase at height 0 falls back too",
            2_535_000,
            TxoReconcileGates.scanTipHeight(progress(SpvSyncState.SYNCED, filters = 0L to 0L, headers = 2_535_000)),
        )
        assertNull(TxoReconcileGates.scanTipHeight(SpvSyncProgressData.EMPTY))
    }

    @Test
    fun everyLoadedWalletRunsOnTheRisingEdgeIntoSteadyState() {
        val coordinator = TxoReconcileCoordinator(cadenceMs = 1_000)
        val wallets = listOf("a", "b")

        assertEquals(emptyList<String>(), coordinator.walletsDue(syncing, wallets, nowMs = 0))
        assertEquals(wallets, coordinator.walletsDue(steady, wallets, nowMs = 10))
        assertEquals(
            "the next tick inside the cadence schedules nothing",
            emptyList<String>(),
            coordinator.walletsDue(steady, wallets, nowMs = 20),
        )
    }

    @Test
    fun aSteadyClientRunsEachWalletAgainOnItsOwnCadence() {
        val coordinator = TxoReconcileCoordinator(cadenceMs = 1_000)
        assertEquals(listOf("a"), coordinator.walletsDue(steady, listOf("a"), nowMs = 0))
        // A wallet loaded while steady never ran: it is due at once, on its
        // own clock.
        assertEquals(listOf("b"), coordinator.walletsDue(steady, listOf("a", "b"), nowMs = 500))
        assertEquals(listOf("a"), coordinator.walletsDue(steady, listOf("a", "b"), nowMs = 1_000))
        assertEquals(listOf("b"), coordinator.walletsDue(steady, listOf("a", "b"), nowMs = 1_500))
        // A completed run restarts that wallet's cadence.
        coordinator.markRun("a", nowMs = 1_900)
        assertEquals(emptyList<String>(), coordinator.walletsDue(steady, listOf("a", "b"), nowMs = 2_000))
        assertEquals(listOf("a", "b"), coordinator.walletsDue(steady, listOf("a", "b"), nowMs = 2_900))
    }

    @Test
    fun aWalletWithARunInFlightIsNeverScheduledOrStartedAgain() {
        val coordinator = TxoReconcileCoordinator(cadenceMs = 1_000)
        assertTrue(coordinator.tryBegin("a"))
        assertFalse("a second run for the same wallet is refused", coordinator.tryBegin("a"))
        assertEquals(
            "not even on the rising edge",
            listOf("b"),
            coordinator.walletsDue(steady, listOf("a", "b"), nowMs = 0),
        )
        coordinator.end("a")
        assertTrue(coordinator.tryBegin("a"))
    }

    @Test
    fun leavingSteadyStateArmsTheNextRisingEdge() {
        val coordinator = TxoReconcileCoordinator(cadenceMs = 1_000_000)
        assertEquals(listOf("a"), coordinator.walletsDue(steady, listOf("a"), nowMs = 0))
        assertEquals(emptyList<String>(), coordinator.walletsDue(syncing, listOf("a"), nowMs = 10))
        assertEquals(
            "back in steady state: due again, cadence or not",
            listOf("a"),
            coordinator.walletsDue(steady, listOf("a"), nowMs = 20),
        )
    }

    @Test
    fun closeAndDeletionStaleTheEpochAndDeletionForgetsTheCadence() {
        val coordinator = TxoReconcileCoordinator(cadenceMs = 1_000)
        val before = coordinator.currentEpoch
        coordinator.bumpEpoch()
        assertTrue("an in-flight run sees its epoch move", coordinator.currentEpoch != before)

        assertEquals(listOf("a"), coordinator.walletsDue(steady, listOf("a"), nowMs = 0))
        coordinator.forget("a")
        assertEquals(
            "a re-added wallet is due at once",
            listOf("a"),
            coordinator.walletsDue(steady, listOf("a"), nowMs = 10),
        )
    }

    @Test
    fun theWalletGateUsesTheEnginesWatermarkWithinTheTipMargin() {
        // Swift's `synced_height + margin >= tip`. The value passed is the
        // ENGINE's watermark: the persisted Room copy trails it by a round,
        // and gating on that skipped the run at the SYNCED edge of a restore
        // (2026-10-02 device run: WALLET_BEHIND_TIP at the transition, the
        // next attempt 30 minutes later).
        assertTrue(TxoReconcileGates.walletCaughtUp(walletSyncedHeight = 1_565_211L, tipHeight = 1_565_211))
        assertTrue(TxoReconcileGates.walletCaughtUp(walletSyncedHeight = 1_565_205L, tipHeight = 1_565_211))
        assertFalse(TxoReconcileGates.walletCaughtUp(walletSyncedHeight = 1_565_204L, tipHeight = 1_565_211))
        // A wallet added behind the tip is still being scanned.
        assertFalse(TxoReconcileGates.walletCaughtUp(walletSyncedHeight = 0L, tipHeight = 1_565_211))
        // The native "no value" sentinel never passes.
        assertFalse(TxoReconcileGates.walletCaughtUp(walletSyncedHeight = -1L, tipHeight = 3))
    }
}
