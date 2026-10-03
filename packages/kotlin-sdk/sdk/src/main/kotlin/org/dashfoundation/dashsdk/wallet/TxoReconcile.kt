package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import java.util.concurrent.atomic.AtomicLong

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

/** Why [PlatformWalletManager.reconcileTxoStore] did not run. */
enum class TxoReconcileSkipReason {
    /** The manager is closed or closing. */
    MANAGER_CLOSED,

    /** The wallet is not loaded in this manager (or has no Room row). */
    WALLET_UNKNOWN,

    /** A run for this wallet is already in flight. */
    ALREADY_RUNNING,

    /** SPV is not in its steady state (see [TxoReconcileGates.isSteadySyncState]). */
    NOT_STEADY_STATE,

    /** The progress carries no scan tip height. */
    TIP_UNAVAILABLE,

    /** The native manager latched a sync fault: rows are missing by design
     *  and a rescan is pending ([PlatformWalletManager.syncFaultDetected]). */
    SYNC_FAULT_DETECTED,

    /** The wallet's durable scan watermark trails the scan tip by more
     *  than [TxoReconcileGates.TIP_MARGIN] blocks — it is still being
     *  scanned. */
    WALLET_BEHIND_TIP,
}

/** The pure gates of the TXO-store reconcile, testable without a manager. */
internal object TxoReconcileGates {
    /** How far the wallet's durable scan watermark may trail the scan tip
     *  for the scan to count as complete for this wallet. */
    const val TIP_MARGIN = 6

    /** Cadence of the automatic run while SPV stays in steady state. */
    const val CADENCE_MS = 30 * 60 * 1_000L

    /**
     * Whether the wallet's own scan watermark (the engine's, see
     * [org.dashfoundation.dashsdk.ffi.WalletManagerNative.walletManagerCoreWalletSyncedHeight])
     * is within [TIP_MARGIN] blocks of the scan tip — Swift's
     * `synced_height + margin >= tip`. The tip says how far the CLIENT got;
     * a wallet added behind it is still being scanned.
     */
    fun walletCaughtUp(walletSyncedHeight: Long, tipHeight: Int): Boolean =
        walletSyncedHeight >= 0 && walletSyncedHeight + TIP_MARGIN >= tipHeight

    /**
     * dash-spv's steady state for a fully synced client is
     * [SpvSyncState.WAIT_FOR_EVENTS] with the filter phase at its target
     * height; [SpvSyncState.SYNCED] is the transient window before it.
     * Either counts — anything else is a scan still running (or not
     * running at all, which the poll publishes as
     * [SpvSyncProgressData.EMPTY]: waiting, with no filter phase).
     */
    fun isSteadySyncState(progress: SpvSyncProgressData): Boolean =
        when (progress.overallState) {
            SpvSyncState.SYNCED -> true
            SpvSyncState.WAIT_FOR_EVENTS -> {
                val filters = progress.filters
                filters != null && filters.targetHeight > 0 && filters.currentHeight >= filters.targetHeight
            }
            SpvSyncState.WAITING_FOR_CONNECTIONS, SpvSyncState.SYNCING, SpvSyncState.ERROR -> false
        }

    /**
     * The scan tip: the filter phase's height (the wallet-relevant one),
     * falling back to the header tip when the filter phase is absent; null
     * when neither carries a height.
     */
    fun scanTipHeight(progress: SpvSyncProgressData): Int? {
        val height = progress.filters?.currentHeight?.takeIf { it > 0 }
            ?: progress.headers?.currentHeight?.takeIf { it > 0 }
            ?: return null
        return height.coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
    }
}

/**
 * The manager-side state of the TXO-store reconcile — the Kotlin twin of
 * Swift's `coreTxoReconcileInFlight` / `coreTxoReconcileLastRunAt` /
 * `coreTxoReconcileWasSteady` / `coreTxoReconcileEpoch`. Thread-safe: the
 * progress poll, scheduled runs and host calls all reach it.
 */
internal class TxoReconcileCoordinator(private val cadenceMs: Long = TxoReconcileGates.CADENCE_MS) {
    private val lock = Any()
    private var wasSteady = false
    private val inFlight = HashSet<String>()
    private val lastRunAtMs = HashMap<String, Long>()
    private val epoch = AtomicLong()

    /**
     * Bumped by manager close and by wallet deletion; a run captures it at
     * start and stops between pages once it moves.
     */
    val currentEpoch: Long get() = epoch.get()

    fun bumpEpoch() {
        epoch.incrementAndGet()
    }

    /**
     * Feed one progress read (every accepted read, not only a changed one:
     * it is the reconcile's only clock, and a quiet steady-state wallet's
     * progress does not change for the whole cadence). Returns the wallets
     * to run now — all of [walletKeys] on the rising edge into steady
     * state, and afterwards each wallet whose last run is at least the
     * cadence old (or that never ran) — never one already in flight. Each
     * returned wallet is stamped NOW, at schedule time, so the next tick
     * does not schedule it again while this run is still gating.
     */
    fun walletsDue(progress: SpvSyncProgressData, walletKeys: Collection<String>, nowMs: Long): List<String> =
        synchronized(lock) {
            val steady = TxoReconcileGates.isSteadySyncState(progress)
            val rising = steady && !wasSteady
            wasSteady = steady
            if (!steady) return@synchronized emptyList()
            walletKeys.filter { key ->
                if (key in inFlight) return@filter false
                val last = lastRunAtMs[key]
                val due = rising || last == null || nowMs - last >= cadenceMs
                if (due) lastRunAtMs[key] = nowMs
                due
            }
        }

    /** Claim the wallet for one run; false when a run is already in flight. */
    fun tryBegin(walletKey: String): Boolean = synchronized(lock) { inFlight.add(walletKey) }

    /** Release the claim [tryBegin] took. */
    fun end(walletKey: String) {
        synchronized(lock) { inFlight.remove(walletKey) }
    }

    /** Record a completed run of the wallet for the cadence. */
    fun markRun(walletKey: String, nowMs: Long) {
        synchronized(lock) { lastRunAtMs[walletKey] = nowMs }
    }

    /** Forget a deleted wallet's cadence. */
    fun forget(walletKey: String) {
        synchronized(lock) { lastRunAtMs.remove(walletKey) }
    }
}
