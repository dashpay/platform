package org.dashfoundation.dashsdk.services

import androidx.room.withTransaction
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.buffer
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.ShieldedBalanceUpdate
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceSnapshot
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceState
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.dashfoundation.dashsdk.wallet.shieldedBalanceKey

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
 * progress events (`on_shielded_sync_*` callbacks). Balances are the ones
 * Rust announces ([PlatformWalletManager.shieldedBalances]): [localBalance],
 * [ordinaryBalance] and [accountBalance] pass them on per wallet, and nothing
 * here reads or adds up a balance. Cumulative counters accumulate
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
     *   balance to show is [shieldedBalance] (the flow), which follows every
     *   change Rust announces, sends between passes included.
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

    /** Orders [bind] and [unbind]; see [BindingTurns]. */
    private val bindingTurns = BindingTurns()

    /** See [withLedgerLifecycle]. */
    private val ledgerLifecycle = Mutex()

    /**
     * Run [step] with no other ledger lifecycle step in flight. Two things
     * take it: a bind's native calls, which restore the wallet's stored
     * notes and watermark from Room into Rust, and a Clear from its Rust
     * reset to the end of its Room wipe. So a bind cannot restore rows into
     * the store a Clear just emptied and then lose them to that Clear's
     * wipe, which would leave Rust holding notes Room no longer has. A bind
     * that arrives during a Clear waits and then restores from the wiped
     * tables.
     */
    internal suspend fun <T> withLedgerLifecycle(step: suspend () -> T): T =
        ledgerLifecycle.withLock { step() }
    private var pollJob: Job? = null

    private val _boundAccounts = MutableStateFlow<List<Int>>(emptyList())

    /** ZIP-32 account indices bound for the current wallet (Swift `boundAccounts`). */
    val boundAccounts: StateFlow<List<Int>> = _boundAccounts.asStateFlow()

    private val _shieldedBalance = MutableStateFlow<Flow<Long?>>(emptyFlow())

    /**
     * The bound wallet's shielded balance (credits): [ordinaryBalance] of the
     * bound wallet. Empty until [bind].
     *
     * The inner flow emits `Long?`. Null means the balance is not known (not
     * announced yet, restoration incomplete, or its amount has no evidence
     * behind it); it is not zero, and a collector should show it as unknown.
     * Earlier versions emitted a non-null `Long` summed from the stored
     * notes.
     */
    val shieldedBalance: StateFlow<Flow<Long?>> = _shieldedBalance.asStateFlow()

    /**
     * Where the balance updates come from: the bound manager's
     * [PlatformWalletManager.shieldedBalances], or nothing while no manager
     * is bound. A flow of flows so that a collector follows a rebind to
     * another manager (another network).
     */
    internal val balanceUpdates =
        MutableStateFlow<Flow<Map<String, ShieldedBalanceUpdate>>>(flowOf(emptyMap()))

    /**
     * [walletId]'s local shielded balances as Rust last announced them, or
     * null while there are no numbers: the wallet was not announced yet, it
     * is not bound, or its restoration is incomplete. Rust announces the
     * state whenever it has changed (after a scan ends, a spend reserves or
     * releases notes, a bind, a Clear), so a collector needs no refresh of
     * its own. An update is the latest state Rust read, not one per change:
     * notes reserved and released again before Rust reads produce none.
     */
    @OptIn(ExperimentalCoroutinesApi::class)
    fun localBalance(walletId: ByteArray): Flow<ShieldedLocalBalanceSnapshot?> {
        val key = shieldedBalanceKey(walletId)
        return balanceUpdates
            .flatMapLatest { it }
            // No buffer between the sources and the collector: a value of
            // the source that was switched away from must not be handed
            // over after the switch.
            .buffer(Channel.RENDEZVOUS)
            .map { updates -> (updates[key]?.state as? ShieldedLocalBalanceState.Ready)?.snapshot }
            .distinctUntilChanged()
    }

    /**
     * [walletId]'s shielded balance (credits): spendable credits of its
     * ordinary accounts, which leaves out identity accounts. The number is
     * Rust's ([ShieldedLocalBalanceSnapshot.ordinaryBalance]); null while it
     * is not known, which is not zero.
     */
    fun ordinaryBalance(walletId: ByteArray): Flow<Long?> =
        localBalance(walletId).map { it?.ordinaryBalance }.distinctUntilChanged()

    /**
     * Credits a send from [account] of [walletId] can spend now, as Rust
     * reports them for that account ([ShieldedLocalBalanceSnapshot.accountBalance]);
     * null while not known or when the account is not bound. A screen that
     * sends from one account shows this, not the wallet's total.
     */
    fun accountBalance(walletId: ByteArray, account: Int): Flow<Long?> =
        localBalance(walletId).map { it?.accountBalance(account) }.distinctUntilChanged()

    /** Whether shielded support is compiled into the native library. */
    val isAvailable: Boolean get() = Sdk.hasShielded()

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
        bindTurn(manager, walletId, dbPath, accounts)
    }

    /**
     * [bind], returning its turn when this bind is still the service's latest
     * as it ends, and null otherwise. A bind suspends in its native calls;
     * when another bind or an [unbind] took a later turn meanwhile, this one
     * stops without attaching its subscriptions, so it cannot point the
     * service back at a manager that was replaced.
     *
     * [onlyIf] is checked under the lock that starts the turn: when it is
     * false nothing is installed and null is returned. [clearLocalState]
     * uses it so that its re-bind starts only while the manager it cleared
     * is still the bound one, with no gap between that check and the bind.
     */
    private suspend fun bindTurn(
        manager: PlatformWalletManager,
        walletId: ByteArray,
        dbPath: String,
        accounts: List<Int>,
        onlyIf: () -> Boolean = { true },
    ): Long? {
        if (!isAvailable) return null
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
        val turn = bindingTurns.beginIf(onlyIf) { stillThisTurn ->
            eventJob?.cancel()
            eventJob = null
            pollJob?.cancel()
            pollJob = null
            this.manager = manager
            this.boundWalletId = walletId.copyOf()
            this.boundDbPath = dbPath
            // Each write below can wake a collector on this thread, and a
            // collector may call unbind() or bind() from there, taking a
            // later turn inside this one. What that turn published must
            // stand, so the rest of this install is skipped once it
            // happened.
            if (stillThisTurn()) _boundAccounts.value = sortedAccounts
            if (stillThisTurn()) _canResume.value = true
            if (stillThisTurn()) _isBound.value = false
            if (stillThisTurn()) _state.value = ShieldedSyncState()
            if (stillThisTurn()) balanceUpdates.value = manager.shieldedBalances
            if (stillThisTurn()) _shieldedBalance.value = ordinaryBalance(walletId)
        } ?: return null

        var bound = false
        try {
            val ran = withLedgerLifecycle {
                // A later bind may have gone through here while this one
                // waited for its turn at the ledger. Its registration (its
                // accounts) is the one that must stand, so this bind leaves
                // the native side alone.
                if (bindingTurns.isLatest(turn)) {
                    manager.configureShielded(dbPath)
                    manager.bindShielded(walletId, sortedAccounts)
                    true
                } else {
                    false
                }
            }
            if (!ran) return null
            bound = true
        } catch (e: Exception) {
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
        // whatever rows persist. Only the latest bind attaches them: one
        // that a later bind or an unbind overtook while it was suspended
        // above leaves the service as the later turn set it.
        val finished = bindingTurns.finish(turn) { stillThisTurn ->
            _isBound.value = bound
            // A collector of [isBound] may have answered that write with an
            // unbind or a bind on this thread. The jobs below belong to this
            // turn and are not started for a service that has moved on.
            if (!stillThisTurn()) return@finish
            // Both jobs write [state] only while this bind is the latest
            // turn: cancelling a job does not stop a write it has already
            // reached, and that write must not land on the state an unbind
            // or a later bind has reset.
            eventJob = scope.launch {
                manager.syncEvents.collect { event ->
                    bindingTurns.finish(turn) { reduce(walletId, event) }
                }
            }
            pollJob = scope.launch { pollSyncing(manager, turn) }
        }
        return turn.takeIf { finished }
    }

    /**
     * Mirror whether a sync pass is in flight into [state], once a second,
     * for as long as [turn] is the latest binding turn.
     */
    private suspend fun pollSyncing(manager: PlatformWalletManager, turn: Long) {
        while (true) {
            // Poll the PASS-IN-FLIGHT flag, not the loop-alive flag. The
            // background loop stays alive (isShieldedSyncRunning == true)
            // for its whole lifetime, so polling that pinned "Syncing…" on
            // forever and left the Clear button's `!isSyncing` gate never
            // satisfiable. Swift polls `isShieldedSyncing()` here for the
            // same reason.
            val syncing = runCatching { manager.isShieldedSyncing() }
                .getOrDefault(false)
            bindingTurns.finish(turn) {
                if (syncing != _state.value.isSyncing) {
                    _state.update { it.copy(isSyncing = syncing) }
                }
            }
            kotlinx.coroutines.delay(POLL_INTERVAL_MS)
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
            withLedgerLifecycle {
                manager.configureShielded(dbPath)
                manager.bindShielded(walletId, sortedAccounts)
            }
            android.util.Log.i(
                TAG,
                "Shielded engine-bound: walletId=$walletIdHex4… accounts=$sortedAccounts",
            )
            true
        } catch (e: Exception) {
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
        bindingTurns.begin { _ -> unbindNow() }
    }

    private fun unbindNow() {
        eventJob?.cancel()
        eventJob = null
        pollJob?.cancel()
        pollJob = null
        manager = null
        boundWalletId = null
        boundDbPath = null
        _boundAccounts.value = emptyList()
        // Not an empty flow: a collector that follows this to its next
        // inner flow must receive "not known", or it keeps showing the
        // balance of the wallet that was unbound.
        _shieldedBalance.value = flowOf(null)
        balanceUpdates.value = flowOf(emptyMap())
        _state.value = ShieldedSyncState()
        _isBound.value = false
        _canResume.value = false
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
        // The binding this Clear is for: its turn and its credentials, read
        // together. A bind or unbind that begins later makes the turn stale,
        // which is how the steps below know the service's mirror has moved
        // on (to another wallet or another manager) and must not be touched.
        val binding = bindingTurns.read {
            ClearedBinding(boundWalletId, manager, boundDbPath, _boundAccounts.value)
        }
        val startTurn = binding.first
        val walletId = binding.second.walletId ?: return
        val mgr = binding.second.manager
        val dbPath = binding.second.dbPath

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
        //
        // The reset and the Room wipe run as one ledger lifecycle step: no
        // bind can restore the still-present rows into the emptied store in
        // between (see [withLedgerLifecycle]).
        val walletIdHex = walletId.joinToString("") { "%02x".format(it) }
        android.util.Log.i(TAG, "Shielded clear: resetting Rust store for $walletIdHex")
        withLedgerLifecycle {
            // Once the reset has started, the wipe must follow even if the
            // caller (a screen that was left) is cancelled: a reset whose
            // wipe never ran would let the next bind restore the old
            // watermark into the emptied tree.
            withContext(NonCancellable) { resetAndWipe(mgr, db) }
        }

        // The counters belong to the binding that was cleared. If a later
        // bind took over meanwhile, they are that binding's now.
        bindingTurns.finish(startTurn) {
            _state.update { it.copy(shieldedBalance = 0, totalScanned = 0, totalNewNotes = 0) }
        }

        // Re-bind + restart so the cold rebuild runs now, not just after a
        // relaunch. bind() re-runs configureShielded (idempotent same-path)
        // + bindShielded (re-registers on the now-empty coordinator; its
        // restore_for_wallet finds no watermark → starts at index 0).
        //
        // The reset dropped every registration on `mgr`, so the binding the
        // service has NOW is the one to bind again, with its own wallet and
        // accounts: the one this Clear started with, or one made on the same
        // manager while the Clear was suspended. Each attempt reads the
        // binding with its turn and starts only if that turn is still the
        // latest, under the lock that starts the bind, so a bind that slips
        // in between is never overwritten with what was read before it; the
        // next attempt reads that newer binding instead. If the service
        // follows another manager (another network) or was unbound, the
        // mirror is left alone.
        if (dbPath == null) {
            android.util.Log.w(TAG, "Shielded clear: no dbPath retained; skipping re-bind (will resync on relaunch)")
            return
        }
        for (attempt in 0 until CLEAR_REBIND_ATTEMPTS) {
            val (readTurn, current) = bindingTurns.read {
                ClearedBinding(boundWalletId, manager, boundDbPath, _boundAccounts.value)
            }
            val currentWallet = current.walletId
            if (current.manager !== mgr || currentWallet == null) break
            val rebound = bindTurn(
                mgr,
                currentWallet,
                current.dbPath ?: dbPath,
                current.accounts.ifEmpty { listOf(0) },
            ) { bindingTurns.isLatest(readTurn) }
            if (rebound != null) break
        }

        // Fleet recovery: clearShieldedStorage dropped EVERY wallet's
        // coordinator registration, and a mirror bind restores only one.
        // Re-register every other loaded wallet so the next pass scans the
        // whole fleet (SH-14/15/16 survive an SH-12 run without a
        // relaunch), and restart the sync loop the reset stopped. This does
        // not depend on who bound the mirror: a bind that overtook the
        // re-bind above registers one wallet and restarts nothing, so the
        // rest is still this Clear's to do. [bindEngine] writes nothing to
        // the service. Skipped only when the service left this manager.
        val mirrorNow = bindingTurns.read { boundWalletId.takeIf { manager === mgr } }.second
        if (bindingTurns.read { manager !== mgr }.second) {
            android.util.Log.i(TAG, "Shielded clear: the service left this manager; no fleet recovery")
            return
        }
        engineBindOtherWallets(
            allWalletIds = mgr.wallets.value.keys,
            mirrorWalletId = mirrorNow?.joinToString("") { "%02x".format(it) }.orEmpty(),
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
    }

    /**
     * The two halves of a Clear that must not be separated: the Rust reset,
     * then the Room wipe. Called inside [withLedgerLifecycle].
     */
    private suspend fun resetAndWipe(mgr: PlatformWalletManager, db: DashDatabase) {
        mgr.clearShieldedStorage()
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
    }

    /** Tear down the service scope permanently. */
    fun close() {
        scope.cancel()
    }

    private companion object {
        const val TAG = "ShieldedService"
        const val POLL_INTERVAL_MS = 1_000L

        /** Times a Clear re-reads the binding when a bind slips in before its re-bind. */
        const val CLEAR_REBIND_ATTEMPTS = 3
    }
}

/** What [ShieldedService.clearLocalState] reads of the binding it clears. */
private class ClearedBinding(
    val walletId: ByteArray?,
    val manager: PlatformWalletManager?,
    val dbPath: String?,
    val accounts: List<Int>,
)

/**
 * Orders the binds and unbinds of one [ShieldedService]. Each takes a turn.
 * What a turn installs at its start runs under one lock, so two turns never
 * interleave their writes, and what a bind attaches at its end runs only if
 * no later turn began meanwhile. A bind suspends between the two (it calls
 * into native code), which is where a later bind or an unbind can overtake
 * it.
 */
internal class BindingTurns {
    private var latest = 0L

    /**
     * Take the next turn and run [install] as its first step. No other
     * thread's turn interleaves with it. [install] is handed a check that
     * says whether its turn is still the latest: an observer it wakes can
     * take a later turn on this same thread (the lock is reentrant), and
     * [install] must not write over what that turn set.
     */
    fun begin(install: (stillThisTurn: () -> Boolean) -> Unit): Long = synchronized(this) {
        // Fixed before [install] runs, so this call returns its own turn
        // whatever a nested turn does to [latest].
        val turn = latest + 1
        latest = turn
        install { latest == turn }
        turn
    }

    /**
     * [begin], but only if [allowed] holds. [allowed] is read under the same
     * lock, so no other turn can begin between the check and this one; when
     * it is false no turn is taken and null is returned.
     */
    fun beginIf(
        allowed: () -> Boolean,
        install: (stillThisTurn: () -> Boolean) -> Unit,
    ): Long? = synchronized(this) {
        if (!allowed()) {
            null
        } else {
            val turn = latest + 1
            latest = turn
            install { latest == turn }
            turn
        }
    }

    /** Whether no turn began after [turn]. */
    fun isLatest(turn: Long): Boolean = synchronized(this) { turn == latest }

    /**
     * The latest turn together with what [read] returns, read in one step so
     * the value is the one that turn installed.
     */
    fun <T> read(read: () -> T): Pair<Long, T> = synchronized(this) { latest to read() }

    /**
     * Run [finish] if [turn] is still the latest, and say whether it was and
     * still is once [finish] returned. [finish] runs under the same lock as
     * every turn's first step and is handed the same check as an install:
     * a collector it wakes can take a later turn on this thread, after which
     * it must write nothing more.
     */
    fun finish(turn: Long, finish: (stillThisTurn: () -> Boolean) -> Unit): Boolean =
        synchronized(this) {
            if (turn != latest) {
                false
            } else {
                finish { latest == turn }
                latest == turn
            }
        }
}
