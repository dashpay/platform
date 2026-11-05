package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The Kotlin side of the TXO-store reconcile's start gate and trigger. The
 * gates, the schedule and their thresholds live in
 * `platform_wallet::manager::txo_reconcile` and are tested there; what
 * Kotlin owns is reading the gate's answer and not launching a second run
 * of a wallet while its first has not ended.
 */
class TxoReconcileTest {

    @Test
    fun everyNativeSkipCodeMapsToItsReason() {
        // rs-platform-wallet-ffi `TXO_RECONCILE_SKIP_*`, in code order.
        val expected = mapOf(
            1 to TxoReconcileSkipReason.WALLET_UNKNOWN,
            2 to TxoReconcileSkipReason.ALREADY_RUNNING,
            3 to TxoReconcileSkipReason.NOT_STEADY_STATE,
            4 to TxoReconcileSkipReason.TIP_UNAVAILABLE,
            5 to TxoReconcileSkipReason.SYNC_FAULT_DETECTED,
            6 to TxoReconcileSkipReason.WALLET_BEHIND_TIP,
        )
        for ((code, reason) in expected) {
            assertEquals(reason, TxoReconcileSkipReason.fromGateCode(code))
        }
        assertEquals(
            "only the manager's own closed check has no native code",
            listOf(TxoReconcileSkipReason.MANAGER_CLOSED),
            TxoReconcileSkipReason.entries.filter { it.gateCode == null },
        )
    }

    @Test
    fun aCodeThisBuildDoesNotKnowIsAContractBreakNotASkip() {
        for (code in listOf(0, 7, -1)) {
            val thrown = runCatching { TxoReconcileSkipReason.fromGateCode(code) }.exceptionOrNull()
            assertTrue("code $code must not map to a reason, got $thrown", thrown is IllegalStateException)
        }
    }

    @Test
    fun aWalletWhoseLaunchedRunHasNotEndedIsNotLaunchedAgain() = runTest {
        // The native schedule reports a wallet due on every tick until a run
        // claims it; a run still queued must not get a second one behind it.
        val launches = TxoReconcileLaunches()
        val release = CompletableDeferred<Unit>()
        var runs = 0
        suspend fun run() {
            runs++
            release.await()
        }

        assertTrue(launches.launch(this, "a") { run() })
        assertFalse("a launch of the same wallet is pending", launches.launch(this, "a") { run() })
        assertTrue("another wallet launches on its own", launches.launch(this, "b") { runs++ })
        advanceUntilIdle()
        assertEquals(2, runs)
        assertTrue("a" in launches)
        assertFalse("b" in launches)

        release.complete(Unit)
        advanceUntilIdle()
        assertFalse("an ended run releases its wallet", "a" in launches)
        assertTrue(launches.launch(this, "a") { runs++ })
        advanceUntilIdle()
        assertEquals(3, runs)
    }

    @Test
    fun aRunThatThrowsOrIsCancelledBeforeItStartsStillReleasesItsWallet() {
        val launches = TxoReconcileLaunches()
        val scope = TestScope(StandardTestDispatcher())

        // A scope whose job is already cancelled: the body never runs, but
        // the job completes, and the wallet is released with it.
        val cancelled = CoroutineScope(scope.coroutineContext + Job().apply { cancel() })
        var ran = false
        assertTrue(launches.launch(cancelled, "a") { ran = true })
        scope.advanceUntilIdle()
        assertFalse(ran)
        assertFalse("a launch that never ran releases its wallet", "a" in launches)

        val handler = kotlinx.coroutines.CoroutineExceptionHandler { _, _ -> }
        val failing = CoroutineScope(scope.coroutineContext + Job() + handler)
        assertTrue(launches.launch(failing, "b") { error("run failed") })
        scope.advanceUntilIdle()
        assertFalse("a run that threw releases its wallet", "b" in launches)
    }
}
