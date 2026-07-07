package org.dashfoundation.dashsdk.persistence

import android.util.Log
import androidx.room.withTransaction
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.ffi.AccountSpecData
import org.dashfoundation.dashsdk.ffi.ContactRequestRestoreData
import org.dashfoundation.dashsdk.ffi.CoreAddressPoolRestoreData
import org.dashfoundation.dashsdk.ffi.CoreAddressRestoreData
import org.dashfoundation.dashsdk.ffi.CoreTxRecordData
import org.dashfoundation.dashsdk.ffi.IdentityKeyRestoreData
import org.dashfoundation.dashsdk.ffi.IdentityRestoreData
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.ffi.PlatformAddressBalanceRestoreData
import org.dashfoundation.dashsdk.ffi.ShieldedActivityData
import org.dashfoundation.dashsdk.ffi.ShieldedNoteData
import org.dashfoundation.dashsdk.ffi.UtxoRestoreData
import org.dashfoundation.dashsdk.ffi.ShieldedOutgoingNoteData
import org.dashfoundation.dashsdk.ffi.ShieldedSyncStateData
import org.dashfoundation.dashsdk.ffi.WalletRestoreData
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayIgnoredSenderEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DpnsNameEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedOutgoingNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
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
 *   defaults to a dedicated executor (override in tests).
 * @param privateKeyDeriver derives + persists the 32-byte private half of
 *   an identity key when the persist callback carries a derivation
 *   breadcrumb. `null` (the default) keeps every key watch-only — the
 *   pre-item-1 behavior — so unit tests and watch-only wallets need no
 *   native/Keystore backing. The manager injects a real implementation
 *   (see [IdentityKeyPrivateKeyDeriver]); tests inject a fake.
 */
class PlatformWalletPersistenceHandler(
    private val database: DashDatabase,
    private val dispatcher: CoroutineDispatcher =
        Executors.newSingleThreadExecutor { r -> Thread(r, "dash-persistence") }
            .asCoroutineDispatcher(),
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
) : NativePersistenceBridge() {

    /**
     * One in-flight store() round. Holds the ordered list of Room
     * mutations staged by the per-kind callbacks, replayed atomically on
     * [onChangesetEnd].
     */
    private class ChangesetBuffer {
        val ops: MutableList<suspend (DashDatabase) -> Unit> = mutableListOf()
    }

    /** Open rounds keyed by walletId hex (a round is per-walletId). */
    private val buffers = HashMap<String, ChangesetBuffer>()

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
        buffers[walletId.toHex()] = ChangesetBuffer()
        0
    }

    override fun onChangesetEnd(walletId: ByteArray, success: Boolean): Int = guarded {
        val key = walletId.toHex()
        val buffer = buffers.remove(key) ?: return@guarded 0
        if (!success) {
            // Rollback: discard every staged write, mirroring
            // `backgroundContext.rollback()`.
            return@guarded 0
        }
        runBlockingCatching {
            database.withTransaction {
                for (op in buffer.ops) {
                    op(database)
                }
            }
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
                    accountIndex = accountIndex,
                    addressIndex = addressIndex,
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
            ) ?: return@stage
            if ((accountTypeTag.toInt() and 0xFF) == ACCOUNT_TYPE_PLATFORM_PAYMENT) {
                // DIP-17 PlatformPayment pool → PlatformAddressEntity
                // (mirror of Swift `persistPlatformPaymentAddresses`). Rust
                // emits the DIP-0018 bech32m form; decode it to the 20-byte
                // hash + address type here so BLAST balance updates (which
                // arrive with `addressHash` only) can upsert the same row.
                val components = decodePlatformAddress(addressBase58) ?: return@stage
                val existing = db.platformAddressDao().getByAddress(addressBase58)
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
            // Spend-flip reconcile: a TXO spent while this tx was still
            // pre-block got its `spendingTxid` linked but kept
            // `isSpent = false` (see onWalletChangesetUtxoSpent's
            // in-block guard). iOS flips it on the next upsert of the
            // spending tx with a confirmed context
            // (`resolveInputOutpoint`); without this pass the flag
            // never converges on Android and the UTXO-restore path
            // (CORE-06) would hand a consumed output back to Rust as
            // spendable after relaunch.
            if (context >= CONTEXT_IN_BLOCK) {
                for (txo in db.txoDao().getUnspentBySpendingTxid(txid)) {
                    db.txoDao().upsert(txo.copy(isSpent = true, lastUpdated = now()))
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
            db.txoDao().upsert(
                TxoEntity(
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
                    isSpent = existing?.isSpent ?: false,
                    walletId = walletId,
                    txid = txid,
                    spendingTxid = existing?.spendingTxid,
                    spendingInputIndex = existing?.spendingInputIndex,
                    accountId = existing?.accountId,
                    coreAddressId = existing?.coreAddressId ?: coreAddressIdIfPresent(db, coreAddressId),
                    createdAt = existing?.createdAt ?: java.util.Date(),
                    lastUpdated = now(),
                ),
            )
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

            // DPNS labels (append-only; upsert by the unique triple).
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
        stage(walletId) { db -> db.identityDao().deleteByIdentityId(identityId) }
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
        val deriver = privateKeyDeriver
        val derivedKeychainId: String? =
            if (derivationIndicesIsSome && deriver != null && !readOnly) {
                runCatching {
                    deriver.deriveAndStore(
                        walletId = if (walletIdIsSome) keyWalletId else walletId,
                        publicKeyData = publicKeyData,
                        identityIndex = identityIndex,
                        keyIndex = keyIndex,
                    )
                }.getOrElse { t ->
                    Log.w(TAG, "identity private-key derive/store failed; key stays watch-only", t)
                    null
                }
            } else {
                null
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
        stage(walletId) { db ->
            db.publicKeyDao().deleteByIdentityAndKeyId(identityId.toBase58String(), keyId)
        }
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
                balance = balance,
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

    // ── Error / threading guards ──────────────────────────────────────

    /**
     * Wrap a persist-callback body so no exception ever crosses the JNI
     * boundary — catch, log, and return non-zero (which flips the round's
     * success flag so [onChangesetEnd] rolls back).
     */
    private inline fun guarded(body: () -> Int): Int =
        try {
            body()
        } catch (t: Throwable) {
            Log.e(TAG, "persistence callback failed", t)
            1
        }

    /** Load-callback variant: on failure log and return [fallback]. */
    private inline fun <T> guardedLoad(fallback: T, body: () -> T): T =
        try {
            body()
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
        private const val TAG = "DashPersistence"

        /** `TransactionContext::InBlock` — spends only count once in-block. */
        private const val CONTEXT_IN_BLOCK = 2

        /** `Network.testnet` rawValue — the Swift fallback network. */
        private const val NETWORK_TESTNET = 1

        /** DIP-17 PlatformPayment account type tag (`accountTypeName` 14). */
        private const val ACCOUNT_TYPE_PLATFORM_PAYMENT = 14

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
     * ([identityIndex], [keyIndex]) on the wallet named by [walletId],
     * store it under the key's public-key hex, and return the stored
     * identifier (to record on the persisted row), or `null` if the key
     * could not be derived/stored (leaving it watch-only).
     *
     * @param publicKeyData the compressed public-key bytes — used as the
     *   storage key so the signer can locate the scalar.
     */
    fun deriveAndStore(
        walletId: ByteArray,
        publicKeyData: ByteArray,
        identityIndex: Int,
        keyIndex: Int,
    ): String?
}

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
