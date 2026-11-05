package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import java.util.concurrent.ConcurrentHashMap

/**
 * What [PlatformWalletManager.reconcileTxoStore] did — the Kotlin twin of
 * Swift's `CoreTxoReconcileOutcome` (dashpay/platform#4638).
 */
sealed class TxoReconcileOutcome {
    /** A gate refused the run; nothing was read or written. */
    data class Skipped(val reason: TxoReconcileSkipReason) : TxoReconcileOutcome()

    /** The run happened; [report] says how far it got and what it changed. */
    data class Reconciled(
        val report: PlatformWalletPersistenceHandler.TxoReconcileReport,
    ) : TxoReconcileOutcome()
}

/**
 * Why [PlatformWalletManager.reconcileTxoStore] did not run. Every reason
 * but [MANAGER_CLOSED] comes from the native start gate
 * (`platform_wallet::manager::txo_reconcile::TxoReconcileSkip`), which owns
 * the thresholds; [gateCode] is its `TXO_RECONCILE_SKIP_*` code.
 * [WALLET_UNKNOWN] is also what a wallet this manager has not loaded gets,
 * checked here before the native gate.
 */
enum class TxoReconcileSkipReason(internal val gateCode: Int?) {
    /** The manager is closed or closing. */
    MANAGER_CLOSED(null),

    /** The wallet is not loaded in this manager, or the engine does not
     *  hold it. */
    WALLET_UNKNOWN(1),

    /** A run for this wallet is already in flight. */
    ALREADY_RUNNING(2),

    /** SPV is not running, or not in its steady state. */
    NOT_STEADY_STATE(3),

    /** The progress carries no scan tip height. */
    TIP_UNAVAILABLE(4),

    /** The native manager latched a sync fault: rows are missing by design
     *  and a rescan is pending ([PlatformWalletManager.syncFaultDetected]). */
    SYNC_FAULT_DETECTED(5),

    /** The wallet's scan watermark, as the engine holds it, trails the scan
     *  tip by more than the native margin — it is still being scanned. */
    WALLET_BEHIND_TIP(6),
    ;

    internal companion object {
        /** The reason a non-run code from the native start gate names. A
         *  code this build does not know is a contract break with the
         *  native library it shipped with, not a reason to guess. */
        fun fromGateCode(code: Int): TxoReconcileSkipReason =
            entries.firstOrNull { it.gateCode == code }
                ?: throw IllegalStateException("unknown TXO reconcile gate code $code")
    }
}

/**
 * End a TXO-store reconcile run: free the engine guard its last page may
 * still hold, then release its native claim — Swift's
 * `endCoreTxoReconcileRun`. The guard goes first and without the native
 * registry: `finish` resolves the manager handle there, which must never be
 * waited for while a guard is held. Neither step can skip the other: a
 * failure is logged by exception type and [walletRef] only, and a claim
 * left behind would refuse every later run of the wallet as
 * [TxoReconcileSkipReason.ALREADY_RUNNING].
 */
internal fun endTxoReconcileRun(walletRef: String, freeHeldVerdicts: () -> Unit, finish: () -> Unit) {
    runCatching(freeHeldVerdicts).onFailure { t ->
        android.util.Log.w(
            "PlatformWalletManager",
            "txos reconcile: verdict release failed error=${t.javaClass.simpleName} wallet=$walletRef",
        )
    }
    runCatching(finish).onFailure { t ->
        android.util.Log.w(
            "PlatformWalletManager",
            "txos reconcile: release failed error=${t.javaClass.simpleName} wallet=$walletRef",
        )
    }
}

/**
 * The automatic reconcile runs a manager has launched and that have not
 * ended yet, by wallet (hex id) — Swift's `coreTxoReconcileLaunched`.
 * Bookkeeping of the manager's own coroutines only: whether and when a run
 * may start is the native schedule's and start gate's. The native schedule
 * reports a wallet due until a run claims it, so without this every poll
 * tick before a launched run reaches its begin would queue another one, and
 * they would run back to back once the first ended.
 */
internal class TxoReconcileLaunches {
    private val launched: MutableSet<String> = ConcurrentHashMap.newKeySet()

    /**
     * Launch [run] for [walletKey] in [scope], unless a launch of the same
     * wallet has not ended; returns whether it launched. The wallet is
     * released when the job completes — on completion rather than in a
     * `finally`, so a job cancelled before its body ran releases it too.
     */
    fun launch(scope: CoroutineScope, walletKey: String, run: suspend () -> Unit): Boolean {
        if (!launched.add(walletKey)) return false
        scope.launch { run() }.invokeOnCompletion { launched.remove(walletKey) }
        return true
    }

    /** Whether a launch of [walletKey] has not ended. */
    operator fun contains(walletKey: String): Boolean = walletKey in launched
}
