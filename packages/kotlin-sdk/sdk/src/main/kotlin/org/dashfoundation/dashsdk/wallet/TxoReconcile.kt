package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler

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
 */
enum class TxoReconcileSkipReason(internal val gateCode: Int?) {
    /** The manager is closed or closing. */
    MANAGER_CLOSED(null),

    /** The wallet is not loaded in this manager. */
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
