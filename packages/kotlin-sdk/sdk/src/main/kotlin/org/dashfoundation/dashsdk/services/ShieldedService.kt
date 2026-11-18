package org.dashfoundation.dashsdk.services

import androidx.room.withTransaction
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.channelFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceSnapshot
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceState
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
 * progress events (`on_shielded_sync_*` callbacks). Balances are numbers Rust
 * computes: [localBalance] keeps the last good local snapshot of each wallet
 * ([PlatformWalletManager.localShieldedBalanceSnapshot]) and reads it again
 * when a sync pass completes or the wallet's stored notes change; nothing
 * here adds up notes. Cumulative counters accumulate
 * across passes since [bind], mirroring Swift's `totalScanned` /
 * `totalNewNotes` / `syncCountSinceLaunch`.
 *
 * [bind] drives the Rust-side shielded bind (configure + bind, like the
 * Swift service) and then reflects the loop; start/stop stay on
 * [PlatformWalletManager].
 *
 * **Single UI mirror + multi-engine-bind** (port of the same design in
 * `ShieldedService.swift`): the service mirrors exactly ONE wallet — the
 * app-level first wallet — for the global Sync-status surface via [bind].
 * [bindEngine] is the additive companion used by
 * `AppContainer.rebindWalletScopedServices()` to engine-register EVERY
 * OTHER loaded wallet into the same network-scoped coordinator (no mirror
 * repoint). A single shielded sync pass then trial-decrypts against the
 * union of all wallets' viewing keys and routes note hits to each
 * wallet's own persister (SH-14/15/16 cross-wallet flows). Per-wallet
 * receive addresses and balances are read on demand
 * ([PlatformWalletManager.shieldedDefaultAddress], [localBalance]) rather
 * than from this singleton mirror.
 */
class ShieldedService(private val database: DashDatabase) {

    /**
     * Rolling shielded sync state — the Kotlin analogue of Swift's
     * `@Published` shielded scalars.
     *
     * @param isSyncing a pass is currently in flight.
     * @param shieldedBalance unspent balance of the wallet's ordinary
     *   accounts (credits) as Rust reported it in the last sync result. The
     *   balance to show is [shieldedBalance] (the flow): it is read from
     *   Rust's local snapshot, so it also follows sends between passes.
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

    private val _shieldedBalance = MutableStateFlow<Flow<Long?>>(emptyFlow())

    /**
     * The bound wallet's shielded balance (credits): [ordinaryBalance] of the
     * bound wallet. Empty until [bind].
     *
     * The inner flow emits `Long?`. Null means the balance is not known yet
     * (no local snapshot has been read, or the last bind failed); it is not
     * zero, and a collector should show it as unknown. Earlier versions
     * emitted a non-null `Long` summed from the stored notes.
     */
    val shieldedBalance: StateFlow<Flow<Long?>> = _shieldedBalance.asStateFlow()

    /** Whether shielded support is compiled into the native library. */
    val isAvailable: Boolean get() = Sdk.hasShielded()

    /** Last good local snapshot of each wallet, by hex wallet id. */
    private val localBalances =
        MutableStateFlow<Map<String, ShieldedLocalBalanceSnapshot>>(emptyMap())

    /** The read in flight for each wallet, so a newer one replaces it. */
    private val localBalanceReads = ConcurrentHashMap<String, Job>()

    /**
     * Counts the times every cached snapshot was dropped (another manager,
     * an unbind, a Clear). A read notes the count when it starts and its
     * result is kept only if the count is still the same, so a read that
     * began against an earlier manager or ledger cannot repopulate the cache.
     */
    private val localBalanceGeneration = AtomicLong(0)

    /**
     * [walletId]'s local shielded balance as Rust computes it, or null while
     * no snapshot is known (not bound yet, restoration incomplete, or the
     * first read has not finished). Emits the last good snapshot and reads a
     * new one when a sync pass completes for the wallet, after a bind, and
     * whenever the wallet's stored notes change (a send or a received note
     * rewrites them; the rows are only the signal, their values are not
     * used). A build without shielded support emits null.
     */
    fun localBalance(walletId: ByteArray): Flow<ShieldedLocalBalanceSnapshot?> {
        if (!isAvailable) return flowOf(null)
        return channelFlow {
            launch {
                database.shieldedDao().observeNotesByWallet(walletId)
                    .collect { refreshLocalBalance(walletId) }
            }
            cachedLocalBalance(walletId).collect { send(it) }
        }
    }

    /**
     * [walletId]'s shielded balance (credits): spendable credits of its
     * ordinary accounts, which leaves out identity accounts. The number is
     * Rust's ([ShieldedLocalBalanceSnapshot.ordinarySpendableCredits]); null
     * while it is not known. See [localBalance].
     */
    fun ordinaryBalance(walletId: ByteArray): Flow<Long?> =
        localBalance(walletId).map { it?.ordinarySpendableCredits }.distinctUntilChanged()

    /**
     * Credits a send from [account] of [walletId] can spend now, as Rust
     * reports them for that account; null while not known or when the
     * account is not bound. A screen that sends from one account shows this,
     * not the wallet's total. See [localBalance].
     */
    fun accountBalance(walletId: ByteArray, account: Int): Flow<Long?> =
        localBalance(walletId).map { it?.accounts?.get(account)?.spendableCredits }
            .distinctUntilChanged()

    /** The cached snapshot of [walletId], without starting a read. */
    internal fun cachedLocalBalance(walletId: ByteArray): Flow<ShieldedLocalBalanceSnapshot?> {
        val key = walletId.toHexKey()
        return localBalances.map { it[key] }.distinctUntilChanged()
    }

    /**
     * Read [walletId]'s local balance from Rust again and keep the result.
     * No-op while no manager is bound. Returns at once; the read runs on the
     * service scope and replaces an earlier read of the same wallet.
     */
    fun refreshLocalBalance(walletId: ByteArray) {
        // The generation is read before the manager. [bind] assigns a new
        // manager before it bumps the generation, so a read never pairs the
        // new generation with the manager it replaced.
        val generation = localBalanceGeneration.get()
        val mgr = manager ?: return
        val id = walletId.copyOf()
        launchLocalBalanceRead(id, generation) { mgr.localShieldedBalanceSnapshot(id) }
    }

    /**
     * Start one read of [walletId]'s local balance through [read], cancelling
     * the read of that wallet still in flight. The result is kept only while
     * [generation] is still the current one.
     */
    internal fun launchLocalBalanceRead(
        walletId: ByteArray,
        generation: Long = localBalanceGeneration.get(),
        read: suspend () -> ShieldedLocalBalanceState,
    ): Job {
        val key = walletId.toHexKey()
        val job = scope.launch(start = CoroutineStart.LAZY) {
            readLocalBalance(key, LOCAL_BALANCE_RETRY_DELAYS_MS, generation, read)
        }
        localBalanceReads.put(key, job)?.cancel()
        job.start()
        return job
    }

    /**
     * Read once, and again after each of [retryDelaysMs] while the read
     * throws. Rust refuses the read (busy) for as long as a scan holds the
     * shielded store, and both signals that start a read can arrive while
     * the scan still holds it; without these few retries the balance would
     * stay stale until the next pass. The last good snapshot is kept when
     * every attempt fails. A result is dropped, and the read ends, once the
     * cache was dropped after [generation] was noted.
     */
    internal suspend fun readLocalBalance(
        key: String,
        retryDelaysMs: List<Long>,
        generation: Long = localBalanceGeneration.get(),
        read: suspend () -> ShieldedLocalBalanceState,
    ) {
        var attempt = 0
        while (true) {
            val state = try {
                read()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                android.util.Log.d(TAG, "Local shielded balance read failed: ${e.message}")
                null
            }
            if (state != null) {
                // Checked inside the update, so a drop that lands between
                // the read and the write still wins.
                localBalances.update {
                    if (localBalanceGeneration.get() == generation) {
                        applyLocalBalanceRead(it, key, state)
                    } else {
                        it
                    }
                }
                return
            }
            if (localBalanceGeneration.get() != generation) return
            if (attempt >= retryDelaysMs.size) return
            delay(retryDelaysMs[attempt])
            attempt += 1
        }
    }

    private val _isBound = MutableStateFlow(false)

    /**
     * Whether the Rust-side shielded bind currently holds for this wallet —
     * observable so the UI recomposes when binding flips (Swift's
     * `@Published isBound`). Flips `true` only on a successful
     * [configureShielded][PlatformWalletManager.configureShielded] +
     * [bindShielded][PlatformWalletManager.bindShielded] pair, back to `false`
     * on a bind failure or a hard [unbind]. Distinct from [canResume]: a
     * failed re-bind leaves us NOT bound but still resumable.
     */
    val isBound: StateFlow<Boolean> = _isBound.asStateFlow()

    private val _canResume = MutableStateFlow(false)

    /**
     * Whether the service has stashed enough bind credentials (manager +
     * walletId + dbPath) to re-bind on demand — port of Swift's `canResume`.
     * Kept `true` across a bind failure AND across [clearLocalState] so the
     * "Clear" / "Sync Now" buttons stay usable even when the tree froze and
     * the Rust bind is momentarily down (exactly when Clear is needed). Only a
     * hard [unbind] (no wallet on the network) clears it. This — NOT the
     * transient [isBound] — is what the Clear button gates on.
     */
    val canResume: StateFlow<Boolean> = _canResume.asStateFlow()

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
        val sortedAccounts = accounts.distinct().sorted()

        // Cancel the prior subscriptions and zero the published mirror, but —
        // UNLIKE unbind() — RETAIN the bind credentials: assign the new set up
        // front so [canResume] (and therefore the Clear/Sync-Now buttons) stay
        // usable even if the Rust bind below fails. This mirrors Swift's
        // `ShieldedService.bind`, which sets `boundWalletId` before the `try`
        // and keeps it on failure; only [isBound] tracks the actual outcome.
        // The earlier port called `unbind()` here, which nulled the manager /
        // walletId / dbPath — so a re-bind whose `bindShielded` step threw
        // (e.g. the mnemonic resolver declined in the background) left the
        // service permanently unbound with no path back, disabling Clear while
        // the coordinator kept syncing. That was the on-device "Clear disabled
        // when needed" defect.
        eventJob?.cancel()
        eventJob = null
        pollJob?.cancel()
        pollJob = null
        val managerChanged = this.manager !== manager
        this.manager = manager
        if (managerChanged) {
            // Another manager is another network: its wallets' snapshots
            // say nothing about this one's. Dropped after the assignment;
            // see [refreshLocalBalance] for why the order matters.
            dropLocalBalances()
        }
        this.boundWalletId = walletId.copyOf()
        this.boundDbPath = dbPath
        _boundAccounts.value = sortedAccounts
        _canResume.value = true
        _isBound.value = false
        _state.value = ShieldedSyncState()
        _shieldedBalance.value = ordinaryBalance(walletId)

        try {
            manager.configureShielded(dbPath)
            manager.bindShielded(walletId, sortedAccounts)
            _isBound.value = true
            // The bind restored the stored ledger and no scan holds the
            // store yet: the read that normally succeeds at launch.
            refreshLocalBalance(walletId)
        } catch (e: Exception) {
            // A failed bind does not vouch for the ledger read before it:
            // the wallet's balance is not known until a bind succeeds.
            dropLocalBalance(walletId)
            // Stay resumable: credentials are retained above, so a later
            // bind() retry (or the Clear→re-bind path) picks up cleanly.
            android.util.Log.w(
                TAG,
                "Shielded bind failed (credentials retained for resume): ${e.message}",
                e,
            )
        }

        // Attach subscriptions regardless of bind outcome (Swift keeps
        // subscribing so a later successful retry / the running loop's events
        // update the mirror). Harmless when unbound: the coordinator emits no
        // events for an unregistered wallet and the balance flow just reflects
        // whatever rows persist.
        eventJob = scope.launch {
            manager.syncEvents.collect { event ->
                // A completed pass changed some wallet's ledger, the mirror's
                // or another's: read that wallet's balance again.
                if (event is WalletSyncEvent.ShieldedResult && changesLocalBalance(event)) {
                    refreshLocalBalance(event.walletId)
                }
                reduce(walletId, event)
            }
        }

        pollJob = scope.launch {
            while (true) {
                // Poll the PASS-IN-FLIGHT flag, not the loop-alive flag. The
                // background loop stays alive (isShieldedSyncRunning == true)
                // for its whole lifetime, so polling that pinned "Syncing…" on
                // forever and left the Clear button's `!isSyncing` gate never
                // satisfiable. Swift polls `isShieldedSyncing()` here for the
                // same reason.
                val syncing = runCatching { manager.isShieldedSyncing() }
                    .getOrDefault(false)
                if (syncing != _state.value.isSyncing) {
                    _state.update { it.copy(isSyncing = syncing) }
                }
                kotlinx.coroutines.delay(POLL_INTERVAL_MS)
            }
        }
    }

    /**
     * Register [walletId]'s shielded sub-wallet with the Rust coordinator
     * WITHOUT repointing this service's display mirror — port of Swift
     * `ShieldedService.bindEngine`.
     *
     * [bind] attaches the single UI mirror (bound wallet, counters,
     * subscriptions) to exactly one wallet — the app-level first wallet.
     * [bindEngine] is the additive companion: it engine-binds EVERY OTHER
     * loaded wallet into the same network-scoped coordinator so a single
     * shielded sync pass trial-decrypts against the union of all wallets'
     * viewing keys and routes note hits to each wallet's own persister.
     * Per-wallet receive addresses and balances are then read on demand
     * rather than from this singleton mirror: the address from
     * [PlatformWalletManager.shieldedDefaultAddress], a wallet's balance from
     * [ordinaryBalance] and one account's from [accountBalance]. Do not sum
     * a wallet's stored notes for a balance: they include the notes of
     * identity accounts, which an ordinary send cannot spend.
     *
     * Best-effort and independent per wallet: a missing mnemonic /
     * declined keystore read for one wallet logs and returns `false`
     * without affecting the others or the mirror. Idempotent — safe to
     * call every rebind pass ([PlatformWalletManager.configureShielded]
     * no-ops on the same path; [PlatformWalletManager.bindShielded]
     * replaces that wallet's registration).
     *
     * No "already bound" fast path on purpose (iOS lesson): the only
     * cheap probe, [PlatformWalletManager.shieldedDefaultAddress],
     * reflects the wallet-level sub-wallet binding — which SURVIVES
     * [PlatformWalletManager.clearShieldedStorage] (Clear drops only the
     * coordinator registrations; there is no sub-wallet unbind FFI).
     * Skipping on that signal would silently leave post-Clear wallets
     * unregistered (sync passes would never scan them again). Coordinator
     * registration has no cheap query, so we always re-bind; the mnemonic
     * read + ZIP-32 re-derivation is low-millisecond per wallet and
     * rebind fires are rare (wallet-set change, network switch, Clear).
     *
     * @return whether the engine registration succeeded; callers running
     *   a best-effort fleet pass may ignore it.
     */
    suspend fun bindEngine(
        manager: PlatformWalletManager,
        walletId: ByteArray,
        dbPath: String,
        accounts: List<Int> = listOf(0),
    ): Boolean {
        if (!isAvailable) return false
        val sortedAccounts = accounts.distinct().sorted()
        val walletIdHex4 = walletId.take(4).joinToString("") { "%02x".format(it) }
        return try {
            manager.configureShielded(dbPath)
            manager.bindShielded(walletId, sortedAccounts)
            android.util.Log.i(
                TAG,
                "Shielded engine-bound: walletId=$walletIdHex4… accounts=$sortedAccounts",
            )
            val id = walletId.copyOf()
            launchLocalBalanceRead(id) { manager.localShieldedBalanceSnapshot(id) }
            true
        } catch (e: Exception) {
            dropLocalBalance(walletId)
            android.util.Log.w(
                TAG,
                "Shielded engine-bind failed for walletId=$walletIdHex4…: ${e.message}",
                e,
            )
            false
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
     * Hard unbind — port of Swift `reset()`. Cancels subscriptions, DROPS the
     * stashed bind credentials (manager / walletId / dbPath / accounts), and
     * zeroes the published state including [isBound] and [canResume]. Used
     * when the active network genuinely has no wallet to bind (a network
     * switch), where there is nothing to resume to. NOT used by [bind]'s
     * re-bind path — that retains credentials so [canResume] survives a failed
     * re-bind (see [bind]).
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
        dropLocalBalances()
        _state.value = ShieldedSyncState()
        _isBound.value = false
        _canResume.value = false
    }

    /** Forget [walletId]'s cached snapshot and stop its read in flight. */
    internal fun dropLocalBalance(walletId: ByteArray) {
        val key = walletId.toHexKey()
        localBalanceReads.remove(key)?.cancel()
        localBalances.update { it - key }
    }

    /** Forget every cached snapshot and stop the reads in flight. */
    private fun dropLocalBalances() {
        // First, so a read that is about to write sees the new count.
        localBalanceGeneration.incrementAndGet()
        localBalanceReads.values.forEach { it.cancel() }
        localBalanceReads.clear()
        localBalances.value = emptyMap()
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
     * 2. **Room wipe — every wallet on THIS network** (INCLUDING
     *    `shielded_sync_states`, the per-subwallet watermark; leaving any
     *    would let [bind]'s `restore_for_wallet` re-seed a caught-up
     *    watermark and re-freeze the tree) in one transaction, and zero the
     *    published counters. Scoped to the active network's wallets
     *    (`walletDao().getByNetwork`), NOT global: [clearShieldedStorage]
     *    empties only this network's `shielded_tree_<network>.sqlite`, so
     *    wiping other networks' rows would drop notes/activity/watermarks
     *    whose Rust trees are still populated → data loss + Room↔Rust
     *    divergence on the next network switch. Within this network the wipe
     *    must cover EVERY wallet (not just the loaded/bound fleet — an
     *    unloaded same-network wallet's surviving watermark would restore a
     *    position ahead of the now-empty shared tree on re-bind and
     *    gate-skip every note, the exact Room↔SQLite watermark-divergence
     *    freeze this Clear exists to fix).
     * 3. **Re-bind + restart sync** — [clearShieldedStorage] dropped every
     *    coordinator registration and quiesced the sync loop, so nothing
     *    would resync until the next app relaunch otherwise. Re-run [bind]
     *    for the mirror wallet (re-registers the viewing keys on the
     *    now-empty coordinator, its `restore_for_wallet` finding no
     *    watermark → starts at index 0), then [bindEngine] every OTHER
     *    loaded wallet — mirror-bind failure does NOT skip the others
     *    ([bind] swallows its errors; the fleet pass runs regardless, the
     *    iOS best-effort lesson) — and restart the background loop, so the
     *    button alone triggers an in-session cold rebuild 0→N for the
     *    whole fleet.
     *
     * Fail-closed: if the Rust reset throws, the exception propagates and the
     * Room rows are left intact — the FFI's contract is that the host must
     * not drop its persistence while the shared tree may still be populated.
     * No-op when unbound.
     */
    suspend fun clearLocalState(db: DashDatabase = database) {
        val walletId = boundWalletId ?: return
        // Snapshot bind credentials up front so the re-bind at the end is
        // driven off a stable set (bind() retains credentials across its own
        // re-entry, but snapshotting keeps this flow independent of that).
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
        // The Rust reset emptied every wallet's ledger on this network, so
        // no cached balance is true any more. Dropped here, before the Room
        // work below can fail and leave the old numbers on screen; the
        // re-bind at the end reads the new ones.
        dropLocalBalances()
        android.util.Log.i(TAG, "Shielded clear: Rust store reset OK; wiping Room rows")

        // Wipe every wallet on THIS network, but NOT other networks'. `mgr`
        // is network-scoped and its shielded tree
        // (`shielded_tree_<network>.sqlite`) is per-network, so the Rust
        // reset above emptied only the ACTIVE network's SHARED tree. A
        // global deleteAll* would drop other networks' notes/activity/
        // watermarks while their Rust trees stay populated → Room/Rust
        // divergence + data loss on the next network switch. Scope to the
        // network via `walletDao().getByNetwork` — NOT `mgr.wallets`, which
        // only holds the successfully-loaded fleet: a same-network wallet
        // that failed restore still has Room rows, and a surviving watermark
        // would restore a position ahead of the now-empty tree on re-bind
        // and gate-skip every note (see doc, step 2).
        val networkWalletIds = db.walletDao()
            .getByNetwork(mgr.network.ffiValue)
            .map { it.walletId }
        db.withTransaction {
            for (wid in networkWalletIds) {
                db.shieldedDao().deleteActivityByWallet(wid)
                db.shieldedDao().deleteOutgoingNotesByWallet(wid)
                db.shieldedDao().deleteNotesByWallet(wid)
                db.shieldedDao().deleteSyncStatesByWallet(wid)
            }
        }
        _state.update { it.copy(shieldedBalance = 0, totalScanned = 0, totalNewNotes = 0) }

        // Re-bind + restart so the cold rebuild runs now, not just after a
        // relaunch. bind() re-runs configureShielded (idempotent same-path)
        // + bindShielded (re-registers on the now-empty coordinator; its
        // restore_for_wallet finds no watermark → starts at index 0).
        if (dbPath != null) {
            bind(mgr, walletId, dbPath, accounts)
            // Fleet recovery: clearShieldedStorage dropped EVERY wallet's
            // coordinator registration, and the mirror bind above restores
            // only one. Re-register every other loaded wallet so the next
            // pass scans the whole fleet (SH-14/15/16 survive an SH-12 run
            // without a relaunch). Runs regardless of the mirror bind's
            // outcome — bind() swallows its errors, and one wallet's
            // failure must not dark the others (iOS best-effort lesson).
            engineBindOtherWallets(
                allWalletIds = mgr.wallets.value.keys,
                mirrorWalletId = walletIdHex,
            ) { otherKey ->
                mgr.wallets.value[otherKey]?.let { other ->
                    bindEngine(mgr, other.walletId, dbPath)
                }
            }
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

        /** Waits between the attempts of one local balance read. */
        val LOCAL_BALANCE_RETRY_DELAYS_MS = listOf(300L, 1_000L, 3_000L)
    }
}

/**
 * Whether a wallet's sync result can have changed its local ledger: a pass
 * that ran and succeeded. A skipped wallet and a cooldown skip scanned
 * nothing.
 */
internal fun changesLocalBalance(result: WalletSyncEvent.ShieldedResult): Boolean =
    result.success && !result.skipped && !result.cooldownSkip

/**
 * The cache after one read of the wallet at [key]: a ready snapshot replaces
 * the entry, an unbound wallet loses it, and an incomplete restoration keeps
 * what was there, since it establishes nothing new.
 */
internal fun applyLocalBalanceRead(
    cache: Map<String, ShieldedLocalBalanceSnapshot>,
    key: String,
    state: ShieldedLocalBalanceState,
): Map<String, ShieldedLocalBalanceSnapshot> = when (state) {
    is ShieldedLocalBalanceState.Ready -> cache + (key to state.snapshot)
    ShieldedLocalBalanceState.Unbound -> cache - key
    ShieldedLocalBalanceState.RestoreIncomplete -> cache
}

private fun ByteArray.toHexKey(): String = joinToString("") { "%02x".format(it) }
