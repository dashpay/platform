package org.dashfoundation.dashsdk.services

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.syncStateScopeId
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent

/**
 * Reactive view of the Rust-owned platform-address (BLAST) sync loop —
 * port of `PlatformBalanceSyncService.swift`.
 *
 * The Swift service spreads its state across `@Published` scalars; here it
 * is a single sealed [PlatformSyncState] StateFlow (idle / syncing /
 * synced(height, timestamp) / error) fed by two sources exactly as Swift
 * does:
 *
 * 1. **Completion events** — [PlatformWalletManager.syncEvents] carries the
 *    per-wallet [WalletSyncEvent.PlatformAddressResult] and the
 *    [WalletSyncEvent.PlatformAddressPassCompleted] boundary from the native
 *    `on_platform_address_sync_completed_fn` callback.
 * 2. **Watermark rows** — the network-scoped
 *    `PlatformAddressesSyncStateEntity` row Rust's persister writes on each
 *    pass, read on [configure] for the startup snapshot (Swift's
 *    `loadCachedSyncState`).
 *
 * [totalPlatformBalance] / [activeAddressCount] are Room `Flow`s straight
 * off [org.dashfoundation.dashsdk.persistence.dao.PlatformAddressDao], the
 * analogue of Swift's `refreshBalanceSnapshot()` sums.
 *
 * This service REFLECTS the loop; it never starts/stops it (that stays on
 * [PlatformWalletManager], which wraps the Rust start/stop entry points).
 * The one exception is [manualSync], which forwards to the manager's
 * user-initiated `sync_now` wrapper — a request, not orchestration.
 */
class PlatformBalanceSyncService(private val database: DashDatabase) {

    /** Sealed reduction of the platform-address sync loop's state. */
    sealed interface PlatformSyncState {
        /** No pass has been observed and none is in flight. */
        data object Idle : PlatformSyncState

        /** A pass is running (mirrors `walletManager.platformAddressSyncIsSyncing`). */
        data object Syncing : PlatformSyncState

        /**
         * A pass completed successfully.
         *
         * @param syncHeight the incremental-sync watermark height.
         * @param syncTimestamp unix seconds of the watermark block (0 if unknown).
         */
        data class Synced(val syncHeight: Long, val syncTimestamp: Long) : PlatformSyncState

        /** The last pass reported a failure for at least one wallet. */
        data class Error(val message: String) : PlatformSyncState
    }

    private val scope = CoroutineScope(SupervisorJob())

    private val _state = MutableStateFlow<PlatformSyncState>(PlatformSyncState.Idle)

    /** Current reduced sync state. */
    val state: StateFlow<PlatformSyncState> = _state.asStateFlow()

    /**
     * Total platform-credit balance across all addresses (sum of non-zero
     * balances) — Swift's `totalPlatformBalance`. Live Room Flow.
     */
    val totalPlatformBalance: Flow<Long> =
        database.platformAddressDao().observeNonZeroBalances()
            .map { rows -> rows.sumOf { it.balance } }

    /** Count of addresses carrying a non-zero balance — Swift's `activeAddressCount`. */
    val activeAddressCount: Flow<Int> =
        database.platformAddressDao().observeNonZeroBalances()
            .map { it.size }

    private var manager: PlatformWalletManager? = null
    private var eventJob: Job? = null
    private var pollJob: Job? = null

    /**
     * Bind to [manager] and start reflecting its sync loop — port of Swift
     * `configure(...)`. Loads the cached watermark for the manager's network
     * (startup snapshot), then subscribes to completion events and a
     * liveness poll fallback. Re-configuring rebinds cleanly.
     */
    fun configure(manager: PlatformWalletManager) {
        reset()
        this.manager = manager

        // Startup snapshot from the persisted watermark (Swift's
        // loadCachedSyncState): a prior successful pass shows as Synced.
        scope.launch {
            val scopeId = syncStateScopeId(manager.network.ffiValue)
            database.platformAddressDao().getSyncState(scopeId)?.let { row ->
                if (_state.value is PlatformSyncState.Idle) {
                    _state.value = PlatformSyncState.Synced(row.syncHeight, row.syncTimestamp)
                }
            }
        }

        // Reduce completion events into the sealed state.
        eventJob = scope.launch {
            manager.syncEvents.collect { event -> reduce(event) }
        }

        // Liveness poll fallback at the manager's cadence — flips to Syncing
        // when a pass is running even if the completion event is missed.
        pollJob = scope.launch {
            while (true) {
                val running = runCatching { manager.isPlatformAddressSyncRunning() }
                    .getOrDefault(false)
                if (running && _state.value !is PlatformSyncState.Syncing) {
                    // Only enter Syncing from a settled state; don't clobber a
                    // freshly-published Synced/Error from the event path.
                    if (_state.value is PlatformSyncState.Idle) {
                        _state.value = PlatformSyncState.Syncing
                    }
                }
                kotlinx.coroutines.delay(POLL_INTERVAL_MS)
            }
        }
    }

    /**
     * Reduce one event. Per-wallet results settle the state
     * (Synced/Error); the pass boundary carries the authoritative timestamp.
     * `PlatformAddressResult` alone doesn't publish `Synced` until we have
     * the pass boundary, matching Swift's per-result-then-snapshot flow.
     */
    internal fun reduce(event: WalletSyncEvent) {
        when (event) {
            is WalletSyncEvent.PlatformAddressResult -> {
                if (!event.success) {
                    _state.value = PlatformSyncState.Error(
                        event.errorMessage ?: "platform-address sync failed",
                    )
                } else {
                    _state.value = PlatformSyncState.Synced(
                        syncHeight = event.newSyncHeight,
                        syncTimestamp = event.newSyncTimestamp,
                    )
                }
            }

            is WalletSyncEvent.PlatformAddressPassCompleted -> {
                // A zero-wallet pass still completed — settle out of Syncing.
                if (event.walletCount == 0 && _state.value is PlatformSyncState.Syncing) {
                    _state.value = PlatformSyncState.Synced(0, event.syncUnixSeconds)
                }
            }

            is WalletSyncEvent.Error -> {
                _state.value = PlatformSyncState.Error(event.message)
            }

            else -> Unit // shielded / generic events are not this service's concern
        }
    }

    /**
     * Request a single user-initiated pass — port of Swift `manualSync()`.
     * Flips to [PlatformSyncState.Syncing] optimistically; the completion
     * event settles it. Forwards to the manager (a request, not
     * orchestration). No-op when unconfigured.
     */
    suspend fun manualSync() {
        val m = manager ?: return
        _state.value = PlatformSyncState.Syncing
        try {
            m.startPlatformAddressSync()
        } catch (e: Exception) {
            _state.value = PlatformSyncState.Error(e.message ?: "manual sync failed")
        }
    }

    /**
     * Unbind and reset to [PlatformSyncState.Idle] — port of Swift
     * `reset()`. Cancels subscriptions but keeps the service reusable via a
     * later [configure].
     */
    fun reset() {
        eventJob?.cancel()
        eventJob = null
        pollJob?.cancel()
        pollJob = null
        manager = null
        _state.value = PlatformSyncState.Idle
    }

    /** Tear down the service scope permanently. */
    fun close() {
        scope.cancel()
    }

    private companion object {
        /** Liveness poll cadence — matches the manager's 1 Hz progress poll. */
        const val POLL_INTERVAL_MS = 1_000L
    }
}
