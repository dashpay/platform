package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The Kotlin side of the TXO-store reconcile's start gate. The gates, the
 * schedule and their thresholds live in
 * `platform_wallet::manager::txo_reconcile` and are tested there; what
 * Kotlin owns is reading the gate's answer.
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
}
