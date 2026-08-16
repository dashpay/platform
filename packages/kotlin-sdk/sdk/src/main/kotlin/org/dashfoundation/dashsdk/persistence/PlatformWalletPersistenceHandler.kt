package org.dashfoundation.dashsdk.persistence

import android.util.Log
import androidx.room.withTransaction
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExecutorCoroutineDispatcher
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.AccountSpecData
import org.dashfoundation.dashsdk.ffi.ContactProfileRestoreData
import org.dashfoundation.dashsdk.ffi.ContactRequestRestoreData
import org.dashfoundation.dashsdk.ffi.CoreAddressPoolRestoreData
import org.dashfoundation.dashsdk.ffi.CoreAddressRestoreData
import org.dashfoundation.dashsdk.ffi.CoreTxRecordData
import org.dashfoundation.dashsdk.ffi.IdentityKeyRestoreData
import org.dashfoundation.dashsdk.ffi.IdentityRestoreData
import org.dashfoundation.dashsdk.ffi.PaymentRestoreData
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.ffi.PlatformAddressBalanceRestoreData
import org.dashfoundation.dashsdk.ffi.ProviderSpecialTxRestoreData
import org.dashfoundation.dashsdk.ffi.ShieldedActivityData
import org.dashfoundation.dashsdk.ffi.ShieldedNoteData
import org.dashfoundation.dashsdk.ffi.UtxoRestoreData
import org.dashfoundation.dashsdk.ffi.ShieldedOutgoingNoteData
import org.dashfoundation.dashsdk.ffi.ShieldedSyncStateData
import org.dashfoundation.dashsdk.ffi.ShieldedViewingKeyData
import org.dashfoundation.dashsdk.ffi.TrackedAssetLockRestoreData
import org.dashfoundation.dashsdk.ffi.UnresolvedAssetLockTxRecordData
import org.dashfoundation.dashsdk.ffi.WalletRestoreData
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayIgnoredSenderEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DpnsNameEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.InvitationEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedOutgoingNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.UInt64Value
import org.dashfoundation.dashsdk.persistence.entities.PendingInputEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionAccountInvolvementEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import java.util.concurrent.Executors

/**
 * Android port of
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletPersistenceHandler.swift`.
 *
 * Receives the platform-wallet-ffi persistence callbacks (via the JNI
 * trampolines in `rs-unified-sdk-jni/src/persistence.rs`) and writes them
 * into Room. **The handler only persists and loads — zero orchestration
 * decisions live here** (see `packages/kotlin-sdk/CLAUDE.md`).
 *
 * ## Threading (mirrors the Swift serial queue)
 *
 * The Swift handler confines its `ModelContext` to one serial dispatch
 * queue and runs every callback body synchronously — Rust needs the
 * return value before it yields back on the Tokio worker thread. We do
 * the same: a single-thread [dispatcher] serializes all callback work and
 * each callback body runs under [runBlocking] on it, so Room's suspend
 * DAOs complete before the trampoline returns to Rust.
 *
 * ## Changeset bracketing (mirrors `beginChangeset` / `endChangeset`)
 *
 * Swift's `ModelContext` batches every per-kind write into its pending
 * buffer between `beginChangeset` and `endChangeset`, then does exactly
 * one `save()` (success) or `rollback()` (failure). Room has no
 * long-lived pending buffer across suspend calls, so we replicate the
 * batching explicitly: while a round is open we append each write to a
 * per-`walletId` [ChangesetBuffer]; [onChangesetEnd] with `success=true`
 * replays the whole buffer inside a single [withTransaction] (one atomic
 * commit per Rust `store()` round), and `success=false` discards it.
 *
 * Callbacks that can fire **outside** a round (metadata, account
 * registrations, address pools, shielded persist — matching the Swift
 * `if !inChangeset { save() }` writers) commit immediately in their own
 * transaction when no round is open.
 *
 * @param database the Room database to persist into.
 * @param dispatcher single-thread dispatcher confining all callback work;
 *   `null` (the production default) creates a dedicated owned executor
 *   that [close] shuts down. Tests inject their own dispatcher, which the
 *   handler never closes.
 * @param privateKeyDeriver derives + persists the 32-byte private half of
 *   an identity key when the persist callback carries a derivation
 *   breadcrumb. `null` (the default) keeps every key watch-only — the
 *   pre-item-1 behavior — so unit tests and watch-only wallets need no
 *   native/Keystore backing. The manager injects a real implementation
 *   (see [IdentityKeyPrivateKeyDeriver]); tests inject a fake.
 */
class PlatformWalletPersistenceHandler(
    private val database: DashDatabase,
    dispatcher: CoroutineDispatcher? = null,
    private val privateKeyDeriver: PrivateKeyDeriver? = null,
    /**
     * Network the owning manager is locked to. Load callbacks that hydrate
     * the Rust manager MUST scope to it (← Swift
     * `PlatformWalletPersistenceHandler.loadWalletList()`'s `self.network`
     * filter) — the Rust loader inserts every returned row unconditionally,
     * so an unscoped list either registers foreign-network wallets or
     * aborts the transactional load on the first cross-network row.
     * Null = unscoped (unit tests exercising raw persistence only).
     */
    private val network: org.dashfoundation.dashsdk.Network? = null,
) : NativePersistenceBridge(), AutoCloseable {

    override fun persistenceCapabilitiesVersion(): Int = PERSISTENCE_CAPABILITIES_VERSION

    override fun persistenceCapabilitiesBits(): Long =
        CAPABILITY_ATOMIC_CHANGESETS or
            CAPABILITY_INVITATIONS or
            CAPABILITY_ASSET_LOCK_FUNDING_INDICES or
            CAPABILITY_SHIELDED_VIEWING_KEYS or
            CAPABILITY_PROVIDER_TRANSACTIONS or
            CAPABILITY_UNSIGNED_TOKEN_STORAGE or
            CAPABILITY_WALLET_RESTORE or
            CAPABILITY_DPNS_NAME_STATES or
            CAPABILITY_TRACKED_ASSET_LOCKS

    /**
     * The single-thread executor created when no [dispatcher] is injected.
     * Owned by this handler and released by [close] — its "dash-persistence"
     * thread is non-daemon, so a manager retired on network switch would
     * otherwise leak one live OS thread per switch, unbounded.
     */
    private val ownedDispatcher: ExecutorCoroutineDispatcher? =
        if (dispatcher == null) {
            Executors.newSingleThreadExecutor { r -> Thread(r, "dash-persistence") }
                .asCoroutineDispatcher()
        } else {
            null
        }

    /** All callback work is confined to this single-thread dispatcher. */
    private val dispatcher: CoroutineDispatcher = dispatcher ?: ownedDispatcher!!

    /**
     * Shut down the owned executor (no-op for an injected dispatcher).
     * The manager invokes this after `nativeDestroy`'s bounded shutdown
     * has quiesced the callback-firing tasks. A native worker that
     * straggled past that shutdown holds its own strong reference to the
     * bridge (Rust owns the callback context and frees it when the worker
     * exits), so a late callback is memory-safe; if it dispatches onto
     * the already-closed executor it is rejected and dropped, which is
     * fine — every persistence hook is reconciliation-based and re-runs
     * on the next launch.
     */
    override fun close() {
        ownedDispatcher?.close()
    }

    /**
     * One in-flight store() round. Holds the ordered list of Room
     * mutations staged by the per-kind callbacks, replayed atomically on
     * [onChangesetEnd].
     */
    private class ChangesetBuffer {
        val ops: MutableList<suspend (DashDatabase) -> Unit> = mutableListOf()

        /**
         * [pendingIdentityKeys] map deltas staged by
         * [onPersistIdentityKeyUpsert] during this round. The matching
         * `PublicKeyEntity` row is only BUFFERED until [onChangesetEnd], so
         * the pending-state change it describes is not true until that row
         * commits: publishing a record early would flag a key whose
         * watch-only row may be discarded by rollback, and publishing a
         * clear early would drop the repair signal for an old watch-only
         * row whose successful re-derive then rolls back (alias cleanup
         * deletes the newly stored scalar). Applied in order, in ONE atomic
         * [MutableStateFlow.update], only after the Room transaction
         * commits; discarded with the buffer on rollback/abort so an
         * aborted round leaves the pre-round map untouched
         * (dashpay/platform#4060, finding de3cf44a71fc).
         */
        val pendingKeyDeltas:
            MutableList<(Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey>> =
                mutableListOf()
    }

    /** Open rounds keyed by walletId hex (a round is per-walletId). */
    private val buffers = HashMap<String, ChangesetBuffer>()

    /**
     * `privkey.*` alias hexes NEWLY CREATED by
     * [PrivateKeyDeriver.deriveAndStore] during the currently-OPEN round,
     * per walletId hex. Only aliases [DerivedKeyStoreResult.wasNewlyCreated]
     * marks true are recorded — re-derives that overwrite an already-valid
     * scalar (add-key flows that store before Rust persistence begins,
     * `disable_keys` re-emitting breadcrumbs on existing keys) must never
     * become rollback-deletion candidates.
     *
     * The alias write happens immediately (the identifier must be baked
     * into the staged row) while the row itself is buffered until
     * [onChangesetEnd] — so between those points the alias exists with no
     * committed row to discover it by. This map is the fence over that
     * gap:
     *  - a FAILED round deletes the aliases it created (their rows never
     *    commit — without this they'd be stranded ciphertext forever);
     *  - the wallet-deletion sweep unions [pendingAliasesFor] into its
     *    Room enumeration so a mid-round wipe still reaches them;
     *  - a SUCCESSFUL round just drops the record (the committed rows
     *    make the aliases discoverable the normal way).
     * All access happens under [callbackExclusion].
     */
    private val pendingRoundAliases = HashMap<String, MutableSet<String>>()

    /**
     * Round-created aliases whose rollback deletion FAILED, per walletId
     * hex. Cleanup state is never dropped until an atomic deletion
     * succeeds: entries are retried at the next [onChangesetBegin] and
     * remain discoverable by the wallet-deletion sweep via
     * [pendingAliasesFor]. All access under [callbackExclusion].
     */
    private val orphanedAliases = HashMap<String, MutableSet<String>>()

    /**
     * Aliases created during the wallet's open round plus any orphans
     * whose earlier cleanup failed (empty when neither exists). Caller
     * must hold [callbackExclusion] (the wallet-deletion sweep does, via
     * [withCallbackExclusion]).
     */
    internal fun pendingAliasesFor(walletId: ByteArray): Set<String> {
        val key = walletId.toHex()
        return (pendingRoundAliases[key].orEmpty() + orphanedAliases[key].orEmpty()).toSet()
    }

    /**
     * Delete round-created aliases whose rows never committed, returning
     * the hexes that could NOT be deleted (never silently dropped).
     * Aliases that a row committed by ANOTHER round now references (same
     * pubkey persisted concurrently — its identifier points at the same
     * `privkey.*` entry) are RETAINED and leave tracking: they are
     * legitimately discoverable through that committed row, and deleting
     * them would break its signing. Aliases a SIBLING wallet's durable
     * owner index claims (via [WalletStorage.storeIfAbsent], no committed
     * row needed) are retained for the same reason, checked ATOMICALLY
     * with the delete by [PrivateKeyDeriver.deleteUnownedStored] — mirrors
     * the cross-wallet check `PlatformWalletManager.removeWallet` runs
     * (there, in the SAME lock hold as its own alias delete; a plain
     * check-then-delete pair here would leave a window for a sibling's
     * [WalletStorage.storeIfAbsent] to adopt an alias between them and
     * lose it to this delete anyway). Caller must hold [callbackExclusion].
     */
    private fun scrubAliases(hexes: Set<String>, walletId: ByteArray): MutableSet<String> {
        if (hexes.isEmpty()) return mutableSetOf()
        val toDelete = hexes.filterTo(mutableSetOf()) { hex ->
            runBlockingResult {
                database.publicKeyDao().getByPublicKeyData(hex.hexToByteArray())
                    .none { it.privateKeyKeychainIdentifier != null }
            }
        }
        if (toDelete.isEmpty()) return mutableSetOf()
        return try {
            privateKeyDeriver?.deleteUnownedStored(toDelete, walletId)
            mutableSetOf()
        } catch (t: Throwable) {
            Log.w(
                TAG,
                "failed to delete ${toDelete.size} rolled-back alias(es); retained for retry",
                t,
            )
            toDelete
        }
    }

    /** Scrub the wallet's round-created aliases; keep failures as orphans. */
    private fun scrubPendingAliases(walletIdHex: String) {
        val pending = pendingRoundAliases.remove(walletIdHex) ?: return
        val remaining = scrubAliases(pending, walletIdHex.hexToByteArray())
        if (remaining.isNotEmpty()) {
            orphanedAliases.getOrPut(walletIdHex) { mutableSetOf() }.addAll(remaining)
        }
    }

    /** Retry any earlier failed cleanup for this wallet. */
    private fun retryOrphanedAliases(walletIdHex: String) {
        val orphans = orphanedAliases.remove(walletIdHex) ?: return
        val remaining = scrubAliases(orphans, walletIdHex.hexToByteArray())
        if (remaining.isNotEmpty()) {
            orphanedAliases[walletIdHex] = remaining
        }
    }

    /**
     * Serializes every persistence callback against compound external
     * sequences (wallet deletion's snapshot → secret delete → cascade).
     * Each [guarded]/[guardedLoad] callback acquires it AT ENTRY on the
     * JNI caller thread — before any hop onto [dispatcher] — so a parked
     * callback never holds the persistence thread, and an exclusion
     * holder may safely run dispatcher-confined work. Callbacks fire
     * while Rust holds the wallet-manager write lock, so an exclusion
     * holder must NEVER call into native code (ABBA deadlock); it must
     * also never re-enter a public locking entry point (non-reentrant).
     */
    private val callbackExclusion = Mutex()

    /**
     * Run [block] with persistence callbacks excluded: no callback (and
     * so no [PrivateKeyDeriver] alias write and no changeset commit) can
     * interleave with it. See [callbackExclusion] for the rules the block
     * must obey (no native calls, no re-entrant locking).
     */
    suspend fun <T> withCallbackExclusion(block: suspend () -> T): T =
        callbackExclusion.withLock { block() }

    /**
     * An identity key whose private-half derivation/storage failed — the
     * key was persisted **watch-only** and cannot sign until re-derived
     * (e.g. via `PlatformWalletManager.repairIdentityKey`).
     */
    data class PendingIdentityKey(
        /** Hex of the wallet the key belongs to. */
        val walletIdHex: String,
        /** Base58 of the owning identity id. */
        val identityIdBase58: String,
        /** On-identity key id. */
        val keyId: Int,
        /** Lowercase hex of the compressed public key (the storage key). */
        val publicKeyHex: String,
        /** Derivation breadcrumb: identity index. */
        val identityIndex: Int,
        /** Derivation breadcrumb: key index. */
        val keyIndex: Int,
        /** Human-readable failure reason (exception message or contract miss). */
        val reason: String,
        /** Epoch millis of the (latest) failure. */
        val failedAtMs: Long,
    )

    private val _pendingIdentityKeys =
        MutableStateFlow<Map<String, PendingIdentityKey>>(emptyMap())

    /**
     * Queryable "keys pending" state: identity keys whose private half
     * could not be derived/stored by [onPersistIdentityKeyUpsert] (keyed by
     * public-key hex). Such keys are persisted watch-only — signing with
     * them fails — so hosts should watch this flow and surface a repair
     * path. An entry clears automatically when a later persist round (or an
     * explicit re-derive that replays the upsert) stores the key.
     *
     * Transactional with the round it belongs to: while a store round is
     * open, record/clear mutations are staged in the round's
     * [ChangesetBuffer] and published only after the Room transaction
     * commits — a rolled-back or aborted round leaves this map exactly as
     * it was before the round (see [ChangesetBuffer.pendingKeyDeltas]).
     * Standalone (non-bracketed) upserts and [markIdentityKeyRepaired]
     * publish immediately.
     */
    val pendingIdentityKeys: StateFlow<Map<String, PendingIdentityKey>> =
        _pendingIdentityKeys.asStateFlow()

    /**
     * Stage a write. If a round is open for [walletId] the op is buffered
     * for the round's single transaction; otherwise it runs immediately
     * in its own transaction (the standalone-callback path).
     */
    private fun stage(walletId: ByteArray, op: suspend (DashDatabase) -> Unit) {
        val key = walletId.toHex()
        val buffer = buffers[key]
        if (buffer != null) {
            buffer.ops.add(op)
        } else {
            runBlockingCatching {
                database.withTransaction { op(database) }
            }
        }
    }

    // ── Bracketing ────────────────────────────────────────────────────

    override fun onChangesetBegin(walletId: ByteArray): Int = guarded {
        val key = walletId.toHex()
        // A pending leftover here means the previous round never reached
        // its end callback (abandoned mid-round) — its rows never
        // committed, so its aliases are orphans; scrub them like a
        // rolled-back round (its staged pending-key deltas vanish with the
        // replaced buffer). Then retry any earlier failed cleanup.
        scrubPendingAliases(key)
        retryOrphanedAliases(key)
        buffers[key] = ChangesetBuffer()
        0
    }

    override fun onChangesetEnd(walletId: ByteArray, success: Boolean): Int = guarded {
        val key = walletId.toHex()
        val buffer = buffers.remove(key) ?: return@guarded 0
        if (!success) {
            // Rollback: discard every staged write, mirroring
            // `backgroundContext.rollback()` — and delete the aliases the
            // deriver already wrote for this round: their rows will never
            // commit, so leaving them would strand undiscoverable
            // identity-key ciphertext in the DataStore forever. The round's
            // staged pending-key deltas are discarded with the buffer, so
            // the pre-round [pendingIdentityKeys] map survives untouched.
            scrubPendingAliases(key)
            return@guarded 0
        }
        try {
            runBlockingCatching {
                database.withTransaction {
                    for (op in buffer.ops) {
                        op(database)
                    }
                }
            }
            // Rows committed — the aliases are discoverable the normal way,
            // and the round's pending-key state changes are now true.
            pendingRoundAliases.remove(key)
            publishPendingKeyDeltas(buffer)
        } catch (t: Throwable) {
            // Commit failed: the staged rows never landed, so the round's
            // aliases are orphans exactly like the !success branch — and its
            // pending-key deltas are equally void (discarded with the buffer).
            scrubPendingAliases(key)
            throw t
        }
        0
    }


    // on_store_fn / on_flush_fn have no Swift analog — they are
    // FFIPersister-level notifications. The begin/end bracket is the only
    // durable transaction boundary, so these stay no-ops (the base class
    // already returns 0).

    // ── Platform address balances (update-only) ───────────────────────

    override fun onPersistAddressBalance(
        walletId: ByteArray,
        addressType: Byte,
        addressHash: ByteArray,
        balance: Long,
        nonce: Int,
        accountIndex: Int,
        addressIndex: Int,
        asOfHeight: Long,
    ): Int = guarded {
        stage(walletId) { db ->
            // Update-only: the row is seeded by the address-pool emit
            // path (persistAccountAddressPools). Skip if it doesn't exist,
            // matching `persistAddressBalances`.
            //
            // Scope by walletId + hash: a hash-only predicate can match
            // another wallet's row in a multi-wallet store (same seed
            // imported on coin-type-sharing networks, watch-only
            // duplicates) — the same fix the Swift handler carries.
            val row = db.platformAddressDao()
                .getByWalletAndAddressHash(walletId, addressHash) ?: return@stage
            db.platformAddressDao().upsert(
                row.copy(
                    balance = balance,
                    nonce = nonce,
                    isUsed = row.isUsed || balance > 0 || nonce > 0,
                    // Balance height pin (← Swift handler's lastSeenHeight
                    // = asOfHeight); platform heights fit an Int.
                    lastSeenHeight = asOfHeight.coerceIn(0, Int.MAX_VALUE.toLong()).toInt(),
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    // ── Sync state (network-scoped) ───────────────────────────────────

    override fun onPersistSyncState(
        walletId: ByteArray,
        syncHeight: Long,
        syncTimestamp: Long,
        lastKnownRecentBlock: Long,
    ): Int = guarded {
        stage(walletId) { db ->
            // Network-scoped, keyed by a "platform-sync:<network>" pseudo
            // id (mirror of `syncStateScopeId`). Resolve the network from
            // the wallet row; skip if unknown.
            val networkRaw = db.walletDao().getByWalletId(walletId)?.networkRaw ?: return@stage
            val scopeId = syncStateScopeId(networkRaw)
            db.platformAddressDao().upsertSyncState(
                PlatformAddressesSyncStateEntity(
                    walletId = scopeId,
                    networkRaw = networkRaw,
                    syncHeight = syncHeight,
                    syncTimestamp = syncTimestamp,
                    lastKnownRecentBlock = lastKnownRecentBlock,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    // ── Wallet metadata (fetch-or-create the wallet row) ──────────────

    override fun onPersistWalletMetadata(
        walletId: ByteArray,
        network: Int,
        walletGroupId: ByteArray,
        birthHeight: Int,
    ): Int = guarded {
        stage(walletId) { db ->
            val existing = db.walletDao().getByWalletId(walletId)
            val row = (existing ?: WalletEntity(walletId = walletId)).copy(
                networkRaw = network,
                walletGroupId = if (walletGroupId.isNotEmpty()) walletGroupId
                else existing?.walletGroupId ?: ByteArray(0),
                birthHeight = birthHeight,
                lastUpdated = now(),
            )
            db.walletDao().upsert(row)
        }
        0
    }

    // ── Account registrations ─────────────────────────────────────────

    override fun onPersistAccountRegistration(
        walletId: ByteArray,
        typeTag: Byte,
        standardTag: Byte,
        index: Int,
        registrationIndex: Int,
        keyClass: Int,
        userIdentityId: ByteArray,
        friendIdentityId: ByteArray,
        accountXpubBytes: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            // Drop if the wallet row is missing (mirrors persistAccount's
            // drop-on-missing; metadata seeds the wallet first).
            if (db.walletDao().getByWalletId(walletId) == null) return@stage
            upsertAccount(
                db,
                walletId,
                typeTag.toInt() and 0xFF,
                index,
                standardTag.toInt() and 0xFF,
                registrationIndex,
                keyClass,
                userIdentityId,
                friendIdentityId,
                xpub = accountXpubBytes.takeIf { it.isNotEmpty() },
            )
        }
        0
    }

    // ── Account address pools ─────────────────────────────────────────

    override fun onPersistAccountAddressPoolEntry(
        walletId: ByteArray,
        accountTypeTag: Byte,
        accountStandardTag: Byte,
        accountIndex: Int,
        accountRegistrationIndex: Int,
        accountKeyClass: Int,
        accountUserIdentityId: ByteArray,
        accountFriendIdentityId: ByteArray,
        poolTypeTag: Byte,
        publicKey: ByteArray,
        hasPublicKey: Boolean,
        addressPoolTypeTag: Byte,
        addressIndex: Int,
        isUsed: Boolean,
        balance: Long,
        addressBase58: String,
        derivationPath: String,
    ): Int = guarded {
        if (addressBase58.isEmpty()) return@guarded 0
        stage(walletId) { db ->
            val account = fetchAccount(
                db, walletId, accountTypeTag.toInt() and 0xFF, accountIndex,
                accountStandardTag.toInt() and 0xFF, accountRegistrationIndex,
                accountKeyClass, accountUserIdentityId, accountFriendIdentityId,
            ) ?: run {
                // The IdentityInvitation pool write is load-bearing: Rust's
                // pre-broadcast gate treats this round's success as "voucher
                // funding index durably recorded" and only then broadcasts.
                // Silently skipping on a missing parent account row would let
                // the funding index reset on restart and re-export the same
                // one-time bearer key. Create the account row (mirroring
                // onWalletChangesetAccountBegin's upsert-on-missing); if it
                // still can't be resolved (e.g. no wallet row), fail the
                // round so create aborts before any funds move.
                if ((accountTypeTag.toInt() and 0xFF) != ACCOUNT_TYPE_IDENTITY_INVITATION) {
                    return@stage
                }
                upsertAccount(
                    db, walletId, accountTypeTag.toInt() and 0xFF, accountIndex,
                    accountStandardTag.toInt() and 0xFF, accountRegistrationIndex,
                    accountKeyClass, accountUserIdentityId, accountFriendIdentityId,
                    xpub = null,
                )
                fetchAccount(
                    db, walletId, accountTypeTag.toInt() and 0xFF, accountIndex,
                    accountStandardTag.toInt() and 0xFF, accountRegistrationIndex,
                    accountKeyClass, accountUserIdentityId, accountFriendIdentityId,
                ) ?: error(
                    "invitation funding account row unresolvable; " +
                        "failing round to keep the funding index durable",
                )
            }
            if ((accountTypeTag.toInt() and 0xFF) == ACCOUNT_TYPE_PLATFORM_PAYMENT) {
                // DIP-17 PlatformPayment pool → PlatformAddressEntity
                // (mirror of Swift `persistPlatformPaymentAddresses`). Rust
                // emits the DIP-0018 bech32m form; decode it to the 20-byte
                // hash + address type here so BLAST balance updates (which
                // arrive with `addressHash` only) can upsert the same row.
                val components = decodePlatformAddress(addressBase58) ?: return@stage
                // Wallet-scoped upsert key — an address-only lookup could
                // grab another wallet's row (same seed imported twice) and
                // reassign its walletId/accountId on the copy() below.
                val existing = db.platformAddressDao().getByWalletAndAddress(walletId, addressBase58)
                val row = (existing ?: PlatformAddressEntity(
                    address = addressBase58,
                    addressType = components.first,
                    addressHash = components.second,
                    accountIndex = accountIndex,
                    addressIndex = addressIndex,
                    derivationPath = derivationPath,
                    walletId = walletId,
                )).copy(
                    addressType = components.first,
                    addressHash = components.second,
                    publicKey = if (hasPublicKey) publicKey else existing?.publicKey ?: ByteArray(0),
                    accountIndex = accountIndex,
                    addressIndex = addressIndex,
                    derivationPath = derivationPath,
                    // Emit is authoritative for `isUsed` on creation; keep a
                    // prior funded flag rather than lowering it under funds.
                    isUsed = when {
                        isUsed -> true
                        existing != null && existing.balance == 0L && existing.nonce == 0 -> false
                        else -> existing?.isUsed ?: false
                    },
                    // Preserve any later BLAST-driven balance; only seed on first sight.
                    balance = if ((existing?.balance ?: 0L) == 0L && balance != 0L) balance
                    else existing?.balance ?: balance,
                    accountId = account.id,
                    walletId = walletId,
                    lastUpdated = now(),
                )
                db.platformAddressDao().upsert(row)
                return@stage
            }
            // Core (on-chain) address pool → CoreAddressEntity.
            db.coreAddressDao().upsert(
                CoreAddressEntity(
                    address = addressBase58,
                    publicKey = if (hasPublicKey) publicKey else ByteArray(0),
                    poolTypeTag = addressPoolTypeTag.toInt() and 0xFF,
                    addressIndex = addressIndex,
                    derivationPath = derivationPath,
                    isUsed = isUsed,
                    balance = balance,
                    accountId = account.id,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    // ── Wallet (core) changeset ───────────────────────────────────────

    override fun onWalletChangesetHeader(
        walletId: ByteArray,
        hasSyncedHeight: Boolean,
        syncedHeight: Int,
        hasBalance: Boolean,
        confirmedDelta: Long,
        unconfirmedDelta: Long,
        immatureDelta: Long,
        lockedDelta: Long,
        lastAppliedChainLockBytes: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            // Drop stale post-deletion callbacks (can't resurrect a wallet).
            val wallet = db.walletDao().getByWalletId(walletId) ?: return@stage
            db.walletDao().upsert(
                wallet.copy(
                    syncedHeight = if (hasSyncedHeight) syncedHeight else wallet.syncedHeight,
                    lastAppliedChainLockBytes = if (lastAppliedChainLockBytes.isNotEmpty())
                        lastAppliedChainLockBytes else wallet.lastAppliedChainLockBytes,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onWalletChangesetAccountBegin(
        walletId: ByteArray,
        accountIndex: Int,
        typeTag: Byte,
        standardTag: Byte,
        registrationIndex: Int,
        keyClass: Int,
        userIdentityId: ByteArray,
        friendIdentityId: ByteArray,
        externalHighestUsed: Int,
        hasExternalHighestUsed: Boolean,
        internalHighestUsed: Int,
        hasInternalHighestUsed: Boolean,
    ): Int = guarded {
        stage(walletId) { db ->
            if (db.walletDao().getByWalletId(walletId) == null) return@stage
            val existing = fetchAccount(
                db, walletId, typeTag.toInt() and 0xFF, accountIndex,
                standardTag.toInt() and 0xFF, registrationIndex, keyClass,
                userIdentityId, friendIdentityId,
            )
            val base = existing ?: run {
                upsertAccount(
                    db, walletId, typeTag.toInt() and 0xFF, accountIndex,
                    standardTag.toInt() and 0xFF, registrationIndex, keyClass,
                    userIdentityId, friendIdentityId, xpub = null,
                )
                fetchAccount(
                    db, walletId, typeTag.toInt() and 0xFF, accountIndex,
                    standardTag.toInt() and 0xFF, registrationIndex, keyClass,
                    userIdentityId, friendIdentityId,
                ) ?: return@stage
            }
            db.accountDao().update(
                base.copy(
                    externalHighestUsed = if (hasExternalHighestUsed) externalHighestUsed
                    else base.externalHighestUsed,
                    internalHighestUsed = if (hasInternalHighestUsed) internalHighestUsed
                    else base.internalHighestUsed,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onWalletChangesetTransaction(
        walletId: ByteArray,
        txid: ByteArray,
        txData: ByteArray,
        context: Int,
        blockHeight: Int,
        blockHash: ByteArray,
        blockTimestamp: Int,
        direction: Int,
        transactionType: String,
        transactionTypeKind: Int,
        netAmount: Long,
        fee: Long,
        hasFee: Boolean,
        label: String,
        firstSeen: Long,
        inputOutpoints: ByteArray,
        inputOutpointCount: Int,
        accountTypeTag: Byte,
        accountStandardTag: Byte,
        accountIndex: Int,
        accountRegistrationIndex: Int,
        accountKeyClass: Int,
        accountUserIdentityId: ByteArray,
        accountFriendIdentityId: ByteArray,
        blockPosition: Int,
        hasBlockPosition: Boolean,
    ): Int = guarded {
        stage(walletId) { db ->
            val existing = db.transactionDao().getByTxid(txid)
            // firstSeen: adopt non-zero from FFI; else keep existing;
            // else stamp now (never leave a placeholder zero).
            val resolvedFirstSeen = when {
                firstSeen != 0L -> firstSeen
                existing != null && existing.firstSeen != 0L -> existing.firstSeen
                else -> nowSeconds()
            }
            db.transactionDao().upsert(
                TransactionEntity(
                    txid = txid,
                    transactionData = txData,
                    context = context,
                    blockHeight = blockHeight,
                    blockHash = blockHash.takeIf { it.any { b -> b.toInt() != 0 } },
                    blockTimestamp = blockTimestamp,
                    blockPosition = blockPosition,
                    hasBlockPosition = hasBlockPosition,
                    direction = direction,
                    transactionType = transactionType.ifEmpty { "Standard" },
                    transactionTypeKind = transactionTypeKind,
                    netAmount = netAmount,
                    fee = if (hasFee) fee else null,
                    label = label,
                    firstSeen = resolvedFirstSeen,
                    createdAt = existing?.createdAt ?: java.util.Date(),
                    lastUpdated = now(),
                ),
            )
            // The JNI trampoline forwards the full enclosing account tuple.
            // Only provider-key accounts (AccountTypeTagFFI 8…11) can own
            // provider-special payload involvement. A provider-kind record
            // merely observed by a Standard account must not leak into an
            // unrelated provider account's restore set.
            val accountType = accountTypeTag.toInt() and 0xFF
            val isProviderAccount = accountType in 8..11
            val isProviderTransaction = transactionTypeKind in 2..5
            if (isProviderAccount && isProviderTransaction) {
                val account = fetchAccount(
                    db,
                    walletId,
                    accountType,
                    accountIndex,
                    accountStandardTag.toInt() and 0xFF,
                    accountRegistrationIndex,
                    accountKeyClass,
                    accountUserIdentityId,
                    accountFriendIdentityId,
                ) ?: error("transaction callback account tuple was not persisted")
                db.transactionDao().upsertInvolvement(
                    TransactionAccountInvolvementEntity(txid, account.id),
                )
            }
            // Reconcile every spent input outpoint against our TXOs — a 1:1
            // port of Swift resolveInputOutpoint
            // (PlatformWalletPersistenceHandler.swift:688-785). `inputOutpoints`
            // carries EVERY input of this spending tx (even ones whose funding
            // TXO isn't known yet — Rust builds it from tx.input directly, not
            // the classified utxos_spent slice), so a spend observed before its
            // funding output can't be lost: if the funding TXO is present we
            // link the spend now; otherwise we stage a pending row that the
            // funding TXO's later upsert drains. Without this the UTXO-restore
            // path (CORE-06) would hand a consumed output back to Rust as
            // spendable after relaunch. Replaces the old getUnspentBySpendingTxid
            // flip pass, which had no Swift analog and could not see
            // out-of-order / unclassified inputs.
            for (i in 0 until inputOutpointCount) {
                val outpoint = inputOutpoints.copyOfRange(i * 36, i * 36 + 36)
                val txo = db.txoDao().getByOutpoint(outpoint)
                if (txo != null) {
                    // Found: link the spend. Monotonic — only a confirmed
                    // (in-block) context flips isSpent; a mempool re-emit never
                    // downgrades a flag that is already true (mirrors spendIsInBlock).
                    db.txoDao().upsert(
                        txo.copy(
                            isSpent = txo.isSpent || context >= CONTEXT_IN_BLOCK,
                            spendingTxid = txid,
                            spendingInputIndex = i,
                            lastUpdated = now(),
                        ),
                    )
                    for (p in db.documentDao().getPendingInputsByOutpoint(outpoint)) {
                        db.documentDao().deletePendingInput(p)
                    }
                } else if (db.documentDao().getPendingInput(outpoint, txid) == null) {
                    // Funding TXO unknown — defer via a pending row (dedup-guarded
                    // on outpoint+spendingTxid). FK parent = the tx row upserted
                    // just above, so the CASCADE relationship holds.
                    db.documentDao().upsertPendingInput(
                        PendingInputEntity(
                            outpoint = outpoint,
                            inputIndex = i,
                            spendingTxid = txid,
                            spendingTransactionTxid = txid,
                            walletId = walletId,
                        ),
                    )
                }
            }
        }
        0
    }

    override fun onWalletChangesetUtxoAdded(
        walletId: ByteArray,
        txid: ByteArray,
        vout: Int,
        amount: Long,
        address: String,
        scriptPubKey: ByteArray,
        height: Int,
        isCoinbase: Boolean,
        isConfirmed: Boolean,
        isInstantLocked: Boolean,
        isLocked: Boolean,
    ): Int = guarded {
        stage(walletId) { db ->
            val outpoint = makeOutpoint(txid, vout)
            // Ensure a parent transaction row exists (stub if missing, so
            // the TXO FK holds; the real tx upsert overwrites it later).
            if (db.transactionDao().getByTxid(txid) == null) {
                db.transactionDao().upsert(
                    TransactionEntity(txid = txid, transactionData = ByteArray(0)),
                )
            }
            val existing = db.txoDao().getByOutpoint(outpoint)
            val coreAddressId = if (address.isNotEmpty()) address else null
            val row = TxoEntity(
                outpoint = outpoint,
                vout = vout,
                amount = amount,
                address = address,
                scriptPubKey = scriptPubKey,
                height = height,
                isCoinbase = isCoinbase,
                isConfirmed = isConfirmed,
                isInstantLocked = isInstantLocked,
                isLocked = isLocked,
                // The wallet is handing this outpoint over as a UTXO, so it
                // holds it unspent — authoritative, and the only thing that
                // lifts a mark with no spender behind it. The sweep path
                // parks the inputs it cannot resolve in exactly that state
                // (`holdSpentWithoutSpender`); a rescan re-delivering the
                // coin lands here and frees it. A row whose spend is still
                // on record keeps its flag — the pending drain below owns
                // that transition. `supersededByTxid` is a different kind of
                // "no spender" — a sweep's winner is known but its row never
                // materialized here — and must not be lifted the same way,
                // or a tombstone the drain below just wrote would be undone
                // by the very next sync round that re-delivers this outpoint.
                isSpent = existing?.isSpent == true &&
                    (existing.spendingTxid != null || existing.supersededByTxid != null),
                walletId = walletId,
                txid = txid,
                spendingTxid = existing?.spendingTxid,
                spendingInputIndex = existing?.spendingInputIndex,
                accountId = existing?.accountId,
                coreAddressId = existing?.coreAddressId ?: coreAddressIdIfPresent(db, coreAddressId),
                createdAt = existing?.createdAt ?: java.util.Date(),
                lastUpdated = now(),
                supersededByTxid = existing?.supersededByTxid,
            )
            db.txoDao().upsert(row)
            // Drain any pending-input rows staged before this funding TXO
            // existed — a 1:1 port of the Swift upsertUtxo drain
            // (PlatformWalletPersistenceHandler.swift:895-953). A spend that
            // arrived first was deferred (see onWalletChangesetTransaction);
            // now that the funding output is here, link the newest pending
            // spend (reorg/double-spend: newest wins) and clear the rows so
            // the UTXO-restore path won't hand this consumed output back to
            // Rust as spendable.
            val pending = db.documentDao().getPendingInputsByOutpoint(outpoint)
            if (pending.isNotEmpty()) {
                val chosen = pending.maxByOrNull { it.createdAt }!!
                val spending = db.transactionDao().getByTxid(chosen.spendingTxid)
                if (chosen.isSweptTombstone) {
                    // `onWalletChangesetTransactionsSwept` repointed this row
                    // at the sweep's winner because the loser it originally
                    // recorded is gone. A sweep's winner is already final —
                    // there is no mempool state to wait out — so `isSpent`
                    // does not gate on `spending` the way an ordinary pending
                    // spend does; that lookup only succeeds when the winner
                    // happens to have its own materialized row, which isn't
                    // guaranteed (and `spendingTxid`'s FK forbids forcing the
                    // reference otherwise). `supersededByTxid` is what makes
                    // the mark durable either way — it is what the recovery
                    // clear above checks so this coin isn't handed back as
                    // spendable on a later sync.
                    db.txoDao().upsert(
                        row.copy(
                            isSpent = true,
                            spendingTxid = spending?.txid ?: row.spendingTxid,
                            spendingInputIndex = chosen.inputIndex,
                            supersededByTxid = chosen.spendingTxid,
                            lastUpdated = now(),
                        ),
                    )
                } else {
                    val spentInBlock = spending != null && spending.context >= CONTEXT_IN_BLOCK
                    db.txoDao().upsert(
                        row.copy(
                            isSpent = row.isSpent || spentInBlock,
                            spendingTxid = chosen.spendingTxid,
                            spendingInputIndex = chosen.inputIndex,
                            lastUpdated = now(),
                        ),
                    )
                }
                for (p in pending) db.documentDao().deletePendingInput(p)
            }
        }
        0
    }

    override fun onWalletChangesetUtxoSpent(
        walletId: ByteArray,
        txid: ByteArray,
        vout: Int,
        spendingTxid: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            val outpoint = makeOutpoint(txid, vout)
            val txo = db.txoDao().getByOutpoint(outpoint) ?: return@stage
            // Only mark spent when the spending tx exists in-block (never
            // flap false on an unresolved spend), mirroring markUtxoSpent.
            val spending = db.transactionDao().getByTxid(spendingTxid)
            val spentInBlock = spending != null && spending.context >= CONTEXT_IN_BLOCK
            db.txoDao().upsert(
                txo.copy(
                    spendingTxid = if (spending != null) spendingTxid else txo.spendingTxid,
                    isSpent = if (spending != null) spentInBlock else txo.isSpent,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onWalletChangesetAccountEnd(walletId: ByteArray, accountIndex: Int): Int = 0

    /**
     * Delete the mirror of transactions the wallet swept.
     *
     * Each was a recorded spend that its winner beat to one of its inputs,
     * so it can never confirm and Rust has already dropped it. Keeping the
     * rows would hand them back at the next load and re-create a balance the
     * wallet has already corrected.
     *
     * The TXOs the transaction created go with it (`txos.txid` cascades).
     * The ones it *spent* split in two, and [releasedOutpoints] is the
     * authority on which is which: an outpoint named there came free, and
     * every other input the loser claimed was taken by the transaction that
     * beat it and is gone for good.
     *
     * That split cannot be worked out here. A swept loser is always
     * unconfirmed upstream, and this store flips `isSpent` only for a
     * spender that reached a block, so the loser holds its inputs by link
     * alone at `isSpent = 0`; deleting the row nils the link and every one
     * of those coins would return to the restore set, the winner's
     * included. Nor can the winner's own row settle it — it may pay only to
     * outside addresses and never be recorded here, and even a relevant one
     * is not guaranteed to land in the same round as the sweep.
     *
     * A held input can also have no `TxoEntity` at all yet — the loser was
     * persisted before its own funding TXO was, so `onWalletChangesetTransaction`
     * parked the claim as a `pending_inputs` row instead (see
     * `PendingInputEntity`). That row's FK cascades on [txids]' own delete
     * below just like the TXOs do, so left alone the claim would vanish with
     * the loser, and the funding TXO's own later `onWalletChangesetUtxoAdded`
     * — even after a restart — would have nothing to tell it the coin isn't
     * really free. [DocumentDao.tombstoneUnreleasedPendingInputs] detaches a
     * held pending input from its doomed loser and repoints it at the
     * corresponding [supersededBy] entry instead, flagged so the drain in
     * `onWalletChangesetUtxoAdded` knows to keep the coin spent — durably,
     * via `TxoEntity.supersededByTxid` — once the funding TXO materializes.
     *
     * A tombstoned row can itself need to move again: [supersededBy] is a
     * winner in this round, but nothing stops it from losing a later round
     * to a further winner while [supersededBy]'s own funding TXO is still
     * unresolved. [DocumentDao.tombstoneUnreleasedPendingInputs] can't see
     * that earlier tombstone — it already detached from the relationship
     * that query matches on — so [DocumentDao.deleteReleasedSweptTombstones]
     * and [DocumentDao.retargetSweptTombstones] look it up the only other
     * way it is still findable, by the scalar `spendingTxid` it was
     * repointed to, and carry it the rest of the chain: deleted if this
     * round finally frees its outpoint, repointed at the new winner if not.
     *
     * All updates run before the delete: the foreign key nulls `spendingTxid`
     * (or, for a pending row already detached above, does nothing) on delete,
     * and after that nothing finds those rows.
     *
     * Transaction rows are keyed by txid alone, shared across wallets by
     * design, and a sweep is a statement about the transaction rather than
     * about one wallet's view of it — so the row goes without narrowing to
     * the emitting wallet.
     */
    override fun onWalletChangesetTransactionsSwept(
        walletId: ByteArray,
        txids: Array<ByteArray>,
        supersededBy: Array<ByteArray>,
        releasedOutpoints: Array<ByteArray>,
    ): Int = guarded {
        stage(walletId) { db ->
            if (db.walletDao().getByWalletId(walletId) == null) return@stage
            // Hold every input first, then free the ones upstream named: the
            // released set is wallet-scoped across the round's removals, so
            // it is applied once rather than per transaction.
            //
            // The order is load-bearing, not cosmetic. Holding detaches the
            // rows this round's removals still claim, and the release only
            // touches detached rows — so a coin some later transaction in the
            // same round already re-claimed keeps that claim instead of being
            // freed out from under it.
            val released = releasedOutpoints.toList()
            for (i in txids.indices) {
                db.txoDao().holdSpentWithoutSpender(txids[i])
                db.documentDao().tombstoneUnreleasedPendingInputs(txids[i], supersededBy[i], released)
                // A pending input an EARLIER sweep already tombstoned to
                // txids[i] (that txid was itself a sweep's winner, and is
                // now being swept in turn) detached from the relationship
                // `tombstoneUnreleasedPendingInputs` above matches on, so it
                // has to be found and carried forward separately — see
                // [DocumentDao.deleteReleasedSweptTombstones].
                db.documentDao().deleteReleasedSweptTombstones(txids[i], released)
                db.documentDao().retargetSweptTombstones(txids[i], supersededBy[i], released)
            }
            for (outpoint in releasedOutpoints) {
                db.txoDao().releaseByOutpoint(outpoint)
            }
            for (txid in txids) {
                db.transactionDao().deleteByTxid(txid)
            }
        }
        0
    }

    // ── Identities ────────────────────────────────────────────────────

    override fun onPersistIdentityUpsert(
        walletId: ByteArray,
        identityId: ByteArray,
        balance: Long,
        revision: Long,
        identityIndexIsSome: Boolean,
        identityIndex: Int,
        status: Byte,
        walletIdIsSome: Boolean,
        identityWalletId: ByteArray,
        dpnsNames: Array<String>,
        dpnsNamesAcquiredAt: LongArray,
        dashpayProfilePresent: Boolean,
        dashpayDisplayName: String?,
        dashpayBio: String?,
        dashpayAvatarUrl: String?,
        dashpayAvatarHash: ByteArray,
        dashpayAvatarHashPresent: Boolean,
        dashpayAvatarFingerprint: ByteArray,
        dashpayAvatarFingerprintPresent: Boolean,
        dashpayPublicMessage: String?,
    ): Int = guarded {
        stage(walletId) { db ->
            val ownerWallet = if (walletIdIsSome) identityWalletId else walletId
            val networkRaw = db.walletDao().getByWalletId(ownerWallet)?.networkRaw
                ?: db.walletDao().getByWalletId(walletId)?.networkRaw
                ?: NETWORK_TESTNET
            val existing = db.identityDao().getByIdentityId(identityId)
            val row = (existing ?: IdentityEntity(
                identityId = identityId,
                networkRaw = networkRaw,
                isLocal = false,
            )).copy(
                balance = balance,
                revision = revision,
                identityIndex = if (identityIndexIsSome) identityIndex
                else existing?.identityIndex ?: 0,
                walletId = if (walletIdIsSome) identityWalletId else existing?.walletId,
                lastUpdated = now(),
            )
            db.identityDao().upsert(row)

            // IdentityEntryFFI carries the complete canonical label set. Drop
            // owned labels that are no longer present; a marketplace state
            // callback in the same changeset re-inserts departed rows with
            // their sold/transferred status and counterparty.
            val canonicalLabels = dpnsNames
                .asSequence()
                .filter { it.isNotEmpty() }
                .map(::normalizeDpnsLabel)
                .toSet()
            for (persisted in db.dpnsNameDao().getAllByIdentity(identityId)) {
                if (persisted.isOwned && persisted.normalizedLabel !in canonicalLabels) {
                    db.dpnsNameDao().delete(persisted)
                }
            }

            // DPNS labels (last-write-wins; upsert by the unique triple).
            for (i in dpnsNames.indices) {
                val label = dpnsNames[i]
                if (label.isEmpty()) continue
                val acquiredAt = dpnsNamesAcquiredAt.getOrElse(i) { 0L }
                val normalized = normalizeDpnsLabel(label)
                val existingName = db.dpnsNameDao().getByUniqueKey(networkRaw, "dash", normalized)
                db.dpnsNameDao().upsert(
                    DpnsNameEntity(
                        networkRaw = networkRaw,
                        label = label,
                        normalizedLabel = normalized,
                        acquiredAt = if (acquiredAt != 0L) acquiredAt
                        else existingName?.acquiredAt ?: 0L,
                        identityId = identityId,
                        documentId = existingName?.documentId,
                        isOwned = true,
                        priceCredits = existingName?.priceCredits,
                        saleStatusRaw = 0,
                        counterpartyIdentityId = null,
                        documentCreatedAtMs = existingName?.documentCreatedAtMs ?: 0L,
                        documentUpdatedAtMs = existingName?.documentUpdatedAtMs ?: 0L,
                        documentTransferredAtMs = existingName?.documentTransferredAtMs ?: 0L,
                        marketplaceUpdatedAt = existingName?.marketplaceUpdatedAt ?: 0L,
                        createdAt = existingName?.createdAt ?: java.util.Date(),
                        lastUpdated = now(),
                    ),
                )
            }

            // DashPay profile: present == "update the whole document";
            // absent == "no update" (never a delete).
            if (dashpayProfilePresent) {
                db.dashpayDao().upsertProfile(
                    DashpayProfileEntity(
                        networkRaw = networkRaw,
                        identityId = identityId,
                        displayName = dashpayDisplayName,
                        publicMessage = dashpayPublicMessage,
                        bio = dashpayBio,
                        avatarUrl = dashpayAvatarUrl,
                        avatarHash = if (dashpayAvatarHashPresent) dashpayAvatarHash else null,
                        avatarFingerprint = if (dashpayAvatarFingerprintPresent)
                            dashpayAvatarFingerprint else null,
                        lastUpdated = now(),
                    ),
                )
            }
        }
        0
    }

    override fun onPersistIdentityRemoval(walletId: ByteArray, identityId: ByteArray): Int = guarded {
        val identityBase58 = identityId.toBase58String()
        stage(walletId) { db -> db.identityDao().deleteByIdentityId(identityId) }
        // The identity delete cascades away all of its public-key rows, so
        // every pending-repair entry for this identity is now a phantom —
        // the key can never be re-derived/repaired into an identity that no
        // longer exists (dashpay/platform#4183 review). Drop them all from
        // [pendingIdentityKeys]. Staged with the round (mirroring
        // [onPersistIdentityKeyRemoval]): published only if the deletion
        // commits and discarded on rollback, so a rolled-back removal keeps
        // the pre-round map intact.
        stagePendingKeyDelta(
            walletId.toHex(),
            clearPendingKeyByIdentityDelta(identityBase58),
        )
        0
    }

    @Suppress("LongParameterList")
    override fun onPersistDpnsNameState(
        walletId: ByteArray,
        documentId: ByteArray,
        walletIdentityId: ByteArray,
        hasCounterparty: Boolean,
        counterpartyId: ByteArray,
        label: String,
        normalizedLabel: String,
        normalizedParentDomainName: String,
        hasPrice: Boolean,
        priceCredits: Long,
        status: Byte,
        createdAtMs: Long,
        updatedAtMs: Long,
        transferredAtMs: Long,
        lastSyncedAtMs: Long,
    ): Int = guarded {
        require(status.toInt() in 0..2) { "unknown DPNS sale status $status" }
        stage(walletId) { db ->
            // The relationship is non-optional. A marketplace sweep can race
            // the first identity snapshot, so skip this row and let the next
            // sync re-emit it instead of rolling back the complete changeset.
            if (db.identityDao().getByIdentityId(walletIdentityId) == null) {
                return@stage
            }
            val networkRaw = db.walletDao().getByWalletId(walletId)?.networkRaw ?: NETWORK_TESTNET
            val existing = db.dpnsNameDao().getByDocumentId(documentId)
                ?: db.dpnsNameDao().getByUniqueKey(
                    networkRaw,
                    normalizedParentDomainName,
                    normalizedLabel,
                )
            db.dpnsNameDao().upsert(
                DpnsNameEntity(
                    networkRaw = networkRaw,
                    label = label,
                    normalizedLabel = normalizedLabel,
                    parentDomainName = normalizedParentDomainName,
                    normalizedParentDomainName = normalizedParentDomainName,
                    acquiredAt = existing?.acquiredAt ?: createdAtMs,
                    identityId = walletIdentityId,
                    documentId = documentId,
                    isOwned = status.toInt() == 0,
                    priceCredits = if (hasPrice) priceCredits else null,
                    saleStatusRaw = status.toInt(),
                    counterpartyIdentityId = if (hasCounterparty) counterpartyId else null,
                    documentCreatedAtMs = createdAtMs,
                    documentUpdatedAtMs = updatedAtMs,
                    documentTransferredAtMs = transferredAtMs,
                    marketplaceUpdatedAt = lastSyncedAtMs,
                    createdAt = existing?.createdAt ?: now(),
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onRemoveDpnsNameState(walletId: ByteArray, documentId: ByteArray): Int = guarded {
        stage(walletId) { db ->
            db.dpnsNameDao().clearMarketplaceByDocumentId(documentId, now())
        }
        0
    }

    // ── Identity keys ─────────────────────────────────────────────────

    override fun onPersistIdentityKeyUpsert(
        walletId: ByteArray,
        identityId: ByteArray,
        keyId: Int,
        purpose: Byte,
        securityLevel: Byte,
        keyType: Byte,
        readOnly: Boolean,
        disabledAtIsSome: Boolean,
        disabledAt: Long,
        publicKeyData: ByteArray,
        publicKeyHash: ByteArray,
        walletIdIsSome: Boolean,
        keyWalletId: ByteArray,
        derivationIndicesIsSome: Boolean,
        identityIndex: Int,
        keyIndex: Int,
        contractBoundsKind: Byte,
        contractBoundsId: ByteArray,
        contractBoundsDocumentType: String?,
    ): Int = guarded {
        // Item 1 — private-key persistence (the CLAUDE.md "one allowed
        // exception" shape). The `IdentityKeyEntryFFI` payload carries only
        // a derivation breadcrumb (`wallet_id` + `identity_index` +
        // `key_index`), NOT the 32-byte scalar. Swift's handler re-derives
        // inline (retrieveMnemonic → seed → path → derive → store), which
        // is exactly the multi-step pipeline `packages/kotlin-sdk/CLAUDE.md`
        // forbids in the language layer. We do NOT replicate it: a single
        // Rust FFI entry point derives the ready bytes and hands them back,
        // Kotlin only encrypts them into Keystore-backed storage.
        //
        // The derive+store side effect runs NOW (synchronously on the
        // persistence dispatcher, which is the Rust caller thread), because
        // the identifier it produces must be baked into the (possibly
        // buffered) row. The deriver uses a wallet-manager-lock-free Rust
        // derive: this callback fires while platform-wallet holds the
        // wallet-manager write lock, so any derive that re-locks it would
        // deadlock (see IdentityNative.deriveIdentityPrivateKeyWithResolver).
        // Skip the derive entirely when the wallet is gone from Room and
        // has no round open — the state after a wallet wipe. This check
        // and the wipe both run under [callbackExclusion], so it's
        // race-free: a late upsert from an operation that survived the
        // wallet's unregistration would otherwise re-write a `privkey.*`
        // alias whose row can never commit (the identity rows are gone),
        // stranding identity-key ciphertext behind a "successful" wipe.
        val roundKey = walletId.toHex()
        val walletStillPersisted = buffers.containsKey(roundKey) ||
            runBlockingResult { database.walletDao().getByWalletId(walletId) != null }
        val deriver = privateKeyDeriver
        // deriveAndStore's own "did this alias already exist" check runs
        // ATOMICALLY with the store (see DerivedKeyStoreResult) — no
        // separate up-front hasStored() call, and so no window for a
        // sibling wallet's concurrent store of the same alias to land
        // between a check and this store and be mis-classified. add-key
        // flows store the scalar before Rust persistence begins, and
        // existing-key operations (disable_keys) re-emit breadcrumbs — an
        // overwrite of an already-valid scalar must never become a
        // rollback-deletion candidate; a failed derive/store is treated as
        // "not newly created" (never wrongly delete; at worst the
        // wallet-deletion sweep still reaches it through the committed row).
        val deriveResult: DerivedKeyStoreResult? =
            if (derivationIndicesIsSome && deriver != null && !readOnly && walletStillPersisted) {
                val keyOwnerWalletId = if (walletIdIsSome) keyWalletId else walletId
                val outcome = runCatching {
                    deriver.deriveAndStore(
                        walletId = keyOwnerWalletId,
                        publicKeyData = publicKeyData,
                        identityIndex = identityIndex,
                        keyIndex = keyIndex,
                        keyType = keyType.toInt() and 0xFF,
                    )
                }
                val id = outcome.getOrNull()
                if (id != null) {
                    // Stored — clear any earlier failure for this pubkey.
                    // Staged with the round (when one is open): if this
                    // round rolls back, alias cleanup deletes the newly
                    // stored scalar, so the old watch-only row must keep
                    // its repair signal (finding de3cf44a71fc).
                    stagePendingKeyDelta(roundKey, clearPendingKeyDelta(publicKeyData.toHex()))
                } else {
                    // NOT silent (dashpay/platform#4053): the key is being
                    // persisted watch-only, so every signature with it will
                    // fail until it is re-derived. Log loudly and record a
                    // queryable pending entry (see [pendingIdentityKeys]).
                    val reason = outcome.exceptionOrNull()?.let { t ->
                        t.message ?: t.javaClass.simpleName
                    } ?: "deriver returned no storage identifier"
                    Log.e(
                        TAG,
                        "identity private-key derive/store FAILED — key " +
                            "${publicKeyData.toHex()} (identity ${identityId.toBase58String()}, " +
                            "keyId $keyId, slot $identityIndex/$keyIndex) is persisted " +
                            "WATCH-ONLY and cannot sign until re-derived " +
                            "(see PlatformWalletPersistenceHandler.pendingIdentityKeys): $reason",
                        outcome.exceptionOrNull(),
                    )
                    // Staged with the round (when one is open): the row is
                    // being persisted watch-only INSIDE the round's buffer,
                    // so if the round aborts that row never commits and the
                    // pending entry would be a phantom (finding de3cf44a71fc).
                    stagePendingKeyDelta(
                        roundKey,
                        recordPendingKeyDelta(
                            PendingIdentityKey(
                                walletIdHex = keyOwnerWalletId.toHex(),
                                identityIdBase58 = identityId.toBase58String(),
                                keyId = keyId,
                                publicKeyHex = publicKeyData.toHex(),
                                identityIndex = identityIndex,
                                keyIndex = keyIndex,
                                reason = reason,
                                failedAtMs = System.currentTimeMillis(),
                            ),
                        ),
                    )
                }
                id
            } else {
                null
            }
        val derivedKeychainId = deriveResult?.identifier
        // While a round is open the row carrying this identifier is only
        // BUFFERED — record a NEWLY-CREATED alias so a rolled-back round
        // (or a wallet wipe racing the round) can still find and delete it.
        if (deriveResult != null && deriveResult.wasNewlyCreated && buffers.containsKey(roundKey)) {
            pendingRoundAliases.getOrPut(roundKey) { mutableSetOf() }
                .add(publicKeyData.toHex())
        }

        stage(walletId) { db ->
            val identityBase58 = identityId.toBase58String()
            val existing = db.publicKeyDao().getByIdentityAndKeyId(identityBase58, keyId)
            // ContractBounds projection → the legacy JSON blob column +
            // doc-type name (Swift stores `[base64(contractId)]` JSON).
            val boundsData = if ((contractBoundsKind.toInt() and 0xFF) != 0)
                contractBoundsIdToJson(contractBoundsId) else null
            val docTypeName = if ((contractBoundsKind.toInt() and 0xFF) == 2)
                contractBoundsDocumentType else null
            val row = PublicKeyEntity(
                id = existing?.id ?: 0,
                keyId = keyId,
                purpose = (purpose.toInt() and 0xFF).toString(),
                securityLevel = (securityLevel.toInt() and 0xFF).toString(),
                keyType = (keyType.toInt() and 0xFF).toString(),
                readOnly = readOnly,
                disabledAt = if (disabledAtIsSome) disabledAt else null,
                publicKeyData = publicKeyData,
                contractBoundsData = boundsData,
                contractBoundsDocumentTypeName = docTypeName,
                // Set to the Keystore identifier when the deriver stored the
                // scalar; otherwise preserve any prior identifier (idempotent
                // re-persist) and fall back to watch-only (null) for
                // watch-only wallets / no-deriver builds.
                privateKeyKeychainIdentifier =
                    derivedKeychainId ?: existing?.privateKeyKeychainIdentifier,
                // Derivation breadcrumbs are recorded whenever Rust supplied
                // them — success AND failure paths (the breadcrumb is not a
                // failure marker; the null identifier is). They make the
                // pending-repair state reconstructible after restart
                // (dashpay/platform#4060 finding 5).
                derivationIdentityIndex =
                    if (derivationIndicesIsSome) identityIndex else existing?.derivationIdentityIndex,
                derivationKeyIndex =
                    if (derivationIndicesIsSome) keyIndex else existing?.derivationKeyIndex,
                identityId = identityBase58,
                identityIdData = identityId,
                createdAt = existing?.createdAt ?: java.util.Date(),
                lastAccessed = now(),
            )
            if (existing == null) db.publicKeyDao().insert(row) else db.publicKeyDao().update(row)
        }
        0
    }

    override fun onPersistIdentityKeyRemoval(
        walletId: ByteArray,
        identityId: ByteArray,
        keyId: Int,
    ): Int = guarded {
        val identityBase58 = identityId.toBase58String()
        stage(walletId) { db ->
            db.publicKeyDao().deleteByIdentityAndKeyId(identityBase58, keyId)
        }
        // The row is gone, so a pending-repair entry for it is now a phantom:
        // the key can never be re-derived/repaired into an identity that no
        // longer carries it (dashpay/platform#4183 review). Drop it from
        // [pendingIdentityKeys]. Staged with the round (mirroring the upsert
        // path): published only if the deletion commits and discarded on
        // rollback, so a rolled-back removal keeps the pre-round map intact.
        stagePendingKeyDelta(
            walletId.toHex(),
            clearPendingKeyByIdentityKeyDelta(identityBase58, keyId),
        )
        0
    }

    // ── Token balances ────────────────────────────────────────────────

    override fun onPersistTokenBalanceUpsert(
        walletId: ByteArray,
        identityId: ByteArray,
        tokenId: ByteArray,
        balance: Long,
    ): Int = guarded {
        stage(walletId) { db ->
            val networkRaw = db.walletDao().getByWalletId(walletId)?.networkRaw ?: NETWORK_TESTNET
            val tokenBase58 = tokenId.toBase58String()
            val existing = db.tokenDao().getBalance(tokenBase58, identityId)
            val row = TokenBalanceEntity(
                id = existing?.id ?: 0,
                tokenId = tokenBase58,
                identityId = identityId,
                balance = UInt64Value.fromRawLongBits(balance),
                frozen = existing?.frozen ?: false,
                networkRaw = existing?.networkRaw ?: networkRaw,
                identityRef = identityId,
                tokenRef = if (db.tokenDao().getTokenById(tokenId) != null) tokenId else existing?.tokenRef,
                createdAt = existing?.createdAt ?: java.util.Date(),
                lastUpdated = now(),
                lastSyncedAt = now(),
            )
            if (existing == null) db.tokenDao().insertBalance(row) else db.tokenDao().updateBalance(row)
        }
        0
    }

    override fun onPersistTokenBalanceRemoval(
        walletId: ByteArray,
        identityId: ByteArray,
        tokenId: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            db.tokenDao().deleteBalance(tokenId.toBase58String(), identityId)
        }
        0
    }

    // ── Contacts ──────────────────────────────────────────────────────

    override fun onPersistContactUpsert(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
        isOutgoing: Boolean,
        senderKeyIndex: Int,
        recipientKeyIndex: Int,
        accountReference: Int,
        encryptedPublicKey: ByteArray,
        encryptedAccountLabel: ByteArray?,
        autoAcceptProof: ByteArray?,
        coreHeightCreatedAt: Int,
        createdAt: Long,
        paymentChannelBroken: Boolean,
        alias: String?,
        note: String?,
        isHidden: Boolean,
        contactAccountLabel: String?,
        acceptedAccounts: IntArray,
    ): Int = guarded {
        stage(walletId) { db ->
            // Owner identity must exist; skip silently otherwise (replayed
            // next round), reading networkRaw off the owner.
            val owner = db.identityDao().getByIdentityId(ownerId) ?: return@stage
            db.dashpayDao().upsertContactRequest(
                DashpayContactRequestEntity(
                    networkRaw = owner.networkRaw,
                    ownerIdentityId = ownerId,
                    contactIdentityId = contactId,
                    isOutgoing = isOutgoing,
                    senderKeyIndex = senderKeyIndex,
                    recipientKeyIndex = recipientKeyIndex,
                    accountReference = accountReference,
                    encryptedPublicKey = encryptedPublicKey,
                    encryptedAccountLabel = encryptedAccountLabel,
                    autoAcceptProof = autoAcceptProof,
                    coreHeightCreatedAt = coreHeightCreatedAt,
                    createdAtMillis = createdAt,
                    paymentChannelBroken = paymentChannelBroken,
                    contactAlias = alias,
                    contactNote = note,
                    contactHidden = isHidden,
                    contactAccountLabel = contactAccountLabel,
                    contactAcceptedAccounts = encodeAcceptedAccounts(acceptedAccounts),
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    /**
     * One per-sender ignore delta (mirror of the Swift
     * `persistContacts` `ignored` loop). An ignore (`isIgnored == true`)
     * drops **every** incoming request row from the sender — suppression
     * is per-sender, so rotated (bumped-`accountReference`) requests go
     * too — and upserts the durable [DashpayIgnoredSenderEntity] row the
     * restore path rehydrates the Rust `ignored_senders` set from. An
     * un-ignore deletes that row so the sender's requests resurface on
     * the next sweep (the Rust side rewinds the cursor to re-fetch them).
     */
    override fun onPersistContactIgnored(
        walletId: ByteArray,
        ownerId: ByteArray,
        senderId: ByteArray,
        isIgnored: Boolean,
    ): Int = guarded {
        stage(walletId) { db ->
            if (isIgnored) {
                db.dashpayDao().deleteContactRequest(ownerId, senderId, isOutgoing = false)
                // Owner identity must exist (networkRaw is read off it);
                // skip silently otherwise — replayed next round.
                val owner = db.identityDao().getByIdentityId(ownerId) ?: return@stage
                db.dashpayDao().upsertIgnoredSender(
                    DashpayIgnoredSenderEntity(
                        networkRaw = owner.networkRaw,
                        ownerIdentityId = ownerId,
                        ignoredSenderId = senderId,
                        ignoredAt = now(),
                    ),
                )
            } else {
                db.dashpayDao().deleteIgnoredSender(ownerId, senderId)
            }
        }
        0
    }

    /**
     * One cached contact-profile delta riding an identity upsert (mirror
     * of the Swift `upsertDashpayContactProfiles` loop). A present entry
     * upserts the [DashpayContactProfileEntity] row; a tombstone
     * (`isPresent == false` — the contact removed their on-chain
     * profile) DELETEs it, so a stale name/avatar can't outlive the
     * on-chain deletion.
     */
    @Suppress("LongParameterList")
    override fun onPersistContactProfileDelta(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
        isPresent: Boolean,
        displayName: String?,
        bio: String?,
        avatarUrl: String?,
        avatarHash: ByteArray,
        avatarHashPresent: Boolean,
        avatarFingerprint: ByteArray,
        avatarFingerprintPresent: Boolean,
        publicMessage: String?,
        checkedAtMs: Long,
    ): Int = guarded {
        stage(walletId) { db ->
            // Owner identity must exist (networkRaw is read off it). In the
            // real flow this lookup always succeeds: the identity upsert is
            // staged into the same buffer BEFORE its contact-profile deltas
            // (persist_identity_upsert loops the deltas after the identity
            // call), so at replay the owner row is visible in the same
            // transaction. The skip is defensive, matching the sibling
            // contact paths.
            val owner = db.identityDao().getByIdentityId(ownerId) ?: return@stage
            if (isPresent) {
                db.dashpayDao().upsertContactProfile(
                    DashpayContactProfileEntity(
                        networkRaw = owner.networkRaw,
                        ownerIdentityId = ownerId,
                        contactIdentityId = contactId,
                        displayName = displayName,
                        publicMessage = publicMessage,
                        bio = bio,
                        avatarUrl = avatarUrl,
                        avatarHash = avatarHash.takeIf { avatarHashPresent },
                        avatarFingerprint = avatarFingerprint.takeIf { avatarFingerprintPresent },
                        checkedAtMs = checkedAtMs,
                        lastUpdated = now(),
                    ),
                )
            } else {
                db.dashpayDao().deleteContactProfile(owner.networkRaw, ownerId, contactId)
            }
        }
        0
    }

    override fun onPersistContactRemovalSent(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
    ): Int = guarded {
        stage(walletId) { db -> db.dashpayDao().deleteContactRequest(ownerId, contactId, true) }
        0
    }

    override fun onPersistContactRemovalIncoming(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
    ): Int = guarded {
        stage(walletId) { db -> db.dashpayDao().deleteContactRequest(ownerId, contactId, false) }
        0
    }

    // ── Asset locks ───────────────────────────────────────────────────

    override fun onPersistAssetLockUpsert(
        walletId: ByteArray,
        outPoint: ByteArray,
        transactionBytes: ByteArray,
        accountIndex: Int,
        fundingType: Byte,
        identityIndex: Int,
        amountDuffs: Long,
        status: Byte,
        proofBytes: ByteArray?,
    ): Int = guarded {
        stage(walletId) { db ->
            val outPointHex = encodeOutPointHex(outPoint)
            val existing = db.assetLockDao().getByOutPointHex(outPointHex)
            db.assetLockDao().upsert(
                AssetLockEntity(
                    outPointHex = outPointHex,
                    walletId = walletId,
                    transactionBytes = transactionBytes,
                    fundingTypeRaw = fundingType.toInt() and 0xFF,
                    identityIndexRaw = identityIndex,
                    accountIndexRaw = accountIndex,
                    amountDuffs = amountDuffs,
                    statusRaw = status.toInt() and 0xFF,
                    proofBytes = proofBytes,
                    createdAt = existing?.createdAt ?: java.util.Date(),
                    updatedAt = now(),
                ),
            )
        }
        0
    }

    override fun onPersistAssetLockRemoval(walletId: ByteArray, outPoint: ByteArray): Int = guarded {
        stage(walletId) { db -> db.assetLockDao().deleteByOutPointHex(encodeOutPointHex(outPoint)) }
        0
    }

    // ── Invitations (DIP-13) ──────────────────────────────────────────

    override fun onPersistInvitationUpsert(
        walletId: ByteArray,
        outPoint: ByteArray,
        fundingIndex: Int,
        amountDuffs: Long,
        expiryUnix: Int,
        createdAtSecs: Int,
        hasInviter: Boolean,
        status: Int,
    ): Int = guarded {
        stage(walletId) { db ->
            val outPointHex = encodeOutPointHex(outPoint)
            val existing = db.invitationDao().getByOutPointHex(outPointHex)
            db.invitationDao().upsert(
                InvitationEntity(
                    outPointHex = outPointHex,
                    rawOutPoint = outPoint,
                    walletId = walletId,
                    fundingIndexRaw = fundingIndex,
                    amountDuffs = amountDuffs,
                    expiryUnix = expiryUnix,
                    createdAtSecs = createdAtSecs,
                    hasInviter = hasInviter,
                    // Claimed/Reclaimed and the reclaim marker are written
                    // locally by the app (Rust emits only Created), so an
                    // existing row keeps them — a Rust re-emit of the same
                    // outpoint must never reset local status.
                    statusRaw = existing?.statusRaw ?: status,
                    reclaimInFlight = existing?.reclaimInFlight ?: false,
                    createdAt = existing?.createdAt ?: java.util.Date(),
                    updatedAt = now(),
                ),
            )
        }
        0
    }

    override fun onPersistInvitationRemoval(walletId: ByteArray, outPoint: ByteArray): Int = guarded {
        stage(walletId) { db -> db.invitationDao().deleteByOutPointHex(encodeOutPointHex(outPoint)) }
        0
    }

    // ── Shielded persist ──────────────────────────────────────────────

    override fun onPersistShieldedNote(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        position: Long,
        cmx: ByteArray,
        nullifier: ByteArray,
        blockHeight: Long,
        isSpent: Byte,
        value: Long,
        noteData: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            db.shieldedDao().upsertNote(
                ShieldedNoteEntity(
                    nullifier = nullifier,
                    walletId = noteWalletId,
                    accountIndex = accountIndex,
                    position = position,
                    cmx = cmx,
                    blockHeight = blockHeight,
                    isSpent = isSpent.toInt() != 0,
                    value = value,
                    noteData = noteData,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onPersistShieldedNullifierSpent(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        nullifier: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            val note = db.shieldedDao().getNoteByNullifier(nullifier) ?: return@stage
            if (!note.isSpent) db.shieldedDao().upsertNote(note.copy(isSpent = true, lastUpdated = now()))
        }
        0
    }

    override fun onPersistShieldedOutgoingNote(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        cmx: ByteArray,
        recipient: ByteArray,
        value: Long,
        blockHeight: Long,
        memo: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            db.shieldedDao().upsertOutgoingNote(
                ShieldedOutgoingNoteEntity(
                    walletId = noteWalletId,
                    accountIndex = accountIndex,
                    cmx = cmx,
                    recipient = recipient,
                    value = value,
                    memo = memo,
                    blockHeight = blockHeight,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onPersistShieldedSyncedIndex(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        lastSyncedIndex: Long,
    ): Int = guarded {
        stage(walletId) { db ->
            val existing = db.shieldedDao().getSyncState(noteWalletId, accountIndex)
            // Monotonic watermark: only advance.
            if (existing != null && existing.lastSyncedIndex >= lastSyncedIndex) return@stage
            db.shieldedDao().upsertSyncState(
                ShieldedSyncStateEntity(
                    walletId = noteWalletId,
                    accountIndex = accountIndex,
                    lastSyncedIndex = lastSyncedIndex,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onPersistShieldedActivity(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        entryId: ByteArray,
        kindTag: Byte,
        direction: Byte,
        status: Byte,
        amount: Long,
        fee: Long,
        hasFee: Boolean,
        blockHeight: Long,
        hasBlockHeight: Boolean,
        createdAtMs: Long,
        identityId: ByteArray,
        hasIdentityId: Boolean,
        counterparty: ByteArray,
        memo: ByteArray,
        noteCmxs: ByteArray,
        spentNullifiers: ByteArray,
    ): Int = guarded {
        stage(walletId) { db ->
            db.shieldedDao().upsertActivity(
                ShieldedActivityEntity(
                    walletId = noteWalletId,
                    accountIndex = accountIndex,
                    entryId = entryId,
                    kindTag = kindTag.toInt() and 0xFF,
                    direction = direction.toInt() and 0xFF,
                    status = status.toInt() and 0xFF,
                    amount = amount,
                    fee = fee,
                    hasFee = hasFee,
                    blockHeight = blockHeight,
                    hasBlockHeight = hasBlockHeight,
                    createdAtMs = createdAtMs,
                    identityId = if (hasIdentityId) identityId else ByteArray(0),
                    counterparty = counterparty,
                    memo = memo,
                    noteCmxs = noteCmxs,
                    spentNullifiers = spentNullifiers,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    override fun onPersistShieldedViewingKey(
        walletId: ByteArray,
        keyWalletId: ByteArray,
        accountIndex: Int,
        fvkBytes: ByteArray,
    ): Int = guarded {
        require(walletId.contentEquals(keyWalletId)) {
            "viewing-key entry walletId does not match its persistence batch"
        }
        stage(walletId) { db ->
            // Entity construction is the validation boundary. A malformed
            // fixed-size field returns non-zero immediately (or rolls back
            // the containing changeset) instead of persisting corruption.
            db.shieldedDao().upsertViewingKey(
                ShieldedViewingKeyEntity(
                    walletId = keyWalletId,
                    accountIndex = accountIndex,
                    fvkBytes = fvkBytes,
                    lastUpdated = now(),
                ),
            )
        }
        0
    }

    // ── Load callbacks ────────────────────────────────────────────────

    override fun onLoadWalletList(): Array<WalletRestoreData> = guardedLoad(emptyArray()) {
        runBlockingResult {
            // Restorable = wallet with ≥1 account carrying an xpub,
            // scoped to the manager's network (see the constructor doc).
            val wallets = network
                ?.let { database.walletDao().getByNetwork(it.ffiValue) }
                ?: database.walletDao().observeAll().first()
            val out = ArrayList<WalletRestoreData>()
            for (w in wallets) {
                val accounts = database.accountDao().observeByWallet(w.walletId).first()
                    .filter { it.accountExtendedPubKeyBytes?.isNotEmpty() == true }
                    .sortedWith(
                        compareBy(
                            { it.accountType }, { it.accountIndex },
                            { it.registrationIndex }, { it.keyClass },
                        ),
                    )
                if (accounts.isEmpty()) continue
                val specs = accounts.map { a ->
                    AccountSpecData(
                        typeTag = a.accountType.toByte(),
                        standardTag = a.standardTag.toByte(),
                        index = a.accountIndex,
                        registrationIndex = a.registrationIndex,
                        keyClass = a.keyClass,
                        userIdentityId = a.userIdentityId,
                        friendIdentityId = a.friendIdentityId,
                        accountXpubBytes = a.accountExtendedPubKeyBytes ?: ByteArray(0),
                    )
                }.toTypedArray()
                val syncState = w.networkRaw?.let {
                    database.platformAddressDao().getSyncState(syncStateScopeId(it))
                }
                // Wallet-owned identities + their public keys, so each
                // restored `Identity` enters the in-memory `IdentityManager`
                // with a populated `public_keys` map — the gap that left
                // cold-started identities keyless and every write rejected
                // at DPP validation (mirror of Swift `buildIdentityRestoreBuffer`).
                val identities = buildIdentityRestoreData(w.walletId)
                // Cached platform-address balances — re-seeds the Rust
                // provider's per-account balance map + `as_of_height` pins
                // on cold start so the next BLAST sync resumes from the
                // persisted absolute instead of an empty found map. Without
                // this a credit at/below the trusted watermark is lost
                // across relaunches (SH-06). Mirror of the Swift
                // `loadCachedBalances` slice on `loadWalletList`.
                val platformAddressBalances = buildPlatformAddressBalances(w.walletId)
                // Unspent Core UTXOs — rehydrates the funds-bearing
                // accounts' UTXO maps (and via Rust's `update_balance`
                // the Core balance) on cold start. Without this the
                // balance reads 0 after every relaunch until a full SPV
                // re-scan (CORE-06). Mirror of the Swift
                // `buildUtxoRestoreBuffer` slice on `loadWalletList`.
                val utxos = buildUtxoRestoreData(w.walletId)
                // Persisted Core address pools — re-seed each funds
                // account's `AddressPool` so every restored address (incl.
                // those past the gap-limit window) maps back to a
                // derivation path; without this a restored UTXO on an
                // out-of-window address can't be signed after a cold
                // restart. Built from the SAME `accounts` list so each
                // pool's account tuple matches a restored account. Mirror
                // of the Swift `buildCoreAddressPoolBuffer` slice.
                val coreAddressPools = buildCoreAddressPoolData(accounts)
                // Tracked asset locks — rehydrate the Rust
                // `unused_asset_locks` map so a registration / top-up
                // funding flow interrupted mid-flight resumes from its
                // latest persisted status instead of re-deriving a fresh
                // lock. ALL rows (Rust drops `Consumed` itself). Mirror of
                // the Swift `buildAssetLockRestoreBuffer` slice.
                val trackedAssetLocks = buildTrackedAssetLockData(w.walletId)
                // Funding-tx records for the `statusRaw < 2` locks — re-seed
                // the in-memory transactions map so the next chain-lock
                // event can cascade-promote a still-`Broadcast` lock whose
                // block was already chain-locked. Mirror of the Swift
                // `buildUnresolvedAssetLockTxRecordBuffer` slice.
                val unresolvedAssetLockTxRecords =
                    buildUnresolvedAssetLockTxRecordData(w.walletId)
                val providerSpecialTxs = buildProviderSpecialTxRestoreData(w.walletId)
                // Persisted last-applied chainlock — decoded + stamped onto
                // the restored `WalletMetadata` so the asset-lock-resume
                // CL-from-metadata fallback can fire at launch without
                // waiting for a fresh SPV chainlock. Empty → null / 0
                // Rust-side. Mirror of the Swift `w.lastAppliedChainLockBytes`
                // slice.
                val lastAppliedChainLockBytes =
                    w.lastAppliedChainLockBytes ?: ByteArray(0)
                out.add(
                    WalletRestoreData(
                        walletId = w.walletId,
                        network = w.networkRaw ?: NETWORK_TESTNET,
                        accountSpecs = specs,
                        platformSyncHeight = syncState?.syncHeight ?: 0L,
                        platformSyncTimestamp = syncState?.syncTimestamp ?: 0L,
                        platformLastKnownRecentBlock = syncState?.lastKnownRecentBlock ?: 0L,
                        birthHeight = w.birthHeight,
                        syncedHeight = w.syncedHeight,
                        // No separate column — Swift reuses syncedHeight.
                        lastProcessedHeight = w.syncedHeight,
                        lastSynced = w.lastSynced,
                        identities = identities,
                        platformAddressBalances = platformAddressBalances,
                        utxos = utxos,
                        coreAddressPools = coreAddressPools,
                        trackedAssetLocks = trackedAssetLocks,
                        unresolvedAssetLockTxRecords = unresolvedAssetLockTxRecords,
                        providerSpecialTxs = providerSpecialTxs,
                        lastAppliedChainLockBytes = lastAppliedChainLockBytes,
                    ),
                )
            }
            out.toTypedArray()
        }
    }

    override fun onLoadShieldedNotes(): Array<ShieldedNoteData> = guardedLoad(emptyArray()) {
        runBlockingResult {
            // Shielded rows carry no wallet FK — read the whole table
            // directly (mirror of the Swift loader's fetch-all).
            database.shieldedDao().getAllNotes().map { n ->
                ShieldedNoteData(
                    walletId = n.walletId,
                    accountIndex = n.accountIndex,
                    position = n.position,
                    cmx = n.cmx,
                    nullifier = n.nullifier,
                    blockHeight = n.blockHeight,
                    isSpent = if (n.isSpent) 1 else 0,
                    value = n.value,
                    noteData = n.noteData,
                )
            }.toTypedArray()
        }
    }

    override fun onLoadShieldedOutgoingNotes(): Array<ShieldedOutgoingNoteData> =
        guardedLoad(emptyArray()) {
            runBlockingResult {
                database.shieldedDao().getAllOutgoingNotes()
                    .filter { it.recipient.size == 43 }
                    .map { n ->
                        ShieldedOutgoingNoteData(
                            walletId = n.walletId,
                            accountIndex = n.accountIndex,
                            cmx = n.cmx,
                            recipient = n.recipient,
                            value = n.value,
                            blockHeight = n.blockHeight,
                            memo = n.memo,
                        )
                    }.toTypedArray()
            }
        }

    override fun onLoadShieldedSyncStates(): Array<ShieldedSyncStateData> =
        guardedLoad(emptyArray()) {
            runBlockingResult {
                database.shieldedDao().getAllSyncStates().map { s ->
                    ShieldedSyncStateData(
                        walletId = s.walletId,
                        accountIndex = s.accountIndex,
                        lastSyncedIndex = s.lastSyncedIndex,
                    )
                }.toTypedArray()
            }
        }

    override fun onLoadShieldedActivity(): Array<ShieldedActivityData> = guardedLoad(emptyArray()) {
        runBlockingResult {
            database.shieldedDao().getAllActivity().map { a ->
                ShieldedActivityData(
                    walletId = a.walletId,
                    accountIndex = a.accountIndex,
                    entryId = a.entryId,
                    kindTag = a.kindTag.toByte(),
                    direction = a.direction.toByte(),
                    status = a.status.toByte(),
                    amount = a.amount,
                    fee = a.fee,
                    hasFee = a.hasFee,
                    blockHeight = a.blockHeight,
                    hasBlockHeight = a.hasBlockHeight,
                    createdAtMs = a.createdAtMs,
                    identityId = a.identityId,
                    hasIdentityId = a.identityId.size == 32,
                    counterparty = a.counterparty,
                    memo = a.memo,
                    noteCmxs = a.noteCmxs,
                    spentNullifiers = a.spentNullifiers,
                )
            }.toTypedArray()
        }
    }

    /**
     * Unlike best-effort cache loaders, a malformed persisted viewing key
     * must fail the native load. Returning an empty array would masquerade as
     * "no persisted key" and silently fall back to mnemonic resolution.
     * Therefore validation/Room exceptions deliberately cross this virtual
     * method into the JNI trampoline, which returns a non-zero FFI load code.
     * The trampoline owns and frees its copied native restore array.
     */
    override fun onLoadShieldedViewingKeys(): Array<ShieldedViewingKeyData> =
        runBlocking {
            callbackExclusion.withLock {
                runBlockingResult {
                    val keys = network?.let { lockedNetwork ->
                        database.walletDao().getByNetwork(lockedNetwork.ffiValue)
                            .flatMap { wallet ->
                                database.shieldedDao().getViewingKeysByWallet(wallet.walletId)
                            }
                    } ?: database.shieldedDao().getAllViewingKeys()
                    keys.map { key ->
                        ShieldedViewingKeyData(
                            walletId = key.walletId,
                            accountIndex = key.accountIndex,
                            fvkBytes = key.fvkBytes,
                        )
                    }.toTypedArray()
                }
            }
        }

    override fun onGetCoreTxRecord(walletId: ByteArray, txid: ByteArray): CoreTxRecordData? =
        guardedLoad(null) {
            runBlockingResult {
                // walletId unused — txid is globally unique.
                val tx = database.transactionDao().getByTxid(txid) ?: return@runBlockingResult null
                if (tx.transactionData.isEmpty()) return@runBlockingResult null
                if (tx.context >= CONTEXT_IN_BLOCK &&
                    (tx.blockHash == null || tx.blockHash.size != 32)
                ) {
                    return@runBlockingResult null
                }
                CoreTxRecordData(
                    contextKind = tx.context.toByte(),
                    blockHeight = tx.blockHeight,
                    blockHash = tx.blockHash ?: ByteArray(32),
                    blockTimestamp = tx.blockTimestamp,
                    txBytes = tx.transactionData,
                )
            }
        }

    // ── Shared load helpers ───────────────────────────────────────────

    /**
     * Assemble the [IdentityRestoreData] rows for one wallet: every
     * wallet-owned identity plus its public keys, sorted by identity index
     * then key id for a deterministic rehydrated `IndexMap` order (mirror
     * of Swift `buildIdentityRestoreBuffer`).
     *
     * Public-key rows are keyed in Room by the identity's **base58** id
     * (`onPersistIdentityKeyUpsert` writes `identityId.toBase58String()`),
     * so we look them up under that encoding. Discriminant strings
     * (`purpose` / `securityLevel` / `keyType` stored as `rawValue`
     * decimal) parse back to `UInt8`; an unparseable value maps to the
     * out-of-range `255` sentinel so the Rust side drops the row rather
     * than coercing it to MASTER/AUTHENTICATION (matches the Swift
     * `UInt8.max` fallback).
     */
    private suspend fun buildIdentityRestoreData(walletId: ByteArray): Array<IdentityRestoreData> {
        val idRows = database.identityDao().observeByWallet(walletId).first()
            .sortedBy { it.identityIndex }
        if (idRows.isEmpty()) return emptyArray()
        return idRows.map { idRow ->
            val base58 = idRow.identityId.toBase58String()
            val keyRows = database.publicKeyDao().observeByIdentityId(base58).first()
                .sortedBy { it.keyId }
                .map { pk ->
                    // ContractBounds → (kind, 32-byte id, doc-type). Inverse
                    // of `contractBoundsIdToJson` on the persist side:
                    //   * no blob        → kind 0 (unbounded)
                    //   * blob + docType → kind 2 (SingleContractDocumentType)
                    //   * blob, no docType → kind 1 (SingleContract)
                    // A blob that fails to decode to 32 bytes degrades to
                    // kind 0 rather than crashing FFI marshalling.
                    val boundsId = pk.contractBoundsData?.let { contractBoundsJsonToId(it) }
                    val (kind, id) = when {
                        boundsId == null -> 0.toByte() to ByteArray(0)
                        pk.contractBoundsDocumentTypeName != null -> 2.toByte() to boundsId
                        else -> 1.toByte() to boundsId
                    }
                    IdentityKeyRestoreData(
                        keyId = pk.keyId,
                        keyType = (pk.keyType.toIntOrNull() ?: 255).toByte(),
                        purpose = (pk.purpose.toIntOrNull() ?: 255).toByte(),
                        securityLevel = (pk.securityLevel.toIntOrNull() ?: 255).toByte(),
                        readOnly = pk.readOnly,
                        data = pk.publicKeyData,
                        contractBoundsKind = kind,
                        contractBoundsId = id,
                        contractBoundsDocumentType =
                            if (kind.toInt() == 2) pk.contractBoundsDocumentTypeName else null,
                    )
                }.toTypedArray()
            // DashPay contact rows — pending + established requests with
            // their contactInfo metadata (mirror of the Swift
            // `buildIdentityRestoreBuffer` contact block). Without these,
            // contacts only re-derive from chain on the first sweep and the
            // owner-private metadata is wiped during the DIP-15
            // deferred-publish window.
            val contactRows = database.dashpayDao()
                .getContactRequestsByOwner(idRow.identityId)
                .map { c ->
                    ContactRequestRestoreData(
                        ownerIdentityId = c.ownerIdentityId,
                        contactIdentityId = c.contactIdentityId,
                        isOutgoing = c.isOutgoing,
                        senderKeyIndex = c.senderKeyIndex,
                        recipientKeyIndex = c.recipientKeyIndex,
                        accountReference = c.accountReference,
                        encryptedPublicKey = c.encryptedPublicKey,
                        encryptedAccountLabel = c.encryptedAccountLabel,
                        autoAcceptProof = c.autoAcceptProof,
                        coreHeightCreatedAt = c.coreHeightCreatedAt,
                        createdAtMillis = c.createdAtMillis,
                        paymentChannelBroken = c.paymentChannelBroken,
                        alias = c.contactAlias,
                        note = c.contactNote,
                        isHidden = c.contactHidden,
                        contactAccountLabel = c.contactAccountLabel,
                        acceptedAccounts = decodeAcceptedAccounts(c.contactAcceptedAccounts),
                    )
                }.toTypedArray()
            // Ignored senders (per-sender mute) — restores the Rust
            // `ignored_senders` set so a previously-ignored sender doesn't
            // resurface after a relaunch. Drop any row with a wrong-length
            // id BEFORE handing it to the trampoline (which fails the whole
            // load on a non-32-byte id — same abort-on-corrupt convention
            // as the Swift buffer builder's up-front filter).
            val ignoredRows = database.dashpayDao()
                .getIgnoredSendersByOwner(idRow.identityId)
                .map { it.ignoredSenderId }
                .filter { it.size == 32 }
                .toTypedArray()
            // Payment history — restores the Rust `dashpay_payments` map
            // so Sent entries (and their memos) survive relaunch; the
            // reconcile sweep can only re-derive Received entries. Rows
            // reach Room solely via `refreshDashPayPayments` (the sweep
            // reconciles in-memory without persisting).
            val paymentRows = database.dashpayDao()
                .getPaymentsByOwner(idRow.identityId)
                // Wrong-length counterparty ids and empty txids are dropped
                // up front: the Rust restore fold inserts whatever key it is
                // given (an empty txid would land as an "" map key rather
                // than being skipped).
                .filter { it.counterpartyIdentityId.size == 32 && it.txid.isNotEmpty() }
                .map { p ->
                    PaymentRestoreData(
                        txid = p.txid,
                        counterpartyId = p.counterpartyIdentityId,
                        amountDuffs = p.amountDuffs,
                        directionRaw = p.directionRaw.toByte(),
                        statusRaw = p.statusRaw.toByte(),
                        memo = p.memo,
                    )
                }.toTypedArray()
            // Cached contact profiles (present only — tombstones deleted
            // the row at persist time) — restores the Rust
            // `contact_profiles` map so the contacts UI shows names +
            // avatars immediately after relaunch.
            val contactProfileRows = database.dashpayDao()
                .getContactProfilesByOwner(idRow.identityId)
                .filter { it.contactIdentityId.size == 32 }
                .map { cp ->
                    ContactProfileRestoreData(
                        contactId = cp.contactIdentityId,
                        displayName = cp.displayName,
                        bio = cp.bio,
                        avatarUrl = cp.avatarUrl,
                        avatarHash = cp.avatarHash,
                        avatarFingerprint = cp.avatarFingerprint,
                        publicMessage = cp.publicMessage,
                        checkedAtMs = cp.checkedAtMs,
                    )
                }.toTypedArray()
            IdentityRestoreData(
                identityId = idRow.identityId,
                balance = idRow.balance,
                revision = idRow.revision,
                identityIndex = idRow.identityIndex,
                // No `status` column on IdentityEntity (matches Swift, which
                // also lacks it) — fall back to Unknown(0); the next identity
                // sync round re-stamps it via the identity changeset path.
                status = 0,
                keys = keyRows,
                contacts = contactRows,
                ignoredSenders = ignoredRows,
                payments = paymentRows,
                contactProfiles = contactProfileRows,
            )
        }.toTypedArray()
    }

    /**
     * Assemble the [PlatformAddressBalanceRestoreData] rows for one
     * wallet: every persisted `platform_addresses` row, mapped to the
     * `AddressBalanceEntryFFI` field set so the Rust load path re-seeds
     * the provider's per-account balance map (mirror of the Swift
     * `loadCachedBalances`).
     *
     * The durable derivation metadata (`addressType`, 20-byte
     * `addressHash`, `accountIndex`, `addressIndex`) and the sync-derived
     * state (`balance`, `nonce`, and the `lastSeenHeight` height pin) all
     * round-trip. The height pin (→ `as_of_height`) is load-bearing for
     * ADDR-09: it MUST carry through unchanged, or a persisted credit at
     * or below the trusted watermark is re-gated off and lost after
     * relaunch (SH-06).
     *
     * Rows whose `addressHash` isn't 20 bytes are dropped up front — the
     * trampoline packs exactly `platformAddressBalances.size` fixed-hash
     * `AddressBalanceEntryFFI` slots, so a wrong-length hash would fail
     * the fixed-length read and abort the whole load (matching the
     * abort-on-corrupt convention of the other restore builders; the
     * Swift buffer builder skips the row for the same reason).
     */
    private suspend fun buildPlatformAddressBalances(
        walletId: ByteArray,
    ): Array<PlatformAddressBalanceRestoreData> =
        database.platformAddressDao().observeByWallet(walletId).first()
            .filter { it.addressHash.size == 20 }
            .map { row ->
                PlatformAddressBalanceRestoreData(
                    addressType = row.addressType.toByte(),
                    addressHash = row.addressHash,
                    balance = row.balance,
                    nonce = row.nonce,
                    accountIndex = row.accountIndex,
                    addressIndex = row.addressIndex,
                    // `lastSeenHeight` is the persisted `as_of_height`
                    // pin (see onPersistAddressBalance); a stored `0`
                    // (never-synced) yields to the first pinned absolute
                    // on the Rust load path — the self-healing full
                    // reconcile.
                    asOfHeight = row.lastSeenHeight.toLong(),
                )
            }.toTypedArray()

    /**
     * Build the unchanged provider-special restore POD inputs for one
     * wallet. Membership comes only from the explicit typed-account join,
     * so payload-only transactions require no TXO. Empty raw bodies and
     * malformed block hashes are diagnosed and skipped here; non-empty
     * consensus bytes remain opaque and Rust performs authoritative decode,
     * diagnosing/skipping malformed payloads without crashing.
     */
    private suspend fun buildProviderSpecialTxRestoreData(
        walletId: ByteArray,
    ): Array<ProviderSpecialTxRestoreData> {
        val out = ArrayList<ProviderSpecialTxRestoreData>()
        for (tx in database.transactionDao().getProviderSpecialTransactionsByWallet(walletId)) {
            if (tx.transactionData.isEmpty()) {
                Log.w(TAG, "load: skipping provider transaction with empty consensus bytes")
                continue
            }
            val hash = tx.blockHash ?: ByteArray(32)
            if (hash.size != 32) {
                Log.w(TAG, "load: skipping provider transaction with ${hash.size}-byte block hash")
                continue
            }
            out.add(
                ProviderSpecialTxRestoreData(
                    txBytes = tx.transactionData,
                    contextRaw = tx.context,
                    blockHeight = tx.blockHeight,
                    blockHash = hash,
                    blockTimestamp = tx.blockTimestamp.toLong() and 0xFFFF_FFFFL,
                    blockPosition = tx.blockPosition,
                    hasBlockPosition = tx.hasBlockPosition,
                    firstSeen = tx.firstSeen,
                ),
            )
        }
        return out.toTypedArray()
    }

    /**
     * Assemble the [UtxoRestoreData] rows for one wallet: every unspent
     * `txos` row, routed to its owning account for the leading
     * account-tag block the Rust load path uses to file the UTXO into
     * the right funds account (mirror of the Swift
     * `buildUtxoRestoreBuffer`).
     *
     * Routing: Swift reads the txo's parent-account relationship; on
     * Android `txos.accountId` is not populated by the changeset write
     * path, so the owning account resolves through the address instead
     * (`txos.address → core_addresses.accountId → accounts`) with the
     * FK as a fast path when present. Rows that resolve to no account
     * are skipped with a log (the Swift builder skips them the same
     * way) — a UTXO the wallet can't attribute can't be routed.
     *
     * Stale-flag guard: a row still `isSpent = false` whose linked
     * spending tx is already in-block was consumed but missed its flip
     * (pre-reconcile rows; see `onWalletChangesetTransaction`). Handing
     * it back to Rust would overstate the balance as spendable, so it
     * is excluded here. Mempool-linked spends (spending tx not yet
     * in-block) stay IN the restore set — same semantics as iOS, where
     * the post-restart catch-up classifier needs the TXO back to
     * recognise the spend.
     *
     * `prevTxid` must be exactly 32 bytes (the trampoline's fixed-length
     * read aborts the whole load otherwise), so wrong-length rows are
     * pre-dropped like the SH-06 hash filter. Account tags outside the
     * u8 range are dropped for the same reason Swift aborts on them —
     * except here the single row is skipped rather than failing the
     * whole load, matching this builder's per-row-skip convention.
     */
    private suspend fun buildUtxoRestoreData(walletId: ByteArray): Array<UtxoRestoreData> {
        val rows = database.txoDao().observeUnspentByWallet(walletId).first()
        if (rows.isEmpty()) return emptyArray()
        val out = ArrayList<UtxoRestoreData>(rows.size)
        // Per-call memo of address → account row (a wallet's UTXOs
        // cluster on few addresses/accounts; avoids N duplicate joins).
        val accountByAddress = HashMap<String, AccountEntity?>()
        for (txo in rows) {
            // Stale-flag guard (see doc): consumed-but-unflipped rows
            // must not rehydrate as spendable.
            val spendingTxid = txo.spendingTxid
            if (spendingTxid != null) {
                val spending = database.transactionDao().getByTxid(spendingTxid)
                if (spending != null && spending.context >= CONTEXT_IN_BLOCK) continue
            }
            val account = txo.accountId?.let { database.accountDao().getById(it) }
                ?: accountByAddress.getOrPut(txo.address) {
                    database.coreAddressDao().getByAddress(txo.address)
                        ?.accountId?.let { database.accountDao().getById(it) }
                }
            if (account == null) {
                Log.w(TAG, "load: skipping UTXO with no resolvable account (address=${txo.address})")
                continue
            }
            if (account.accountType !in 0..255) {
                Log.w(TAG, "load: skipping UTXO with out-of-range accountType=${account.accountType}")
                continue
            }
            val typeTag = account.accountType.toByte()
            val prevTxid = txo.txid ?: txo.outpoint.copyOfRange(0, minOf(32, txo.outpoint.size))
            if (prevTxid.size != 32) {
                Log.w(TAG, "load: skipping UTXO with ${prevTxid.size}-byte txid")
                continue
            }
            out.add(
                UtxoRestoreData(
                    typeTag = typeTag,
                    standardTag = account.standardTag.toByte(),
                    accountIndex = account.accountIndex,
                    registrationIndex = account.registrationIndex,
                    keyClass = account.keyClass,
                    userIdentityId = account.userIdentityId,
                    friendIdentityId = account.friendIdentityId,
                    prevTxid = prevTxid,
                    vout = txo.vout,
                    valueDuffs = txo.amount,
                    scriptPubKey = txo.scriptPubKey,
                    height = txo.height,
                    isCoinbase = txo.isCoinbase,
                    isConfirmed = txo.isConfirmed,
                    isInstantLocked = txo.isInstantLocked,
                    isLocked = txo.isLocked,
                ),
            )
        }
        return out.toTypedArray()
    }

    /**
     * Assemble the [CoreAddressPoolRestoreData] rows for one wallet's
     * accounts: for each account, its persisted `core_addresses` grouped
     * by pool type (external / internal / absent), so the Rust load path
     * can re-seed each funds account's `AddressPool` — restoring the
     * derivation-path mapping for every address, including those past the
     * gap-limit window `ManagedWalletInfo::from_wallet` pre-derives.
     *
     * Without this, a restored UTXO on an out-of-window address has no
     * derivation-path mapping and the wallet cannot sign a core-to-core
     * spend after a cold restart. Mirror of the Swift
     * `buildCoreAddressPoolBuffer`.
     *
     * The account tuple carried on each pool matches an account already
     * present in the restored wallet (built from the SAME `accounts` list
     * `onLoadWalletList` gathered), with empty xpub bytes — the Rust
     * loader ignores the xpub on this path (`account_xpub_bytes` is null on
     * `AccountAddressPoolFFI.account`). Accounts with no persisted
     * addresses emit no pool. Groups are sorted by pool tag to match the
     * Swift ordering.
     */
    private suspend fun buildCoreAddressPoolData(
        accounts: List<AccountEntity>,
    ): Array<CoreAddressPoolRestoreData> {
        val out = ArrayList<CoreAddressPoolRestoreData>()
        for (account in accounts) {
            val rows = database.coreAddressDao().observeByAccount(account.id).first()
            if (rows.isEmpty()) continue
            val spec = AccountSpecData(
                typeTag = account.accountType.toByte(),
                standardTag = account.standardTag.toByte(),
                index = account.accountIndex,
                registrationIndex = account.registrationIndex,
                keyClass = account.keyClass,
                userIdentityId = account.userIdentityId,
                friendIdentityId = account.friendIdentityId,
                // Loader ignores the xpub on this path; the account
                // already carries it via `accountSpecs`.
                accountXpubBytes = ByteArray(0),
            )
            // Group the account's addresses by pool type, then emit one
            // pool per tag in ascending tag order (Swift ordering).
            rows.groupBy { it.poolTypeTag }
                .toSortedMap()
                .forEach { (poolTypeTag, poolRows) ->
                    val addresses = poolRows.map { row ->
                        CoreAddressRestoreData(
                            // `has_public_key` is derived Rust-side from
                            // `publicKey.size == 33`; pass the bytes through.
                            publicKey = row.publicKey,
                            poolTypeTag = poolTypeTag.toByte(),
                            addressIndex = row.addressIndex,
                            isUsed = row.isUsed,
                            balance = row.balance,
                            addressBase58 = row.address,
                            derivationPath = row.derivationPath,
                        )
                    }.toTypedArray()
                    out.add(
                        CoreAddressPoolRestoreData(
                            account = spec,
                            poolTypeTag = poolTypeTag.toByte(),
                            addresses = addresses,
                        ),
                    )
                }
        }
        return out.toTypedArray()
    }

    /**
     * Assemble the [TrackedAssetLockRestoreData] rows for one wallet: every
     * persisted `asset_locks` row (ALL statuses — the Rust load path
     * `build_unused_asset_locks` is the sole filter and skips `Consumed`
     * itself, so this builder emits terminal rows too, matching the Swift
     * `loadCachedAssetLocksOnQueue`).
     *
     * The persisted primary key `outPointHex` is display-order; [decodeOutPointHex]
     * flips it back to the 36-byte wire form the Rust trampoline reads via a
     * fixed-length field, so a malformed key drops the row rather than
     * aborting the whole load. A row with empty `transactionBytes` is broken
     * (the Rust loader rejects it) and is dropped here. `fundingTypeRaw` /
     * `statusRaw` outside `0..255` are dropped-and-logged — the same
     * altered-state hazard the Swift `UInt8(exactly:)` guard rejects (a
     * clamping cast would silently rewrite the lock's effective enum value).
     *
     * Without this, an interrupted identity registration re-derives a fresh
     * asset lock on relaunch instead of resuming the persisted one. Mirror
     * of the Swift `buildAssetLockRestoreBuffer`.
     */
    private suspend fun buildTrackedAssetLockData(
        walletId: ByteArray,
    ): Array<TrackedAssetLockRestoreData> {
        val rows = database.assetLockDao().observeByWallet(walletId).first()
        if (rows.isEmpty()) return emptyArray()
        val out = ArrayList<TrackedAssetLockRestoreData>(rows.size)
        for (row in rows) {
            val outPoint = decodeOutPointHex(row.outPointHex)
            if (outPoint == null) {
                Log.w(TAG, "load: dropping asset-lock row with malformed outPointHex=${row.outPointHex}")
                continue
            }
            if (row.transactionBytes.isEmpty()) {
                Log.w(TAG, "load: dropping asset-lock row with empty transactionBytes: ${row.outPointHex}")
                continue
            }
            if (row.fundingTypeRaw !in 0..255) {
                Log.w(TAG, "load: dropping asset-lock row ${row.outPointHex} — fundingTypeRaw out of u8 range: ${row.fundingTypeRaw}")
                continue
            }
            if (row.statusRaw !in 0..255) {
                Log.w(TAG, "load: dropping asset-lock row ${row.outPointHex} — statusRaw out of u8 range: ${row.statusRaw}")
                continue
            }
            out.add(
                TrackedAssetLockRestoreData(
                    outPoint = outPoint,
                    transactionBytes = row.transactionBytes,
                    accountIndex = row.accountIndexRaw,
                    fundingType = row.fundingTypeRaw.toByte(),
                    identityIndex = row.identityIndexRaw,
                    amountDuffs = row.amountDuffs,
                    status = row.statusRaw.toByte(),
                    // Rust maps empty → null / 0 (an absent proof).
                    proofBytes = row.proofBytes ?: ByteArray(0),
                ),
            )
        }
        return out.toTypedArray()
    }

    /**
     * Assemble the [UnresolvedAssetLockTxRecordData] rows for one wallet:
     * one per asset-lock row at `statusRaw < 2` (Built / Broadcast) whose
     * funding tx has a matching `transactions` row. The Rust load path
     * re-inserts each into the matching BIP44 account's in-memory
     * `transactions()` map so the next chain-lock event can cascade-promote
     * it via `apply_chain_lock`.
     *
     * The wire-order txid is the first 32 bytes of the decoded outpoint; a
     * malformed outpoint, a missing transaction row, or an empty
     * `transactionData` blob drops the record (Rust can't reconstruct the
     * funding body without its consensus bytes). Mirror of the Swift
     * `buildUnresolvedAssetLockTxRecordBuffer`.
     */
    private suspend fun buildUnresolvedAssetLockTxRecordData(
        walletId: ByteArray,
    ): Array<UnresolvedAssetLockTxRecordData> {
        val locks = database.assetLockDao().getUnresolvedByWallet(walletId)
        if (locks.isEmpty()) return emptyArray()
        val out = ArrayList<UnresolvedAssetLockTxRecordData>(locks.size)
        for (lock in locks) {
            val outPoint = decodeOutPointHex(lock.outPointHex) ?: continue
            val txid = outPoint.copyOfRange(0, 32)
            val tx = database.transactionDao().getByTxid(txid) ?: continue
            if (tx.transactionData.isEmpty()) continue
            out.add(
                UnresolvedAssetLockTxRecordData(
                    accountIndex = lock.accountIndexRaw,
                    txBytes = tx.transactionData,
                    contextRaw = tx.context,
                    blockHeight = tx.blockHeight,
                    blockHash = tx.blockHash ?: ByteArray(0),
                    blockTimestamp = tx.blockTimestamp.toLong(),
                    firstSeen = tx.firstSeen,
                ),
            )
        }
        return out.toTypedArray()
    }

    // ── Shared write helpers ──────────────────────────────────────────

    /**
     * Upsert an account by its identity tuple. Room forbids `@Upsert` on
     * `accounts` (the surrogate PK + unique tuple would delete-and-reinsert,
     * cascading away children), so we fetch-then-insert/update like Swift.
     */
    private suspend fun upsertAccount(
        db: DashDatabase,
        walletId: ByteArray,
        accountType: Int,
        accountIndex: Int,
        standardTag: Int,
        registrationIndex: Int,
        keyClass: Int,
        userIdentityId: ByteArray,
        friendIdentityId: ByteArray,
        xpub: ByteArray?,
    ) {
        val existing = fetchAccount(
            db, walletId, accountType, accountIndex, standardTag,
            registrationIndex, keyClass, userIdentityId, friendIdentityId,
        )
        val row = (existing ?: AccountEntity(
            walletId = walletId,
            accountType = accountType,
            accountIndex = accountIndex,
            accountTypeName = accountTypeName(accountType, standardTag),
        )).copy(
            standardTag = standardTag,
            registrationIndex = registrationIndex,
            keyClass = keyClass,
            userIdentityId = userIdentityId,
            friendIdentityId = friendIdentityId,
            accountExtendedPubKeyBytes = xpub ?: existing?.accountExtendedPubKeyBytes,
            accountTypeName = accountTypeName(accountType, standardTag),
            lastUpdated = now(),
        )
        if (existing == null) db.accountDao().insert(row) else db.accountDao().update(row)
    }

    /**
     * Fetch the account matching the full identity tuple. `getByKey`
     * narrows on `(walletId, accountType, accountIndex)`; the remaining
     * fields are verified in code, mirroring the Swift lookup.
     */
    private suspend fun fetchAccount(
        db: DashDatabase,
        walletId: ByteArray,
        accountType: Int,
        accountIndex: Int,
        standardTag: Int,
        registrationIndex: Int,
        keyClass: Int,
        userIdentityId: ByteArray,
        friendIdentityId: ByteArray,
    ): AccountEntity? =
        db.accountDao().getByKey(walletId, accountType, accountIndex).firstOrNull { c ->
            c.standardTag == standardTag &&
                c.registrationIndex == registrationIndex &&
                c.keyClass == keyClass &&
                c.userIdentityId.contentEquals(userIdentityId) &&
                c.friendIdentityId.contentEquals(friendIdentityId)
        }

    private suspend fun coreAddressIdIfPresent(db: DashDatabase, address: String?): String? {
        if (address == null) return null
        return if (db.coreAddressDao().getByAddress(address) != null) address else null
    }

    // ── Wallet deletion (port of PlatformWalletPersistenceHandler.swift
    //    `deleteWalletData` / `identityIdsForWallet`) ───────────────────

    /**
     * The identity ids owned by [walletId] (empty if the wallet row is
     * absent) — port of Swift `identityIdsForWallet(walletId:)`.
     *
     * [PlatformWalletManager.removeWallet] reads these BEFORE the Room
     * wipe so it can purge each identity's Keystore private keys while the
     * `public_keys` rows still exist to enumerate the pubkey hexes.
     */
    suspend fun identityIdsForWallet(walletId: ByteArray): List<ByteArray> =
        runBlockingResult {
            database.identityDao().observeByWallet(walletId).first().map { it.identityId }
        }

    /**
     * Wipe a wallet's Room footprint — port of Swift `deleteWalletData`.
     *
     * SQLite cascades cleanly in one pass (unlike SwiftData, which fatals
     * nullifying a non-optional inverse mid-save and forces the Swift
     * multi-phase delete), so the wallet-owned CASCADE children — accounts
     * → core/platform addresses, and every identity's public keys / DPNS
     * names / DashPay profile / contact requests / contact profiles /
     * payments / ignored senders — drop when their parent row is deleted.
     * The rows this delete must handle explicitly are:
     *  - identities (SET_NULL from wallet, Swift `.nullify`) + their
     *    SET_NULL `token_balances`;
     *  - the walletId-keyed tables with no wallet FK: txos, pending
     *    inputs, asset locks, invitations, platform addresses + sync
     *    state, and the five shielded (Orchard) tables.
     *
     * The platform-addresses network sync-state row is shared across a
     * network's wallets (keyed by [syncStateScopeId]); it is dropped only
     * when no sibling wallet remains on this network — mirroring Swift's
     * sibling check. Idempotent: deleting an already-removed wallet is a
     * no-op.
     *
     * Runs under [withCallbackExclusion] so no persistence callback (and
     * so no changeset commit) can interleave with the cascade. A caller
     * already inside the exclusion (the wallet-deletion sequence) must
     * use [deleteWalletDataLocked] instead — the mutex is not reentrant.
     */
    suspend fun deleteWalletData(walletId: ByteArray) =
        withCallbackExclusion { deleteWalletDataLocked(walletId) }

    /**
     * [deleteWalletData] body; the caller must hold [callbackExclusion]
     * (via [withCallbackExclusion]).
     */
    internal suspend fun deleteWalletDataLocked(walletId: ByteArray) = runBlockingCatching {
        // Discard any open changeset round for this wallet: its staged ops
        // would otherwise commit AFTER the cascade below and resurrect
        // rows for the deleted wallet. The pending/orphaned alias records
        // are dropped too — the wallet-deletion sweep has already consumed
        // them (unioned into its enumeration) before calling this.
        buffers.remove(walletId.toHex())
        pendingRoundAliases.remove(walletId.toHex())
        orphanedAliases.remove(walletId.toHex())
        database.withTransaction {
            val walletRow = database.walletDao().getByWalletId(walletId)
            val walletNetwork = walletRow?.networkRaw

            // Identities: SET_NULL from wallet, so delete them (and their
            // SET_NULL token balances) explicitly. The identity delete
            // cascades every other identity child (public keys, DPNS
            // names, DashPay rows).
            val identities = database.identityDao().observeByWallet(walletId).first()
            for (identity in identities) {
                database.tokenDao().deleteBalancesByIdentity(identity.identityId)
                database.identityDao().deleteByIdentityId(identity.identityId)
            }

            // walletId-keyed tables without a wallet FK — the wallet-row
            // delete below does not reach them.
            database.txoDao().deleteByWallet(walletId)
            database.documentDao().deletePendingInputsByWallet(walletId)
            database.assetLockDao().deleteByWallet(walletId)
            database.invitationDao().deleteByWallet(walletId)
            database.platformAddressDao().deleteByWallet(walletId)
            database.shieldedDao().deleteNotesByWallet(walletId)
            database.shieldedDao().deleteOutgoingNotesByWallet(walletId)
            database.shieldedDao().deleteActivityByWallet(walletId)
            database.shieldedDao().deleteSyncStatesByWallet(walletId)
            database.shieldedDao().deleteViewingKeysByWallet(walletId)

            // The wallet row itself — cascades accounts → core / platform
            // addresses.
            database.walletDao().deleteByWalletId(walletId)

            // Orphan-transaction sweep (Swift's post-delete pass): drop
            // transactions no TXO / pending-input references anymore.
            database.transactionDao().deleteOrphanTransactions()

            // Network sync-state is shared by every wallet on this
            // network; only drop it once the last one is gone.
            if (walletNetwork != null) {
                val siblings = database.walletDao().getByNetwork(walletNetwork)
                    .count { !it.walletId.contentEquals(walletId) }
                if (siblings == 0) {
                    database.platformAddressDao().deleteSyncState(syncStateScopeId(walletNetwork))
                }
            }
        }
        // Only AFTER the delete transaction commits: prune every pending
        // repair entry scoped to this wallet. The cascade above removed all
        // of the wallet's identities and their public-key rows, so those
        // entries are now phantoms whose rows and derivation breadcrumbs no
        // longer exist — leaving them would keep signalling hosts to repair
        // keys that can never be re-derived (dashpay/platform#4183 review).
        // Placed past the transaction (not staged with a round) so a throw
        // that rolls the delete back skips this line and preserves the valid
        // signals (Room's cascade cannot mutate this process-local StateFlow).
        _pendingIdentityKeys.update(clearPendingKeyByWalletDelta(walletId.toHex()))
    }

    // ── Pending identity-key bookkeeping (#4053) ──────────────────────

    // Every publish to the map goes through `MutableStateFlow.update`
    // (atomic compare-and-set) rather than a plain read-modify-write on
    // `.value`: the persistence callback publishes on the Rust caller
    // thread while `markIdentityKeyRepaired` can clear from an arbitrary
    // host thread (via PlatformWalletManager.repairIdentityKey). A
    // non-atomic read-then-write could interleave and drop one of the two
    // mutations — losing a record leaves a watch-only key with no queryable
    // pending state, losing a clear leaves a repaired key stale.
    //
    // The mutations themselves are expressed as pure map deltas so the
    // upsert callback can STAGE them with the round's [ChangesetBuffer]
    // instead of publishing mid-round (finding de3cf44a71fc): the pending
    // state a delta describes only becomes true when the round's Room
    // transaction commits, and an aborted round must leave no trace.

    private fun recordPendingKeyDelta(
        entry: PendingIdentityKey,
    ): (Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey> =
        { it + (entry.publicKeyHex to entry) }

    private fun clearPendingKeyDelta(
        publicKeyHex: String,
    ): (Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey> =
        { if (publicKeyHex in it) it - publicKeyHex else it }

    /**
     * Delta that drops any pending entry for the identity key
     * ([identityIdBase58], [keyId]) — the shape [onPersistIdentityKeyRemoval]
     * has (the map is keyed by public-key hex, which a removal callback does
     * not carry, so it matches on the entry's identity + keyId instead).
     * No-op (returns the same map instance) when nothing matches, mirroring
     * [clearPendingKeyDelta] so an unrelated removal publishes no map update.
     */
    private fun clearPendingKeyByIdentityKeyDelta(
        identityIdBase58: String,
        keyId: Int,
    ): (Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey> =
        { map ->
            if (map.values.any { it.identityIdBase58 == identityIdBase58 && it.keyId == keyId }) {
                map.filterValues {
                    !(it.identityIdBase58 == identityIdBase58 && it.keyId == keyId)
                }
            } else {
                map
            }
        }

    /**
     * Delta that drops EVERY pending entry belonging to identity
     * [identityIdBase58] (all of its key ids) — the shape
     * [onPersistIdentityRemoval] has. When an identity is removed Room
     * cascades away all of its `public_keys` rows, so each of its pending
     * repair entries is now a phantom (the key can never be re-derived into
     * an identity that no longer exists). No-op (returns the same map
     * instance) when nothing matches, mirroring [clearPendingKeyDelta] so an
     * unrelated removal publishes no map update.
     */
    private fun clearPendingKeyByIdentityDelta(
        identityIdBase58: String,
    ): (Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey> =
        { map ->
            if (map.values.any { it.identityIdBase58 == identityIdBase58 }) {
                map.filterValues { it.identityIdBase58 != identityIdBase58 }
            } else {
                map
            }
        }

    /**
     * Delta that drops EVERY pending entry belonging to wallet
     * [walletIdHex] — the shape [deleteWalletDataLocked] has. A wallet wipe
     * cascades away all of its identities and their `public_keys` rows, so
     * every pending repair entry scoped to it is a phantom afterwards.
     * No-op (returns the same map instance) when nothing matches.
     */
    private fun clearPendingKeyByWalletDelta(
        walletIdHex: String,
    ): (Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey> =
        { map ->
            if (map.values.any { it.walletIdHex == walletIdHex }) {
                map.filterValues { it.walletIdHex != walletIdHex }
            } else {
                map
            }
        }

    /**
     * Stage [delta] with the wallet's open round (published atomically by
     * [publishPendingKeyDeltas] after the round's transaction commits,
     * discarded on rollback/abort), or publish immediately when no round is
     * open — the standalone-callback path, whose Room write also commits
     * immediately. Caller must hold [callbackExclusion] (every persist
     * callback does), which also guards [buffers].
     */
    private fun stagePendingKeyDelta(
        walletIdHex: String,
        delta: (Map<String, PendingIdentityKey>) -> Map<String, PendingIdentityKey>,
    ) {
        val buffer = buffers[walletIdHex]
        if (buffer != null) {
            buffer.pendingKeyDeltas.add(delta)
        } else {
            _pendingIdentityKeys.update(delta)
        }
    }

    /** Publish a committed round's staged deltas in ONE atomic map update. */
    private fun publishPendingKeyDeltas(buffer: ChangesetBuffer) {
        if (buffer.pendingKeyDeltas.isEmpty()) return
        _pendingIdentityKeys.update { map ->
            buffer.pendingKeyDeltas.fold(map) { acc, delta -> delta(acc) }
        }
    }

    /**
     * Drop [publicKeyHex] from [pendingIdentityKeys] after a successful
     * out-of-band repair.
     *
     * [onPersistIdentityKeyUpsert] is the only *persist-callback* path that
     * clears a pending entry, but [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.repairIdentityKey]
     * re-derives and stores the private key directly through the deriver,
     * bypassing that callback — so it must call this on success or a repaired
     * key would linger in [pendingIdentityKeys] until an unrelated re-persist
     * happens to fire for the same key. Idempotent: clearing an absent key is a
     * no-op. Publishes immediately (never staged with a round): the repair's
     * scalar store already happened out-of-band, not inside any changeset.
     */
    internal fun markIdentityKeyRepaired(publicKeyHex: String) {
        _pendingIdentityKeys.update(clearPendingKeyDelta(publicKeyHex))
    }

    /**
     * Re-derive, verify, and durably repair the identity key identified by
     * [publicKeyData] — the orchestration behind
     * `PlatformWalletManager.repairIdentityKey`, hoisted here so it is
     * unit-testable (the manager cannot be constructed on the JVM) and so it
     * shares this handler's authoritative [pendingIdentityKeys] state.
     *
     * ## Derivation source (dashpay/platform#4060 blocker 1)
     *
     * The derivation indices are read from the PERSISTED `public_keys` row's
     * derivation breadcrumbs ([PublicKeyEntity.derivationIdentityIndex] /
     * [PublicKeyEntity.derivationKeyIndex]) — NEVER from a caller-supplied
     * key id. A caller-supplied index (e.g. the DPP key id) can derive a
     * DIFFERENT valid scalar that round-trips through encrypt/decrypt fine;
     * the deriver's [PrivateKeyDeriver.deriveAndStore] `force` path then
     * proves the derived PUBLIC key equals [publicKeyData] BEFORE persisting
     * and throws [org.dashfoundation.dashsdk.security.IdentityKeyDerivationMismatchException]
     * on mismatch (nothing persisted, pending state untouched). A row with no
     * breadcrumbs cannot be safely repaired, so the repair fails without
     * clearing pending.
     *
     * ## Durability (dashpay/platform#4060 blocker 3)
     *
     * The durable Room write (recording the storage identifier so the restart
     * reconstruction does not resurrect the key) fails CLOSED: if it throws,
     * the pending state is NOT cleared and the failure propagates, so the live
     * session and a subsequent restart agree the repair is still pending. A
     * swallowed durable-write failure that still cleared live pending state
     * would let the session believe the repair was done while a restart's
     * reconstruction resurrected it. Only after the blob is verified
     * recoverable AND the durable write commits is the key dropped from
     * [pendingIdentityKeys].
     *
     * @param verifyRecoverable the real-decrypt probe
     *   (`WalletStorage.probeIdentityKeyRecoverability`) proving the just-written
     *   blob actually opens; injected by the manager (this handler holds no
     *   `WalletStorage`).
     * @param persistDurableIdentifier the durable Room update (default: the
     *   production `public_keys` write); a seam so a failed durable write —
     *   which must NOT clear pending — is exercisable in tests.
     * @return the recorded storage identifier, or null when the deriver
     *   declined to store (pending left intact). Throws (pending left intact)
     *   on a derivation/verification/durable-write failure.
     */
    internal suspend fun repairIdentityKeyDurably(
        walletId: ByteArray,
        publicKeyData: ByteArray,
        verifyRecoverable: suspend (pubkeyHex: String) -> Boolean,
        persistDurableIdentifier: suspend (storageIdentifier: String) -> Unit = { storageIdentifier ->
            database.publicKeyDao().getByPublicKeyData(publicKeyData).forEach { row ->
                if (row.privateKeyKeychainIdentifier != storageIdentifier) {
                    database.publicKeyDao()
                        .update(row.copy(privateKeyKeychainIdentifier = storageIdentifier))
                }
            }
        },
    ): String? {
        val pubkeyHex = publicKeyData.toHex()
        val deriver = privateKeyDeriver
            ?: throw DashSdkError.PlatformWallet.SigningKeyUnavailable(
                "identity-key repair for $pubkeyHex has no private-key deriver wired; " +
                    "the key remains unusable and pending state is left intact",
            )

        // BLOCKER 1: read the derivation indices (and the DPP key type, so the
        // deriver's ownership check interprets publicKeyData correctly for
        // HASH160-typed keys — dashpay/platform#4183 review) from the persisted
        // row, never from the caller. A row lacking breadcrumbs cannot be
        // safely repaired (we would have to guess the slot), so fail WITHOUT
        // clearing pending.
        val breadcrumbs = database.publicKeyDao().getByPublicKeyData(publicKeyData)
            .firstNotNullOfOrNull { row ->
                val identityIndex = row.derivationIdentityIndex
                val keyIndex = row.derivationKeyIndex
                if (identityIndex != null && keyIndex != null) {
                    Triple(identityIndex, keyIndex, row.keyType.toIntOrNull() ?: 0)
                } else {
                    null
                }
            } ?: throw DashSdkError.PlatformWallet.SigningKeyUnavailable(
                "cannot repair identity key $pubkeyHex: no derivation breadcrumbs are " +
                    "persisted for it (derivationIdentityIndex/derivationKeyIndex are " +
                    "null) — the correct slot is unknown; pending state left intact",
            )
        val (identityIndex, keyIndex, keyType) = breadcrumbs

        // force = true routes through WalletStorage.replacePrivateKey and, in
        // the production deriver, derives the KEYPAIR and verifies the derived
        // public key equals publicKeyData (HASH160-hashed first for HASH160 key
        // types) BEFORE any store — a mismatch throws
        // IdentityKeyDerivationMismatchException here, so nothing below runs
        // and pending is never cleared (BLOCKER 1).
        val storageIdentifier = deriver.deriveAndStore(
            walletId = walletId,
            publicKeyData = publicKeyData,
            identityIndex = identityIndex,
            keyIndex = keyIndex,
            keyType = keyType,
            force = true,
        )?.identifier ?: return null

        // Independent confirmation the stored blob actually decrypts.
        if (!verifyRecoverable(pubkeyHex)) {
            throw DashSdkError.PlatformWallet.SigningKeyUnavailable(
                "identity-key repair stored a blob that does not decrypt for pubkey " +
                    "$pubkeyHex (slot $identityIndex/$keyIndex) — the key remains " +
                    "unusable; pending state left intact",
            )
        }

        // BLOCKER 3: the durable write fails CLOSED. Record the identifier on
        // the Room rows so the restart reconstruction does not resurrect this
        // key — but if that write throws, DO NOT clear pending. A swallowed
        // failure that still cleared live state would resurrect the repair
        // after restart while the session believed it was done. Let it
        // propagate; pending stays intact and the repair is retryable.
        persistDurableIdentifier(storageIdentifier)

        // Durable write committed and blob verified — now it is safe to drop
        // the pending-repair signal.
        markIdentityKeyRepaired(pubkeyHex)
        return storageIdentifier
    }

    /**
     * Durable bookkeeping for a sign-time
     * `KeyPermanentlyInvalidatedException` (#4060 round-2 finding 3): null
     * out `privateKeyKeychainIdentifier` on every `public_keys` row carrying
     * [pubkeyHex], then re-run the pending-repair reconstruction so
     * [pendingIdentityKeys] seeds NOW — not just after the next restart.
     *
     * Load-bearing for LEGACY-alias-backed keys: the legacy Keystore aliases
     * are read-only (no deletion boundary), so after a KPIE the CHEAP
     * capability check keeps reporting the blob signable forever
     * (`hasLegacyKeysKey()` stays true) — the null identifier is the only
     * durable signal the reconstruction's usability filter can see. Harmless
     * for policy-alias keys (their generation-checked deletion already flips
     * the fingerprint gate; this merely accelerates the in-process seed).
     * Wired from `KeystoreSigner.onSigningKeyInvalidated` via
     * `PlatformWalletManager`. Rows without derivation breadcrumbs
     * (pre-v8 legacy rows not yet re-persisted) cannot seed a repair slot —
     * the identifier null-out still lands, so they seed as soon as the next
     * persist round back-fills the breadcrumbs.
     */
    internal suspend fun recordSigningKeyInvalidated(
        pubkeyHex: String,
        isPrivateKeyDecryptable: suspend (pubkeyHex: String) -> Boolean,
    ) {
        val publicKeyData = pubkeyHex.hexToByteArray()
        for (row in database.publicKeyDao().getByPublicKeyData(publicKeyData)) {
            if (row.privateKeyKeychainIdentifier != null) {
                database.publicKeyDao().update(row.copy(privateKeyKeychainIdentifier = null))
            }
        }
        reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = isPrivateKeyDecryptable,
            reason = "signing key permanently invalidated",
        )
    }

    /**
     * Rebuild [pendingIdentityKeys] from persistence after a process restart
     * (dashpay/platform#4060 finding 5) — the in-memory map is process-
     * lifetime only, but the durable `public_keys` rows carry the derivation
     * breadcrumbs. A row is (re-)seeded when it has breadcrumbs AND its
     * private half is unusable: either no keychain identifier was ever
     * recorded (the derive failed at persist time), or the identifier exists
     * but [isPrivateKeyDecryptable] (the CHEAP capability check — no
     * decrypt, no prompt, no key generation) rejects the stored blob — the
     * second disjunct resurrects the repair slot for blobs stranded by a
     * Keystore keypair replacement, not just never-derived ones. Read-only
     * keys are never seeded (they are not ours to derive).
     *
     * Seeding is ONE atomic [MutableStateFlow.update]; live entries (from
     * callbacks that already fired this process) are never overwritten —
     * their reason/timestamp are fresher. Publishes immediately: no round is
     * open at load time, same as [markIdentityKeyRepaired].
     *
     * Called by `PlatformWalletManager.loadPersistedWallets` after the Room
     * rows are loaded, before the manager is handed to the host; the wallet
     * scoping comes from each row's identity (network + wallet id), matching
     * this handler's [network] when set.
     */
    internal suspend fun reconstructPendingIdentityKeysFromPersistence(
        isPrivateKeyDecryptable: suspend (pubkeyHex: String) -> Boolean,
        nowMs: Long = System.currentTimeMillis(),
        reason: String = "reconstructed from persistence after restart",
    ) {
        val rows = database.publicKeyDao().getWithDerivationBreadcrumbs()
        if (rows.isEmpty()) return
        val entries = mutableListOf<PendingIdentityKey>()
        for (row in rows) {
            if (row.readOnly) continue
            val identityIndex = row.derivationIdentityIndex ?: continue
            val keyIndex = row.derivationKeyIndex ?: continue
            val identityIdData = row.identityIdData ?: continue
            val identity = database.identityDao().getByIdentityId(identityIdData) ?: continue
            val networkRaw = network?.ffiValue
            if (networkRaw != null && identity.networkRaw != networkRaw) continue
            val walletId = identity.walletId ?: continue
            val pubkeyHex = row.publicKeyData.toHex()
            val usable = row.privateKeyKeychainIdentifier != null &&
                try {
                    isPrivateKeyDecryptable(pubkeyHex)
                } catch (cancellation: kotlin.coroutines.cancellation.CancellationException) {
                    // The probe is suspend; runCatching turned its cancellation
                    // into `false`, marking the row pending and returning
                    // normally so callers (loadPersistedWallets, invalidation
                    // bookkeeping) never observed the cancellation. Rethrow to
                    // preserve structured concurrency; only genuine probe
                    // failures become an unusable result (dashpay/platform#4183).
                    throw cancellation
                } catch (_: Throwable) {
                    false
                }
            if (usable) continue
            entries += PendingIdentityKey(
                walletIdHex = walletId.toHex(),
                identityIdBase58 = row.identityId,
                keyId = row.keyId,
                publicKeyHex = pubkeyHex,
                identityIndex = identityIndex,
                keyIndex = keyIndex,
                reason = reason,
                failedAtMs = nowMs,
            )
        }
        if (entries.isEmpty()) return
        _pendingIdentityKeys.update { map ->
            entries.fold(map) { acc, entry ->
                if (entry.publicKeyHex in acc) acc else acc + (entry.publicKeyHex to entry)
            }
        }
    }

    // ── Error / threading guards ──────────────────────────────────────

    /**
     * Wrap a persist-callback body so no exception ever crosses the JNI
     * boundary — catch, log, and return non-zero (which flips the round's
     * success flag so [onChangesetEnd] rolls back). The body runs under
     * [callbackExclusion], acquired here on the JNI caller thread before
     * any [dispatcher] hop, so callbacks serialize against compound
     * external sequences (wallet deletion) without ever parking the
     * persistence thread.
     */
    private fun guarded(body: () -> Int): Int =
        try {
            runBlocking { callbackExclusion.withLock { body() } }
        } catch (t: Throwable) {
            Log.e(TAG, "persistence callback failed", t)
            1
        }

    /**
     * Load-callback variant: on failure log and return [fallback]. Takes
     * [callbackExclusion] like [guarded] — loads read the same state the
     * deletion sequence mutates.
     */
    private fun <T> guardedLoad(fallback: T, body: () -> T): T =
        try {
            runBlocking { callbackExclusion.withLock { body() } }
        } catch (t: Throwable) {
            Log.e(TAG, "persistence load callback failed", t)
            fallback
        }

    private fun runBlockingCatching(block: suspend () -> Unit) {
        runBlocking(dispatcher) { block() }
    }

    private fun <T> runBlockingResult(block: suspend () -> T): T =
        runBlocking(dispatcher) { block() }

    companion object {
        internal const val PERSISTENCE_CAPABILITIES_VERSION: Int = 1
        internal const val CAPABILITY_ATOMIC_CHANGESETS: Long = 0x01
        internal const val CAPABILITY_INVITATIONS: Long = 0x02
        internal const val CAPABILITY_ASSET_LOCK_FUNDING_INDICES: Long = 0x04
        internal const val CAPABILITY_SHIELDED_VIEWING_KEYS: Long = 0x08
        internal const val CAPABILITY_PROVIDER_TRANSACTIONS: Long = 0x10
        internal const val CAPABILITY_UNSIGNED_TOKEN_STORAGE: Long = 0x20
        internal const val CAPABILITY_WALLET_RESTORE: Long = 0x80
        internal const val CAPABILITY_DPNS_NAME_STATES: Long = 0x100
        internal const val CAPABILITY_TRACKED_ASSET_LOCKS: Long = 0x200

        private const val TAG = "DashPersistence"

        /** `TransactionContext::InBlock` — spends only count once in-block. */
        private const val CONTEXT_IN_BLOCK = 2

        /** `Network.testnet` rawValue — the Swift fallback network. */
        private const val NETWORK_TESTNET = 1

        /** DIP-17 PlatformPayment account type tag (`accountTypeName` 14). */
        private const val ACCOUNT_TYPE_PLATFORM_PAYMENT = 14

        /** DIP-13 IdentityInvitation account type tag (`AccountTypeTagFFI` 5). */
        private const val ACCOUNT_TYPE_IDENTITY_INVITATION = 5

        private val HEX = "0123456789abcdef".toCharArray()
    }
}

/**
 * Derives + persists the private half of an identity key on behalf of
 * [PlatformWalletPersistenceHandler.onPersistIdentityKeyUpsert].
 *
 * The persist callback carries only a derivation breadcrumb; this
 * collaborator turns that into the ready 32-byte scalar (via a single
 * Rust FFI entry point — no derivation logic in Kotlin) and encrypts it
 * into Keystore-backed storage. Injected so the handler stays free of
 * native/Keystore coupling and unit tests can supply a fake.
 *
 * The default production wiring is [IdentityKeyPrivateKeyDeriver], created
 * by `PlatformWalletManager`. See `packages/kotlin-sdk/CLAUDE.md` — this is
 * the "one allowed exception" (Rust derives, Kotlin only encrypts/stores).
 */
interface PrivateKeyDeriver {
    /**
     * Derive the 32-byte scalar for the identity key at
     * ([identityIndex], [keyIndex]) on the wallet named by [walletId] and
     * store it under the key's public-key hex — ATOMICALLY with the
     * "did this already exist" check ([DerivedKeyStoreResult.wasNewlyCreated]),
     * so a sibling wallet's concurrent store of the same alias can't land
     * between a separate check and this store and be mis-classified.
     * Returns `null` if the key could not be derived/stored (leaving it
     * watch-only).
     *
     * @param publicKeyData the on-chain public-key data — the compressed
     *   pubkey, or the 20-byte HASH160 for a HASH160 key type — used as the
     *   storage key so the signer can locate the scalar.
     * @param keyType the DPP `KeyType` discriminant of this key. Only the
     *   [force] repair path consults it: it tells the pubkey-ownership check
     *   whether [publicKeyData] is the raw derived pubkey or its HASH160, so a
     *   HASH160-type key (`ECDSA_HASH160` = 2, `EDDSA_25519_HASH160` = 4) is
     *   verified by hashing the derived pubkey rather than comparing raw bytes
     *   that can never match (dashpay/platform#4183 review). Defaults to
     *   `ECDSA_SECP256K1` (0) for the non-repair store path, which does no
     *   pubkey comparison.
     * @param force when true, skip the "already usable" short-circuit and
     *   REPLACE the stored entry unconditionally — the repair path
     *   (dashpay/platform#4060 finding 6), where a shape+fingerprint-valid
     *   but undecryptable blob must not suppress the re-derive. The
     *   persistence-callback call site keeps the default `false` (idempotent
     *   upserts must not re-derive on every sync).
     */
    fun deriveAndStore(
        walletId: ByteArray,
        publicKeyData: ByteArray,
        identityIndex: Int,
        keyIndex: Int,
        keyType: Int = 0,
        force: Boolean = false,
    ): DerivedKeyStoreResult?

    /**
     * Delete each of [pubkeyHexes] that no wallet OTHER than
     * [excludingWalletId] durably owns, ATOMICALLY with that ownership
     * check — the rollback counterpart of [deriveAndStore], used when a
     * changeset round that wrote aliases fails (their rows never commit,
     * so the ciphertext would otherwise be stranded undiscoverably). A
     * sibling wallet can have already called [WalletStorage.storeIfAbsent]
     * and adopted one of these (shared, deterministic) aliases while its
     * own row is still uncommitted — checking ownership and deleting in
     * one atomic step (not two separate calls) is what stops that
     * sibling's adopted key from being deleted anyway in the window
     * between them. Must THROW on an atomicity failure — the caller keeps
     * the cleanup record alive until a deletion succeeds.
     */
    fun deleteUnownedStored(pubkeyHexes: Collection<String>, excludingWalletId: ByteArray): Set<String>
}

/**
 * Outcome of [PrivateKeyDeriver.deriveAndStore].
 *
 * @param identifier the stored identifier (to record on the persisted row).
 * @param wasNewlyCreated false if a scalar already existed under
 *   [identifier]'s alias (under any owner) at the time of the atomic
 *   check-and-store — distinguishes a round that CREATES an alias
 *   (rollback must delete it) from one that confirms/overwrites an
 *   already-valid scalar (rollback must leave it alone).
 */
data class DerivedKeyStoreResult(val identifier: String, val wasNewlyCreated: Boolean)

// ── Free functions (unit-testable, no `this`) ─────────────────────────

/** Lowercase hex of a byte array (used as the changeset-buffer key). */
/**
 * Pack DIP-15 accepted-account indices (`u32`s crossing JNI as an
 * `IntArray` bit-pattern) into a big-endian 4-bytes-per-entry BLOB for
 * the `contactAcceptedAccounts` column. Empty → null (matches the
 * absent-optional convention of the other nullable columns).
 */
internal fun encodeAcceptedAccounts(accounts: IntArray): ByteArray? {
    if (accounts.isEmpty()) return null
    val buffer = java.nio.ByteBuffer.allocate(accounts.size * 4) // big-endian by default
    accounts.forEach { buffer.putInt(it) }
    return buffer.array()
}

/** Inverse of [encodeAcceptedAccounts]; null / ragged tail-safe. */
internal fun decodeAcceptedAccounts(blob: ByteArray?): IntArray {
    if (blob == null || blob.size < 4) return IntArray(0)
    val buffer = java.nio.ByteBuffer.wrap(blob)
    return IntArray(blob.size / 4) { buffer.int }
}

internal fun ByteArray.toHex(): String {
    val out = CharArray(size * 2)
    val hex = "0123456789abcdef".toCharArray()
    for (i in indices) {
        val v = this[i].toInt() and 0xFF
        out[i * 2] = hex[v ushr 4]
        out[i * 2 + 1] = hex[v and 0x0F]
    }
    return String(out)
}

/** Inverse of [toHex]; requires an even-length lowercase/uppercase hex string. */
internal fun String.hexToByteArray(): ByteArray {
    require(length % 2 == 0) { "hex string must have even length, got $length" }
    return ByteArray(length / 2) { i ->
        ((digitToInt(this[i * 2]) shl 4) or digitToInt(this[i * 2 + 1])).toByte()
    }
}

private fun digitToInt(c: Char): Int = when (c) {
    in '0'..'9' -> c - '0'
    in 'a'..'f' -> c - 'a' + 10
    in 'A'..'F' -> c - 'A' + 10
    else -> throw IllegalArgumentException("invalid hex digit '$c'")
}

/**
 * Base58 string for id columns Swift stores as base58
 * (`PersistentPublicKey.identityId`, `PersistentTokenBalance.tokenId`).
 *
 * Swift's `Data.toBase58String()` and the FFI helper
 * `dash_sdk_utils_hex_to_base58` both render a DPP `Identifier` via
 * `Identifier::to_string(Encoding::Base58)`, which is **plain base58 of
 * the raw 32 bytes** — NOT base58check (no version byte, no 4-byte
 * checksum). This encoder matches that exactly using the Bitcoin/IPFS
 * alphabet, so Kotlin-side uniqueness keys line up with what Swift wrote
 * (and with the human-readable id a user would compare against). The
 * value is display / lookup only; Rust never reads it back across the FFI.
 */
internal fun ByteArray.toBase58String(): String = base58Encode(this)

/** Bitcoin/IPFS base58 alphabet (matches `bs58` / DPP Identifier). */
private const val BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

/**
 * Plain base58 encode (big-endian), preserving leading-zero bytes as
 * leading `'1'`s — the standard bs58 behaviour DPP's `Encoding::Base58`
 * uses. Pure Kotlin so it needs no JNI hop for a display-only key.
 */
internal fun base58Encode(input: ByteArray): String {
    if (input.isEmpty()) return ""
    // Count leading zero bytes → leading '1' chars.
    var zeros = 0
    while (zeros < input.size && input[zeros].toInt() == 0) zeros++

    // Base-256 → base-58 via repeated division on a working copy.
    val buffer = input.copyOf()
    val encoded = StringBuilder()
    var start = zeros
    while (start < buffer.size) {
        var remainder = 0
        for (i in start until buffer.size) {
            val acc = (remainder shl 8) or (buffer[i].toInt() and 0xFF)
            buffer[i] = (acc / 58).toByte()
            remainder = acc % 58
        }
        encoded.append(BASE58_ALPHABET[remainder])
        if (buffer[start].toInt() == 0) start++
    }
    encoded.reverse()
    return "1".repeat(zeros) + encoded
}

/**
 * Encode a 36-byte outpoint (`txid` wire-order ++ `vout` LE) as the Swift
 * `outPointHex` = display-order txid hex + ":" + vout. Display order is the
 * byte-reversed txid.
 */
internal fun encodeOutPointHex(outPoint: ByteArray): String {
    require(outPoint.size == 36) { "outpoint must be 36 bytes, got ${outPoint.size}" }
    val txidWire = outPoint.copyOfRange(0, 32)
    val displayTxid = txidWire.reversedArray()
    val vout = (outPoint[32].toInt() and 0xFF) or
        ((outPoint[33].toInt() and 0xFF) shl 8) or
        ((outPoint[34].toInt() and 0xFF) shl 16) or
        ((outPoint[35].toInt() and 0xFF) shl 24)
    return "${displayTxid.toHex()}:$vout"
}

/**
 * Inverse of [encodeOutPointHex]: parse the persisted display-order
 * `outPointHex` (`<display-txid-hex>:<vout>`) back into the 36-byte
 * outpoint Rust expects — 32-byte WIRE-order txid (the display hex
 * reversed) followed by the 4-byte little-endian vout. Returns `null` for
 * any malformed input (missing `:`, non-64-char / non-hex txid, or a vout
 * that isn't a valid unsigned 32-bit decimal), so callers can drop the row
 * rather than manufacture a bad outpoint. Mirror of the Swift
 * `decodeOutPointHex`.
 */
internal fun decodeOutPointHex(hex: String): ByteArray? {
    val sep = hex.indexOf(':')
    if (sep < 0) return null
    val txidHex = hex.substring(0, sep)
    val voutStr = hex.substring(sep + 1)
    if (txidHex.length != 64) return null
    val vout = voutStr.toUIntOrNull()?.toInt() ?: return null
    val displayTxid = ByteArray(32)
    for (i in 0 until 32) {
        val hi = Character.digit(txidHex[i * 2], 16)
        val lo = Character.digit(txidHex[i * 2 + 1], 16)
        if (hi < 0 || lo < 0) return null
        displayTxid[i] = ((hi shl 4) or lo).toByte()
    }
    // Reverse display order back to wire order for bytes 0..31.
    val out = ByteArray(36)
    for (i in 0 until 32) out[i] = displayTxid[31 - i]
    // LE-encode the vout into bytes 32..35.
    out[32] = (vout and 0xFF).toByte()
    out[33] = ((vout ushr 8) and 0xFF).toByte()
    out[34] = ((vout ushr 16) and 0xFF).toByte()
    out[35] = ((vout ushr 24) and 0xFF).toByte()
    return out
}

/** Build a 36-byte outpoint from a wire-order txid + vout (matches `makeOutpoint`). */
internal fun makeOutpoint(txid: ByteArray, vout: Int): ByteArray {
    val out = ByteArray(36)
    System.arraycopy(txid, 0, out, 0, minOf(32, txid.size))
    out[32] = (vout and 0xFF).toByte()
    out[33] = ((vout ushr 8) and 0xFF).toByte()
    out[34] = ((vout ushr 16) and 0xFF).toByte()
    out[35] = ((vout ushr 24) and 0xFF).toByte()
    return out
}

/**
 * DPNS homograph-safe normalization (mirror of
 * `PersistentDPNSName.normalize`): o/O→0, i/I/l/L→1, else lowercase.
 */
internal fun normalizeDpnsLabel(label: String): String =
    buildString {
        for (c in label) {
            when (c) {
                'o', 'O' -> append('0')
                'i', 'I', 'l', 'L' -> append('1')
                else -> append(c.lowercaseChar())
            }
        }
    }

/**
 * Network-scoped sync-state pseudo id: UTF-8 of `"platform-sync:<network>"`
 * zero-padded to 32 bytes (mirror of `syncStateScopeId`).
 */
internal fun syncStateScopeId(networkRaw: Int): ByteArray {
    val name = when (networkRaw) {
        0 -> "mainnet"
        1 -> "testnet"
        2 -> "devnet"
        3 -> "regtest"
        else -> "testnet"
    }
    val prefix = "platform-sync:$name".toByteArray(Charsets.UTF_8)
    val out = ByteArray(32)
    System.arraycopy(prefix, 0, out, 0, minOf(prefix.size, 32))
    return out
}

/** Display name for an account type tag (mirror of the Swift helper). */
internal fun accountTypeName(accountType: Int, standardTag: Int): String = when (accountType) {
    0 -> if (standardTag == 1) "standardBip32" else "standardBip44"
    1 -> "coinJoin"
    2 -> "identityRegistration"
    3 -> "identityTopUp"
    4 -> "identityTopUpNotBoundToIdentity"
    5 -> "identityInvitation"
    6 -> "assetLockAddressTopUp"
    7 -> "assetLockShieldedAddressTopUp"
    8 -> "providerVotingKeys"
    9 -> "providerOwnerKeys"
    10 -> "providerOperatorKeys"
    11 -> "providerPlatformKeys"
    12 -> "dashpayReceivingFunds"
    13 -> "dashpayExternalAccount"
    14 -> "platformPayment"
    15 -> "identityAuthenticationEcdsa"
    16 -> "identityAuthenticationBls"
    else -> "unknown"
}

/**
 * Project a 32-byte contract-bounds id into the legacy JSON blob shape
 * Swift persists (`[base64(contractId)]`).
 */
internal fun contractBoundsIdToJson(contractBoundsId: ByteArray): ByteArray {
    val b64 = android.util.Base64.encodeToString(contractBoundsId, android.util.Base64.NO_WRAP)
    return "[\"$b64\"]".toByteArray(Charsets.UTF_8)
}

/**
 * Inverse of [contractBoundsIdToJson]: decode the legacy `[base64(id)]`
 * JSON blob back to the 32-byte contract id used on the identity-key
 * restore path. Returns null on any decode failure or a non-32-byte
 * payload (the caller degrades such a row to "no contract bounds" rather
 * than crashing FFI marshalling). Minimal string parse — the blob is
 * always the single-element form this class wrote, never arbitrary JSON.
 */
internal fun contractBoundsJsonToId(blob: ByteArray): ByteArray? {
    val text = blob.toString(Charsets.UTF_8).trim()
    // Expected shape: ["<base64>"]  — pull the first quoted element.
    val open = text.indexOf('"')
    if (open < 0) return null
    val close = text.indexOf('"', open + 1)
    if (close <= open) return null
    val b64 = text.substring(open + 1, close)
    val id = runCatching {
        android.util.Base64.decode(b64, android.util.Base64.NO_WRAP)
    }.getOrNull() ?: return null
    return if (id.size == 32) id else null
}

private fun now(): java.util.Date = java.util.Date()
private fun nowSeconds(): Long = System.currentTimeMillis() / 1000

/**
 * Split a DIP-0018 bech32m platform address into `(addressType, 20-byte
 * hash)` — mirror of Swift `platformAddressComponents(fromBech32m:)`.
 * Pure decoding (marshalling, not policy): the HRP is `dash`/`tdash`, the
 * 21-byte payload is a DIP-0018 type byte followed by the RIPEMD160 hash.
 * Type bytes: `0xb0` → P2PKH (stored as 0), `0x80` → P2SH (stored as 1).
 * Returns null on any decode failure or unexpected type byte.
 */
internal fun decodePlatformAddress(address: String): Pair<Int, ByteArray>? {
    val decoded = Bech32m.decode(address.lowercase()) ?: return null
    if (decoded.hrp != "dash" && decoded.hrp != "tdash") return null
    if (decoded.data.size != 21) return null
    val typeByte = decoded.data[0].toInt() and 0xFF
    val hash = decoded.data.copyOfRange(1, 21)
    return when (typeByte) {
        0xb0 -> 0 to hash
        0x80 -> 1 to hash
        else -> null
    }
}

/**
 * Minimal BIP-350 bech32m decoder (marshalling only — no address policy).
 * Mirrors the subset of iOS `Bech32m` the platform-address path needs:
 * verify the bech32m checksum and return `(hrp, 8-bit converted data)`.
 */
internal object Bech32m {
    private const val CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
    private const val BECH32M_CONST = 0x2bc830a3

    data class Decoded(val hrp: String, val data: ByteArray)

    fun decode(input: String): Decoded? {
        if (input.any { it.code < 33 || it.code > 126 }) return null
        // No mixed case (already lowercased by caller, but be defensive).
        if (input != input.lowercase() && input != input.uppercase()) return null
        val s = input.lowercase()
        val pos = s.lastIndexOf('1')
        if (pos < 1 || pos + 7 > s.length) return null
        val hrp = s.substring(0, pos)
        val dataPart = s.substring(pos + 1)
        val values = IntArray(dataPart.length)
        for (i in dataPart.indices) {
            val idx = CHARSET.indexOf(dataPart[i])
            if (idx < 0) return null
            values[i] = idx
        }
        if (polymod(hrpExpand(hrp) + values.toList()) != BECH32M_CONST) return null
        val payload5 = values.copyOfRange(0, values.size - 6).toList()
        val payload8 = convertBits(payload5, 5, 8, false) ?: return null
        return Decoded(hrp, payload8.map { it.toByte() }.toByteArray())
    }

    private fun hrpExpand(hrp: String): List<Int> {
        val high = hrp.map { it.code shr 5 }
        val low = hrp.map { it.code and 31 }
        return high + listOf(0) + low
    }

    private fun polymod(values: List<Int>): Int {
        val gen = intArrayOf(0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3)
        var chk = 1
        for (v in values) {
            val top = chk shr 25
            chk = ((chk and 0x1ffffff) shl 5) xor v
            for (i in 0 until 5) {
                if (((top shr i) and 1) != 0) chk = chk xor gen[i]
            }
        }
        return chk
    }

    private fun convertBits(data: List<Int>, from: Int, to: Int, pad: Boolean): List<Int>? {
        var acc = 0
        var bits = 0
        val out = ArrayList<Int>()
        val maxv = (1 shl to) - 1
        for (value in data) {
            if (value < 0 || (value shr from) != 0) return null
            acc = (acc shl from) or value
            bits += from
            while (bits >= to) {
                bits -= to
                out.add((acc shr bits) and maxv)
            }
        }
        if (pad) {
            if (bits > 0) out.add((acc shl (to - bits)) and maxv)
        } else if (bits >= from || ((acc shl (to - bits)) and maxv) != 0) {
            return null
        }
        return out
    }
}
