package org.dashfoundation.dashsdk.services

import androidx.room.withTransaction
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent

/**
 * Reactive view of the Rust-owned shielded (Orchard) sync loop — port of
 * `ShieldedService.swift`.
 *
 * Only functional when the native library was built with shielded support
 * ([Sdk.hasShielded]); [bind] is a no-op otherwise (matching Swift's
 * `isBound` gate) and [shieldedBalance] stays empty.
 *
 * State is a single [ShieldedSyncState] StateFlow (isSyncing, totalScanned,
 * totalNewNotes, syncCountSinceLaunch) fed by the manager's completion +
 * progress events (`on_shielded_sync_*` callbacks). [shieldedBalance] is a
 * Room `Flow` summing unspent notes for the bound wallet — the analogue of
 * Swift's CoreContentView unspent scan. Cumulative counters accumulate
 * across passes since [bind], mirroring Swift's `totalScanned` /
 * `totalNewNotes` / `syncCountSinceLaunch`.
 *
 * [bind] drives the Rust-side shielded bind (configure + bind, like the
 * Swift service) and then reflects the loop; start/stop stay on
 * [PlatformWalletManager].
 */
class ShieldedService(private val database: DashDatabase) {

    /**
     * Rolling shielded sync state — the Kotlin analogue of Swift's
     * `@Published` shielded scalars.
     *
     * @param isSyncing a pass is currently in flight.
     * @param shieldedBalance last observed unspent balance (credits); the
     *   authoritative live value is [shieldedBalance] (the Room Flow).
     * @param lastNewNotes new notes in the most recent pass.
     * @param lastNewlySpent notes newly spent in the most recent pass.
     * @param totalScanned cumulative encrypted notes scanned since [bind].
     * @param totalNewNotes cumulative decrypted notes since [bind].
     * @param syncCountSinceLaunch completed passes observed since [bind].
     * @param treeLeavesCommitted latest committed Orchard-tree leaf count.
     * @param treeTotalTarget on-chain leaf total (0 = indeterminate).
     */
    data class ShieldedSyncState(
        val isSyncing: Boolean = false,
        val shieldedBalance: Long = 0,
        val lastNewNotes: Int = 0,
        val lastNewlySpent: Int = 0,
        val totalScanned: Long = 0,
        val totalNewNotes: Long = 0,
        /**
         * Live in-pass cumulative scan counter (Rust progress callback) —
         * distinct from the lifetime [totalScanned], which only grows on
         * successful completion (← Swift `currentShieldedSyncScanned` vs
         * `totalScanned`, ShieldedService.swift:335/756).
         */
        val currentSyncScanned: Long = 0,
        val syncCountSinceLaunch: Int = 0,
        val treeLeavesCommitted: Long = 0,
        val treeTotalTarget: Long = 0,
    )

    private val scope = CoroutineScope(SupervisorJob())

    private val _state = MutableStateFlow(ShieldedSyncState())

    /** Current rolling shielded sync state. */
    val state: StateFlow<ShieldedSyncState> = _state.asStateFlow()

    private var manager: PlatformWalletManager? = null
    private var boundWalletId: ByteArray? = null
    /**
     * Commitment-tree SQLite path last passed to [bind]. Retained so
     * [clearLocalState] can re-bind after the Rust-side reset drops every
     * coordinator registration — the cold rebuild can't start until the
     * wallet re-registers its viewing keys on the (now empty) coordinator.
     */
    private var boundDbPath: String? = null
    private var eventJob: Job? = null
    private var pollJob: Job? = null

    private val _boundAccounts = MutableStateFlow<List<Int>>(emptyList())

    /** ZIP-32 account indices bound for the current wallet (Swift `boundAccounts`). */
    val boundAccounts: StateFlow<List<Int>> = _boundAccounts.asStateFlow()

    private val _shieldedBalance = MutableStateFlow<Flow<Long>>(emptyFlow())

    /**
     * Live unspent shielded balance (credits) for the bound wallet, summed
     * from `shielded_notes` where `isSpent = 0` — Swift's CoreContentView
     * balance scan. Empty until [bind].
     */
    val shieldedBalance: StateFlow<Flow<Long>> = _shieldedBalance.asStateFlow()

    /** Whether shielded support is compiled into the native library. */
    val isAvailable: Boolean get() = Sdk.hasShielded()

    /** Whether the service is currently bound to a wallet. */
    val isBound: Boolean get() = boundWalletId != null

    /**
     * Bind to [manager]'s [walletId] shielded sub-wallet and start
     * reflecting the loop — port of Swift
     * `ShieldedService.bind(walletManager:walletId:network:resolver:accounts:)`
     * (`SwiftExampleApp/Core/Services/ShieldedService.swift`). No-op
     * (leaves state at its defaults) when shielded support is absent.
     * Re-binding rebinds cleanly and resets the cumulative counters.
     *
     * Like iOS, the service drives the Rust-side bind itself:
     * [PlatformWalletManager.configureShielded] with [dbPath] (idempotent
     * at the path level — first call opens the per-network commitment-tree
     * SQLite file, same-path repeats no-op) then
     * [PlatformWalletManager.bindShielded] (resolver-driven mnemonic
     * lookup, ZIP-32 derivation per [accounts]). The pair is best-effort:
     * on failure (no mnemonic in the store, biometric prompt declined,
     * etc.) the service stays unbound — logged, since [ShieldedSyncState]
     * carries no error field (Swift records `lastError`) — and nothing is
     * thrown; a later [bind] retry picks up cleanly.
     *
     * @param dbPath absolute path of the per-network commitment-tree
     *   SQLite file (Swift `ShieldedService.dbPath(for:)`); the caller
     *   supplies it because the service holds no Android `Context`.
     * @param accounts ZIP-32 account indices to bind (deduped + sorted,
     *   matching Swift); published as [boundAccounts] on success.
     */
    suspend fun bind(
        manager: PlatformWalletManager,
        walletId: ByteArray,
        dbPath: String,
        accounts: List<Int> = listOf(0),
    ) {
        if (!isAvailable) return
        unbind()
        val sortedAccounts = accounts.distinct().sorted()
        try {
            manager.configureShielded(dbPath)
            manager.bindShielded(walletId, sortedAccounts)
        } catch (e: Exception) {
            android.util.Log.w(TAG, "Shielded bind failed: ${e.message}", e)
            return
        }
        this.manager = manager
        this.boundWalletId = walletId.copyOf()
        this.boundDbPath = dbPath
        _boundAccounts.value = sortedAccounts
        _state.value = ShieldedSyncState()
        _shieldedBalance.value = database.shieldedDao()
            .observeUnspentNotesByWallet(walletId)
            .map { notes -> notes.sumOf { it.value } }

        eventJob = scope.launch {
            manager.syncEvents.collect { event -> reduce(walletId, event) }
        }

        pollJob = scope.launch {
            while (true) {
                val running = runCatching { manager.isShieldedSyncRunning() }
                    .getOrDefault(false)
                if (running != _state.value.isSyncing) {
                    _state.update { it.copy(isSyncing = running) }
                }
                kotlinx.coroutines.delay(POLL_INTERVAL_MS)
            }
        }
    }

    /**
     * Reduce one event for the bound wallet. Per-wallet completion results
     * accumulate the cumulative counters; a `cooldownSkip` preserves the
     * prior cache (its numeric fields are zero). Progress / tree events
     * update the live counters mid-pass.
     */
    internal fun reduce(walletId: ByteArray, event: WalletSyncEvent) {
        when (event) {
            is WalletSyncEvent.ShieldedResult -> {
                if (!event.walletId.contentEquals(walletId)) return
                if (event.cooldownSkip || event.skipped) return
                if (!event.success) return
                _state.update { s ->
                    s.copy(
                        shieldedBalance = event.balance,
                        lastNewNotes = event.newNotes,
                        lastNewlySpent = event.newlySpent,
                        totalScanned = s.totalScanned + event.totalScanned,
                        totalNewNotes = s.totalNewNotes + event.newNotes,
                        currentSyncScanned = 0,
                    )
                }
            }

            is WalletSyncEvent.ShieldedPassCompleted -> {
                _state.update { it.copy(syncCountSinceLaunch = it.syncCountSinceLaunch + 1) }
            }

            is WalletSyncEvent.ShieldedProgress -> {
                _state.update { it.copy(currentSyncScanned = event.cumulativeScanned) }
            }

            is WalletSyncEvent.ShieldedTreeProgress -> {
                _state.update {
                    it.copy(
                        treeLeavesCommitted = event.leavesCommitted,
                        treeTotalTarget = event.totalTarget,
                    )
                }
            }

            else -> Unit
        }
    }

    /**
     * Unbind — port of Swift `reset()` / the soft-cleanup path. Cancels
     * subscriptions, clears the bound wallet, and zeroes the published
     * state, keeping the service reusable via a later [bind].
     */
    fun unbind() {
        eventJob?.cancel()
        eventJob = null
        pollJob?.cancel()
        pollJob = null
        manager = null
        boundWalletId = null
        boundDbPath = null
        _boundAccounts.value = emptyList()
        _shieldedBalance.value = emptyFlow()
        _state.value = ShieldedSyncState()
    }

    /**
     * Reset shielded state for the bound wallet — port of Swift
     * `ShieldedService.clearLocalState(modelContext:)`. Two ordered halves:
     *
     * 1. **Rust-side reset FIRST** via
     *    [PlatformWalletManager.clearShieldedStorage]
     *    (`platform_wallet_manager_shielded_clear`): quiesce the background
     *    sync loop, drop every wallet registration from the network-scoped
     *    coordinator, empty the shared commitment tree, and reset the
     *    caught-up cooldown. This MUST run before the Room wipe — the
     *    coordinator keeps every bound wallet registered, so without it the
     *    next sync pass's persister callback immediately re-creates the very
     *    rows we're about to delete. It also resets the on-disk tree size, so
     *    the post-clear resync cold-rebuilds from index 0 instead of
     *    gate-skipping every re-downloaded position against a stale tree
     *    (the frozen-tree / zero-notes desync this Clear exists to fix).
     * 2. **Room wipe** — delete the wallet's shielded rows (INCLUDING
     *    `shielded_sync_states`, the per-subwallet watermark; leaving it
     *    would let [bind]'s `restore_for_wallet` re-seed a caught-up
     *    watermark and re-freeze the tree) in one transaction, and zero the
     *    published counters.
     * 3. **Re-bind + restart sync** — [clearShieldedStorage] dropped every
     *    coordinator registration and quiesced the sync loop, so nothing
     *    would resync until the next app relaunch otherwise. Re-run [bind]
     *    (re-registers the viewing keys on the now-empty coordinator, its
     *    `restore_for_wallet` finding no watermark → starts at index 0) and
     *    restart the background loop, so the button alone triggers an
     *    in-session cold rebuild 0→N. Mirrors Swift's
     *    "keep bind credentials, re-bind on next sync" intent, done eagerly.
     *
     * Fail-closed: if the Rust reset throws, the exception propagates and the
     * Room rows are left intact — the FFI's contract is that the host must
     * not drop its persistence while the shared tree may still be populated.
     * No-op when unbound.
     */
    suspend fun clearLocalState(db: DashDatabase = database) {
        val walletId = boundWalletId ?: return
        // Snapshot bind credentials BEFORE the wipe: bind() → unbind() nulls
        // these, and we need them to re-bind at the end.
        val mgr = manager
        val dbPath = boundDbPath
        val accounts = _boundAccounts.value.ifEmpty { listOf(0) }

        // A bound service must have a manager (both are set together in
        // bind, cleared together in unbind). If it doesn't, the Rust-side
        // reset can't run — fail LOUDLY rather than silently wiping only
        // Room and leaving the on-disk tree at its full size (which would
        // re-freeze the tree on the next cold resync).
        if (mgr == null) {
            error("Shielded clear: bound wallet has no manager — cannot reset the Rust store")
        }

        // Rust-side reset FIRST (empties the on-disk commitment tree +
        // watermarks, durably). This THROWS on failure (native FFI error →
        // DashSDKException), and we deliberately do NOT catch it: the Room
        // wipe below must not run unless the tree was truly reset, else the
        // host drops its rows while the shared tree stays populated and the
        // next resync gate-skips every position. Fail-closed by propagation.
        val walletIdHex = walletId.joinToString("") { "%02x".format(it) }
        android.util.Log.i(TAG, "Shielded clear: resetting Rust store for $walletIdHex")
        mgr.clearShieldedStorage()
        android.util.Log.i(TAG, "Shielded clear: Rust store reset OK; wiping Room rows")

        db.withTransaction {
            db.shieldedDao().deleteActivityByWallet(walletId)
            db.shieldedDao().deleteOutgoingNotesByWallet(walletId)
            db.shieldedDao().deleteNotesByWallet(walletId)
            db.shieldedDao().deleteSyncStatesByWallet(walletId)
        }
        _state.update { it.copy(shieldedBalance = 0, totalScanned = 0, totalNewNotes = 0) }

        // Re-bind + restart so the cold rebuild runs now, not just after a
        // relaunch. bind() re-runs configureShielded (idempotent same-path)
        // + bindShielded (re-registers on the now-empty coordinator; its
        // restore_for_wallet finds no watermark → starts at index 0).
        if (dbPath != null) {
            bind(mgr, walletId, dbPath, accounts)
            val notRunning = runCatching { !mgr.isShieldedSyncRunning() }.getOrDefault(false)
            if (isAvailable && notRunning) {
                runCatching { mgr.startShieldedSync() }.onFailure {
                    android.util.Log.w(TAG, "restart shielded sync after clear failed: ${it.message}", it)
                }
            }
        } else {
            android.util.Log.w(TAG, "Shielded clear: no dbPath retained; skipping re-bind (will resync on relaunch)")
        }
    }

    /** Tear down the service scope permanently. */
    fun close() {
        scope.cancel()
    }

    private companion object {
        const val TAG = "ShieldedService"
        const val POLL_INTERVAL_MS = 1_000L
    }
}
