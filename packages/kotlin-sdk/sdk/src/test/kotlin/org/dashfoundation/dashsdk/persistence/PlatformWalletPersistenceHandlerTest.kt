package org.dashfoundation.dashsdk.persistence

import android.content.Context
import android.database.Cursor
import android.database.sqlite.SQLiteException
import android.os.CancellationSignal
import androidx.room.Room
import androidx.sqlite.db.SupportSQLiteDatabase
import androidx.sqlite.db.SupportSQLiteOpenHelper
import androidx.sqlite.db.SupportSQLiteQuery
import androidx.sqlite.db.framework.FrameworkSQLiteOpenHelperFactory
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.wallet.PlatformWalletPersistenceCapabilities
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.PendingInputEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Robolectric round-trip tests over [PlatformWalletPersistenceHandler] +
 * in-memory [DashDatabase].
 *
 * Validates the transactional bracketing contract (begin → buffered
 * writes → end(false) discards / end(true) commits) and the per-callback
 * field mapping against the Room DAOs, mirroring
 * `PlatformWalletPersistenceHandler.swift` semantics.
 *
 * The handler is constructed with `Dispatchers.Unconfined` so its
 * `runBlocking` bodies execute inline on the test thread (the production
 * default is a dedicated single-thread executor).
 */
@RunWith(RobolectricTestRunner::class)
class PlatformWalletPersistenceHandlerTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

    private val walletId = ByteArray(32) { 1 }
    private val groupId = ByteArray(32) { 2 }
    private val testnet = 1

    /**
     * What the sweep pass "decodes" from a stored record's bytes, keyed by
     * txid. Every fixture records transactions with dummy bytes
     * (`ByteArray(10) { 5 }`) that key-wallet-ffi could never decode, and
     * the native decoder is not loadable under Robolectric anyway, so
     * [recordTransaction] registers each record's `inputOutpoints` here and
     * [storedInputs] hands them back. A txid never registered throws, as
     * the production decoder would on bytes it cannot parse — a fixture
     * that seeds a loser row directly must register its inputs.
     */
    private val recordedInputs = HashMap<String, List<ByteArray>>()

    private val storedInputs = StoredTransactionInputs { txid, _ ->
        recordedInputs[txid.toHex()]
            ?: error("test decoder: no inputs registered for ${txid.toHex()}")
    }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = newHandler()
    }

    @After
    fun tearDown() {
        db.close()
    }

    /** A handler over [db] wired to the test decoder — also the suite's "restart" idiom. */
    private fun newHandler(deriver: PrivateKeyDeriver? = null): PlatformWalletPersistenceHandler =
        PlatformWalletPersistenceHandler(
            db,
            Dispatchers.Unconfined,
            deriver,
            storedTransactionInputs = storedInputs,
        )

    /**
     * `onWalletChangesetTransaction` through the test decoder: registers
     * the record's [inputOutpointCount] input outpoints under [txid] so a
     * later sweep of it can key its hold by outpoint, then forwards the
     * call unchanged.
     */
    private fun recordTransaction(
        h: PlatformWalletPersistenceHandler,
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
        accountTypeTag: Byte = (-1).toByte(),
        accountStandardTag: Byte = 0,
        accountIndex: Int = -1,
        accountRegistrationIndex: Int = 0,
        accountKeyClass: Int = 0,
        accountUserIdentityId: ByteArray = ByteArray(0),
        accountFriendIdentityId: ByteArray = ByteArray(0),
        blockPosition: Int = 0,
        hasBlockPosition: Boolean = false,
    ): Int {
        registerInputs(txid, List(inputOutpointCount) { i -> inputOutpoints.copyOfRange(i * 36, i * 36 + 36) })
        return h.onWalletChangesetTransaction(
            walletId, txid, txData, context, blockHeight, blockHash, blockTimestamp, direction,
            transactionType, transactionTypeKind, netAmount, fee, hasFee, label, firstSeen,
            inputOutpoints, inputOutpointCount, accountTypeTag, accountStandardTag, accountIndex,
            accountRegistrationIndex, accountKeyClass, accountUserIdentityId,
            accountFriendIdentityId, blockPosition, hasBlockPosition,
        )
    }

    /** Register what the test decoder returns for [txid] (for rows seeded directly). */
    private fun registerInputs(txid: ByteArray, inputs: List<ByteArray>) {
        recordedInputs[txid.toHex()] = inputs
    }

    /**
     * The sweep slot as the JNI trampoline packs it: [losers] as one flat
     * 32·N array plus count, [released] as one flat 36·M array plus count,
     * and the winner's mined height as the `(has, height)` pair — -1 here
     * means an IS-locked, unmined winner (`has = false`).
     */
    private fun sweep(
        h: PlatformWalletPersistenceHandler,
        wallet: ByteArray,
        losers: List<ByteArray>,
        winner: ByteArray,
        released: List<ByteArray>,
        winnerMinedHeight: Int,
    ): Int = h.onWalletChangesetTransactionsSwept(
        wallet,
        losers.fold(ByteArray(0)) { acc, txid -> acc + txid },
        losers.size,
        winner,
        released.fold(ByteArray(0)) { acc, outpoint -> acc + outpoint },
        released.size,
        winnerMinedHeight >= 0,
        if (winnerMinedHeight >= 0) winnerMinedHeight else 0,
    )

    /** One committed round carrying a single sweep batch. */
    private fun sweepRound(
        wallet: ByteArray,
        losers: List<ByteArray>,
        winner: ByteArray,
        released: List<ByteArray> = emptyList(),
        winnerMinedHeight: Int = 400,
        h: PlatformWalletPersistenceHandler = handler,
    ) {
        h.onChangesetBegin(wallet)
        assertEquals(0, sweep(h, wallet, losers, winner, released, winnerMinedHeight))
        assertEquals(0, h.onChangesetEnd(wallet, success = true))
    }

    /**
     * The wallet + BIP44 account + one `CoreAddressEntity` prologue every
     * restore-facing fixture needs: a TXO on [address] routes to the
     * account through `core_addresses` (Android txos carry no accountId
     * FK), which is what `onLoadWalletList` needs to hand it back.
     */
    private suspend fun seedWalletWithAddress(
        wallet: ByteArray,
        address: String,
        xpubFill: Byte = 30,
    ) {
        handler.onPersistWalletMetadata(wallet, testnet, groupId, 0)
        handler.onPersistAccountRegistration(
            wallet, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { xpubFill },
        )
        val account = db.accountDao().observeByWallet(wallet).first().single()
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = address,
                poolTypeTag = 0,
                addressIndex = 0,
                derivationPath = "m/44'/1'/0'/0/0",
                accountId = account.id,
            ),
        )
    }

    @Test
    fun persistenceCapabilitiesAreExplicitAndFailClosedByDefault() {
        val noOpBridge = object : NativePersistenceBridge() {}
        assertEquals(0, noOpBridge.persistenceCapabilitiesVersion())
        assertEquals(0L, noOpBridge.persistenceCapabilitiesBits())

        assertEquals(1, handler.persistenceCapabilitiesVersion())
        assertEquals(0xbbfL, handler.persistenceCapabilitiesBits())
        // Android has no pending-contact-crypto callback, so it must not
        // attest that semantic contract.
        assertEquals(0L, handler.persistenceCapabilitiesBits() and 0x40L)

        val diagnostic = PlatformWalletPersistenceCapabilities(
            handler.persistenceCapabilitiesVersion(),
            handler.persistenceCapabilitiesBits(),
        )
        assertTrue(diagnostic.contains(PlatformWalletPersistenceCapabilities.ATOMIC_CHANGESETS))
        assertTrue(diagnostic.contains(PlatformWalletPersistenceCapabilities.INVITATIONS))
        assertTrue(diagnostic.contains(PlatformWalletPersistenceCapabilities.DPNS_NAME_STATES))
        assertTrue(diagnostic.contains(PlatformWalletPersistenceCapabilities.TRACKED_ASSET_LOCKS))
        assertTrue(diagnostic.contains(PlatformWalletPersistenceCapabilities.CORE_SWEEP_REMOVAL))
    }

    @Test
    fun sweepSlotDefaultIsTheBenignIgnoreWhateverTheDeclaredBitsSay() {
        // The gate against "declared the bit, never overrode the slot" is
        // not in Kotlin any more: the JNI layer wires the sweep slot only
        // for a bridge whose class overrides the method
        // (`bridge_overrides` in rs-unified-sdk-jni), and Rust derives the
        // effective capability from "slot present AND bit declared" — a
        // declaring-but-not-overriding subclass never gets the slot, so
        // Rust strips the bit and the watermark with it. The inherited
        // body is therefore the benign ignore for every subclass; a runtime
        // bit inspection here would gate one bit out of eleven that all
        // share the declared-but-not-overridden hazard.
        val declaringButNotOverriding = object : NativePersistenceBridge() {
            override fun persistenceCapabilitiesBits(): Long =
                NativePersistenceBridge.CAPABILITY_CORE_SWEEP_REMOVAL
        }
        val nonAttesting = object : NativePersistenceBridge() {}
        val walletId = ByteArray(32) { 1 }
        for (bridge in listOf(declaringButNotOverriding, nonAttesting)) {
            assertEquals(
                0,
                bridge.onWalletChangesetTransactionsSwept(
                    walletId, ByteArray(32) { 2 }, 1, ByteArray(32) { 3 }, ByteArray(0), 0, true, 400,
                ),
            )
        }
        assertEquals(
            "the diagnostic mirror aliases the bridge's declaration, so the two cannot drift",
            NativePersistenceBridge.CAPABILITY_CORE_SWEEP_REMOVAL,
            PlatformWalletPersistenceCapabilities.CORE_SWEEP_REMOVAL,
        )
    }

    // ── Standalone (non-bracketed) writes ─────────────────────────────

    @Test
    fun walletMetadataCreatesTheWalletRow() = runTest {
        assertEquals(0, handler.onPersistWalletMetadata(walletId, testnet, groupId, 1_000_000))

        val wallet = db.walletDao().getByWalletId(walletId)
        assertNotNull(wallet)
        assertEquals(testnet, wallet!!.networkRaw)
        assertEquals(1_000_000, wallet.birthHeight)
        assertTrue(groupId.contentEquals(wallet.walletGroupId))
    }

    @Test
    fun accountRegistrationInsertsAnAccountRow() = runTest {
        // Wallet must exist first (metadata seeds it; registration drops
        // on a missing wallet, matching Swift).
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 3 }

        val code = handler.onPersistAccountRegistration(
            walletId = walletId,
            typeTag = 0, // Standard
            standardTag = 0, // BIP44
            index = 0,
            registrationIndex = 0,
            keyClass = 0,
            userIdentityId = ByteArray(0),
            friendIdentityId = ByteArray(0),
            accountXpubBytes = xpub,
        )
        assertEquals(0, code)

        val accounts = db.accountDao().observeByWallet(walletId).first()
        assertEquals(1, accounts.size)
        assertEquals(0, accounts[0].accountType)
        assertEquals("standardBip44", accounts[0].accountTypeName)
        assertTrue(xpub.contentEquals(accounts[0].accountExtendedPubKeyBytes!!))
    }

    @Test
    fun tokenBalanceCallbackDecodesSignedCarrierAsUnsignedBits() = runTest {
        val identityId = ByteArray(32) { 3 }
        val tokenId = ByteArray(32) { 4 }
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        db.identityDao().upsert(
            IdentityEntity(identityId = identityId, networkRaw = testnet, walletId = walletId),
        )

        assertEquals(
            0,
            handler.onPersistTokenBalanceUpsert(
                walletId = walletId,
                identityId = identityId,
                tokenId = tokenId,
                balance = Long.MIN_VALUE,
            ),
        )

        val stored = db.tokenDao().getBalance(tokenId.toBase58String(), identityId)
        assertEquals(1uL shl 63, stored!!.balance.value)
    }

    @Test
    fun orchardViewingKeyCallbackUpsertsAndLoadRestoresExactBytes() = runTest {
        val fvk1 = ByteArray(96) { 0x31 }
        val fvk2 = ByteArray(96) { 0x32 }
        assertEquals(0, handler.onPersistShieldedViewingKey(walletId, walletId, 7, fvk1))
        assertEquals(0, handler.onPersistShieldedViewingKey(walletId, walletId, 7, fvk2))

        val rows = handler.onLoadShieldedViewingKeys()
        assertEquals(1, rows.size)
        assertTrue(walletId.contentEquals(rows.single().walletId))
        assertEquals(7, rows.single().accountIndex)
        assertTrue(fvk2.contentEquals(rows.single().fvkBytes))
    }

    @Test
    fun orchardViewingKeyPersistAndLoadFailClosedOnMalformedLength() = runTest {
        assertEquals(
            1,
            handler.onPersistShieldedViewingKey(walletId, walletId, 0, ByteArray(95)),
        )
        assertTrue(db.shieldedDao().getAllViewingKeys().isEmpty())

        // Bypass the validated entity/DAO write path to model an externally
        // corrupted database. Restore must throw into JNI, not return an
        // empty array that would trigger mnemonic fallback.
        db.openHelper.writableDatabase.execSQL(
            "INSERT INTO shielded_viewing_keys " +
                "(walletId, accountIndex, fvkBytes, lastUpdated) VALUES (?, 0, ?, 0)",
            arrayOf(walletId, ByteArray(95)),
        )
        assertThrows(IllegalArgumentException::class.java) {
            handler.onLoadShieldedViewingKeys()
        }
    }

    @Test
    fun orchardViewingKeyLoadExcludesOtherNetworksAndPersistRejectsCrossWalletEntry() = runTest {
        val testnetWallet = ByteArray(32) { 0x51 }
        val mainnetWallet = ByteArray(32) { 0x52 }
        db.walletDao().upsert(WalletEntity(testnetWallet, networkRaw = Network.TESTNET.ffiValue))
        db.walletDao().upsert(WalletEntity(mainnetWallet, networkRaw = Network.MAINNET.ffiValue))
        db.shieldedDao().upsertViewingKey(
            org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity(
                testnetWallet, 0, ByteArray(96) { 1 },
            ),
        )
        db.shieldedDao().upsertViewingKey(
            org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity(
                mainnetWallet, 0, ByteArray(96) { 2 },
            ),
        )
        // A malformed foreign-network row must be excluded in SQL before
        // entity validation, so it cannot poison the locked-network load.
        db.openHelper.writableDatabase.execSQL(
            "INSERT INTO shielded_viewing_keys " +
                "(walletId, accountIndex, fvkBytes, lastUpdated) VALUES (?, 1, ?, 0)",
            arrayOf(mainnetWallet, ByteArray(95)),
        )
        val scoped = PlatformWalletPersistenceHandler(
            database = db,
            dispatcher = Dispatchers.Unconfined,
            network = Network.TESTNET,
        )

        val restored = scoped.onLoadShieldedViewingKeys()
        assertEquals(1, restored.size)
        assertTrue(testnetWallet.contentEquals(restored.single().walletId))
        assertEquals(
            1,
            scoped.onPersistShieldedViewingKey(
                testnetWallet, mainnetWallet, 2, ByteArray(96),
            ),
        )
        assertNull(db.shieldedDao().getViewingKey(mainnetWallet, 2))
        scoped.close()
    }

    @Test
    fun providerRestoreStagingIsPayloadOnlyWalletScopedAndOrderedByBlockPosition() = runTest {
        val siblingWalletId = ByteArray(32) { 0x22 }
        val siblingGroupId = ByteArray(32) { 0x23 }
        registerRestorableProviderAccount(walletId, groupId, accountIndex = 7)
        registerRestorableProviderAccount(siblingWalletId, siblingGroupId, accountIndex = 8)

        // Kotlin-only staging markers: native restore tests must use valid
        // ProReg/ProUp consensus fixtures because Rust authoritatively decodes
        // and will reject these one-byte bodies. Both wallet-A records share a block. Persist them in reverse order
        // to prove restore uses Core's explicit block position, not callback
        // or insertion order. No TXO is created for either payload-only tx.
        persistProviderTransaction(
            walletId, accountIndex = 7, marker = 2, kind = 2,
            blockHeight = 500, blockPosition = 2,
        )
        persistProviderTransaction(
            walletId, accountIndex = 7, marker = 1, kind = 5,
            blockHeight = 500, blockPosition = 1,
        )
        persistProviderTransaction(
            siblingWalletId, accountIndex = 8, marker = 9, kind = 3,
            blockHeight = 499, blockPosition = 0,
        )

        assertTrue(db.txoDao().observeUnspentByWallet(walletId).first().isEmpty())
        val restores = handler.onLoadWalletList().associateBy { it.walletId.first() }
        assertEquals(listOf(1.toByte(), 2.toByte()), restores[1]!!.providerSpecialTxs.map { it.txBytes[0] })
        assertEquals(listOf(9.toByte()), restores[0x22]!!.providerSpecialTxs.map { it.txBytes[0] })
        val first = restores[1]!!.providerSpecialTxs.first()
        assertEquals(500, first.blockHeight)
        assertEquals(1, first.blockPosition)
        assertTrue(first.hasBlockPosition)
        assertEquals(32, first.blockHash.size)
    }

    @Test
    fun providerKindOnStandardAccountDoesNotStageIntoUnrelatedProviderAccount() = runTest {
        registerRestorableProviderAccount(walletId, groupId, accountIndex = 7)
        assertEquals(
            0,
            handler.onPersistAccountRegistration(
                walletId = walletId,
                typeTag = 0,
                standardTag = 0,
                index = 0,
                registrationIndex = 0,
                keyClass = 0,
                userIdentityId = ByteArray(0),
                friendIdentityId = ByteArray(0),
                accountXpubBytes = ByteArray(78) { 0x33 },
            ),
        )

        persistProviderTransaction(
            walletId, accountIndex = 0, marker = 7, kind = 2,
            blockHeight = 600, blockPosition = 0, accountTypeTag = 0,
        )

        val txid = ByteArray(32) { 7 }
        assertEquals(0, db.transactionDao().countInvolvements(txid))
        assertTrue(handler.onLoadWalletList().single().providerSpecialTxs.isEmpty())
    }

    @Test
    fun providerRestoreStagingSkipsOnlyHostStructuralCorruptionBeforeRustDecode() = runTest {
        registerRestorableProviderAccount(walletId, groupId, accountIndex = 7)
        persistProviderTransaction(
            walletId, accountIndex = 7, marker = 4, kind = 4,
            blockHeight = 12, blockPosition = 0,
        )
        // The one-byte body is deliberately only a non-empty staging marker,
        // not a valid provider transaction. Empty consensus bytes and a bad
        // fixed hash are host-structural corruption and are skipped here;
        // Rust must reject this non-empty undecodable marker in its native
        // restore test without crashing.
        persistProviderTransaction(
            walletId, accountIndex = 7, marker = null, kind = 2,
            blockHeight = 13, blockPosition = 0,
        )
        persistProviderTransaction(
            walletId, accountIndex = 7, marker = 6, kind = 3,
            blockHeight = 14, blockPosition = 0, blockHash = ByteArray(31) { 6 },
        )

        val restored = handler.onLoadWalletList().single().providerSpecialTxs
        assertEquals(1, restored.size)
        assertEquals(4.toByte(), restored.single().txBytes.single())
    }

    private fun registerRestorableProviderAccount(
        id: ByteArray,
        group: ByteArray,
        accountIndex: Int,
    ) {
        assertEquals(0, handler.onPersistWalletMetadata(id, testnet, group, 0))
        assertEquals(
            0,
            handler.onPersistAccountRegistration(
                walletId = id,
                typeTag = 9, // ProviderOwnerKeys
                standardTag = 0,
                index = accountIndex,
                registrationIndex = 0,
                keyClass = 0,
                userIdentityId = ByteArray(0),
                friendIdentityId = ByteArray(0),
                accountXpubBytes = ByteArray(78) { accountIndex.toByte() },
            ),
        )
    }

    private fun persistProviderTransaction(
        id: ByteArray,
        accountIndex: Int,
        marker: Int?,
        kind: Int,
        blockHeight: Int,
        blockPosition: Int,
        blockHash: ByteArray = ByteArray(32) { marker?.toByte() ?: 1 },
        accountTypeTag: Byte = 9,
    ) {
        val txid = ByteArray(32) { (marker ?: 0).toByte() }
        assertEquals(
            0,
            recordTransaction(
                handler,
                walletId = id,
                txid = txid,
                txData = marker?.let { byteArrayOf(it.toByte()) } ?: ByteArray(0),
                context = 2,
                blockHeight = blockHeight,
                blockHash = blockHash,
                blockTimestamp = 1_700_000_000,
                direction = 0,
                transactionType = "Provider",
                transactionTypeKind = kind,
                netAmount = 0,
                fee = 0,
                hasFee = false,
                label = "",
                firstSeen = blockHeight.toLong(),
                inputOutpoints = ByteArray(0),
                inputOutpointCount = 0,
                accountTypeTag = accountTypeTag,
                accountStandardTag = 0,
                accountIndex = accountIndex,
                accountRegistrationIndex = 0,
                accountKeyClass = 0,
                accountUserIdentityId = ByteArray(0),
                accountFriendIdentityId = ByteArray(0),
                blockPosition = blockPosition,
                hasBlockPosition = true,
            ),
        )
    }

    // ── Transactional bracketing ──────────────────────────────────────

    @Test
    fun changesetRollbackDiscardsBufferedWrites() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        handler.onChangesetBegin(walletId)
        // A sync-state write buffered inside the round.
        handler.onPersistSyncState(walletId, syncHeight = 500, syncTimestamp = 111, lastKnownRecentBlock = 400)
        // Nothing committed yet.
        assertNull(db.platformAddressDao().getSyncState(syncStateScopeId(testnet)))

        handler.onChangesetEnd(walletId, success = false)
        // Rolled back — still nothing.
        assertNull(db.platformAddressDao().getSyncState(syncStateScopeId(testnet)))
    }

    @Test
    fun changesetCommitFlushesBufferedWritesAtomically() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        handler.onChangesetBegin(walletId)
        handler.onPersistSyncState(walletId, syncHeight = 900, syncTimestamp = 222, lastKnownRecentBlock = 800)
        // Still buffered.
        assertNull(db.platformAddressDao().getSyncState(syncStateScopeId(testnet)))

        assertEquals(0, handler.onChangesetEnd(walletId, success = true))

        val state = db.platformAddressDao().getSyncState(syncStateScopeId(testnet))
        assertNotNull(state)
        assertEquals(900L, state!!.syncHeight)
        assertEquals(222L, state.syncTimestamp)
        assertEquals(800L, state.lastKnownRecentBlock)
        assertEquals(testnet, state.networkRaw)
    }

    // ── Core wallet changeset ─────────────────────────────────────────

    @Test
    fun walletChangesetHeaderUpdatesSyncedHeight() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetHeader(
            walletId = walletId,
            hasSyncedHeight = true,
            syncedHeight = 123_456,
            hasBalance = false,
            confirmedDelta = 0,
            unconfirmedDelta = 0,
            immatureDelta = 0,
            lockedDelta = 0,
            lastAppliedChainLockBytes = ByteArray(0),
        )
        handler.onChangesetEnd(walletId, success = true)

        assertEquals(123_456, db.walletDao().getByWalletId(walletId)!!.syncedHeight)
    }

    /**
     * dashpay/platform#4069 (Kotlin half of signature C): the
     * `syncedHeight` watermark is written by [onWalletChangesetHeader]
     * into the SAME buffered transaction as the TXO/tx rows of its
     * changeset. A round that rolls back (`success = false`) must
     * therefore NOT advance the persisted watermark — otherwise the
     * durable watermark could outrun the rows it implies, exactly the
     * "empty-and-scanned after restart" corruption in #4069. Pins the
     * rollback path for the core header specifically (the sibling
     * `changesetRollbackDiscardsBufferedWrites` only covers the
     * platform sync-state write).
     */
    @Test
    fun walletChangesetHeaderDoesNotAdvanceSyncedHeightOnRollback() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        // Establish a committed baseline watermark.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetHeader(
            walletId = walletId,
            hasSyncedHeight = true,
            syncedHeight = 1_000,
            hasBalance = false,
            confirmedDelta = 0,
            unconfirmedDelta = 0,
            immatureDelta = 0,
            lockedDelta = 0,
            lastAppliedChainLockBytes = ByteArray(0),
        )
        handler.onChangesetEnd(walletId, success = true)
        assertEquals(1_000, db.walletDao().getByWalletId(walletId)!!.syncedHeight)

        // A later round tries to advance the watermark but rolls back.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetHeader(
            walletId = walletId,
            hasSyncedHeight = true,
            syncedHeight = 2_000,
            hasBalance = false,
            confirmedDelta = 0,
            unconfirmedDelta = 0,
            immatureDelta = 0,
            lockedDelta = 0,
            lastAppliedChainLockBytes = ByteArray(0),
        )
        handler.onChangesetEnd(walletId, success = false)

        // Watermark stays at the last committed value — never the
        // rolled-back 2_000.
        assertEquals(1_000, db.walletDao().getByWalletId(walletId)!!.syncedHeight)
    }

    @Test
    fun walletChangesetAddsAccountUtxoAndTransaction() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val txid = ByteArray(32) { 9 }

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetAccountBegin(
            walletId = walletId,
            accountIndex = 0,
            typeTag = 0,
            standardTag = 0,
            registrationIndex = 0,
            keyClass = 0,
            userIdentityId = ByteArray(0),
            friendIdentityId = ByteArray(0),
            externalHighestUsed = 5,
            hasExternalHighestUsed = true,
            internalHighestUsed = -1,
            hasInternalHighestUsed = false,
        )
        recordTransaction(
            handler,
            walletId = walletId,
            txid = txid,
            txData = ByteArray(10) { 4 },
            context = 2, // InBlock
            blockHeight = 100,
            blockHash = ByteArray(32) { 7 },
            blockTimestamp = 1_700_000_000,
            direction = 0,
            transactionType = "Standard",
            transactionTypeKind = 0,
            netAmount = 50_000,
            fee = 200,
            hasFee = true,
            label = "",
            firstSeen = 1_699_999_000,
            inputOutpoints = ByteArray(0),
            inputOutpointCount = 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId = walletId,
            txid = txid,
            vout = 0,
            amount = 50_000,
            address = "yTestAddr",
            scriptPubKey = ByteArray(25) { 6 },
            height = 100,
            isCoinbase = false,
            isConfirmed = true,
            isInstantLocked = false,
            isLocked = false,
        )
        handler.onWalletChangesetAccountEnd(walletId, accountIndex = 0)
        handler.onChangesetEnd(walletId, success = true)

        val account = db.accountDao().observeByWallet(walletId).first().single()
        assertEquals(5, account.externalHighestUsed)

        val tx = db.transactionDao().getByTxid(txid)
        assertNotNull(tx)
        assertEquals(50_000, tx!!.netAmount)
        assertEquals(200L, tx.fee)

        val txo = db.txoDao().getByOutpoint(makeOutpoint(txid, 0))
        assertNotNull(txo)
        assertEquals(50_000, txo!!.amount)
        assertTrue(walletId.contentEquals(txo.walletId))
    }

    // ── Address balances (update-only) ────────────────────────────────

    @Test
    fun addressBalanceUpdatesAnExistingPlatformAddressRow() = runTest {
        // Seed a platform-address row (the pool-emit path in production).
        val hash = ByteArray(20) { 8 }
        db.platformAddressDao().upsert(
            PlatformAddressEntity(
                address = "dash1seed",
                addressType = 0,
                addressHash = hash,
                accountIndex = 0,
                addressIndex = 0,
                derivationPath = "m/9'/5'/17'/0'/0'/0",
                walletId = walletId,
            ),
        )

        handler.onChangesetBegin(walletId)
        handler.onPersistAddressBalance(
            walletId = walletId,
            addressType = 0,
            addressHash = hash,
            balance = 12_345,
            nonce = 3,
            accountIndex = 0,
            addressIndex = 0,
            asOfHeight = 777,
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.platformAddressDao().getByWalletAndAddressHash(walletId, hash)
        assertNotNull(row)
        assertEquals(12_345, row!!.balance)
        assertEquals(3, row.nonce)
        assertEquals(777, row.lastSeenHeight)
        assertTrue(row.isUsed)
    }

    @Test
    fun addressBalanceConflictPreservesDerivationIndicesAcrossRestart() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 30 },
        )

        // The pool emit owns this immutable address/index mapping.
        val hash = ByteArray(20) { 18 }
        val canonicalPath = "m/9'/5'/17'/2'/0'/7"
        db.platformAddressDao().upsert(
            PlatformAddressEntity(
                address = "dash1conflict",
                addressType = 0,
                addressHash = hash,
                accountIndex = 2,
                addressIndex = 7,
                derivationPath = canonicalPath,
                walletId = walletId,
            ),
        )

        // Conflict-removal can report address A with the competing address
        // B's tuple. The callback updates A's balance snapshot only; B's
        // tuple must never replace A's authoritative derivation identity.
        handler.onChangesetBegin(walletId)
        handler.onPersistAddressBalance(
            walletId = walletId,
            addressType = 0,
            addressHash = hash,
            balance = 0,
            nonce = 0,
            accountIndex = 9,
            addressIndex = 13,
            asOfHeight = 800,
        )
        handler.onChangesetEnd(walletId, success = true)

        val stored = db.platformAddressDao().getByWalletAndAddressHash(walletId, hash)
        assertNotNull(stored)
        assertEquals(2, stored!!.accountIndex)
        assertEquals(7, stored.addressIndex)
        assertEquals(canonicalPath, stored.derivationPath)

        // A fresh handler models process restart. Its restore payload must
        // carry the canonical tuple, not the conflicting callback tuple.
        val restarted = newHandler()
        val restored = restarted.onLoadWalletList().single().platformAddressBalances.single()
        assertEquals(2, restored.accountIndex)
        assertEquals(7, restored.addressIndex)
        assertTrue(hash.contentEquals(restored.addressHash))

        // A later valid credit remains attached to the same canonical row.
        restarted.onChangesetBegin(walletId)
        restarted.onPersistAddressBalance(
            walletId = walletId,
            addressType = 0,
            addressHash = hash,
            balance = 25_000,
            nonce = 1,
            accountIndex = 2,
            addressIndex = 7,
            asOfHeight = 801,
        )
        restarted.onChangesetEnd(walletId, success = true)
        val credited = db.platformAddressDao().getByWalletAndAddressHash(walletId, hash)
        assertNotNull(credited)
        assertEquals(25_000L, credited!!.balance)
        assertEquals(2, credited.accountIndex)
        assertEquals(7, credited.addressIndex)
        assertEquals(canonicalPath, credited.derivationPath)
    }

    @Test
    fun addressBalanceForUnknownHashIsANoOp() = runTest {
        handler.onChangesetBegin(walletId)
        handler.onPersistAddressBalance(
            walletId = walletId,
            addressType = 0,
            addressHash = ByteArray(20) { 44 },
            balance = 1,
            nonce = 0,
            accountIndex = 0,
            addressIndex = 0,
            asOfHeight = 0,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(db.platformAddressDao().getAllByAddressHash(ByteArray(20) { 44 }).isEmpty())
        assertEquals(0L, db.platformAddressDao().count().first())
    }

    // ── Identities ────────────────────────────────────────────────────

    @Test
    fun identityUpsertWritesRowWithDpnsAndProfile() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 10 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId = walletId,
            identityId = identityId,
            balance = 42_000,
            revision = 2,
            identityIndexIsSome = true,
            identityIndex = 0,
            status = 2,
            walletIdIsSome = true,
            identityWalletId = walletId,
            dpnsNames = arrayOf("Alice"),
            dpnsNamesAcquiredAt = longArrayOf(1_700_000_000_000L),
            dashpayProfilePresent = true,
            dashpayDisplayName = "Alice",
            dashpayBio = null,
            dashpayAvatarUrl = "https://x/y.png",
            dashpayAvatarHash = ByteArray(32) { 5 },
            dashpayAvatarHashPresent = true,
            dashpayAvatarFingerprint = ByteArray(8),
            dashpayAvatarFingerprintPresent = false,
            dashpayPublicMessage = "hi",
        )
        handler.onChangesetEnd(walletId, success = true)

        val identity = db.identityDao().getByIdentityId(identityId)
        assertNotNull(identity)
        assertEquals(42_000, identity!!.balance)
        assertEquals(2, identity.revision)
        assertTrue(walletId.contentEquals(identity.walletId!!))

        val names = db.dpnsNameDao().observeByIdentity(identityId).first()
        assertEquals(1, names.size)
        assertEquals("Alice", names[0].label)
        assertEquals("a11ce", names[0].normalizedLabel) // A→a, l→1, i→1, lowercased

        val profile = db.dashpayDao().getProfile(testnet, identityId)
        assertNotNull(profile)
        assertEquals("Alice", profile!!.displayName)
        assertEquals("hi", profile.publicMessage)
        assertNotNull(profile.avatarHash)
        assertNull(profile.avatarFingerprint)
    }

    @Test
    fun identityDpnsSnapshotRemovesStaleOwnedLabels() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 12 }

        fun persistSnapshot(vararg names: String) {
            handler.onChangesetBegin(walletId)
            handler.onPersistIdentityUpsert(
                walletId, identityId, 1, 0, false, 0, 0, true, walletId,
                names.toList().toTypedArray(), LongArray(names.size), false, null, null, null,
                ByteArray(32), false, ByteArray(8), false, null,
            )
            handler.onChangesetEnd(walletId, success = true)
        }

        persistSnapshot("Alice", "Bob")
        assertEquals(2, db.dpnsNameDao().observeByIdentity(identityId).first().size)

        persistSnapshot("Alice")
        val current = db.dpnsNameDao().observeByIdentity(identityId).first()
        assertEquals(listOf("Alice"), current.map { it.label })
        assertEquals(1, db.dpnsNameDao().observeMarketplaceByIdentity(identityId).first().size)
    }

    @Test
    fun marketplaceStateRetainsDepartedNameAndCanClearIt() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 13 }
        val documentId = ByteArray(32) { 14 }
        val buyerId = ByteArray(32) { 15 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId, identityId, 1, 0, false, 0, 0, true, walletId,
            emptyArray(), longArrayOf(), false, null, null, null,
            ByteArray(32), false, ByteArray(8), false, null,
        )
        handler.onPersistDpnsNameState(
            walletId = walletId,
            documentId = documentId,
            walletIdentityId = identityId,
            hasCounterparty = true,
            counterpartyId = buyerId,
            label = "Alice",
            normalizedLabel = "a11ce",
            normalizedParentDomainName = "dash",
            hasPrice = false,
            priceCredits = 0,
            status = 1,
            createdAtMs = 100,
            updatedAtMs = 200,
            transferredAtMs = 300,
            lastSyncedAtMs = 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(db.dpnsNameDao().observeByIdentity(identityId).first().isEmpty())
        val retained = db.dpnsNameDao().observeMarketplaceByIdentity(identityId).first().single()
        assertTrue(documentId.contentEquals(retained.documentId!!))
        assertFalse(retained.isOwned)
        assertEquals(1, retained.saleStatusRaw)
        assertTrue(buyerId.contentEquals(retained.counterpartyIdentityId!!))
        assertEquals(100L, retained.documentCreatedAtMs)
        assertEquals(200L, retained.documentUpdatedAtMs)
        assertEquals(300L, retained.documentTransferredAtMs)
        assertEquals(400L, retained.marketplaceUpdatedAt)

        handler.onChangesetBegin(walletId)
        handler.onRemoveDpnsNameState(walletId, documentId)
        handler.onChangesetEnd(walletId, success = true)
        assertNull(db.dpnsNameDao().getByDocumentId(documentId))
        val labelCache = db.dpnsNameDao().observeMarketplaceByIdentity(identityId).first().single()
        assertEquals("Alice", labelCache.label)
        assertFalse(labelCache.isOwned)
        assertNull(labelCache.documentId)
        assertNull(labelCache.priceCredits)
        assertEquals(0, labelCache.saleStatusRaw)
        assertNull(labelCache.counterpartyIdentityId)
        assertEquals(0L, labelCache.documentCreatedAtMs)
        assertEquals(0L, labelCache.documentUpdatedAtMs)
        assertEquals(0L, labelCache.documentTransferredAtMs)
        assertEquals(0L, labelCache.marketplaceUpdatedAt)
    }

    @Test
    fun marketplaceStateSkipsUnknownIdentityWithoutRollingBackRound() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val missingIdentityId = ByteArray(32) { 16 }
        val documentId = ByteArray(32) { 17 }

        handler.onChangesetBegin(walletId)
        assertEquals(
            0,
            handler.onPersistDpnsNameState(
                walletId = walletId,
                documentId = documentId,
                walletIdentityId = missingIdentityId,
                hasCounterparty = false,
                counterpartyId = ByteArray(32),
                label = "Orphan",
                normalizedLabel = "0rphan",
                normalizedParentDomainName = "dash",
                hasPrice = false,
                priceCredits = 0,
                status = 0,
                createdAtMs = 100,
                updatedAtMs = 200,
                transferredAtMs = 0,
                lastSyncedAtMs = 300,
            ),
        )
        assertEquals(0, handler.onChangesetEnd(walletId, success = true))

        assertNull(db.dpnsNameDao().getByDocumentId(documentId))
    }

    @Test
    fun identityRemovalDeletesTheRow() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 11 }
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId, identityId, 1, 0, false, 0, 0, false, ByteArray(32),
            emptyArray(), longArrayOf(), false, null, null, null,
            ByteArray(32), false, ByteArray(8), false, null,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertNotNull(db.identityDao().getByIdentityId(identityId))

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityRemoval(walletId, identityId)
        handler.onChangesetEnd(walletId, success = true)
        assertNull(db.identityDao().getByIdentityId(identityId))
    }

    // ── Identity-key private-key derivation (item 1) ──────────────────

    /**
     * Records the [PrivateKeyDeriver] calls and returns a canned
     * identifier — stands in for the native/Keystore-backed
     * `IdentityKeyPrivateKeyDeriver` so the handler's derive→persist wiring
     * is exercised without a live FFI / Android Keystore.
     */
    private class FakeDeriver(private val id: String? = "privkey.deadbeef") : PrivateKeyDeriver {
        val calls = mutableListOf<Triple<ByteArray, Int, Int>>()
        var lastPublicKey: ByteArray? = null
        val deletedAliases = mutableListOf<String>()

        /** Aliases that existed BEFORE the round (app-stored, re-derives). */
        val preExisting = mutableSetOf<String>()

        /** Simulate an atomic DataStore deletion failure. */
        var failDeletions = false

        override fun deriveAndStore(
            walletId: ByteArray,
            publicKeyData: ByteArray,
            identityIndex: Int,
            keyIndex: Int,
            keyType: Int,
            force: Boolean,
        ): DerivedKeyStoreResult? {
            calls.add(Triple(walletId, identityIndex, keyIndex))
            lastPublicKey = publicKeyData
            val pubkeyHex = publicKeyData.toHex()
            return id?.let { DerivedKeyStoreResult(it, wasNewlyCreated = pubkeyHex !in preExisting) }
        }

        /** Aliases a SIBLING wallet's durable owner index claims. */
        val ownedByAnotherWallet = mutableSetOf<String>()

        override fun deleteUnownedStored(
            pubkeyHexes: Collection<String>,
            excludingWalletId: ByteArray,
        ): Set<String> {
            if (failDeletions) throw IllegalStateException("simulated DataStore edit failure")
            val toDelete = pubkeyHexes.filterTo(mutableSetOf()) { it !in ownedByAnotherWallet }
            deletedAliases.addAll(toDelete)
            return toDelete
        }
    }

    /** Seed the wallet + identity rows a public-key row FKs onto. */
    private suspend fun seedIdentity(identityId: ByteArray) {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId, identityId, 1, 0, true, 0, 2, true, walletId,
            emptyArray(), longArrayOf(), false, null, null, null,
            ByteArray(32), false, ByteArray(8), false, null,
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun identityKeyUpsertDerivesAndRecordsPrivateKeyIdentifier() = runTest {
        val deriver = FakeDeriver()
        handler = newHandler(deriver)

        val identityId = ByteArray(32) { 12 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 7 }

        handler.onChangesetBegin(walletId)
        val code = handler.onPersistIdentityKeyUpsert(
            walletId = walletId,
            identityId = identityId,
            keyId = 0,
            purpose = 0,
            securityLevel = 0,
            keyType = 0,
            readOnly = false,
            disabledAtIsSome = false,
            disabledAt = 0,
            publicKeyData = pubkey,
            publicKeyHash = ByteArray(20),
            walletIdIsSome = true,
            keyWalletId = walletId,
            derivationIndicesIsSome = true,
            identityIndex = 3,
            keyIndex = 5,
            contractBoundsKind = 0,
            contractBoundsId = ByteArray(32),
            contractBoundsDocumentType = null,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertEquals(0, code)

        // Deriver invoked once with the persisted breadcrumb + pubkey.
        assertEquals(1, deriver.calls.size)
        assertTrue(walletId.contentEquals(deriver.calls[0].first))
        assertEquals(3, deriver.calls[0].second)
        assertEquals(5, deriver.calls[0].third)
        assertTrue(pubkey.contentEquals(deriver.lastPublicKey!!))

        // The row records the identifier the deriver returned.
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertNotNull(row)
        assertEquals("privkey.deadbeef", row!!.privateKeyKeychainIdentifier)
    }

    @Test
    fun identityKeyUpsertStaysWatchOnlyWithoutDeriver() = runTest {
        // Default handler has no deriver — keys stay watch-only.
        val identityId = ByteArray(32) { 13 }
        seedIdentity(identityId)

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            ByteArray(33) { 8 }, ByteArray(20), true, walletId,
            true, 0, 0, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertNotNull(row)
        assertNull(row!!.privateKeyKeychainIdentifier)
    }

    @Test
    fun identityKeyUpsertSkipsDeriveForWatchOnlyKey() = runTest {
        val deriver = FakeDeriver()
        handler = newHandler(deriver)

        val identityId = ByteArray(32) { 14 }
        seedIdentity(identityId)

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, /* readOnly = */ true, false, 0,
            ByteArray(33) { 9 }, ByteArray(20), true, walletId,
            /* derivationIndicesIsSome = */ false, 0, 0, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = true)

        // No breadcrumb + read-only ⇒ deriver never consulted.
        assertTrue(deriver.calls.isEmpty())
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertNull(row!!.privateKeyKeychainIdentifier)
    }

    @Test
    fun rolledBackRoundScrubsDeriverWrittenAliases() = runTest {
        val deriver = FakeDeriver()
        handler = newHandler(deriver)

        val identityId = ByteArray(32) { 15 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 10 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 0, 0, 0, ByteArray(32), null,
        )
        // The alias write happened immediately; the row is only buffered —
        // this is the gap the pending-alias fence covers.
        assertEquals(setOf(pubkey.toHex()), handler.pendingAliasesFor(walletId))

        handler.onChangesetEnd(walletId, success = false)

        // The rolled-back round deleted the alias it wrote (its row never
        // committed) and dropped the tracking record.
        assertEquals(listOf(pubkey.toHex()), deriver.deletedAliases)
        assertTrue(handler.pendingAliasesFor(walletId).isEmpty())
        assertNull(db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0))
    }

    @Test
    fun rolledBackRoundDoesNotScrubPreExistingAliases() = runTest {
        val deriver = FakeDeriver()
        handler = newHandler(deriver)

        val identityId = ByteArray(32) { 17 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 12 }
        // The alias predates the round (add-key flows store the scalar
        // before Rust persistence begins; disable_keys re-emits
        // breadcrumbs for existing keys) — a re-derive overwrite must not
        // become a rollback-deletion candidate.
        deriver.preExisting.add(pubkey.toHex())

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 0, 0, 0, ByteArray(32), null,
        )
        assertTrue(handler.pendingAliasesFor(walletId).isEmpty())
        handler.onChangesetEnd(walletId, success = false)

        assertTrue(deriver.deletedAliases.isEmpty())
    }

    @Test
    fun failedAliasDeletionRetainsCleanupStateUntilRetrySucceeds() = runTest {
        val deriver = FakeDeriver()
        handler = newHandler(deriver)

        val identityId = ByteArray(32) { 18 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 13 }

        deriver.failDeletions = true
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 0, 0, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = false)

        // Deletion failed atomically: nothing was deleted, and the cleanup
        // record survives so the wallet-deletion sweep (or a retry) can
        // still find the orphan — never silently dropped.
        assertTrue(deriver.deletedAliases.isEmpty())
        assertEquals(setOf(pubkey.toHex()), handler.pendingAliasesFor(walletId))

        // The next round retries the orphan cleanup and succeeds.
        deriver.failDeletions = false
        handler.onChangesetBegin(walletId)
        assertEquals(listOf(pubkey.toHex()), deriver.deletedAliases)
        assertTrue(handler.pendingAliasesFor(walletId).isEmpty())
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun committedRoundKeepsDeriverWrittenAliases() = runTest {
        val deriver = FakeDeriver()
        handler = newHandler(deriver)

        val identityId = ByteArray(32) { 16 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 11 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 0, 0, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Committed rows make the alias discoverable — nothing scrubbed,
        // tracking dropped, identifier recorded on the row.
        assertTrue(deriver.deletedAliases.isEmpty())
        assertTrue(handler.pendingAliasesFor(walletId).isEmpty())
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertEquals("privkey.deadbeef", row!!.privateKeyKeychainIdentifier)
    }

    // ── Pending identity keys (dashpay/platform#4053: no silent skip) ──

    /** Deriver that always throws — the derive/storage-failure path. */
    private class ThrowingDeriver : PrivateKeyDeriver {
        override fun deriveAndStore(
            walletId: ByteArray,
            publicKeyData: ByteArray,
            identityIndex: Int,
            keyIndex: Int,
            keyType: Int,
            force: Boolean,
        ): DerivedKeyStoreResult = throw IllegalStateException("keystore unavailable")

        override fun deleteUnownedStored(
            pubkeyHexes: Collection<String>,
            excludingWalletId: ByteArray,
        ): Set<String> = emptySet()
    }

    private fun upsertIdentityKey(pubkey: ByteArray, identityId: ByteArray) {
        upsertIdentityKeyWithKeyId(pubkey, identityId, keyId = 0)
    }

    private fun upsertIdentityKeyWithKeyId(pubkey: ByteArray, identityId: ByteArray, keyId: Int) {
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, keyId, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 3, 5, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun derivationFailureIsRecordedAsAPendingIdentityKey() = runTest {
        handler = newHandler(ThrowingDeriver())

        val identityId = ByteArray(32) { 15 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 10 }
        upsertIdentityKey(pubkey, identityId)

        // The key row persists watch-only (no identifier) — same as before…
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertNotNull(row)
        assertNull(row!!.privateKeyKeychainIdentifier)

        // …but the failure is now queryable instead of silent.
        val pending = handler.pendingIdentityKeys.value
        val entry = pending[pubkey.toHex()]
        assertNotNull("expected a pending entry for the failed key", entry)
        assertEquals(walletId.toHex(), entry!!.walletIdHex)
        assertEquals(identityId.toBase58String(), entry.identityIdBase58)
        assertEquals(0, entry.keyId)
        assertEquals(3, entry.identityIndex)
        assertEquals(5, entry.keyIndex)
        assertEquals("keystore unavailable", entry.reason)
    }

    @Test
    fun deriverReturningNullIsAlsoRecordedAsPending() = runTest {
        handler = PlatformWalletPersistenceHandler(
            db, Dispatchers.Unconfined, FakeDeriver(id = null),
        )

        val identityId = ByteArray(32) { 16 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 11 }
        upsertIdentityKey(pubkey, identityId)

        val entry = handler.pendingIdentityKeys.value[pubkey.toHex()]
        assertNotNull(entry)
        assertEquals("deriver returned no storage identifier", entry!!.reason)
    }

    @Test
    fun laterSuccessfulDeriveClearsThePendingEntry() = runTest {
        // First round fails…
        var boom = true
        val flaky = object : PrivateKeyDeriver {
            override fun deriveAndStore(
                walletId: ByteArray,
                publicKeyData: ByteArray,
                identityIndex: Int,
                keyIndex: Int,
                keyType: Int,
                force: Boolean,
            ): DerivedKeyStoreResult =
                if (boom) {
                    throw IllegalStateException("transient")
                } else {
                    DerivedKeyStoreResult("privkey.cafebabe", wasNewlyCreated = true)
                }

            override fun deleteUnownedStored(
                pubkeyHexes: Collection<String>,
                excludingWalletId: ByteArray,
            ): Set<String> = emptySet()
        }
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, flaky)

        val identityId = ByteArray(32) { 17 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 12 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        // …a re-persist (e.g. the next sync round) succeeds and clears it.
        boom = false
        upsertIdentityKey(pubkey, identityId)
        assertNull(handler.pendingIdentityKeys.value[pubkey.toHex()])
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertEquals("privkey.cafebabe", row!!.privateKeyKeychainIdentifier)
    }

    @Test
    fun markIdentityKeyRepairedClearsThePendingEntry() = runTest {
        // A derive failure records the key as pending…
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 18 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 13 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        // …and a successful out-of-band repair (PlatformWalletManager.repairIdentityKey
        // stores directly through the deriver, never re-firing onPersistIdentityKeyUpsert)
        // clears it via this hook.
        handler.markIdentityKeyRepaired(pubkey.toHex())
        assertNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        // Idempotent: a second clear (or one for an unknown key) is a no-op.
        handler.markIdentityKeyRepaired(pubkey.toHex())
        handler.markIdentityKeyRepaired(ByteArray(33) { 99 }.toHex())
        assertTrue(handler.pendingIdentityKeys.value.isEmpty())
    }

    @Test
    fun identityKeyRemovalClearsThePendingEntry() = runTest {
        // dashpay/platform#4183 review: a pending-repair entry must not outlive
        // the key it describes. A derive failure records the key as pending;
        // removing that key (onPersistIdentityKeyRemoval) must drop the now-
        // phantom entry — a repair could never re-derive a key into an identity
        // that no longer carries it.
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 20 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 15 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        // A committed removal round deletes the row AND clears the pending entry.
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyRemoval(walletId, identityId, 0)
        handler.onChangesetEnd(walletId, success = true)

        assertNull(db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0))
        assertTrue(handler.pendingIdentityKeys.value.isEmpty())
    }

    @Test
    fun rolledBackIdentityKeyRemovalKeepsThePendingEntry() = runTest {
        // The removal's pending-clear is staged with the round (mirroring the
        // upsert path): an aborted round discards both the row deletion and the
        // pending-clear, so the pre-round pending entry survives untouched.
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 21 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 16 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyRemoval(walletId, identityId, 0)
        handler.onChangesetEnd(walletId, success = false)

        assertNotNull(db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0))
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])
    }

    @Test
    fun identityRemovalClearsEveryPendingEntryForThatIdentity() = runTest {
        // dashpay/platform#4183 review: deleting an identity cascades away ALL
        // of its public-key rows, so every pending-repair entry for that
        // identity is a phantom afterwards — a repair could never re-derive a
        // key into an identity that no longer exists. All of them must clear
        // (not just one keyId, as onPersistIdentityKeyRemoval handles).
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 22 }
        seedIdentity(identityId)
        // Two watch-only keys under the same identity, different keyIds.
        val pubkey0 = ByteArray(33) { 17 }
        val pubkey1 = ByteArray(33) { 18 }
        upsertIdentityKeyWithKeyId(pubkey0, identityId, keyId = 0)
        upsertIdentityKeyWithKeyId(pubkey1, identityId, keyId = 1)
        assertEquals(2, handler.pendingIdentityKeys.value.size)

        // A committed identity-removal round deletes the rows AND clears every
        // pending entry for the identity.
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityRemoval(walletId, identityId)
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(handler.pendingIdentityKeys.value.isEmpty())
    }

    @Test
    fun rolledBackIdentityRemovalKeepsThePendingEntries() = runTest {
        // The identity-removal pending-clear is staged with the round: an
        // aborted round discards both the identity deletion and the clear, so
        // the pre-round pending entry survives untouched.
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 23 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 19 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityRemoval(walletId, identityId)
        handler.onChangesetEnd(walletId, success = false)

        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])
    }

    @Test
    fun walletDeletionClearsWalletScopedPendingEntries() = runTest {
        // dashpay/platform#4183 review: a wallet wipe cascades away all of its
        // identities and their public-key rows, so every pending-repair entry
        // scoped to that wallet is a phantom afterwards. deleteWalletData must
        // prune them (Room's cascade cannot mutate the process-local
        // StateFlow).
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 24 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 20 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        handler.deleteWalletData(walletId)

        assertTrue(handler.pendingIdentityKeys.value.isEmpty())
    }

    /**
     * Regression (dashpay/platform#4060, finding de3cf44a71fc): the pending
     * record is staged with the round, not published mid-round — the
     * watch-only row it describes is only buffered until [onChangesetEnd],
     * so an aborted round (which discards that row) must leave no phantom
     * pending entry behind.
     */
    @Test
    fun abortedRoundLeavesNoPhantomPendingKeyState() = runTest {
        handler = newHandler(ThrowingDeriver())

        val identityId = ByteArray(32) { 19 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 14 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 3, 5, 0, ByteArray(32), null,
        )
        // Mid-round the record is only STAGED: the watch-only row it
        // describes has not committed yet.
        assertTrue(handler.pendingIdentityKeys.value.isEmpty())

        handler.onChangesetEnd(walletId, success = false)

        // The aborted round discarded the watch-only row — its staged
        // pending entry must vanish with it, not survive as a phantom.
        assertTrue(handler.pendingIdentityKeys.value.isEmpty())
        assertNull(db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0))

        // The same failure in a round that COMMITS still publishes.
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])
    }

    /**
     * Regression (dashpay/platform#4060, finding de3cf44a71fc), the converse
     * flow: an earlier watch-only key is pending; a retry round derives
     * successfully (staging the clear) but then ABORTS. Rollback's alias
     * cleanup deletes the newly stored scalar, so the old watch-only row —
     * still the persisted truth — must keep its repair signal instead of
     * losing it to a mid-round clear.
     */
    @Test
    fun abortedRetryRoundPreservesThePendingRepairSignal() = runTest {
        var boom = true
        val flaky = object : PrivateKeyDeriver {
            val deletedAliases = mutableListOf<String>()

            override fun deriveAndStore(
                walletId: ByteArray,
                publicKeyData: ByteArray,
                identityIndex: Int,
                keyIndex: Int,
                keyType: Int,
                force: Boolean,
            ): DerivedKeyStoreResult =
                if (boom) {
                    throw IllegalStateException("transient")
                } else {
                    DerivedKeyStoreResult("privkey.cafebabe", wasNewlyCreated = true)
                }

            override fun deleteUnownedStored(
                pubkeyHexes: Collection<String>,
                excludingWalletId: ByteArray,
            ): Set<String> {
                deletedAliases.addAll(pubkeyHexes)
                return pubkeyHexes.toSet()
            }
        }
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, flaky)

        val identityId = ByteArray(32) { 20 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 15 }

        // A committed failing round records the watch-only key as pending.
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        // The retry round derives + stores successfully (the clear is
        // staged)… and then the round rolls back.
        boom = false
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            true, 3, 5, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = false)

        // Rollback scrubbed the round's newly stored scalar; the old
        // watch-only row is still the persisted truth, so the repair signal
        // must survive the aborted round's staged clear.
        assertEquals(listOf(pubkey.toHex()), flaky.deletedAliases)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])
    }

    // ── Pending-key reconstruction after restart (#4060 finding 5) ─────

    @Test
    fun reconstructionSeedsPendingFromBreadcrumbRowsWithNullIdentifier() = runTest {
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 21 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 12 }
        upsertIdentityKey(pubkey, identityId) // derive fails → watch-only + breadcrumbs

        // Model a process restart: a fresh handler starts with an empty
        // in-memory map, then rebuilds it from the durable rows.
        val restarted = newHandler()
        assertTrue(restarted.pendingIdentityKeys.value.isEmpty())
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { false },
            nowMs = 42L,
        )

        val entry = restarted.pendingIdentityKeys.value[pubkey.toHex()]
        assertNotNull("breadcrumb row with null identifier must re-seed", entry)
        assertEquals(walletId.toHex(), entry!!.walletIdHex)
        assertEquals(identityId.toBase58String(), entry.identityIdBase58)
        assertEquals(0, entry.keyId)
        assertEquals(3, entry.identityIndex)
        assertEquals(5, entry.keyIndex)
        assertEquals("reconstructed from persistence after restart", entry.reason)
        assertEquals(42L, entry.failedAtMs)
    }

    @Test
    fun reconstructionSeedsStrandedBlobRowsDespiteRecordedIdentifier() = runTest {
        // The derive SUCCEEDED at persist time (identifier recorded), but the
        // stored blob no longer passes the cheap capability check — e.g. the
        // Keystore keypair was replaced. The repair slot must resurface.
        handler = newHandler(FakeDeriver())
        val identityId = ByteArray(32) { 22 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 13 }
        upsertIdentityKey(pubkey, identityId)
        assertTrue(handler.pendingIdentityKeys.value.isEmpty()) // healthy at persist time

        val restarted = newHandler()
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { false }, // blob stranded
        )
        assertNotNull(restarted.pendingIdentityKeys.value[pubkey.toHex()])
    }

    @Test
    fun reconstructionSkipsHealthyRows() = runTest {
        // Identifier recorded AND the blob still decrypts: nothing to repair,
        // so a restart must not fabricate pending state.
        handler = newHandler(FakeDeriver())
        val identityId = ByteArray(32) { 23 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 14 }
        upsertIdentityKey(pubkey, identityId)

        val restarted = newHandler()
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { true },
        )
        assertTrue(restarted.pendingIdentityKeys.value.isEmpty())
    }

    @Test
    fun repairedRowUpdatePreventsReseeding() = runTest {
        // A failed derive leaves a pending row; the repair path later records
        // the identifier on the Room row (and the blob decrypts). The next
        // restart's reconstruction must NOT resurrect the repaired key.
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 24 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 16 }
        upsertIdentityKey(pubkey, identityId)

        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)!!
        assertNull(row.privateKeyKeychainIdentifier)
        db.publicKeyDao().update(
            row.copy(privateKeyKeychainIdentifier = "privkey." + pubkey.toHex()),
        )

        val restarted = newHandler()
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { true },
        )
        assertTrue(restarted.pendingIdentityKeys.value.isEmpty())
    }

    /**
     * #4060 round-2 finding 3: a KeyPermanentlyInvalidatedException on a
     * LEGACY-alias-backed key is invisible to the cheap capability check —
     * the legacy aliases are read-only (no deletion boundary), so
     * `hasLegacyKeysKey()` / `isPrivateKeyDecryptable` stay true forever and
     * neither canSignWith nor the restart reconstruction ever notices. The
     * sign path's invalidation hook must write the durable signal (null the
     * Room identifier) and seed pendingIdentityKeys immediately, EVEN while
     * the cheap check still claims the key is usable.
     */
    @Test
    fun signingKeyInvalidationSeedsPendingDespiteAUsableCheapCheck() = runTest {
        handler = newHandler(FakeDeriver())
        val identityId = ByteArray(32) { 26 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 18 }
        upsertIdentityKey(pubkey, identityId) // healthy: identifier + breadcrumbs
        assertTrue(handler.pendingIdentityKeys.value.isEmpty())

        // Legacy-alias KPIE: the cheap check KEEPS reporting usable (true).
        handler.recordSigningKeyInvalidated(pubkey.toHex()) { true }

        // Durable: the Room identifier is nulled…
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)!!
        assertNull(row.privateKeyKeychainIdentifier)
        // …and the pending state seeds NOW, with the invalidation reason.
        val entry = handler.pendingIdentityKeys.value[pubkey.toHex()]
        assertNotNull("invalidation must seed a pending entry", entry)
        assertEquals("signing key permanently invalidated", entry!!.reason)
        assertEquals(3, entry.identityIndex)
        assertEquals(5, entry.keyIndex)

        // And the SAME durable path re-seeds after a restart, still despite
        // the cheap check claiming usable.
        val restarted = newHandler()
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { true },
        )
        assertNotNull(restarted.pendingIdentityKeys.value[pubkey.toHex()])
    }

    @Test
    fun reconstructionNeverOverwritesALiveEntry() = runTest {
        handler = newHandler(ThrowingDeriver())
        val identityId = ByteArray(32) { 25 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 17 }
        upsertIdentityKey(pubkey, identityId)

        // The live entry (fresh reason/timestamp) must win over the
        // reconstructed placeholder.
        val liveReason = handler.pendingIdentityKeys.value[pubkey.toHex()]!!.reason
        handler.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { false },
        )
        assertEquals(liveReason, handler.pendingIdentityKeys.value[pubkey.toHex()]!!.reason)
    }

    // ── Durable, pubkey-verified repair (#4060 blockers 1 & 3) ─────────

    /**
     * Models the production [org.dashfoundation.dashsdk.security.IdentityKeyPrivateKeyDeriver]:
     * a persist-time derive fails (returns null → the key seeds as pending),
     * while a repair (`force`) derives the KEYPAIR and verifies the derived
     * PUBLIC key equals `publicKeyData` BEFORE storing — a wrong slot throws
     * [IdentityKeyDerivationMismatchException] without persisting anything.
     */
    private class VerifyingRepairDeriver : PrivateKeyDeriver {
        var lastForceCall: Triple<ByteArray, Int, Int>? = null
        val storedFor = mutableSetOf<String>()

        override fun deriveAndStore(
            walletId: ByteArray,
            publicKeyData: ByteArray,
            identityIndex: Int,
            keyIndex: Int,
            keyType: Int,
            force: Boolean,
        ): DerivedKeyStoreResult? {
            if (!force) {
                // Persist-time failure → the key is recorded watch-only +
                // pending (breadcrumbs still land on the row).
                return null
            }
            lastForceCall = Triple(walletId, identityIndex, keyIndex)
            val derivedPublic = fakePubkeyFor(identityIndex, keyIndex)
            // BLOCKER 1: verify BEFORE persistence; wrong slot → no store.
            if (!derivedPublic.contentEquals(publicKeyData)) {
                throw org.dashfoundation.dashsdk.security.IdentityKeyDerivationMismatchException(
                    "derived pubkey for slot $identityIndex/$keyIndex does not match request",
                )
            }
            storedFor.add(publicKeyData.toHex())
            return DerivedKeyStoreResult("privkey." + publicKeyData.toHex(), wasNewlyCreated = true)
        }

        override fun deleteUnownedStored(
            pubkeyHexes: Collection<String>,
            excludingWalletId: ByteArray,
        ): Set<String> = emptySet()

        companion object {
            /** Deterministic stand-in for the Rust keypair public half. */
            fun fakePubkeyFor(identityIndex: Int, keyIndex: Int): ByteArray =
                ByteArray(33).also { it[0] = identityIndex.toByte(); it[1] = keyIndex.toByte() }
        }
    }

    @Test
    fun repairWithCorrectBreadcrumbsDerivesVerifiesAndClearsPending() = runTest {
        val deriver = VerifyingRepairDeriver()
        handler = newHandler(deriver)
        val identityId = ByteArray(32) { 30 }
        seedIdentity(identityId)

        // upsertIdentityKey records breadcrumbs 3/5; the pubkey MUST be the
        // one those breadcrumbs derive, so the repair verification passes.
        val pubkey = VerifyingRepairDeriver.fakePubkeyFor(3, 5)
        upsertIdentityKey(pubkey, identityId)
        assertNotNull("persist-time failure seeds pending", handler.pendingIdentityKeys.value[pubkey.toHex()])

        var probed = false
        val id = handler.repairIdentityKeyDurably(
            walletId = walletId,
            publicKeyData = pubkey,
            verifyRecoverable = { probed = true; true },
        )

        assertEquals("privkey." + pubkey.toHex(), id)
        // Derived from the PERSISTED breadcrumbs (3/5), not any caller index.
        assertEquals(Triple(walletId.toHex(), 3, 5), deriver.lastForceCall!!.let {
            Triple(it.first.toHex(), it.second, it.third)
        })
        assertTrue("blob decrypt verified", probed)
        // Pending cleared and the row now carries the durable identifier.
        assertNull(handler.pendingIdentityKeys.value[pubkey.toHex()])
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertEquals("privkey." + pubkey.toHex(), row!!.privateKeyKeychainIdentifier)
    }

    @Test
    fun repairWithMismatchedBreadcrumbsIsRejectedAndLeavesPending() = runTest {
        val deriver = VerifyingRepairDeriver()
        handler = newHandler(deriver)
        val identityId = ByteArray(32) { 31 }
        seedIdentity(identityId)

        // The row's breadcrumbs (3/5) derive fakePubkeyFor(3,5), which is NOT
        // this pubkey — modelling wrong/corrupt breadcrumbs. The repair must
        // reject rather than persist a different, unusable key.
        val pubkey = ByteArray(33) { 77 }
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        var probed = false
        var thrown: Throwable? = null
        try {
            handler.repairIdentityKeyDurably(
                walletId = walletId,
                publicKeyData = pubkey,
                verifyRecoverable = { probed = true; true },
            )
        } catch (t: Throwable) {
            thrown = t
        }
        assertTrue(
            "wrong-slot repair must throw IdentityKeyDerivationMismatchException, got $thrown",
            thrown is org.dashfoundation.dashsdk.security.IdentityKeyDerivationMismatchException,
        )

        // Nothing persisted, blob never even probed, pending intact.
        assertFalse("verification must not run after a derive-mismatch", probed)
        assertTrue(deriver.storedFor.isEmpty())
        assertNotNull(
            "a rejected repair must NOT clear pending",
            handler.pendingIdentityKeys.value[pubkey.toHex()],
        )
        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)
        assertNull(row!!.privateKeyKeychainIdentifier)
    }

    @Test
    fun repairWithoutPersistedBreadcrumbsFailsAndLeavesPending() = runTest {
        val deriver = VerifyingRepairDeriver()
        handler = newHandler(deriver)
        val identityId = ByteArray(32) { 32 }
        seedIdentity(identityId)

        // A key persisted WITHOUT derivation breadcrumbs (derivationIndicesIsSome
        // = false) — the correct slot is unknown, so repair must fail closed.
        val pubkey = ByteArray(33) { 44 }
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId, identityId, 0, 0, 0, 0, false, false, 0,
            pubkey, ByteArray(20), true, walletId,
            false, 0, 0, 0, ByteArray(32), null,
        )
        handler.onChangesetEnd(walletId, success = true)
        var thrown: Throwable? = null
        try {
            handler.repairIdentityKeyDurably(
                walletId = walletId,
                publicKeyData = pubkey,
                verifyRecoverable = { true },
            )
        } catch (t: Throwable) {
            thrown = t
        }
        assertTrue(
            "repair without breadcrumbs must fail with SigningKeyUnavailable, got $thrown",
            thrown is DashSdkError.PlatformWallet.SigningKeyUnavailable,
        )
        assertNull(deriver.lastForceCall)
    }

    @Test
    fun repairWithFailedDurableWriteLeavesPendingIntact() = runTest {
        val deriver = VerifyingRepairDeriver()
        handler = newHandler(deriver)
        val identityId = ByteArray(32) { 33 }
        seedIdentity(identityId)

        val pubkey = VerifyingRepairDeriver.fakePubkeyFor(3, 5)
        upsertIdentityKey(pubkey, identityId)
        assertNotNull(handler.pendingIdentityKeys.value[pubkey.toHex()])

        // BLOCKER 3: derive + verify succeed, but the durable Room write fails.
        // Pending MUST stay so a restart and this session agree the repair is
        // still outstanding — a swallowed failure would resurrect it after
        // restart while the session believed it was done.
        var thrown: Throwable? = null
        try {
            handler.repairIdentityKeyDurably(
                walletId = walletId,
                publicKeyData = pubkey,
                verifyRecoverable = { true },
                persistDurableIdentifier = { throw java.io.IOException("durable write failed") },
            )
        } catch (t: Throwable) {
            thrown = t
        }
        assertTrue("durable-write failure must propagate, got $thrown", thrown is java.io.IOException)
        assertNotNull(
            "a failed durable write must NOT clear pending",
            handler.pendingIdentityKeys.value[pubkey.toHex()],
        )
    }

    // ── Shielded load round-trip ──────────────────────────────────────

    @Test
    fun shieldedNotePersistThenLoadRoundTrips() = runTest {
        val nullifier = ByteArray(32) { 20 }
        val cmx = ByteArray(32) { 21 }
        val noteData = ByteArray(115) { 22 }

        handler.onChangesetBegin(walletId)
        handler.onPersistShieldedNote(
            walletId = walletId,
            noteWalletId = walletId,
            accountIndex = 0,
            position = 7,
            cmx = cmx,
            nullifier = nullifier,
            blockHeight = 50,
            isSpent = 0,
            value = 100_000,
            noteData = noteData,
        )
        handler.onChangesetEnd(walletId, success = true)

        val loaded = handler.onLoadShieldedNotes()
        assertEquals(1, loaded.size)
        val note = loaded[0]
        assertTrue(walletId.contentEquals(note.walletId))
        assertEquals(0, note.accountIndex)
        assertEquals(7L, note.position)
        assertTrue(cmx.contentEquals(note.cmx))
        assertTrue(nullifier.contentEquals(note.nullifier))
        assertEquals(100_000L, note.value)
        assertEquals(0.toByte(), note.isSpent)
        assertTrue(noteData.contentEquals(note.noteData))
    }

    @Test
    fun shieldedSyncStateAdvancesMonotonically() = runTest {
        handler.onChangesetBegin(walletId)
        handler.onPersistShieldedSyncedIndex(walletId, walletId, 0, 100)
        handler.onChangesetEnd(walletId, success = true)

        // A lower watermark must not regress.
        handler.onChangesetBegin(walletId)
        handler.onPersistShieldedSyncedIndex(walletId, walletId, 0, 50)
        handler.onChangesetEnd(walletId, success = true)

        assertEquals(100L, db.shieldedDao().getSyncState(walletId, 0)!!.lastSyncedIndex)
    }

    // ── Wallet-list load ──────────────────────────────────────────────

    @Test
    fun loadWalletListReturnsRestorableWallets() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 1_000_000)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        handler.onChangesetBegin(walletId)
        handler.onPersistSyncState(walletId, 700, 333, 600)
        handler.onChangesetEnd(walletId, success = true)

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        val entry = list[0]
        assertTrue(walletId.contentEquals(entry.walletId))
        assertEquals(testnet, entry.network)
        assertEquals(1, entry.accountSpecs.size)
        assertTrue(xpub.contentEquals(entry.accountSpecs[0].accountXpubBytes))
        assertEquals(700L, entry.platformSyncHeight)
        assertEquals(333L, entry.platformSyncTimestamp)
        assertEquals(600L, entry.platformLastKnownRecentBlock)
    }

    @Test
    fun loadWalletListSkipsWalletsWithoutXpubAccounts() = runTest {
        // Wallet exists but has no account carrying an xpub → not restorable.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        assertTrue(handler.onLoadWalletList().isEmpty())
    }

    @Test
    fun loadWalletListRoundTripsPlatformAddressBalancesWithHeightPin() = runTest {
        // SH-06 regression: the persisted platform-address balance +
        // its `as_of_height` pin MUST come back on the restore row, or a
        // credit at/below the trusted watermark is re-gated off (ADDR-09
        // double-count guard) and lost after every relaunch on Android.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )

        // Seed the platform-address row (the pool-emit path in
        // production), then land a BLAST balance + height pin on it.
        val hash = ByteArray(20) { 8 }
        db.platformAddressDao().upsert(
            PlatformAddressEntity(
                address = "dash1seed",
                addressType = 0,
                addressHash = hash,
                accountIndex = 2,
                addressIndex = 0,
                derivationPath = "m/9'/5'/17'/2'/0'/0",
                walletId = walletId,
            ),
        )
        handler.onChangesetBegin(walletId)
        handler.onPersistAddressBalance(
            walletId = walletId,
            addressType = 0,
            addressHash = hash,
            balance = 5_000_000,
            nonce = 1,
            accountIndex = 2,
            addressIndex = 0,
            asOfHeight = 380_987,
        )
        handler.onChangesetEnd(walletId, success = true)

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        val balances = list[0].platformAddressBalances
        assertEquals(1, balances.size)
        val restored = balances[0]
        assertEquals(0.toByte(), restored.addressType)
        assertTrue(hash.contentEquals(restored.addressHash))
        assertEquals(5_000_000L, restored.balance)
        assertEquals(1, restored.nonce)
        assertEquals(2, restored.accountIndex)
        assertEquals(0, restored.addressIndex)
        // The height pin must survive the round-trip unchanged.
        assertEquals(380_987L, restored.asOfHeight)
    }

    /**
     * The bridge emits a round's `transactions` before its `utxos_added`
     * (`rs-unified-sdk-jni/src/persistence.rs`, `persist_changeset_account`;
     * same order as Swift's `applyAccountChangeset`). A spend whose funding
     * output arrives in the SAME round therefore stages a pending row first
     * and must drain it when the TXO lands a few ops later: the coin ends
     * the round linked to its spender, spent per the spender's context, with
     * no pending row left behind. Separate-round drains are covered
     * elsewhere; this pins the one-round fold.
     */
    @Test
    fun aFundingOutputAndItsSpenderInOneRoundLeaveTheCoinLinkedAndSpent() = runTest {
        seedWalletWithAddress(walletId, "ySameRoundAddr")

        val fundingTxid = ByteArray(32) { 61 }
        val spendingTxid = ByteArray(32) { 62 }
        val outpoint = makeOutpoint(fundingTxid, 0)

        handler.onChangesetBegin(walletId)
        // The spender first — its input has no TXO yet, so this stages a
        // pending row keyed by the outpoint.
        recordTransaction(
            handler,
            walletId, spendingTxid, ByteArray(10) { 5 }, 2, 101, ByteArray(32) { 8 },
            1_700_000_200, 1, "Standard", 0, -60_000, 0, false, "", 1_700_000_100,
            outpoint, 1,
        )
        // Then the funding output, in the same round.
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 60_000, "ySameRoundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val txo = db.txoDao().getByOutpoint(outpoint)
        assertNotNull("the funding output materialised", txo)
        assertTrue("the in-block spender's claim drained onto the TXO", spendingTxid.contentEquals(txo!!.spendingTxid))
        assertEquals("vin index carried from the staged claim", 0, txo.spendingInputIndex)
        assertTrue("spent per the spender's in-block context", txo.isSpent)
        assertTrue(
            "the staged claim is consumed by the drain, not left behind",
            db.documentDao().getPendingInputsByOutpoint(outpoint).isEmpty(),
        )
        assertTrue(
            "and the coin is not handed back as spendable",
            handler.onLoadWalletList().single().utxos.none { it.prevTxid.contentEquals(fundingTxid) && it.vout == 0 },
        )
    }

    @Test
    fun loadWalletListRestoresUnspentUtxosAndExcludesConfirmedSpends() = runTest {
        // CORE-06 regression: persisted unspent TXOs must come back on
        // the restore row (routed to their owning account through
        // core_addresses — Android txos carry no accountId FK), and a
        // TXO whose spend has confirmed must NOT rehydrate as spendable.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 21 }
        val spendingTxid = ByteArray(32) { 22 }
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0, // funding tx: no inputs of ours
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 60_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 1, 40_000, "yUtxoAddr", ByteArray(25) { 9 },
            100, false, true, true, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Spend vout 1 while the spending tx is still pre-block
        // (InstantSend): the linkage lands but `isSpent` must not flip,
        // and the row stays in the restore set (iOS semantics — the
        // post-restart classifier needs the TXO back).
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, spendingTxid, ByteArray(10) { 5 }, 1, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_100,
            makeOutpoint(fundingTxid, 1), 1, // spends fundingTxid:1
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 1, spendingTxid)
        handler.onChangesetEnd(walletId, success = true)
        val linked = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 1))
        assertNotNull(linked)
        assertFalse(linked!!.isSpent)
        assertEquals(2, handler.onLoadWalletList().single().utxos.size)

        // The spending tx confirms in-block: the tx-upsert reconcile
        // must flip `isSpent` (the flag would otherwise never converge
        // — the CORE-06 over-count hazard)…
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, spendingTxid, ByteArray(10) { 5 }, 2, 101, ByteArray(32) { 8 },
            1_700_000_200, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_100,
            makeOutpoint(fundingTxid, 1), 1, // spends fundingTxid:1
        )
        handler.onChangesetEnd(walletId, success = true)
        assertTrue(db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 1))!!.isSpent)

        // …and the restore set shrinks to the truly-unspent row, fully
        // round-tripped with its account routing tag.
        val restored = handler.onLoadWalletList().single().utxos.single()
        assertEquals(0.toByte(), restored.typeTag)
        assertEquals(0.toByte(), restored.standardTag)
        assertEquals(0, restored.accountIndex)
        assertTrue(fundingTxid.contentEquals(restored.prevTxid))
        assertEquals(0, restored.vout)
        assertEquals(60_000L, restored.valueDuffs)
        assertEquals(25, restored.scriptPubKey.size)
        assertEquals(100, restored.height)
        assertTrue(restored.isConfirmed)
        assertFalse(restored.isInstantLocked)
    }

    @Test
    fun sweptTransactionIsDeletedAndFreesOnlyItsOwnInputs() = runTest {
        // A recorded spend that a later, final transaction beat to an input
        // can never confirm; Rust drops it and names it here. The mirror has
        // to drop it too — otherwise the row comes back on the next load and
        // re-creates a balance the wallet already corrected.
        //
        // Shape: the loser (unconfirmed, as every swept loser is) spends A
        // and B; the winner is wallet-relevant, in-block, and takes only A.
        // A must stay out of the restore set, B must return to it.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 41 }
        val sweptTxid = ByteArray(32) { 42 }
        val winnerTxid = ByteArray(32) { 44 }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 140_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        // A (vout 0) and B (vout 1).
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 1, 40_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        // The doomed transaction: mempool context — upstream only ever
        // sweeps unconfirmed records, so its inputs are linked to it without
        // `isSpent` ever flipping.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, sweptTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -140_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0) + makeOutpoint(fundingTxid, 1), 2,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, sweptTxid)
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 1, sweptTxid)
        handler.onWalletChangesetUtxoAdded(
            walletId, sweptTxid, 0, 60_000, "yUtxoAddr", ByteArray(25) { 6 },
            0, false, false, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertFalse(
            "a pre-block spender links but must not flip isSpent",
            db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent,
        )

        // The winner confirms, taking A, then the sweep runs — the ordering
        // the persist path guarantees inside one round.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, winnerTxid, ByteArray(10) { 6 }, 2, 102, ByteArray(32) { 9 },
            1_700_000_200, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_150,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, winnerTxid)
        sweep(handler, walletId, listOf(sweptTxid), winnerTxid, listOf(makeOutpoint(fundingTxid, 1)), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertNull("the swept transaction row is gone", db.transactionDao().getByTxid(sweptTxid))
        assertNull(
            "the change it created is gone with it",
            db.txoDao().getByOutpoint(makeOutpoint(sweptTxid, 0)),
        )
        assertNotNull("the funding transaction is untouched", db.transactionDao().getByTxid(fundingTxid))

        val winnerTaken = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!
        assertTrue("the coin the winner took stays spent", winnerTaken.isSpent)
        assertTrue(winnerTxid.contentEquals(winnerTaken.spendingTxid))

        // B was only ever claimed by the loser, so it is spendable again.
        val released = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 1))!!
        assertFalse("the loser's own input is free again", released.isSpent)
        assertNull(released.spendingTxid)
        val restored = handler.onLoadWalletList().single().utxos.single()
        assertEquals(1, restored.vout)
    }

    @Test
    fun anAbsentWinnerStillKeepsItsOwnInputSpent() = runTest {
        // The winner can spend our coin and pay only outside addresses. It
        // sweeps the loser all the same, but no record for it ever reaches
        // the persister — so nothing in this store could work out that the
        // coin is gone. Upstream can, and reports it by leaving the coin out
        // of the released set. A swept loser is unconfirmed, so its input is
        // linked at `isSpent = 0`; deleting the loser and stopping there
        // would return a coin the chain has already spent as spendable.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 45 }
        val sweptTxid = ByteArray(32) { 46 }
        val irrelevantWinner = ByteArray(32) { 47 }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, sweptTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, sweptTxid)
        handler.onChangesetEnd(walletId, success = true)
        assertFalse(db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent)

        handler.onChangesetBegin(walletId)
        // Upstream knows the winner took this coin even though it never
        // reports the winner itself, so nothing is released.
        sweep(handler, walletId, listOf(sweptTxid), irrelevantWinner, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertNull(db.transactionDao().getByTxid(sweptTxid))
        val held = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!
        assertTrue("the coin the unrecorded winner may have taken is held", held.isSpent)
        assertNull("with no spender invented for it", held.spendingTxid)
        assertTrue(
            "but with the winner stamped, the same attribution SQLite " +
                "records as spent_in_txid",
            irrelevantWinner.contentEquals(held.supersededByTxid),
        )
        assertTrue(
            "and it stays out of the restore set",
            handler.onLoadWalletList().single().utxos.isEmpty(),
        )

    }

    @Test
    fun aStampedUnlinkedCoinTheWalletRedeliversUnspentFollowsTheWallet() = runTest {
        // The rule this test USED to pin was the opposite — "a re-delivery
        // cannot outrank the sweep's verdict, only a release frees a
        // stamped hold". That rule locked a real coin out forever: a
        // materialised coin is one the wallet knows, any network-final
        // spender of a coin it knows is wallet-relevant by BIP158 prevout
        // matching, so the wallet's own scan re-discovers the spend — and
        // if it instead re-delivers the coin UNSPENT, the winner was reorged
        // out (or was never mined) and there is nothing to hold it against.
        // On this side of the FFI a row at `isSpent = true` is never
        // restored to Rust again, so refusing meant the coin was gone for
        // good. Same answer as the SQLite store's upsert valve, which now
        // holds only never-materialised placeholders: a stamped, UNLINKED
        // row the wallet hands back as a UTXO is cleared, stamp included.
        // A row still LINKED to a spender keeps its flag — the link is the
        // store's recorded spend attribution and the sweep pass owns it.
        seedWalletWithAddress(walletId, "yUtxoAddr")
        val fundingTxid = ByteArray(32) { 48 }
        val coin = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 49 }
        val irrelevantWinner = ByteArray(32) { 54 }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        recordTransaction(
            handler,
            walletId, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_050,
            coin, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        sweepRound(walletId, listOf(loserTxid), irrelevantWinner)
        val held = db.txoDao().getByOutpoint(coin)!!
        assertTrue("sanity: held by the stamp, unlinked", held.isSpent && held.spendingTxid == null)
        assertTrue(irrelevantWinner.contentEquals(held.supersededByTxid))

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val redelivered = db.txoDao().getByOutpoint(coin)!!
        assertFalse("the wallet re-delivering the coin unspent lifts the hold", redelivered.isSpent)
        assertNull("stamp included", redelivered.supersededByTxid)
        assertEquals(1, handler.onLoadWalletList().single().utxos.size)
    }

    @Test
    fun aWinnersLateSpentEmitDoesNotDowngradeAStampedHold() = runTest {
        // The winner's own record can reach this store only after the sweep
        // and the funding TXO already did — IS-locked, not yet in a block.
        // Its record pass is monotonic and merely links the spender, but
        // the utxos_spent emit that rides with it resolved the in-block
        // gate to false and wrote it, flipping a durable stamped hold back
        // into the restore set until the winner confirmed — contradicting
        // the verdict the sweep already recorded.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 56 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 57 }
        val winnerTxid = ByteArray(32) { 58 }

        // The doomed spend, before its funding output.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // The sweep holds the claim; the funding TXO then materializes it
        // as a stamped hold.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(loserTxid), winnerTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertTrue(db.txoDao().getByOutpoint(pOutpoint)!!.isSpent)

        // The winner's own record finally arrives, IS-locked (context 1 <
        // in-block), with the spent emit riding along the way a real round
        // delivers both.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, winnerTxid, ByteArray(10) { 6 }, 1, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_060,
            pOutpoint, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, winnerTxid)
        handler.onChangesetEnd(walletId, success = true)

        val held = db.txoDao().getByOutpoint(pOutpoint)!!
        assertTrue(
            "the winner's own unconfirmed arrival must not downgrade the stamped hold",
            held.isSpent,
        )
        assertTrue(winnerTxid.contentEquals(held.supersededByTxid))
        assertTrue(
            "the spender is linked all the same",
            winnerTxid.contentEquals(held.spendingTxid),
        )
        assertTrue(handler.onLoadWalletList().single().utxos.isEmpty())
    }

    @Test
    fun aReleaseNamingACoinASettledSpenderStillClaimsIsRefused() = runTest {
        // The pruned-finalized-release defect, on this store's terms: a
        // chainlocked spender F is pruned upstream to a bare txid, so a
        // later loser L that pays this wallet while reusing F's input (plus
        // an attacker-owned one) sweeps with F's coin wrongly named in
        // `releasedOutpoints`. F's row and its `spendingTxid` link survive
        // HERE, and the link guard keeps L's record pass from stealing the
        // attribution — so the release pass finds F's coin linked to a
        // stored network-final spender and refuses it, while the coin only
        // L claimed still comes free in the same batch. The restore surface is the
        // restart: what `onLoadWalletList` hands back is what a relaunch
        // spends from.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 60 }
        val settledCoin = makeOutpoint(fundingTxid, 0)
        val losersOwnCoin = makeOutpoint(fundingTxid, 1)
        val attackerInput = makeOutpoint(ByteArray(32) { 61 }, 0)
        val finalizedTxid = ByteArray(32) { 62 }
        val loserTxid = ByteArray(32) { 63 }
        val winnerTxid = ByteArray(32) { 64 }

        // Fund both coins.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 200_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 1, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        // F: the chainlocked spender of `settledCoin` — upstream keeps only
        // its txid from here on; this store keeps the row and the link.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, finalizedTxid, ByteArray(10) { 5 }, 3, 120, ByteArray(32) { 8 },
            1_700_000_100, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_050,
            settledCoin, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        val linked = db.txoDao().getByOutpoint(settledCoin)!!
        assertTrue("sanity: F's spend marked", linked.isSpent)
        assertTrue("sanity: F holds the link", finalizedTxid.contentEquals(linked.spendingTxid))

        // L: arrives after F's pruning — pays this wallet, reuses F's input
        // alongside the attacker's and one coin of its own. Its record pass
        // must NOT steal F's link.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loserTxid, ByteArray(10) { 6 }, 0, 0, ByteArray(32),
            0, 0, "Standard", 0, 50_000, 0, false, "", 1_700_000_200,
            settledCoin + attackerInput + losersOwnCoin, 3,
        )
        handler.onChangesetEnd(walletId, success = true)
        val guarded = db.txoDao().getByOutpoint(settledCoin)!!
        assertTrue(
            "a settled spender's link is not stolen by a conflicting record",
            finalizedTxid.contentEquals(guarded.spendingTxid),
        )
        assertTrue(
            "the loser's own coin links normally",
            loserTxid.contentEquals(db.txoDao().getByOutpoint(losersOwnCoin)!!.spendingTxid),
        )

        // W (final) beats L on the attacker input alone. Upstream's release
        // set — computed from live records that no longer include F — wrongly
        // names F's coin alongside the loser's own.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(loserTxid), winnerTxid, listOf(settledCoin, losersOwnCoin), 400)
        handler.onChangesetEnd(walletId, success = true)

        val settled = db.txoDao().getByOutpoint(settledCoin)!!
        assertTrue(
            "a released coin a settled stored spender still claims must stay spent",
            settled.isSpent,
        )
        assertTrue(finalizedTxid.contentEquals(settled.spendingTxid))
        val freed = db.txoDao().getByOutpoint(losersOwnCoin)!!
        assertFalse("a coin only the swept loser claimed must come free", freed.isSpent)
        assertNull(freed.spendingTxid)
        assertEquals(
            "the restore surface hands back exactly the freed coin",
            1,
            handler.onLoadWalletList().single().utxos.size,
        )
    }

    @Test
    fun aPreStampHoldStillFreesOnRedelivery() = runTest {
        // The same rule as
        // aStampedUnlinkedCoinTheWalletRedeliversUnspentFollowsTheWallet,
        // for the shape no current writer produces: a coin held spent with
        // neither a spender nor a `supersededByTxid` stamp. An UNLINKED row
        // follows the wallet whatever it carries, so the wallet
        // re-delivering it as a UTXO lifts the mark; only a link is spend
        // evidence a re-delivery leaves alone.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 55 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        db.transactionDao().upsert(
            TransactionEntity(txid = fundingTxid, transactionData = ByteArray(0)),
        )
        db.txoDao().upsert(
            TxoEntity(
                outpoint = pOutpoint,
                vout = 0,
                amount = 100_000,
                address = "yUtxoAddr",
                isSpent = true,
                walletId = walletId,
                txid = fundingTxid,
            ),
        )

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertFalse(
            "a hold with nothing durable behind it frees on re-delivery",
            db.txoDao().getByOutpoint(pOutpoint)!!.isSpent,
        )
        assertEquals(1, handler.onLoadWalletList().single().utxos.size)
    }

    @Test
    fun aReleasedCoinAlreadyReclaimedInTheSameRoundKeepsItsNewSpender() = runTest {
        // A round can carry both a release and a later transaction that
        // legitimately spends the freed coin: merging folds several events
        // together, and every record is written before sweeps are processed.
        // By the time the release runs the coin is claimed again, and freeing
        // it would hand a spent coin back to the restore set.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 50 }
        val sweptTxid = ByteArray(32) { 51 }
        val winnerTxid = ByteArray(32) { 52 }
        val reclaimerTxid = ByteArray(32) { 53 }
        val freedCoin = makeOutpoint(fundingTxid, 1)

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 140_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 1, 40_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        // The doomed transaction claims both coins, unconfirmed as every
        // swept loser is.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, sweptTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -140_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0) + freedCoin, 2,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, sweptTxid)
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 1, sweptTxid)
        handler.onChangesetEnd(walletId, success = true)

        // One round now carries the winner, the sweep releasing the coin the
        // winner did not take, and a later transaction that already spent
        // that freed coin. Records are applied first, sweeps last.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, winnerTxid, ByteArray(10) { 6 }, 2, 101, ByteArray(32) { 8 },
            1_700_000_100, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_090,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, winnerTxid)
        recordTransaction(
            handler,
            walletId, reclaimerTxid, ByteArray(10) { 7 }, 2, 102, ByteArray(32) { 9 },
            1_700_000_200, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_150,
            freedCoin, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 1, reclaimerTxid)
        sweep(handler, walletId, listOf(sweptTxid), winnerTxid, listOf(freedCoin), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertNull("the swept transaction row is still gone", db.transactionDao().getByTxid(sweptTxid))

        val reclaimed = db.txoDao().getByOutpoint(freedCoin)!!
        assertTrue(
            "the later spender keeps its claim",
            reclaimerTxid.contentEquals(reclaimed.spendingTxid),
        )
        assertTrue("so the coin stays spent", reclaimed.isSpent)
        assertTrue(
            "and never returns to the restore set",
            handler.onLoadWalletList().single().utxos.isEmpty(),
        )
    }

    @Test
    fun aLaterSweepKeepingACoinSpentOverridesAnEarlierRelease() = runTest {
        // JNI delivers one call per sweep batch, in order. The first frees a
        // coin, a second transaction spends it, and the second sweep removes
        // that spender while freeing nothing — its own winner took the coin.
        // The later answer has to win, which is what applying the calls in
        // sequence gives: each one holds its losers' inputs before releasing.
        seedWalletWithAddress(walletId, "yUtxoAddr")

        val fundingTxid = ByteArray(32) { 70 }
        val firstLoser = ByteArray(32) { 71 }
        val secondLoser = ByteArray(32) { 72 }
        val contested = makeOutpoint(fundingTxid, 0)

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Both losers claim the coin; each is unconfirmed, as swept losers are.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, firstLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_050,
            contested, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, firstLoser)
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, secondLoser, ByteArray(10) { 6 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_100,
            contested, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, secondLoser)
        handler.onChangesetEnd(walletId, success = true)

        // One round, two batches, in order.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(firstLoser), ByteArray(32) { 73 }, listOf(contested), 400)
        sweep(handler, walletId, listOf(secondLoser), ByteArray(32) { 74 }, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        val row = db.txoDao().getByOutpoint(contested)!!
        assertTrue("the later sweep kept the coin spent", row.isSpent)
        assertTrue(
            "so it stays out of the restore set",
            handler.onLoadWalletList().single().utxos.isEmpty(),
        )
    }

    /**
     * Seed the review finding's exact shape: one loser transaction shared by
     * two wallets, spending one coin from each. Upstream computes each
     * wallet's released set independently
     * (`per_wallet_released_outpoints`), and neither wallet's own winner row
     * is ever created here — matching the "the winner can pay only outside
     * addresses" case the released set exists to handle. Both coins live in
     * the same funding transaction purely for setup convenience; what makes
     * the loser shared is that it spends a TXO owned by each wallet.
     *
     * Returns the funding txid and the loser txid so callers can build the
     * outpoints and drive the sweep.
     */
    private suspend fun seedSharedLoserAcrossTwoWallets(walletA: ByteArray, walletB: ByteArray): Pair<ByteArray, ByteArray> {
        handler.onPersistWalletMetadata(walletA, testnet, groupId, 0)
        handler.onPersistWalletMetadata(walletB, testnet, groupId, 0)
        // Distinct xpubs — `accountExtendedPubKeyBytes` carries a unique
        // index, so two accounts sharing one would silently fail the second
        // registration (`guarded` swallows the constraint violation).
        handler.onPersistAccountRegistration(
            walletA, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 30 },
        )
        handler.onPersistAccountRegistration(
            walletB, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 31 },
        )
        val accountA = db.accountDao().observeByWallet(walletA).first().single()
        val accountB = db.accountDao().observeByWallet(walletB).first().single()
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = "yWalletA", poolTypeTag = 0, addressIndex = 0,
                derivationPath = "m/44'/1'/0'/0/0", accountId = accountA.id,
            ),
        )
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = "yWalletB", poolTypeTag = 0, addressIndex = 0,
                derivationPath = "m/44'/1'/0'/0/0", accountId = accountB.id,
            ),
        )

        val fundingTxid = ByteArray(32) { 80 }
        val loserTxid = ByteArray(32) { 81 }

        // P (vout 0) — wallet A's coin.
        handler.onChangesetBegin(walletA)
        recordTransaction(
            handler,
            walletA, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 140_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletA, fundingTxid, 0, 100_000, "yWalletA", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletA, success = true)

        // Q (vout 1) — wallet B's coin, same funding transaction.
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetUtxoAdded(
            walletB, fundingTxid, 1, 40_000, "yWalletB", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletB, success = true)

        // The shared loser: unconfirmed, spends both P and Q.
        handler.onChangesetBegin(walletA)
        recordTransaction(
            handler,
            walletA, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -140_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0) + makeOutpoint(fundingTxid, 1), 2,
        )
        handler.onWalletChangesetUtxoSpent(walletA, fundingTxid, 0, loserTxid)
        handler.onWalletChangesetUtxoSpent(walletA, fundingTxid, 1, loserTxid)
        handler.onChangesetEnd(walletA, success = true)

        return fundingTxid to loserTxid
    }

    @Test
    fun sharedLoserAppliesEachWalletsOwnReleaseSetRegardlessOfOrder_walletBThenWalletA() = runTest {
        // The hold is global, the release is per wallet. The FIRST callback
        // that sees the sweep holds EVERY wallet's rows for the loser's
        // inputs (stamped with the winner, links to the loser detached) and
        // deletes the loser's row outright; each wallet's own callback then
        // applies ITS released set to ITS rows, by outpoint — so a later
        // callback for the same loser, finding no row, still frees what it
        // was entitled to. Wallet B (which releases nothing) runs first: it
        // holds A's coin too — conservatively, until A's own verdict lands.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 82 }
        val p = makeOutpoint(fundingTxid, 0)
        val q = makeOutpoint(fundingTxid, 1)

        sweepRound(walletB, listOf(loserTxid), winnerTxid)

        assertNull(
            "the first callback deletes the shared row — the hold outlives it",
            db.transactionDao().getByTxid(loserTxid),
        )
        val heldP = db.txoDao().getByOutpoint(p)!!
        assertTrue("wallet A's coin is held until A's own release names it", heldP.isSpent)
        assertNull("the link to the dead loser is detached", heldP.spendingTxid)
        assertTrue(winnerTxid.contentEquals(heldP.supersededByTxid))

        // Wallet A second: the loser's row is gone, and its release of P
        // still lands by outpoint.
        sweepRound(walletId, listOf(loserTxid), winnerTxid, released = listOf(p))

        val freedP = db.txoDao().getByOutpoint(p)!!
        assertFalse("wallet A's own release must free its own coin", freedP.isSpent)
        assertNull(freedP.spendingTxid)
        assertNull("the stamp goes with the hold", freedP.supersededByTxid)

        val heldQ = db.txoDao().getByOutpoint(q)!!
        assertTrue("wallet B's own decision to hold Q survives wallet A's callback", heldQ.isSpent)
        assertNull(heldQ.spendingTxid)
        assertTrue(winnerTxid.contentEquals(heldQ.supersededByTxid))
    }

    @Test
    fun sharedLoserAppliesEachWalletsOwnReleaseSetRegardlessOfOrder_walletAThenWalletB() = runTest {
        // Mirror of the ordering above: wallet A (which releases P) runs
        // first and holds B's coin; B's callback releases nothing. The end
        // state must be the same.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 92 }
        val p = makeOutpoint(fundingTxid, 0)
        val q = makeOutpoint(fundingTxid, 1)

        sweepRound(walletId, listOf(loserTxid), winnerTxid, released = listOf(p))

        assertNull("the first callback deletes the shared row", db.transactionDao().getByTxid(loserTxid))
        val heldQ = db.txoDao().getByOutpoint(q)!!
        assertTrue("wallet B's coin is held by A's callback until B's own verdict", heldQ.isSpent)
        assertNull(heldQ.spendingTxid)
        assertTrue(winnerTxid.contentEquals(heldQ.supersededByTxid))
        assertFalse("wallet A's own coin came free at once", db.txoDao().getByOutpoint(p)!!.isSpent)

        sweepRound(walletB, listOf(loserTxid), winnerTxid)

        val freedP = db.txoDao().getByOutpoint(p)!!
        assertFalse("wallet A's earlier release must survive wallet B's callback", freedP.isSpent)
        assertNull(freedP.spendingTxid)

        val stillHeldQ = db.txoDao().getByOutpoint(q)!!
        assertTrue("wallet B's own decision to hold its coin must stick", stillHeldQ.isSpent)
        assertNull(stillHeldQ.spendingTxid)
    }

    /**
     * [seedSharedLoserAcrossTwoWallets] plus an output of the loser's own —
     * phantom money, since a transaction that never confirms funded
     * nothing. Driven through the ordinary [onWalletChangesetUtxoAdded]
     * write path, the same as every other row in this fixture, rather than
     * reaching into the DB directly.
     */
    private suspend fun seedSharedLoserWithOwnOutputAcrossTwoWallets(
        walletA: ByteArray,
        walletB: ByteArray,
    ): Pair<ByteArray, ByteArray> {
        val (fundingTxid, loserTxid) = seedSharedLoserAcrossTwoWallets(walletA, walletB)
        handler.onChangesetBegin(walletA)
        handler.onWalletChangesetUtxoAdded(
            walletA, loserTxid, 2, 60_000, "yLoserChange", ByteArray(25) { 6 },
            0, false, false, false, false,
        )
        handler.onChangesetEnd(walletA, success = true)
        return fundingTxid to loserTxid
    }

    @Test
    fun sharedLoserOutputAndCoreTxRecordAreExcludedAfterOnlyOneWalletsCallbackCommits() = runTest {
        // `commit_batch` calls `store()` once per wallet and each commits
        // independently, so wallet A's callback may never arrive at all — a
        // crash, a rejection, or simply never coming. One committed callback
        // must already be the whole removal: the row and its phantom output
        // gone, `onGetCoreTxRecord` blind to it, and A's coin HELD rather
        // than restorable — a missing callback leaves a coin conservatively
        // held, never a wrongly-spent or resurrectable one.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserWithOwnOutputAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 82 }
        val p = makeOutpoint(fundingTxid, 0)
        val phantomOutput = makeOutpoint(loserTxid, 2)

        // Only wallet B's callback ever runs, and it releases nothing.
        sweepRound(walletB, listOf(loserTxid), winnerTxid)

        assertNull("one committed callback deletes the row", db.transactionDao().getByTxid(loserTxid))
        assertNull("and the loser's own output with it", db.txoDao().getByOutpoint(phantomOutput))
        val heldP = db.txoDao().getByOutpoint(p)!!
        assertTrue("wallet A's coin is held, not returned, while A's verdict is missing", heldP.isSpent)
        assertTrue(winnerTxid.contentEquals(heldP.supersededByTxid))

        // "Restart": a fresh handler bound to the same underlying store.
        // Wallet A's own callback never happens.
        val restarted = newHandler()

        assertNull("the phantom output must not resurrect across a restart", db.txoDao().getByOutpoint(phantomOutput))
        assertNull(
            "wallet A must not read the swept loser back as a live transaction",
            restarted.onGetCoreTxRecord(walletId, loserTxid),
        )
        val utxosA = restarted.onLoadWalletList().first { it.walletId.contentEquals(walletId) }.utxos
        assertTrue(
            "neither the phantom output nor the held coin is handed back as restorable",
            utxosA.isEmpty(),
        )
    }

    @Test
    fun twoWalletsEachReleaseTheirOwnPendingClaimOnASharedLoser() = runTest {
        // A shared loser holds one unresolved pending claim per wallet.
        // Upstream computes each wallet's released set from that wallet's
        // own records (every input of the loser that the winner did not
        // take and no surviving record of that wallet still claims), so
        // both wallets name both coins. The first callback (A) deletes its
        // own released claim, tombstones B's — B's verdict is not in yet,
        // and a callback that never arrives must leave a coin held — and
        // deletes the row; B's callback, finding no row, still applies its
        // release by outpoint and deletes its tombstone. No row and no
        // claim survives, and never a freed tombstone.
        val walletB = ByteArray(32) { 8 }
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        handler.onPersistWalletMetadata(walletB, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 65 }
        val pA = makeOutpoint(fundingTxid, 8)
        val pB = makeOutpoint(fundingTxid, 9)
        val loserTxid = ByteArray(32) { 66 }
        val winnerTxid = ByteArray(32) { 67 }

        // The loser's row plus one still-unfunded pending claim per wallet
        // — what each wallet's own record pass would have staged.
        db.transactionDao().upsert(
            TransactionEntity(txid = loserTxid, transactionData = ByteArray(10) { 5 }),
        )
        registerInputs(loserTxid, listOf(pA, pB))
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = pA, inputIndex = 0, spendingTxid = loserTxid,
                spendingTransactionTxid = loserTxid, walletId = walletId,
            ),
        )
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = pB, inputIndex = 1, spendingTxid = loserTxid,
                spendingTransactionTxid = loserTxid, walletId = walletB,
            ),
        )

        sweepRound(walletId, listOf(loserTxid), winnerTxid, released = listOf(pA, pB))
        assertNull("the first callback deletes the row", db.transactionDao().getByTxid(loserTxid))
        assertTrue("A's released claim is deleted outright", db.documentDao().getPendingInputsByOutpoint(pA).isEmpty())
        val heldB = db.documentDao().getPendingInputsByOutpoint(pB).single()
        assertTrue("B's claim is held until B's own verdict", heldB.isSweptTombstone)
        assertTrue(walletB.contentEquals(heldB.walletId))
        assertTrue(winnerTxid.contentEquals(heldB.spendingTxid))

        sweepRound(walletB, listOf(loserTxid), winnerTxid, released = listOf(pA, pB))
        assertTrue(
            "B's release reaches its tombstone with the row already gone",
            db.documentDao().getPendingInputsByOutpoint(pB).isEmpty(),
        )
        assertTrue(db.documentDao().getPendingInputsByOutpoint(pA).isEmpty())
    }

    @Test
    fun aReinstatingRecordInALaterRoundRevivesASweptTransactionAndItsOutputs() = runTest {
        // Cross-round reinstatement: the sweep and its reinstating record
        // land in two SEPARATE callback rounds. Upstream's sweep state is
        // not monotonic — per CoreChangeSet::merge's documented
        // IS-lock-precedence sequence, a transaction swept by an IS-locked
        // conflict can return chainlocked and sweep that conflict in turn —
        // and the sweep deleted the row outright, so the later record is
        // simply an ordinary record of a txid this store no longer holds:
        // nothing marks it as "the reinstating one", nothing can refuse it,
        // and its output rides along in the same round.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserWithOwnOutputAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 82 }
        val p = makeOutpoint(fundingTxid, 0)
        val phantomOutput = makeOutpoint(loserTxid, 2)

        // Round 1: only wallet B's own sweep callback runs, releasing
        // nothing — the row, the phantom output and A's coin's link are gone;
        // A's coin is held by the stamp.
        sweepRound(walletB, listOf(loserTxid), winnerTxid)
        assertNull("sanity: the row is gone after round 1", db.transactionDao().getByTxid(loserTxid))
        assertNull("sanity: the loser's own output is gone after round 1", db.txoDao().getByOutpoint(phantomOutput))
        assertTrue("sanity: A's coin is held", db.txoDao().getByOutpoint(p)!!.isSpent)

        // Round 2, a SEPARATE callback: the wallet returns chainlocked, with
        // its own output riding along — transaction before utxo per the
        // JNI bridge's account ordering.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loserTxid, ByteArray(10) { 5 }, 3, 200, ByteArray(32) { 8 },
            1_700_000_200, 1, "Standard", 0, -140_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, loserTxid, 2, 60_000, "yLoserChange", ByteArray(25) { 6 },
            200, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val reinstated = db.transactionDao().getByTxid(loserTxid)!!
        assertEquals(200, reinstated.blockHeight)

        val revivedOutput = db.txoDao().getByOutpoint(phantomOutput)
        assertNotNull("the reinstated transaction's own output must come back", revivedOutput)
        assertEquals(60_000L, revivedOutput!!.amount)

        val reclaimedP = db.txoDao().getByOutpoint(p)!!
        assertTrue("wallet A's coin stays spent — now by its own live record", reclaimedP.isSpent)
        assertTrue(
            "the stamped, unlinked row adopts the reinstated spender's link",
            loserTxid.contentEquals(reclaimedP.spendingTxid),
        )

        assertNotNull(
            "wallet A must be able to read the reinstated transaction as live again",
            handler.onGetCoreTxRecord(walletId, loserTxid),
        )

        // "Restart": the reinstatement has to be durable.
        val restarted = newHandler()
        assertNotNull("the reinstatement must survive a restart", db.transactionDao().getByTxid(loserTxid))
        assertNotNull("the revived output must survive a restart", db.txoDao().getByOutpoint(phantomOutput))
        assertTrue("the reclaimed input must survive a restart", db.txoDao().getByOutpoint(p)!!.isSpent)
        assertNotNull(
            "the reinstated transaction must still be readable as live after a restart",
            restarted.onGetCoreTxRecord(walletId, loserTxid),
        )
    }

    @Test
    fun aSweepReleasingMoreOutpointsThanSqliteCanBindStillCommits() = runTest {
        // The released set's size follows the input count of a transaction a
        // remote sender chooses, so it is not bounded by anything this wallet
        // controls. Binding it one variable per outpoint crosses the
        // 999-variable ceiling API 29's framework SQLite still carries: the
        // statement throws, the whole atomic round fails, and the watermark
        // freezes on a loser that would be re-swept into the same failure
        // after every restart.
        //
        // The count is far past 999 because this suite runs on the host's
        // SQLite, whose own ceiling is much higher — at 1200 the pre-fix code
        // passed here while still being broken on API 29. What this pins is
        // therefore the property that matters, that the query arity does not
        // grow with the set at all, rather than one platform's exact limit.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )

        val loser = ByteArray(32) { 80 }
        // Comfortably past the limit, and past the 1000-variable default of
        // newer SQLite too.
        val released = (0 until 40000).map { i ->
            makeOutpoint(ByteArray(32) { 81 }, i)
        }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -1_000, 0, false, "", 1_700_000_000,
            ByteArray(0), 0,
        )
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        val code = sweep(handler, walletId, listOf(loser), ByteArray(32) { 82 }, released, 400)
        val committed = handler.onChangesetEnd(walletId, success = true)

        assertEquals("the sweep callback must not fail on a large release set", 0, code)
        assertEquals(0, committed)
        assertNull("and the round must actually commit", db.transactionDao().getByTxid(loser))
    }

    @Test
    fun sweptTransactionRollsBackWithItsRound() = runTest {
        // The deletion is staged in the same buffered transaction as every
        // other write in the round, so a round that fails must not take the
        // rows with it.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val txid = ByteArray(32) { 43 }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, txid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(txid), ByteArray(32) { 44 }, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = false)

        assertNotNull(db.transactionDao().getByTxid(txid))
    }

    @Test
    fun spendBeforeFundingReconcilesViaPendingInputAndExcludesFromRestore() = runTest {
        // CORE-06, out-of-order arrival: an in-block spending tx is persisted
        // BEFORE its funding TXO is known (Rust's utxos_spent slice is empty
        // because the previous output wasn't classified yet). The spend must
        // not be lost — `inputOutpoints` stages a pending-input row that the
        // funding TXO's later upsert drains, so the consumed output is excluded
        // from the restore set instead of being handed back to Rust as
        // spendable. 1:1 mirror of Swift resolveInputOutpoint + upsertUtxo drain.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 41 }
        val spendingTxid = ByteArray(32) { 42 }

        // Changeset 1: the in-block spending tx arrives first. Its funding TXO
        // is unknown, so a pending-input row is staged (no utxos_spent fires).
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, spendingTxid, ByteArray(10) { 5 }, 2, 101, ByteArray(32) { 8 },
            1_700_000_200, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_100,
            makeOutpoint(fundingTxid, 0), 1, // spends fundingTxid:0 (TXO unknown)
        )
        handler.onChangesetEnd(walletId, success = true)

        val staged = db.documentDao().getPendingInputsByOutpoint(makeOutpoint(fundingTxid, 0))
        assertEquals(1, staged.size)
        assertTrue(spendingTxid.contentEquals(staged.single().spendingTxid))
        // Funding TXO absent → nothing to restore yet.
        assertEquals(0, handler.onLoadWalletList().single().utxos.size)

        // Changeset 2: the funding TXO finally lands. The drain links the spend
        // (in-block → isSpent) and clears the pending row.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val txo = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))
        assertNotNull(txo)
        assertTrue(txo!!.isSpent)
        assertTrue(spendingTxid.contentEquals(txo.spendingTxid!!))
        assertTrue(
            db.documentDao().getPendingInputsByOutpoint(makeOutpoint(fundingTxid, 0)).isEmpty(),
        )
        // The consumed output must NOT be handed back to Rust as spendable.
        assertEquals(0, handler.onLoadWalletList().single().utxos.size)
    }

    @Test
    fun sweptSpendBeforeFundingSurvivesRestartAndStaysSpentWhenFunded() = runTest {
        // The loser can be persisted before its own funding output ever is
        // (see spendBeforeFundingReconcilesViaPendingInputAndExcludesFromRestore
        // above) — the spend arrives as a `pending_inputs` row rather than a
        // `TxoEntity` update. When the sweep holds that input (it's not in
        // `releasedOutpoints`), there is no TXO row to mark — the only record
        // of the claim is the pending row, which cascades away with the loser
        // it names (`spendingTransactionTxid`'s FK) unless
        // `onWalletChangesetTransactionsSwept` rescues it first. This is the
        // regression the review finding described: seed the pending spend,
        // sweep it, restart the store, and only then let the funding UTXO
        // arrive. The coin must come back spent, attributed to the winner,
        // not as a fresh unspent row.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 61 }
        val sweptTxid = ByteArray(32) { 62 }
        val winnerTxid = ByteArray(32) { 64 }

        // Changeset 1: the doomed spend arrives with no prior
        // `onWalletChangesetUtxoAdded` for `fundingTxid:0` — the funding side
        // of that outpoint has not been observed yet.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, sweptTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertNull(
            "sanity: the funding TXO has not arrived yet",
            db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0)),
        )
        assertEquals(
            1,
            db.documentDao().getPendingInputsByOutpoint(makeOutpoint(fundingTxid, 0)).size,
        )

        // Changeset 2: the sweep holds the input (not in `releasedOutpoints`),
        // with nothing on hand to update.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(sweptTxid), winnerTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertNull("the loser is gone", db.transactionDao().getByTxid(sweptTxid))

        // Restart: a fresh persister loading the same on-disk store — same
        // Room database, new handler, matching this suite's own restart
        // idiom (e.g. addressBalanceConflictPreservesDerivationIndicesAcrossRestart above).
        val restarted = newHandler()

        // The funding transaction finally arrives and hands the outpoint
        // back as a UTXO — the ordinary path a rescan or late block takes.
        restarted.onChangesetBegin(walletId)
        restarted.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        restarted.onChangesetEnd(walletId, success = true)

        val coin = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))
        assertNotNull("the funding UTXO's own upsert must still create the row", coin)
        assertTrue(
            "the winner's claim must survive the loser's deletion, a restart, " +
                "and the funding UTXO's own arrival",
            coin!!.isSpent,
        )
        assertTrue(winnerTxid.contentEquals(coin.supersededByTxid))
        assertEquals(0, restarted.onLoadWalletList().single().utxos.size)
    }

    @Test
    fun aWinnersOwnPendingRowDoesNotEvaporateTheSweepTombstone() = runTest {
        // Records precede sweeps within a round, so a wallet-relevant winner
        // whose own funding side is ALSO unobserved stages an ordinary
        // pending row for the same outpoint moments before the sweep
        // repoints the loser's row into a tombstone. The tombstone keeps the
        // loser's original, older `createdAt`, so the drain's newest-wins
        // pick would select the winner's ordinary row, take the gated
        // branch (`isSpent` stays false until the winner confirms — never,
        // for an IS-locked unconfirmed winner), skip the `supersededByTxid`
        // stamp, and delete every pending row including the tombstone: the
        // durable hold evaporates and the consumed coin re-enters the
        // restore set.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 91 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 92 }
        val winnerTxid = ByteArray(32) { 93 }

        // Changeset 1: the doomed spend arrives before its funding output.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // The loser's pending row must be strictly older than the winner's,
        // as it always is in reality — `createdAt` has millisecond
        // resolution and both rows land in the same test-run instant
        // otherwise.
        Thread.sleep(5)

        // Changeset 2: the winner's record (IS-locked, still unconfirmed)
        // and the sweep it caused, records first — the order the persist
        // path guarantees inside one round.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, winnerTxid, ByteArray(10) { 6 }, 1, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_060,
            pOutpoint, 1,
        )
        sweep(handler, walletId, listOf(loserTxid), winnerTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        // Sanity: the coexisting pair this regression is about — the
        // winner's ordinary row plus the repointed tombstone.
        val rows = db.documentDao().getPendingInputsByOutpoint(pOutpoint)
        assertEquals(2, rows.size)
        assertEquals(1, rows.count { it.isSweptTombstone })

        // The funding TXO finally arrives and drains both rows.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val coin = db.txoDao().getByOutpoint(pOutpoint)!!
        assertTrue(
            "the sweep's hold must survive the winner's own coexisting pending row",
            coin.isSpent,
        )
        assertTrue(winnerTxid.contentEquals(coin.supersededByTxid))
        assertTrue(
            "the consumed coin must stay out of the restore set",
            handler.onLoadWalletList().single().utxos.isEmpty(),
        )
    }

    @Test
    fun aBatchSweepingParentAndChildDeletesTheChildsClaimOnTheParentsOutput() = runTest {
        // The multi-loser batch shape upstream's descendant closure always
        // produces — parent P and child C removed together — which no
        // fixture here ever exercised: C spends P:0, still unfunded, so the
        // claim lives as a pending row. Upstream never releases a
        // loser-funded outpoint, so without a co-swept check the sweep
        // tombstones the claim to the winner — and P's chainlocked
        // reinstatement then re-delivers P:0 straight into the
        // tombstone-outranks drain: isSpent = true, supersededByTxid =
        // winner, a hold on a coin the winner never took. A dead parent's
        // output is nobody's coin; the claim must be deleted with the
        // batch.
        seedWalletWithAddress(walletId, "yFundAddr")

        val parentTxid = ByteArray(32) { 101 } // P — record never persisted
        val pOutpoint = makeOutpoint(parentTxid, 0)
        val childTxid = ByteArray(32) { 102 } // C
        val winnerTxid = ByteArray(32) { 103 } // W

        // C arrives spending the still-unfunded P:0 — parked as a pending
        // claim.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, childTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_100,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertEquals(1, db.documentDao().getPendingInputsByOutpoint(pOutpoint).size)

        // One batch removes both; upstream excludes P:0 from the released
        // set because its funder is itself a loser.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(parentTxid, childTxid), winnerTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(
            "a claim on a co-swept parent's output must be deleted, not tombstoned",
            db.documentDao().getPendingInputsByOutpoint(pOutpoint).isEmpty(),
        )

        // The chainlocked return: P reinstated with its output re-delivered
        // must land spendable — nothing the batch left behind may hold it.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, parentTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val coin = db.txoDao().getByOutpoint(pOutpoint)!!
        assertFalse(
            "the reinstated parent's output must not be wedged by its dead child's claim",
            coin.isSpent,
        )
        assertNull(coin.supersededByTxid)
        assertEquals(1, handler.onLoadWalletList().single().utxos.size)
    }

    @Test
    fun chainedSweepBeforeFundingReleasesAnEarlierTombstoneOnASecondSweep() = runTest {
        // Regression for the review finding on
        // sweptSpendBeforeFundingSurvivesRestartAndStaysSpentWhenFunded above:
        // that fix repoints a held-but-unfunded pending input at its sweep's
        // winner and detaches it from `spendingTransactionTxid` so it
        // survives the loser's cascade-delete. But a SECOND sweep of that
        // winner — the sweep's staged-row fetch matches
        // `spendingTransactionTxid = :txid`, which the first tombstoning
        // already cleared to null — cannot find the row that way anymore.
        // L spends P; W spends P and Q and sweeps L, holding the still-
        // unfunded P; X spends Q and sweeps W, this time releasing P. P's
        // funding TXO finally arrives and must come back spendable.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 71 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val qOutpoint = makeOutpoint(ByteArray(32) { 72 }, 0)
        val firstLoserTxid = ByteArray(32) { 73 } // L
        val secondLoserTxid = ByteArray(32) { 74 } // W
        val finalWinnerTxid = ByteArray(32) { 75 } // X

        // L spends only P, and P's funding side has never been observed.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, firstLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_070,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // First sweep: W beats L, holding P (still unfunded).
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(firstLoserTxid), secondLoserTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        val tombstone = db.documentDao().getPendingInputsByOutpoint(pOutpoint).single()
        assertTrue("the first sweep must tombstone the pending row", tombstone.isSweptTombstone)
        assertTrue(secondLoserTxid.contentEquals(tombstone.spendingTxid))
        assertNull(
            "the tombstone must have detached from the doomed loser's FK",
            tombstone.spendingTransactionTxid,
        )

        // W's own record — spends P and Q — must be on hand for the second
        // sweep to find, the same requirement any sweep of a wallet-relevant
        // loser has.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, secondLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_071,
            pOutpoint + qOutpoint, 2,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Second sweep: X beats W, releasing P this time.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(secondLoserTxid), finalWinnerTxid, listOf(pOutpoint), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(
            "a released outpoint's tombstone must not survive a chained sweep",
            db.documentDao().getPendingInputsByOutpoint(pOutpoint).isEmpty(),
        )

        // P's funding TXO finally arrives.
        val restarted = newHandler()
        restarted.onChangesetBegin(walletId)
        restarted.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        restarted.onChangesetEnd(walletId, success = true)

        val coin = db.txoDao().getByOutpoint(pOutpoint)
        assertNotNull(coin)
        assertFalse(
            "the final sweep released this coin, so it must come back spendable " +
                "even though an earlier sweep in the chain had tombstoned it",
            coin!!.isSpent,
        )
    }

    @Test
    fun aReleasedCoinDropsItsDeadWinnersMarker() = runTest {
        // The funding-BEFORE-release ordering of the chained scenario above:
        // the funding TXO arrives between the sweep that held the coin and
        // the sweep that frees it, so the tombstone drains into
        // `TxoEntity.supersededByTxid` and the pending row is gone by the
        // time the release runs. The release must clear that column with
        // the hold: W has no stored row, so its stamp cannot veto, and a
        // released coin keeping its dead winner's marker would read as a
        // durable claim to every later hold on this outpoint.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 96 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 97 } // L
        val intermediateWinner = ByteArray(32) { 98 } // W — never recorded here
        val finalWinner = ByteArray(32) { 99 } // X

        // L spends the still-unfunded P.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_090,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // First sweep: W beats L, holding P.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(loserTxid), intermediateWinner, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        // P's funding TXO arrives NOW — the drain consumes the tombstone
        // and stamps the claim onto the row itself.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val stamped = db.txoDao().getByOutpoint(pOutpoint)!!
        assertTrue("sanity: the drained claim holds the coin", stamped.isSpent)
        assertTrue(intermediateWinner.contentEquals(stamped.supersededByTxid))

        // Second sweep: X beats W, and this time upstream frees P.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(intermediateWinner), finalWinner, listOf(pOutpoint), 400)
        handler.onChangesetEnd(walletId, success = true)

        val freed = db.txoDao().getByOutpoint(pOutpoint)!!
        assertFalse("the released coin is spendable again", freed.isSpent)
        assertNull(
            "and its dead winner's marker goes with the hold it carried",
            freed.supersededByTxid,
        )
        assertEquals(1, handler.onLoadWalletList().single().utxos.size)
    }

    @Test
    fun chainedSweepBeforeFundingRepointsAnEarlierTombstoneToTheNewWinner() = runTest {
        // The held (not released) half of the chained scenario above: the
        // second sweep keeps P spent instead of releasing it, and the
        // tombstone must end up attributed to the NEW winner rather than the
        // intermediate one that no longer has a row.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 81 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val firstLoserTxid = ByteArray(32) { 83 } // L
        val secondLoserTxid = ByteArray(32) { 84 } // W
        val finalWinnerTxid = ByteArray(32) { 85 } // X

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, firstLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_080,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // First sweep: W beats L, holding P.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(firstLoserTxid), secondLoserTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        // W's own record, needed by the second sweep below.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, secondLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_081,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Second sweep: X beats W, still holding the same input.
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(secondLoserTxid), finalWinnerTxid, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        val tombstone = db.documentDao().getPendingInputsByOutpoint(pOutpoint).single()
        assertTrue(tombstone.isSweptTombstone)
        assertTrue(
            "the tombstone must be repointed at the FINAL winner, not the " +
                "intermediate one the second sweep already removed",
            finalWinnerTxid.contentEquals(tombstone.spendingTxid),
        )

        val restarted = newHandler()
        restarted.onChangesetBegin(walletId)
        restarted.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        restarted.onChangesetEnd(walletId, success = true)

        val coin = db.txoDao().getByOutpoint(pOutpoint)
        assertNotNull(coin)
        assertTrue(
            "the final winner's claim must survive both sweeps and the " +
                "funding UTXO's own arrival",
            coin!!.isSpent,
        )
        assertTrue(finalWinnerTxid.contentEquals(coin.supersededByTxid))
    }

    @Test
    fun sharedWinnerDeletedByAnotherWalletsCallbackStillAppliesThisWalletsReleaseToItsOwnTombstones() = runTest {
        // Multi-wallet continuation of the chained-before-funding scenarios
        // above. The hold is global and the row goes with the FIRST
        // callback, so wallet B's callback for the shared winner W arrives
        // after W's row is gone: it must still apply B's own release by
        // outpoint to B's own tombstones (deleting a released one, never
        // leaving a freed tombstone), while the held tombstones — every
        // wallet's — were already re-pointed at X by A's callback.
        val walletB = ByteArray(32) { 9 }
        seedWalletWithAddress(walletId, "yWalletA", xpubFill = 30)
        seedWalletWithAddress(walletB, "yWalletB", xpubFill = 31)

        val fundingTxid = ByteArray(32) { 101 }
        val pA = makeOutpoint(fundingTxid, 0)
        val pB = makeOutpoint(fundingTxid, 1)
        val rB = makeOutpoint(fundingTxid, 2)
        val sharedLoser = ByteArray(32) { 103 } // L
        val sharedWinner = ByteArray(32) { 104 } // W
        val finalWinner = ByteArray(32) { 105 } // X

        // The shared loser L claims one still-unfunded coin of wallet A's
        // and two of wallet B's. Its record arrives through wallet A's
        // round; a pending row carries the wallet of the round that wrote
        // it, so wallet B's two claims are seeded directly in the exact
        // shape B's own round would have written them.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, sharedLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_090,
            pA + pB + rB, 3,
        )
        handler.onChangesetEnd(walletId, success = true)
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = pB, inputIndex = 1, spendingTxid = sharedLoser,
                spendingTransactionTxid = sharedLoser, walletId = walletB,
            ),
        )
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = rB, inputIndex = 2, spendingTxid = sharedLoser,
                spendingTransactionTxid = sharedLoser, walletId = walletB,
            ),
        )

        // First sweep: W beats L, holding everything (nothing funded,
        // nothing released). A's callback tombstones every wallet's claim
        // and deletes L; B's callback finds nothing left to do.
        sweepRound(walletId, listOf(sharedLoser), sharedWinner)
        assertNull("L is gone with the first callback", db.transactionDao().getByTxid(sharedLoser))
        sweepRound(walletB, listOf(sharedLoser), sharedWinner)
        for (outpoint in listOf(pA, pB, rB)) {
            val rows = db.documentDao().getPendingInputsByOutpoint(outpoint)
            assertTrue("every claim on ${outpoint.toHex()} is a tombstone held by W", rows.all { it.isSweptTombstone && sharedWinner.contentEquals(it.spendingTxid) })
        }

        // W's own record arrives through A's round, claiming all three
        // outpoints. A's `(pA, W)` tombstone occupies the duplicate-guard
        // key; B's tombstones are B's, so A stages its own ordinary claims
        // on pB and rB.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, sharedWinner, ByteArray(10) { 6 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_091,
            pA + pB + rB, 3,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Second sweep: X beats W. Wallet A's callback runs first, releasing
        // pA and rB (X took only pB) — its own claims on those are deleted,
        // B's claims on them are held until B speaks — and deletes W's row.
        sweepRound(walletId, listOf(sharedWinner), finalWinner, released = listOf(pA, rB))
        assertNull(
            "sanity: wallet A's callback deleted the shared winner row — the premise " +
                "wallet B's callback below has to survive",
            db.transactionDao().getByTxid(sharedWinner),
        )
        assertTrue("A's released claim on pA is gone", db.documentDao().getPendingInputsByOutpoint(pA).isEmpty())
        val heldForB = db.documentDao().getPendingInputsByOutpoint(rB).single()
        assertTrue("B's claim on rB is held by A's callback, re-pointed at X", heldForB.isSweptTombstone)
        assertTrue(walletB.contentEquals(heldForB.walletId))
        assertTrue(finalWinner.contentEquals(heldForB.spendingTxid))

        // Wallet B's callback arrives after the row is gone, releasing rB
        // and holding pB.
        sweepRound(walletB, listOf(sharedWinner), finalWinner, released = listOf(rB))

        val heldTombstones = db.documentDao().getPendingInputsByOutpoint(pB)
        assertTrue(heldTombstones.isNotEmpty())
        for (tombstone in heldTombstones) {
            assertTrue(tombstone.isSweptTombstone)
            assertTrue(
                "the held tombstones follow the chain to X even though W's row was " +
                    "already deleted by wallet A's callback",
                finalWinner.contentEquals(tombstone.spendingTxid),
            )
        }
        assertTrue(
            "wallet B's release reaches its tombstone even though W's row was " +
                "already deleted by wallet A's callback",
            db.documentDao().getPendingInputsByOutpoint(rB).isEmpty(),
        )

        // The funding TXOs finally arrive, one round per owning wallet.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yWalletA", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetUtxoAdded(
            walletB, fundingTxid, 1, 40_000, "yWalletB", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onWalletChangesetUtxoAdded(
            walletB, fundingTxid, 2, 20_000, "yWalletB", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletB, success = true)

        assertFalse("wallet A's released coin comes back spendable", db.txoDao().getByOutpoint(pA)!!.isSpent)
        val heldCoin = db.txoDao().getByOutpoint(pB)!!
        assertTrue("wallet B's held coin stays spent", heldCoin.isSpent)
        assertTrue(
            "the held coin must be attributed to the final winner, not the deleted W",
            finalWinner.contentEquals(heldCoin.supersededByTxid),
        )
        val releasedCoin = db.txoDao().getByOutpoint(rB)!!
        assertFalse(
            "wallet B's released coin must not resurrect spent under the obsolete winner",
            releasedCoin.isSpent,
        )
        assertNull(releasedCoin.supersededByTxid)
    }

    @Test
    fun anotherWalletsTombstoneStillHoldsACoinAtDrainWhenTheOwnerHasNone() = runTest {
        // The per-wallet half of the drain preference: the delivering
        // wallet's own tombstone is preferred, but when it has none, any
        // tombstone on the outpoint still holds — the stamp is a txid fact,
        // not a per-wallet one, and the owner's callback may simply never
        // have arrived. Wallet A recorded a loser spending B's still-unfunded
        // coin; only A's sweep callback ever ran.
        val walletB = ByteArray(32) { 9 }
        seedWalletWithAddress(walletId, "yWalletA", xpubFill = 30)
        seedWalletWithAddress(walletB, "yWalletB", xpubFill = 31)
        val fundingTxid = ByteArray(32) { 0x61 }
        val coinOfB = makeOutpoint(fundingTxid, 0)
        val loser = ByteArray(32) { 0x62 }
        val winner = ByteArray(32) { 0x63 }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, loser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_090,
            coinOfB, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        sweepRound(walletId, listOf(loser), winner)
        val tombstone = db.documentDao().getPendingInputsByOutpoint(coinOfB).single()
        assertTrue(tombstone.isSweptTombstone && walletId.contentEquals(tombstone.walletId))

        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetUtxoAdded(
            walletB, fundingTxid, 0, 40_000, "yWalletB", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletB, success = true)

        val coin = db.txoDao().getByOutpoint(coinOfB)!!
        assertTrue("A's tombstone holds B's coin at drain", coin.isSpent)
        assertTrue(winner.contentEquals(coin.supersededByTxid))
        assertTrue("and the drained rows are gone", db.documentDao().getPendingInputsByOutpoint(coinOfB).isEmpty())
    }

    // ── Outpoint-keyed holds, settled claims, round-scoped passes ─────

    /**
     * Wallet, address, and one funded coin at `fundingTxid:0` (in-block,
     * recorded + delivered in one round). Returns the coin's outpoint.
     */
    private suspend fun seedFundedCoin(fundingTxid: ByteArray, address: String = "yUtxoAddr"): ByteArray {
        seedWalletWithAddress(walletId, address)
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, address, ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        return makeOutpoint(fundingTxid, 0)
    }

    /** One committed round recording a mempool spend of [inputs] by [txid]. */
    private fun recordMempoolSpend(txid: ByteArray, vararg inputs: ByteArray, context: Int = 0, h: PlatformWalletPersistenceHandler = handler) {
        h.onChangesetBegin(walletId)
        recordTransaction(
            h,
            walletId, txid, ByteArray(10) { 5 }, context, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
            inputs.fold(ByteArray(0)) { acc, op -> acc + op }, inputs.size,
        )
        h.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun aWinnerRecordedInTheSameRoundDoesNotHideTheLosersInputFromTheHold() = runTest {
        // The hold is keyed by OUTPOINT, decoded from the loser's stored
        // bytes, not by which rows still link to the loser. Own coin O is
        // linked to mempool loser L. One round carries the winner's record
        // (IS-locked, spends O) and the sweep of L. Records precede sweeps,
        // so W takes the link first — at `isSpent = 0`, since only a block
        // flips the flag on the record channel — and a hold keyed by
        // `spendingTxid = L` then finds nothing: after a restart the store
        // hands O back as spendable while the winner sits unmined. With the
        // hold keyed by L's decoded inputs, O is stamped whatever it links
        // to, and the link to W is kept.
        val fundingTxid = ByteArray(32) { 0x30 }
        val coin = seedFundedCoin(fundingTxid)
        val loser = ByteArray(32) { 0x31 }
        val winner = ByteArray(32) { 0x32 }
        recordMempoolSpend(loser, coin)
        assertTrue(loser.contentEquals(db.txoDao().getByOutpoint(coin)!!.spendingTxid))

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, winner, ByteArray(10) { 6 }, 1, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_060,
            coin, 1,
        )
        sweep(handler, walletId, listOf(loser), winner, emptyList(), -1)
        handler.onChangesetEnd(walletId, success = true)

        val held = db.txoDao().getByOutpoint(coin)!!
        assertTrue("the coin the winner took is held although its link moved before the sweep", held.isSpent)
        assertTrue(winner.contentEquals(held.supersededByTxid))
        assertTrue("the link to the winner is kept — only a link to the loser is detached", winner.contentEquals(held.spendingTxid))
        assertNull(db.transactionDao().getByTxid(loser))
        assertTrue("and it stays out of the restore set", newHandler().onLoadWalletList().single().utxos.isEmpty())
    }

    @Test
    fun aLoserWithNoStoredBytesStillHoldsTheCoinsLinkedToIt() = runTest {
        // The record-lost fallback: a loser whose row carries no bytes (a
        // stub `utxos_added` wrote, or a record whose data never arrived)
        // cannot name its inputs, so the rows still linked to it and the
        // pending rows still claimed by it are the input set. The coin is
        // held all the same.
        val fundingTxid = ByteArray(32) { 0x33 }
        val coin = seedFundedCoin(fundingTxid)
        val loser = ByteArray(32) { 0x34 }
        val winner = ByteArray(32) { 0x35 }
        db.transactionDao().upsert(TransactionEntity(txid = loser, transactionData = ByteArray(0)))
        db.txoDao().upsert(db.txoDao().getByOutpoint(coin)!!.copy(spendingTxid = loser, spendingInputIndex = 0))

        sweepRound(walletId, listOf(loser), winner)

        val held = db.txoDao().getByOutpoint(coin)!!
        assertTrue(held.isSpent)
        assertNull(held.spendingTxid)
        assertTrue(winner.contentEquals(held.supersededByTxid))
        assertNull(db.transactionDao().getByTxid(loser))
    }

    @Test
    fun aLoserWhoseStoredBytesCannotBeDecodedFailsTheRoundClosed() = runTest {
        // A stored record the decoder rejects fails the round rather than
        // sweeping a loser whose inputs are unknown: the typed key named the
        // row a swept loser, and processing it blind could free the wrong
        // coins. Same verdict as the SQLite store's `apply_sweep` on a bad
        // blob. The round rolls back, so nothing — not even the delete —
        // lands.
        val fundingTxid = ByteArray(32) { 0x36 }
        val coin = seedFundedCoin(fundingTxid)
        val loser = ByteArray(32) { 0x37 }
        val winner = ByteArray(32) { 0x38 }
        recordMempoolSpend(loser, coin)
        recordedInputs.remove(loser.toHex())

        handler.onChangesetBegin(walletId)
        assertEquals(0, sweep(handler, walletId, listOf(loser), winner, emptyList(), 400))
        assertEquals("the round is refused", 1, handler.onChangesetEnd(walletId, success = true))

        assertNotNull("nothing landed: the loser's row survives", db.transactionDao().getByTxid(loser))
        val untouched = db.txoDao().getByOutpoint(coin)!!
        assertFalse(untouched.isSpent)
        assertTrue(loser.contentEquals(untouched.spendingTxid))
    }

    @Test
    fun aReleaseOfACoinItsStoredFinalWinnerSpendsIsRefusedByTheStamp() = runTest {
        // The settled-claim veto by STAMP, on a row with no settled link to
        // veto through. W (IS-locked, stored, spends O) was recorded before
        // O's funding arrived, so its claim was a pending row; the sweep of
        // L tombstoned L's claim to W, and O's arrival drained the
        // tombstone into a stamp — unlinked, because a drain never mints a
        // link, and W's own ordinary claim went with the drain. A later
        // conflicting mempool L2 adopts the link; L3 IS-locks L2's other
        // input and sweeps L2 with O in its released set — upstream's live
        // view has no record claiming O. The stored W is a network-final
        // claim on O, so the release is refused; without the stamp veto the
        // hold pass would detach L2 and the release would flip a provably
        // consumed coin unspent.
        seedWalletWithAddress(walletId, "yUtxoAddr")
        val fundingTxid = ByteArray(32) { 0x39 }
        val coin = makeOutpoint(fundingTxid, 0)
        val other = makeOutpoint(ByteArray(32) { 0x3A }, 0)
        val loser = ByteArray(32) { 0x3B }
        val winner = ByteArray(32) { 0x3C }
        val laterLoser = ByteArray(32) { 0x3D }
        val finalWinner = ByteArray(32) { 0x3E }
        recordMempoolSpend(loser, coin)
        recordMempoolSpend(winner, coin, context = 1)
        sweepRound(walletId, listOf(loser), winner, winnerMinedHeight = -1)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        val held = db.txoDao().getByOutpoint(coin)!!
        assertTrue("sanity: held by the stamp, unlinked", held.isSpent && held.spendingTxid == null)
        assertTrue(winner.contentEquals(held.supersededByTxid))

        // A conflicting mempool spend adopts the link; the hold is the stamp.
        recordMempoolSpend(laterLoser, coin, other)
        val adopted = db.txoDao().getByOutpoint(coin)!!
        assertTrue(laterLoser.contentEquals(adopted.spendingTxid))
        assertTrue("adoption keeps the flag and the stamp", adopted.isSpent)
        assertTrue(winner.contentEquals(adopted.supersededByTxid))

        sweepRound(walletId, listOf(laterLoser), finalWinner, released = listOf(coin), winnerMinedHeight = -1)

        val stillHeld = db.txoDao().getByOutpoint(coin)!!
        assertTrue("a release of a coin a stored final winner spends is refused", stillHeld.isSpent)
        assertTrue(winner.contentEquals(stillHeld.supersededByTxid))
        assertNull("the dead link is detached all the same", stillHeld.spendingTxid)
        assertTrue(newHandler().onLoadWalletList().single().utxos.isEmpty())
    }

    @Test
    fun aStampNamingAFinalTransactionThatDoesNotSpendTheCoinDoesNotVeto() = runTest {
        // The stamp alone is not proof the winner took the coin: a hold
        // stamps the winner on EVERY non-released input of a loser, and an
        // input can be unreleased because a different surviving record
        // claims it. So the veto reads the stamped winner's stored bytes —
        // as the SQLite store's claim scan reads every claimant's inputs —
        // and vetoes only when they spend the coin. Here W (IS-locked,
        // stored) spends only P; O was held under W's stamp because own
        // record R also claimed it; when R is swept with O released, W's
        // stamp must not strand O.
        val fundingTxid = ByteArray(32) { 0x40 }
        val coin = seedFundedCoin(fundingTxid)
        val p = makeOutpoint(ByteArray(32) { 0x41 }, 0)
        val loser = ByteArray(32) { 0x42 }
        val rival = ByteArray(32) { 0x43 }
        val winner = ByteArray(32) { 0x44 }
        val laterWinner = ByteArray(32) { 0x45 }
        recordMempoolSpend(loser, coin, p)
        recordMempoolSpend(rival, coin)
        recordMempoolSpend(winner, p, context = 1)
        // W beats L on P; O is not released because R still claims it.
        sweepRound(walletId, listOf(loser), winner, winnerMinedHeight = -1)
        val held = db.txoDao().getByOutpoint(coin)!!
        assertTrue("sanity: held under W's stamp, linked to R", held.isSpent)
        assertTrue(winner.contentEquals(held.supersededByTxid))
        assertTrue(rival.contentEquals(held.spendingTxid))

        // R is beaten in turn and O comes free.
        sweepRound(walletId, listOf(rival), laterWinner, released = listOf(coin), winnerMinedHeight = -1)

        val freed = db.txoDao().getByOutpoint(coin)!!
        assertFalse("W never spent O, so its stamp does not veto the release", freed.isSpent)
        assertNull(freed.supersededByTxid)
        assertNull(freed.spendingTxid)
    }

    @Test
    fun aConflictingMempoolSpentEmitDoesNotLowerAHealedSpendFlag() = runTest {
        // `isSpent` is monotonic on the `utxos_spent` channel. O is linked
        // to asset-lock funding tx F stuck at mempool context and was
        // healed to `isSpent = 1` (the SPV-miss case). A conflicting mempool
        // spend N arrives via `utxos_spent`: F is not settled, so N takes
        // the link — but the flag must not be re-answered from N's context.
        // Before the fix it was, O re-entered the restore set, and the
        // asset-lock heal was no longer consulted because the link was N's.
        val fundingTxid = ByteArray(32) { 0x46 }
        val coin = seedFundedCoin(fundingTxid)
        val lockTx = ByteArray(32) { 0x47 }
        val conflicting = ByteArray(32) { 0x48 }
        recordMempoolSpend(lockTx, coin)
        db.txoDao().markSpentBySpendingTxid(lockTx, java.util.Date())
        assertTrue("sanity: healed", db.txoDao().getByOutpoint(coin)!!.isSpent)
        db.transactionDao().upsert(TransactionEntity(txid = conflicting, transactionData = ByteArray(10) { 9 }))

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, conflicting)
        handler.onChangesetEnd(walletId, success = true)

        val row = db.txoDao().getByOutpoint(coin)!!
        assertTrue("a mempool usurper never lowers the flag", row.isSpent)
        assertTrue("though it takes the link from a mempool spender", conflicting.contentEquals(row.spendingTxid))
    }

    @Test
    fun aTombstoneWhoseFundingArrivesInTheFinalizingRoundDrainsBeforeTheCollector() = runTest {
        // The collector runs once per round, at the END — after every
        // account slice and every sweep. A tombstone T (O → W, mined 400)
        // survives from an earlier round; the chainlock already covers 400.
        // A later round folds a backward rescan delivering O together with
        // the synced height that completes the boundary. Collecting at the
        // header would delete T before the drain could move its hold onto
        // O, and O would land unspent although W provably consumed it.
        seedWalletWithAddress(walletId, "yFundAddr")
        chainLockHeightRound(handler, 10_000)
        val fundingTxid = ByteArray(32) { 0x49 }
        val coin = makeOutpoint(fundingTxid, 0)
        val loser = ByteArray(32) { 0x4A }
        val winner = ByteArray(32) { 0x4B }
        seedSweptTombstone(coin, loser, winner, winnerMinedHeight = 400)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetHeader(
            walletId = walletId, hasSyncedHeight = true, syncedHeight = 400, hasBalance = false,
            confirmedDelta = 0, unconfirmedDelta = 0, immatureDelta = 0, lockedDelta = 0,
            lastAppliedChainLockBytes = ByteArray(84) { 9 },
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val drained = db.txoDao().getByOutpoint(coin)!!
        assertTrue("the funding delivery drained the tombstone before anything collected it", drained.isSpent)
        assertTrue(winner.contentEquals(drained.supersededByTxid))
        assertTrue(db.documentDao().getPendingInputsByOutpoint(coin).isEmpty())
    }

    @Test
    fun theCoSweptSetSpansEveryBatchOfTheRound() = runTest {
        // One round carries two batches: {P by W1} then {C by W2}, where
        // child C's pending row names P:0. Evaluated per batch, the second
        // batch does not know P is swept and tombstones the claim to W2 —
        // a hold on a dead parent's output that wedges P's chainlocked
        // reinstatement. Evaluated against the union of the round's txids,
        // the claim is deleted.
        seedWalletWithAddress(walletId, "yFundAddr")
        val parent = ByteArray(32) { 0x4C }
        val child = ByteArray(32) { 0x4D }
        val w1 = ByteArray(32) { 0x4E }
        val w2 = ByteArray(32) { 0x4F }
        val parentOutput = makeOutpoint(parent, 0)
        recordMempoolSpend(parent, makeOutpoint(ByteArray(32) { 0x50 }, 0))
        recordMempoolSpend(child, parentOutput)
        assertEquals(1, db.documentDao().getPendingInputsByOutpoint(parentOutput).size)

        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(parent), w1, emptyList(), 400)
        sweep(handler, walletId, listOf(child), w2, emptyList(), 400)
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(
            "the child's claim on the co-swept parent's output is deleted, not tombstoned",
            db.documentDao().getPendingInputsByOutpoint(parentOutput).isEmpty(),
        )
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, parent, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertFalse("the reinstated parent's output lands spendable", db.txoDao().getByOutpoint(parentOutput)!!.isSpent)
    }

    @Test
    fun aReleaseNamingAnOutputOfACoSweptParentDeletesItRatherThanFreeingIt() = runTest {
        // A released outpoint whose funding transaction is swept in this
        // round is deleted whatever its shape: a coin created by a dead
        // transaction cannot be unspent, only gone. P's output materialised
        // (P's own record never did — a stub row carries it); C spends it;
        // the round sweeps both and a release names P:0.
        seedWalletWithAddress(walletId, "yFundAddr")
        val parent = ByteArray(32) { 0x51 }
        val child = ByteArray(32) { 0x52 }
        val winner = ByteArray(32) { 0x53 }
        val parentOutput = makeOutpoint(parent, 0)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, parent, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            0, false, false, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        recordMempoolSpend(child, parentOutput)
        assertTrue(child.contentEquals(db.txoDao().getByOutpoint(parentOutput)!!.spendingTxid))

        sweepRound(walletId, listOf(parent, child), winner, released = listOf(parentOutput))

        assertNull("a dead parent's output is deleted, never freed", db.txoDao().getByOutpoint(parentOutput))
        assertNull(db.transactionDao().getByTxid(parent))
        assertNull(db.transactionDao().getByTxid(child))
    }

    @Test
    fun aDrainedTombstoneStampsWithoutLinkingSoALaterReleaseCanFreeTheCoin() = runTest {
        // A drained tombstone STAMPS, it never mints a spender link — even
        // when the winner's own row exists. An input can be unreleased
        // because another live record claims it, not because the winner
        // took it; a link to W would make the coin non-releasable when that
        // record is swept in turn with O released. Own L (spends O + P) and
        // own R (spends O + Q); W (spends P only) sweeps L; O is not
        // released (R claims it). O's funding arrives: the tombstone drains
        // into a stamp, unlinked. W2 sweeps R with O released: O comes free.
        seedWalletWithAddress(walletId, "yFundAddr")
        val fundingTxid = ByteArray(32) { 0x54 }
        val coin = makeOutpoint(fundingTxid, 0)
        val p = makeOutpoint(ByteArray(32) { 0x55 }, 0)
        val q = makeOutpoint(ByteArray(32) { 0x56 }, 0)
        val loser = ByteArray(32) { 0x57 }
        val rival = ByteArray(32) { 0x58 }
        val winner = ByteArray(32) { 0x59 }
        val laterWinner = ByteArray(32) { 0x5A }
        recordMempoolSpend(loser, coin, p)
        recordMempoolSpend(rival, coin, q)
        recordMempoolSpend(winner, p, context = 1)
        sweepRound(walletId, listOf(loser), winner, winnerMinedHeight = -1)
        assertTrue(db.documentDao().getPendingInputsByOutpoint(coin).any { it.isSweptTombstone })

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)
        val drained = db.txoDao().getByOutpoint(coin)!!
        assertTrue(drained.isSpent)
        assertTrue(winner.contentEquals(drained.supersededByTxid))
        assertNull("the drain stamps; it does not link the winner", drained.spendingTxid)
        assertNull(drained.spendingInputIndex)

        sweepRound(walletId, listOf(rival), laterWinner, released = listOf(coin), winnerMinedHeight = -1)
        val freed = db.txoDao().getByOutpoint(coin)!!
        assertFalse("a stamped, unlinked coin is exactly what a release can free", freed.isSpent)
        assertNull(freed.supersededByTxid)
    }

    @Test
    fun aSecondWalletRecordingTheSameSpendGetsItsOwnPendingRow() = runTest {
        // Pending rows are per (outpoint, spendingTxid, walletId). Sweep
        // holds and releases are decided per wallet, so a second wallet
        // recording the same spend of a not-yet-materialised coin must get
        // its own row — with one shared row, the first wallet's release or
        // collector could erase the only hold the second was entitled to
        // keep.
        val walletB = ByteArray(32) { 9 }
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        handler.onPersistWalletMetadata(walletB, testnet, groupId, 0)
        val coin = makeOutpoint(ByteArray(32) { 0x5B }, 0)
        val spender = ByteArray(32) { 0x5C }
        for (wallet in listOf(walletId, walletB)) {
            handler.onChangesetBegin(wallet)
            recordTransaction(
                handler,
                wallet, spender, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
                0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
                coin, 1,
            )
            handler.onChangesetEnd(wallet, success = true)
        }
        val rows = db.documentDao().getPendingInputsByOutpoint(coin)
        assertEquals(2, rows.size)
        assertEquals(
            setOf(walletId.toHex(), walletB.toHex()),
            rows.map { it.walletId.toHex() }.toSet(),
        )
        // And a re-emit for the same wallet is still deduplicated.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, spender, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
            coin, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertEquals(2, db.documentDao().getPendingInputsByOutpoint(coin).size)
    }

    @Test
    fun aRefusedClaimDoesNotEraseAnotherWalletsTombstoneOnTheOutpoint() = runTest {
        // The found-TXO branch of the record channel prunes pending rows on
        // the outpoint. When the arriving record's claim is REFUSED (a
        // settled spender keeps the link), only that record's own rows are
        // stale; another wallet's tombstone on the outpoint is that
        // wallet's hold and not this record's to erase. When the claim is
        // accepted, only this wallet's ordinary rows go.
        val walletB = ByteArray(32) { 9 }
        val fundingTxid = ByteArray(32) { 0x5D }
        val coin = seedFundedCoin(fundingTxid)
        handler.onPersistWalletMetadata(walletB, testnet, groupId, 0)
        val settled = ByteArray(32) { 0x5E }
        val usurper = ByteArray(32) { 0x5F }
        val someWinner = ByteArray(32) { 0x60 }
        recordMempoolSpend(settled, coin, context = 1)
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = coin, inputIndex = 0, spendingTxid = someWinner,
                spendingTransactionTxid = null, walletId = walletB, isSweptTombstone = true,
            ),
        )

        recordMempoolSpend(usurper, coin)

        val row = db.txoDao().getByOutpoint(coin)!!
        assertTrue("sanity: the settled spender kept its link", settled.contentEquals(row.spendingTxid))
        val survivor = db.documentDao().getPendingInputsByOutpoint(coin).single()
        assertTrue("wallet B's tombstone survives a refused claim", survivor.isSweptTombstone)
        assertTrue(walletB.contentEquals(survivor.walletId))
    }

    @Test
    fun aBatchSweepingMoreLosersThanSqliteCanBindStillCommits() = runTest {
        // The loser side of the arity discipline: every per-batch statement
        // is a chunked `IN (:chunk)` form, so a batch of more losers than
        // SQLite can bind in one statement still commits. The count is past
        // the host's own ceiling (32766) for the same reason
        // aSweepReleasingMoreOutpointsThanSqliteCanBindStillCommits gives:
        // what is pinned is that the arity does not grow with the batch.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val count = 33_000
        val losers = (0 until count).map { i ->
            ByteArray(32).also { it[0] = (i and 0xFF).toByte(); it[1] = (i shr 8).toByte(); it[2] = 0x7E }
        }
        // Stub rows (no bytes — nothing to decode), seeded in one SQL
        // transaction; every one is a loser this batch names.
        val raw = db.openHelper.writableDatabase
        raw.beginTransaction()
        try {
            val insert = raw.compileStatement(
                "INSERT INTO transactions (txid, transactionData, context, blockHeight, " +
                    "blockTimestamp, blockPosition, hasBlockPosition, direction, transactionType, " +
                    "transactionTypeKind, netAmount, label, firstSeen, createdAt, lastUpdated) " +
                    "VALUES (?, x'', 0, 0, 0, 0, 0, 0, 'Standard', 0, 0, '', 0, 0, 0)",
            )
            for (loser in losers) {
                insert.bindBlob(1, loser)
                insert.executeInsert()
            }
            raw.setTransactionSuccessful()
        } finally {
            raw.endTransaction()
        }
        assertEquals(count.toLong(), db.transactionDao().count().first())
        val winner = ByteArray(32) { 0x7C }

        sweepRound(walletId, losers, winner)

        assertEquals("every loser's row is gone", 0L, db.transactionDao().count().first())
    }

    @Test
    fun aSweepBatchWhosePackedLengthDisagreesWithItsCountFailsTheRound() = runTest {
        // The trampoline ships txids and released outpoints as flat arrays
        // plus counts; a descriptor or packing drift must fail the round,
        // never silently truncate a sweep.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        handler.onChangesetBegin(walletId)
        val code = handler.onWalletChangesetTransactionsSwept(
            walletId, ByteArray(31), 1, ByteArray(32) { 3 }, ByteArray(0), 0, true, 400,
        )
        assertTrue("a malformed batch is refused at the callback", code != 0)
        handler.onChangesetEnd(walletId, success = false)
    }

    @Test
    fun loadWalletListRestoresCoreAddressPoolsBeyondGapWindow() = runTest {
        // prior-2 regression: the persisted Core address pools must come
        // back on the restore row so every restored address maps to its
        // derivation path — including addresses PAST the gap-limit window
        // (`DEFAULT_GAP_LIMIT` = 20) that `ManagedWalletInfo::from_wallet`
        // pre-derives. Without this, a restored UTXO on an out-of-window
        // address has no derivation-path mapping and the wallet cannot
        // sign a core-to-core spend after a cold restart. Mirror of the
        // Swift `buildCoreAddressPoolBuffer` round-trip.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val account = db.accountDao().observeByWallet(walletId).first().single()

        // An external (pool tag 0) address well beyond the gap window,
        // used and carrying a balance + a full derivation path + pubkey.
        val pubkey = ByteArray(33) { 4 }
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = "yFarAddr",
                publicKey = pubkey,
                poolTypeTag = 0,
                addressIndex = 100,
                derivationPath = "m/44'/1'/0'/0/100",
                isUsed = true,
                balance = 12_345,
                accountId = account.id,
            ),
        )
        // A second, unused internal (pool tag 1) address — proves grouping
        // by pool type emits a distinct pool for the change chain.
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = "yChangeAddr",
                publicKey = ByteArray(0),
                poolTypeTag = 1,
                addressIndex = 3,
                derivationPath = "m/44'/1'/0'/1/3",
                isUsed = false,
                accountId = account.id,
            ),
        )

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        val pools = list[0].coreAddressPools
        // One pool per (account, poolType) group, ascending tag order.
        assertEquals(2, pools.size)

        val external = pools[0]
        assertEquals(0.toByte(), external.poolTypeTag)
        // The pool routes via the account tuple (xpub omitted — the loader
        // ignores it on this path).
        assertEquals(0.toByte(), external.account.typeTag)
        assertEquals(0, external.account.index)
        assertEquals(0, external.account.accountXpubBytes.size)
        assertEquals(1, external.addresses.size)
        val far = external.addresses[0]
        assertEquals("yFarAddr", far.addressBase58)
        // The out-of-window address keeps its derivation path — the whole
        // point of the fix.
        assertEquals("m/44'/1'/0'/0/100", far.derivationPath)
        assertEquals(100, far.addressIndex)
        assertTrue(far.isUsed)
        assertEquals(12_345L, far.balance)
        assertTrue(pubkey.contentEquals(far.publicKey))
        assertEquals(0.toByte(), far.poolTypeTag)

        val internal = pools[1]
        assertEquals(1.toByte(), internal.poolTypeTag)
        assertEquals(1, internal.addresses.size)
        val change = internal.addresses[0]
        assertEquals("yChangeAddr", change.addressBase58)
        assertEquals("m/44'/1'/0'/1/3", change.derivationPath)
        assertEquals(3, change.addressIndex)
        assertFalse(change.isUsed)
        // No pubkey persisted → empty (Rust derives has_public_key = false).
        assertEquals(0, change.publicKey.size)
    }

    @Test
    fun loadWalletListRoundTripsIdentityKeysWithContractBounds() = runTest {
        // Signing-critical restore path: a cold-started wallet must get its
        // identities and public keys back exactly as persisted — keyId,
        // repr(u8) discriminants, key bytes, and the (kind, id, docType)
        // contract-bounds triple (kind 2 = SingleContractDocumentType).
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val identityId = ByteArray(32) { 12 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 7 }
        val boundsId = ByteArray(32) { 21 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityKeyUpsert(
            walletId = walletId,
            identityId = identityId,
            keyId = 4,
            purpose = 1,
            securityLevel = 2,
            keyType = 0,
            readOnly = true,
            disabledAtIsSome = false,
            disabledAt = 0,
            publicKeyData = pubkey,
            publicKeyHash = ByteArray(20),
            walletIdIsSome = true,
            keyWalletId = walletId,
            derivationIndicesIsSome = false,
            identityIndex = 0,
            keyIndex = 0,
            contractBoundsKind = 2,
            contractBoundsId = boundsId,
            contractBoundsDocumentType = "contactRequest",
        )
        handler.onChangesetEnd(walletId, success = true)

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        assertEquals(1, list[0].identities.size)
        val identity = list[0].identities[0]
        assertTrue(identityId.contentEquals(identity.identityId))
        assertEquals(1, identity.keys.size)
        val key = identity.keys[0]
        assertEquals(4, key.keyId)
        assertEquals(0.toByte(), key.keyType)
        assertEquals(1.toByte(), key.purpose)
        assertEquals(2.toByte(), key.securityLevel)
        assertTrue(key.readOnly)
        assertTrue(pubkey.contentEquals(key.data))
        assertEquals(2.toByte(), key.contractBoundsKind)
        assertTrue(boundsId.contentEquals(key.contractBoundsId))
        assertEquals("contactRequest", key.contractBoundsDocumentType)
    }

    // ── DashPay contacts: upsert metadata, ignore delta, restore ──────

    /** Persist one incoming contact row for [senderId] owned by [ownerId]. */
    private suspend fun persistIncomingContact(ownerId: ByteArray, senderId: ByteArray) {
        handler.onChangesetBegin(walletId)
        handler.onPersistContactUpsert(
            walletId = walletId,
            ownerId = ownerId,
            contactId = senderId,
            isOutgoing = false,
            senderKeyIndex = 2,
            recipientKeyIndex = 3,
            accountReference = 4,
            encryptedPublicKey = ByteArray(96) { 5 },
            encryptedAccountLabel = ByteArray(3) { 6 },
            autoAcceptProof = null,
            coreHeightCreatedAt = 100_000,
            createdAt = 1_700_000_000_000,
            paymentChannelBroken = true,
            alias = "ally",
            note = "a note",
            isHidden = true,
            contactAccountLabel = "Main wallet",
            acceptedAccounts = intArrayOf(0, 7),
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun contactUpsertPersistsEstablishedRowMetadata() = runTest {
        // The contactInfo / DIP-15 metadata block added by upstream #3841
        // must land in Room — dropping it here would wipe alias/note/hidden
        // (and the broken-channel flag) on Android relative to Swift.
        val ownerId = ByteArray(32) { 15 }
        val senderId = ByteArray(32) { 16 }
        seedIdentity(ownerId)
        persistIncomingContact(ownerId, senderId)

        val rows = db.dashpayDao().getContactRequestsByOwner(ownerId)
        assertEquals(1, rows.size)
        val row = rows[0]
        assertTrue(row.paymentChannelBroken)
        assertEquals("ally", row.contactAlias)
        assertEquals("a note", row.contactNote)
        assertTrue(row.contactHidden)
        assertEquals("Main wallet", row.contactAccountLabel)
        assertTrue(
            intArrayOf(0, 7).contentEquals(decodeAcceptedAccounts(row.contactAcceptedAccounts)),
        )
    }

    @Test
    fun contactIgnoreDeltaDropsIncomingRowAndRoundTripsIgnoredSender() = runTest {
        // Ignore (isIgnored=true): the sender's incoming row goes and a
        // durable ignored-sender row appears; un-ignore deletes it again.
        val ownerId = ByteArray(32) { 17 }
        val senderId = ByteArray(32) { 18 }
        seedIdentity(ownerId)
        persistIncomingContact(ownerId, senderId)

        handler.onChangesetBegin(walletId)
        assertEquals(0, handler.onPersistContactIgnored(walletId, ownerId, senderId, true))
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(db.dashpayDao().getContactRequestsByOwner(ownerId).isEmpty())
        val ignored = db.dashpayDao().getIgnoredSendersByOwner(ownerId)
        assertEquals(1, ignored.size)
        assertTrue(senderId.contentEquals(ignored[0].ignoredSenderId))
        assertEquals(testnet, ignored[0].networkRaw)

        handler.onChangesetBegin(walletId)
        assertEquals(0, handler.onPersistContactIgnored(walletId, ownerId, senderId, false))
        handler.onChangesetEnd(walletId, success = true)
        assertTrue(db.dashpayDao().getIgnoredSendersByOwner(ownerId).isEmpty())
    }

    @Test
    fun loadWalletListRoundTripsContactsAndIgnoredSenders() = runTest {
        // Relaunch-durability: the restore rows must carry the contact
        // (with its metadata) and the ignored-sender id back to Rust, or
        // contact metadata is wiped and ignored senders resurface after
        // every cold start.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val ownerId = ByteArray(32) { 19 }
        val contactId = ByteArray(32) { 20 }
        val mutedId = ByteArray(32) { 21 }
        seedIdentity(ownerId)
        persistIncomingContact(ownerId, contactId)
        handler.onChangesetBegin(walletId)
        handler.onPersistContactIgnored(walletId, ownerId, mutedId, true)
        handler.onChangesetEnd(walletId, success = true)

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        assertEquals(1, list[0].identities.size)
        val identity = list[0].identities[0]

        assertEquals(1, identity.contacts.size)
        val contact = identity.contacts[0]
        assertTrue(ownerId.contentEquals(contact.ownerIdentityId))
        assertTrue(contactId.contentEquals(contact.contactIdentityId))
        assertFalse(contact.isOutgoing)
        assertEquals(2, contact.senderKeyIndex)
        assertEquals(3, contact.recipientKeyIndex)
        assertEquals(4, contact.accountReference)
        assertTrue(ByteArray(96) { 5 }.contentEquals(contact.encryptedPublicKey))
        assertTrue(ByteArray(3) { 6 }.contentEquals(contact.encryptedAccountLabel!!))
        assertNull(contact.autoAcceptProof)
        assertEquals(100_000, contact.coreHeightCreatedAt)
        assertEquals(1_700_000_000_000, contact.createdAtMillis)
        assertTrue(contact.paymentChannelBroken)
        assertEquals("ally", contact.alias)
        assertEquals("a note", contact.note)
        assertTrue(contact.isHidden)
        assertEquals("Main wallet", contact.contactAccountLabel)
        assertTrue(intArrayOf(0, 7).contentEquals(contact.acceptedAccounts))

        assertEquals(1, identity.ignoredSenders.size)
        assertTrue(mutedId.contentEquals(identity.ignoredSenders[0]))
    }

    @Test
    fun loadWalletListScopesToTheHandlerNetwork() = runTest {
        // A network-scoped handler must never hand the Rust loader a
        // foreign-network row — the loader inserts unconditionally, and a
        // single cross-network row aborts the whole transactional load.
        val scoped = PlatformWalletPersistenceHandler(
            db,
            Dispatchers.Unconfined,
            network = org.dashfoundation.dashsdk.Network.TESTNET,
        )
        val xpub = ByteArray(78) { 30 }
        scoped.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        scoped.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val mainnetWallet = ByteArray(32) { 9 }
        val mainnet = org.dashfoundation.dashsdk.Network.MAINNET.ffiValue
        scoped.onPersistWalletMetadata(mainnetWallet, mainnet, groupId, 0)
        scoped.onPersistAccountRegistration(
            mainnetWallet, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )

        val list = scoped.onLoadWalletList()
        assertEquals(1, list.size)
        assertTrue(walletId.contentEquals(list[0].walletId))
    }

    // ── DashPay contact profiles: delta upsert / tombstone, restore ───

    /** Persist one present contact-profile delta for [contactId] owned by [ownerId]. */
    private suspend fun persistContactProfile(ownerId: ByteArray, contactId: ByteArray) {
        handler.onChangesetBegin(walletId)
        handler.onPersistContactProfileDelta(
            walletId = walletId,
            ownerId = ownerId,
            contactId = contactId,
            isPresent = true,
            displayName = "Bob",
            bio = null,
            avatarUrl = "https://x/bob.png",
            avatarHash = ByteArray(32) { 23 },
            avatarHashPresent = true,
            avatarFingerprint = ByteArray(8),
            avatarFingerprintPresent = false,
            publicMessage = "yo",
            checkedAtMs = 1_700_000_111_000,
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun contactProfileDeltaUpsertsPresentRow() = runTest {
        // A present `IdentityEntryFFI.contact_profiles` row must land as a
        // cached contact-profile row — with the avatar byte fields gated on
        // their `_present` flags (all-zero is a valid hash value, so
        // nullability must come from the flag, not the bytes).
        val ownerId = ByteArray(32) { 22 }
        val contactId = ByteArray(32) { 33 }
        seedIdentity(ownerId)
        persistContactProfile(ownerId, contactId)

        val rows = db.dashpayDao().getContactProfilesByOwner(ownerId)
        assertEquals(1, rows.size)
        val row = rows[0]
        assertEquals(testnet, row.networkRaw)
        assertTrue(contactId.contentEquals(row.contactIdentityId))
        assertEquals("Bob", row.displayName)
        assertNull(row.bio)
        assertEquals("https://x/bob.png", row.avatarUrl)
        assertTrue(ByteArray(32) { 23 }.contentEquals(row.avatarHash!!))
        assertNull(row.avatarFingerprint)
        assertEquals("yo", row.publicMessage)
        assertEquals(1_700_000_111_000, row.checkedAtMs)
    }

    @Test
    fun contactProfileTombstoneDeletesTheRow() = runTest {
        // An `is_present == false` delta means the contact removed their
        // on-chain profile: the persisted row must be DELETED. An
        // upsert-only pipeline would show the stale name/avatar forever.
        val ownerId = ByteArray(32) { 24 }
        val contactId = ByteArray(32) { 25 }
        seedIdentity(ownerId)
        persistContactProfile(ownerId, contactId)
        assertEquals(1, db.dashpayDao().getContactProfilesByOwner(ownerId).size)

        handler.onChangesetBegin(walletId)
        val code = handler.onPersistContactProfileDelta(
            walletId = walletId,
            ownerId = ownerId,
            contactId = contactId,
            isPresent = false,
            displayName = null,
            bio = null,
            avatarUrl = null,
            avatarHash = ByteArray(32),
            avatarHashPresent = false,
            avatarFingerprint = ByteArray(8),
            avatarFingerprintPresent = false,
            publicMessage = null,
            checkedAtMs = 1_700_000_222_000,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertEquals(0, code)

        assertTrue(db.dashpayDao().getContactProfilesByOwner(ownerId).isEmpty())
    }

    @Test
    fun contactProfileDeltaIsDiscardedOnChangesetRollback() = runTest {
        // The delta must ride the stage() buffer like every other
        // changeset write: a rolled-back round leaves no row (an eager
        // write here would survive a failed Rust-side round and desync
        // the mirror from the authoritative state).
        val ownerId = ByteArray(32) { 28 }
        val contactId = ByteArray(32) { 29 }
        seedIdentity(ownerId)

        handler.onChangesetBegin(walletId)
        handler.onPersistContactProfileDelta(
            walletId = walletId,
            ownerId = ownerId,
            contactId = contactId,
            isPresent = true,
            displayName = "Ghost",
            bio = null,
            avatarUrl = null,
            avatarHash = ByteArray(32),
            avatarHashPresent = false,
            avatarFingerprint = ByteArray(8),
            avatarFingerprintPresent = false,
            publicMessage = null,
            checkedAtMs = 1,
        )
        // Still buffered.
        assertTrue(db.dashpayDao().getContactProfilesByOwner(ownerId).isEmpty())
        handler.onChangesetEnd(walletId, success = false)

        assertTrue(db.dashpayDao().getContactProfilesByOwner(ownerId).isEmpty())
    }

    @Test
    fun loadWalletListRoundTripsPaymentsAndContactProfiles() = runTest {
        // Relaunch-durability for the two #3841 stores: payments (Sent
        // entries + memos are NOT re-derivable from UTXOs — losing them
        // here loses them forever) and the contact-profile cache (without
        // it the contacts UI shows raw identity ids until the next
        // profile sweep re-fetches every contact).
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val ownerId = ByteArray(32) { 26 }
        val contactId = ByteArray(32) { 27 }
        seedIdentity(ownerId)
        persistContactProfile(ownerId, contactId)
        // Payments are pull-persisted (refreshDashPayPayments → DAO); the
        // load path reads whatever rows the refresh landed.
        db.dashpayDao().upsertPayments(
            listOf(
                org.dashfoundation.dashsdk.persistence.entities.DashpayPaymentEntity(
                    networkRaw = testnet,
                    ownerIdentityId = ownerId,
                    counterpartyIdentityId = contactId,
                    amountDuffs = 123_456,
                    directionRaw = 0, // Sent
                    statusRaw = 1, // Confirmed
                    txid = "aa".repeat(32),
                    memo = "for pizza",
                ),
            ),
        )

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        val identity = list[0].identities.single()

        assertEquals(1, identity.payments.size)
        val payment = identity.payments[0]
        assertEquals("aa".repeat(32), payment.txid)
        assertTrue(contactId.contentEquals(payment.counterpartyId))
        assertEquals(123_456L, payment.amountDuffs)
        assertEquals(0.toByte(), payment.directionRaw)
        assertEquals(1.toByte(), payment.statusRaw)
        assertEquals("for pizza", payment.memo)

        assertEquals(1, identity.contactProfiles.size)
        val profile = identity.contactProfiles[0]
        assertTrue(contactId.contentEquals(profile.contactId))
        assertEquals("Bob", profile.displayName)
        assertNull(profile.bio)
        assertEquals("https://x/bob.png", profile.avatarUrl)
        assertTrue(ByteArray(32) { 23 }.contentEquals(profile.avatarHash!!))
        assertNull(profile.avatarFingerprint)
        assertEquals("yo", profile.publicMessage)
        assertEquals(1_700_000_111_000, profile.checkedAtMs)
    }

    // ── Free-function encoders ────────────────────────────────────────

    @Test
    fun outPointHexEncodesDisplayOrderTxidAndVout() {
        val txid = ByteArray(32) { it.toByte() } // 00 01 02 … 1f (wire order)
        val outpoint = makeOutpoint(txid, 5)
        val hex = encodeOutPointHex(outpoint)
        // Display order reverses the txid; vout appended decimal.
        assertTrue(hex.endsWith(":5"))
        assertTrue(hex.startsWith("1f1e1d")) // reversed leading bytes
    }

    @Test
    fun outPointHexDecodeIsExactInverseOfEncode() {
        // Round-trip a known outpoint (wire-order txid + vout) through
        // encode → decode and back; the decoded 36 bytes must equal the
        // original, proving `decodeOutPointHex` is the exact inverse used
        // to rebuild the Rust-side outpoint from the persisted display-hex
        // key. Parity with the Swift `decodeOutPointHex` round-trip.
        val txid = ByteArray(32) { it.toByte() } // 00 01 … 1f wire order
        val outpoint = makeOutpoint(txid, 7)
        val hex = encodeOutPointHex(outpoint)
        val decoded = decodeOutPointHex(hex)
        assertNotNull(decoded)
        assertTrue(outpoint.contentEquals(decoded!!))
        // The wire txid is recoverable as the first 32 bytes (the join key
        // for the unresolved-record path).
        assertTrue(txid.contentEquals(decoded.copyOfRange(0, 32)))
        // Malformed inputs fail closed.
        assertNull(decodeOutPointHex("not-an-outpoint"))
        assertNull(decodeOutPointHex("${"ab".repeat(31)}:0")) // 62-char txid
    }

    @Test
    fun loadWalletListRestoresAssetLockResumeState() = runTest {
        // prior-1 regression: the JNI wallet-restore path must carry the
        // persisted asset-lock resume state across a cold restart —
        // tracked asset locks (ALL statuses; Rust drops Consumed itself),
        // the unresolved funding-tx records for the still-Broadcast rows
        // joined to their transaction, and the last-applied chainlock.
        // Mirror of the Swift `buildAssetLockRestoreBuffer` /
        // `buildUnresolvedAssetLockTxRecordBuffer` / `lastAppliedChainLockBytes`
        // round-trips.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )

        // Stamp the last-applied chainlock onto the wallet (bincode blob is
        // opaque to Kotlin — round-tripped verbatim).
        val chainLockBytes = ByteArray(48) { (it + 1).toByte() }
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetHeader(
            walletId, false, 0, false, 0, 0, 0, 0, chainLockBytes,
        )
        handler.onChangesetEnd(walletId, success = true)

        // A still-Broadcast (statusRaw 1) asset lock WITH a matching
        // funding transaction — the resumable + unresolved case.
        val fundingTxid = ByteArray(32) { 51 }
        val fundingOutpoint = makeOutpoint(fundingTxid, 0)
        val fundingTxData = ByteArray(24) { 52 }
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, fundingTxData, 2, 200, ByteArray(32) { 60 },
            1_700_000_000, 0, "Standard", 0, 90_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0, // funding tx: no inputs of ours
        )
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = fundingOutpoint,
            transactionBytes = fundingTxData,
            accountIndex = 0,
            fundingType = 0, // IdentityRegistration
            identityIndex = 0,
            amountDuffs = 90_000,
            status = 1, // Broadcast (< 2 → resumable + unresolved)
            proofBytes = null,
        )
        // A terminal Consumed (statusRaw 4) asset lock — the Kotlin builder
        // emits it (Rust `build_unused_asset_locks` is the sole Consumed
        // filter); it is NOT eligible for the unresolved-record set.
        val consumedTxid = ByteArray(32) { 71 }
        val consumedOutpoint = makeOutpoint(consumedTxid, 1)
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = consumedOutpoint,
            transactionBytes = ByteArray(16) { 72 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 1,
            amountDuffs = 55_000,
            status = 4, // Consumed
            proofBytes = ByteArray(8) { 73 },
        )
        handler.onChangesetEnd(walletId, success = true)

        val list = handler.onLoadWalletList()
        assertEquals(1, list.size)
        val entry = list[0]

        // ── Last-applied chainlock round-trips verbatim ──────────────
        assertTrue(chainLockBytes.contentEquals(entry.lastAppliedChainLockBytes))

        // ── Tracked asset locks: BOTH rows, including the Consumed one ─
        // (Kotlin does not filter; Rust drops Consumed at load).
        val tracked = entry.trackedAssetLocks.sortedBy { it.status }
        assertEquals(2, tracked.size)

        val broadcast = tracked[0]
        assertEquals(1.toByte(), broadcast.status)
        assertTrue(fundingOutpoint.contentEquals(broadcast.outPoint))
        assertTrue(fundingTxData.contentEquals(broadcast.transactionBytes))
        assertEquals(0, broadcast.accountIndex)
        assertEquals(0.toByte(), broadcast.fundingType)
        assertEquals(0, broadcast.identityIndex)
        assertEquals(90_000L, broadcast.amountDuffs)
        // No proof yet on a Broadcast lock → empty (Rust maps to null/0).
        assertEquals(0, broadcast.proofBytes.size)

        val consumed = tracked[1]
        assertEquals(4.toByte(), consumed.status)
        assertTrue(consumedOutpoint.contentEquals(consumed.outPoint))
        assertEquals(1, consumed.identityIndex)
        assertEquals(55_000L, consumed.amountDuffs)
        assertEquals(8, consumed.proofBytes.size)

        // ── Unresolved funding-tx records: only the statusRaw < 2 row ──
        // joined to its persisted transaction (the Consumed row is excluded
        // by the getUnresolvedByWallet filter).
        assertEquals(1, entry.unresolvedAssetLockTxRecords.size)
        val rec = entry.unresolvedAssetLockTxRecords[0]
        assertEquals(0, rec.accountIndex)
        assertTrue(fundingTxData.contentEquals(rec.txBytes))
        assertEquals(2, rec.contextRaw) // InBlock
        assertEquals(200, rec.blockHeight)
        assertEquals(32, rec.blockHash.size)
        assertTrue(ByteArray(32) { 60 }.contentEquals(rec.blockHash))
        assertEquals(1_700_000_000L, rec.blockTimestamp)
        assertEquals(1_699_999_000L, rec.firstSeen)
    }

    @Test
    fun assetLockPersistRoundTrips() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 40 }, 1)
        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = outpoint,
            transactionBytes = ByteArray(20) { 41 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 0,
            amountDuffs = 100_000,
            status = 1, // Broadcast
            proofBytes = null,
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.assetLockDao().getByOutPointHex(encodeOutPointHex(outpoint))
        assertNotNull(row)
        assertEquals(100_000L, row!!.amountDuffs)
        assertEquals(1, row.statusRaw)
        assertFalse(row.proofBytes != null)
    }

    @Test
    fun assetLockUpsertNeverRegressesAConsumedRow() = runTest {
        // The upsert-side twin of the delete guard below, matching Swift's
        // skip and SQLite's WHERE clause: Consumed is the terminal state,
        // and a stale reconstruction/enrichment snapshot folded after the
        // live consumption write must not regress it.
        val outpoint = makeOutpoint(ByteArray(32) { 48 }, 0)
        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = outpoint,
            transactionBytes = ByteArray(20) { 49 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 0,
            amountDuffs = 70_000,
            status = 4, // Consumed — terminal
            proofBytes = ByteArray(8) { 50 },
        )
        // The stale snapshot arrives after the consumption write.
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = outpoint,
            transactionBytes = ByteArray(20) { 49 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 0,
            amountDuffs = 70_000,
            status = 1, // Broadcast — a stale pre-consumption view
            proofBytes = null,
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.assetLockDao().getByOutPointHex(encodeOutPointHex(outpoint))
        assertNotNull(row)
        assertEquals(
            "a stale non-Consumed snapshot must not regress the terminal",
            4,
            row!!.statusRaw,
        )
    }

    @Test
    fun assetLockRemovalNeverDeletesAConsumedRow() = runTest {
        // Parity with SQLite (`status != 'consumed'`) and Swift
        // (`statusRaw == 4` skip): a Consumed row is deliberately retained
        // for historical lookup, and neither removal producer — a
        // rejected-at-broadcast Built row, or the sweep cascade for a swept
        // funding tx — can legitimately name one, so a removal reaching a
        // consumed row is by construction a stale write. Kotlin deleted
        // unconditionally.
        val liveOutpoint = makeOutpoint(ByteArray(32) { 43 }, 0)
        val consumedOutpoint = makeOutpoint(ByteArray(32) { 44 }, 1)
        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = liveOutpoint,
            transactionBytes = ByteArray(20) { 45 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 0,
            amountDuffs = 100_000,
            status = 1, // Broadcast — a removal may take this one
            proofBytes = null,
        )
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = consumedOutpoint,
            transactionBytes = ByteArray(20) { 46 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 1,
            amountDuffs = 55_000,
            status = 4, // Consumed — terminal, retained for history
            proofBytes = ByteArray(8) { 47 },
        )
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockRemoval(walletId, liveOutpoint)
        handler.onPersistAssetLockRemoval(walletId, consumedOutpoint)
        handler.onChangesetEnd(walletId, success = true)

        assertNull(
            "a live row is removable",
            db.assetLockDao().getByOutPointHex(encodeOutPointHex(liveOutpoint)),
        )
        val consumed = db.assetLockDao().getByOutPointHex(encodeOutPointHex(consumedOutpoint))
        assertNotNull("a stale removal must never take the Consumed terminal", consumed)
        assertEquals(4, consumed!!.statusRaw)
    }

    // ── Invitations (DIP-13) ──────────────────────────────────────────

    @Test
    fun invitationPersistRoundTripsEveryField() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 50 }, 2)
        handler.onChangesetBegin(walletId)
        assertEquals(
            0,
            handler.onPersistInvitationUpsert(
                walletId = walletId,
                outPoint = outpoint,
                fundingIndex = 3,
                amountDuffs = 3_000_000,
                expiryUnix = 1_800_086_400,
                createdAtSecs = 1_800_000_000,
                hasInviter = true,
                status = 0, // Created
            ),
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.invitationDao().getByOutPointHex(encodeOutPointHex(outpoint))
        assertNotNull(row)
        assertTrue(outpoint.contentEquals(row!!.rawOutPoint))
        assertTrue(walletId.contentEquals(row.walletId))
        assertEquals(3, row.fundingIndexRaw)
        assertEquals(3_000_000L, row.amountDuffs)
        assertEquals(1_800_086_400, row.expiryUnix)
        assertEquals(1_800_000_000, row.createdAtSecs)
        assertTrue(row.hasInviter)
        assertEquals(0, row.statusRaw)
        assertFalse(row.reclaimInFlight)
    }

    @Test
    fun invitationUpsertPreservesClientWrittenStatusAndMarker() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 51 }, 0)
        val hex = encodeOutPointHex(outpoint)
        handler.onPersistInvitationUpsert(
            walletId, outpoint,
            fundingIndex = 0, amountDuffs = 300_000, expiryUnix = 10,
            createdAtSecs = 5, hasInviter = false, status = 0,
        )
        // The app writes the terminal status + marker locally (Rust never
        // emits transitions); a Rust re-emit of the same outpoint with the
        // original Created status must not reset them.
        db.invitationDao().setStatusAndMarker(hex, 2, true, 99L)

        handler.onPersistInvitationUpsert(
            walletId, outpoint,
            fundingIndex = 0, amountDuffs = 300_000, expiryUnix = 10,
            createdAtSecs = 5, hasInviter = false, status = 0,
        )

        val row = db.invitationDao().getByOutPointHex(hex)
        assertEquals(2, row!!.statusRaw)
        assertTrue(row.reclaimInFlight)
    }

    @Test
    fun invitationRemovalDeletesTheRowByTheSameKey() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 52 }, 7)
        handler.onPersistInvitationUpsert(
            walletId, outpoint,
            fundingIndex = 1, amountDuffs = 300_000, expiryUnix = 1,
            createdAtSecs = 1, hasInviter = false, status = 0,
        )
        assertNotNull(db.invitationDao().getByOutPointHex(encodeOutPointHex(outpoint)))

        assertEquals(0, handler.onPersistInvitationRemoval(walletId, outpoint))
        assertNull(db.invitationDao().getByOutPointHex(encodeOutPointHex(outpoint)))
    }

    @Test
    fun invitationWriteInRolledBackRoundNeverLands() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 53 }, 0)
        handler.onChangesetBegin(walletId)
        handler.onPersistInvitationUpsert(
            walletId, outpoint,
            fundingIndex = 0, amountDuffs = 300_000, expiryUnix = 1,
            createdAtSecs = 1, hasInviter = false, status = 0,
        )
        handler.onChangesetEnd(walletId, success = false)

        assertNull(db.invitationDao().getByOutPointHex(encodeOutPointHex(outpoint)))
    }

    // ── Invitation funding-index pool durability ──────────────────────

    private fun poolEntry(accountTypeTag: Byte): Int =
        handler.onPersistAccountAddressPoolEntry(
            walletId = walletId,
            accountTypeTag = accountTypeTag,
            accountStandardTag = 0,
            accountIndex = 0,
            accountRegistrationIndex = 0,
            accountKeyClass = 0,
            accountUserIdentityId = ByteArray(0),
            accountFriendIdentityId = ByteArray(0),
            poolTypeTag = 3, // AbsentHardened
            publicKey = ByteArray(33) { 9 },
            hasPublicKey = true,
            addressPoolTypeTag = 3,
            addressIndex = 0,
            isUsed = true,
            balance = 0,
            addressBase58 = "yTestInvitationPoolAddress000000",
            derivationPath = "m/9'/1'/5'/3'/0'",
        )

    @Test
    fun invitationPoolEntryCreatesTheMissingAccountRow() = runTest {
        // Wallet exists, but the IdentityInvitation account row was never
        // registered (e.g. an install predating invitation support). The
        // pool write is the funding-index durability record, so it must
        // create the account row instead of silently skipping.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        handler.onChangesetBegin(walletId)
        assertEquals(0, poolEntry(accountTypeTag = 5))
        assertEquals(0, handler.onChangesetEnd(walletId, success = true))

        val accounts = db.accountDao().observeByWallet(walletId).first()
        assertEquals(1, accounts.size)
        assertEquals(5, accounts[0].accountType)
        val addresses = db.coreAddressDao().observeByAccount(accounts[0].id).first()
        assertEquals(1, addresses.size)
    }

    @Test
    fun invitationPoolEntryWithoutWalletFailsTheRound() = runTest {
        // No wallet row at all: the account row cannot be created (FK), so
        // the round must FAIL — a silently-skipped write here would let
        // Rust broadcast a voucher whose funding index never became
        // durable (the voucher-key-reuse defect class).
        handler.onChangesetBegin(walletId)
        assertEquals(0, poolEntry(accountTypeTag = 5)) // staged, not yet run
        assertEquals(1, handler.onChangesetEnd(walletId, success = true))
    }

    @Test
    fun nonInvitationPoolEntryWithMissingAccountStillSkipsSilently() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        handler.onChangesetBegin(walletId)
        assertEquals(0, poolEntry(accountTypeTag = 0)) // Standard
        assertEquals(0, handler.onChangesetEnd(walletId, success = true))

        // Unchanged pre-invitation behavior: no account row conjured.
        assertTrue(db.accountDao().observeByWallet(walletId).first().isEmpty())
    }

    // ── Asset-lock spend visibility ────────────────────────────────────

    /**
     * An asset-lock tx burns its value into the special-tx payload and often
     * has no wallet-owned standard output, so SPV block matching can miss it:
     * the spender's transaction row never advances past mempool context and
     * the in-block flip in onWalletChangesetTransaction never runs — the
     * funding TXO sits at isSpent=false (spendingTxid set) FOREVER, and every
     * isSpent-based balance read overstates the wallet. The lock's own status
     * DOES keep arriving; from InstantSendLocked on, the upsert must flip
     * linked TXOs.
     */
    @Test
    fun assetLockStatusAdvanceFlipsItsFundingTxos() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val lockTxid = ByteArray(32) { 7 }
        db.transactionDao().upsert(
            TransactionEntity(txid = lockTxid, transactionData = ByteArray(4), context = 0),
        )
        val outpoint = ByteArray(36) { 9 }
        db.txoDao().upsert(
            TxoEntity(
                outpoint = outpoint,
                vout = 0,
                amount = 1_000_000,
                address = "yTest",
                walletId = walletId,
                spendingTxid = lockTxid,
                spendingInputIndex = 0,
                isSpent = false,
            ),
        )

        // Broadcast (1) must NOT flip — the network holds no lock yet and a
        // pre-broadcast abort could still release the inputs.
        handler.onPersistAssetLockUpsert(
            walletId, lockTxid + ByteArray(4), ByteArray(4), 0, 1, 0, 999_545, 1, null,
        )
        assertFalse(db.txoDao().getByOutpoint(outpoint)!!.isSpent)

        // InstantSendLocked (2): the network has locked the inputs — flip.
        handler.onPersistAssetLockUpsert(
            walletId, lockTxid + ByteArray(4), ByteArray(4), 0, 1, 0, 999_545, 2, null,
        )
        assertTrue(db.txoDao().getByOutpoint(outpoint)!!.isSpent)
    }

    /** The Consumed (4) terminal upsert heals rows a missed IS/CL never flipped. */
    @Test
    fun assetLockConsumedHealsAStaleUnspentRow() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val lockTxid = ByteArray(32) { 8 }
        db.transactionDao().upsert(
            TransactionEntity(txid = lockTxid, transactionData = ByteArray(4), context = 0),
        )
        val outpoint = ByteArray(36) { 10 }
        db.txoDao().upsert(
            TxoEntity(
                outpoint = outpoint,
                vout = 0,
                amount = 9_999_545,
                address = "yTest2",
                walletId = walletId,
                spendingTxid = lockTxid,
                spendingInputIndex = 0,
                isSpent = false,
            ),
        )

        handler.onPersistAssetLockUpsert(
            walletId, lockTxid + ByteArray(4), ByteArray(4), 0, 1, 0, 9_999_545, 4, null,
        )
        assertTrue(db.txoDao().getByOutpoint(outpoint)!!.isSpent)
    }

    /**
     * Registers [wallet] with one BIP44 account, one owned [address], and
     * one plain unspent output on `<txid>:0` — the ordinary restorable
     * shape, with no spender linked.
     */
    private suspend fun seedRestorableWallet(
        wallet: ByteArray,
        address: String,
        txid: ByteArray,
        xpubFill: Byte,
    ) {
        seedWalletWithAddress(wallet, address, xpubFill)

        handler.onChangesetBegin(wallet)
        handler.onWalletChangesetUtxoAdded(
            wallet, txid, 0, 999_545, address, ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(wallet, success = true)
    }

    /**
     * Seeds the state an OLDER build left behind on [wallet]: a funding
     * TXO whose spender is the asset-lock transaction [lockTxid], stuck
     * at MEMPOOL context because SPV block matching never matched the
     * lock, so `onWalletChangesetTransaction`'s in-block flip never ran
     * and the row stays `isSpent = false` with `spendingTxid` linked.
     */
    private suspend fun seedFundingTxoSpentByAMempoolAssetLock(
        wallet: ByteArray,
        address: String,
        fundingTxid: ByteArray,
        lockTxid: ByteArray,
        xpubFill: Byte,
    ) {
        seedRestorableWallet(wallet, address, fundingTxid, xpubFill)

        handler.onChangesetBegin(wallet)
        recordTransaction(
            handler,
            wallet, lockTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "AssetLock", 0, -999_545, 0, false, "", 1_700_000_100,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onChangesetEnd(wallet, success = true)

        val txo = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!
        assertFalse("precondition: the missed flip leaves the row unspent", txo.isSpent)
        assertTrue(lockTxid.contentEquals(txo.spendingTxid!!))
    }

    /**
     * A directly-written asset-lock row, the way a build predating the
     * callback-time reconcile would have left it: terminal `Consumed`,
     * no further upsert coming.
     */
    private suspend fun seedConsumedAssetLockRow(
        wallet: ByteArray,
        lockTxid: ByteArray,
        vout: Int,
    ) {
        db.assetLockDao().upsert(
            AssetLockEntity(
                outPointHex = encodeOutPointHex(makeOutpoint(lockTxid, vout)),
                walletId = wallet,
                transactionBytes = ByteArray(10) { 5 },
                fundingTypeRaw = 0,
                identityIndexRaw = 0,
                amountDuffs = 999_545,
                statusRaw = 4,
            ),
        )
    }

    private fun restoredUtxoCount(wallet: ByteArray): Int = restoredUtxoTxids(wallet).size

    /** Hex prev-txids the restore hands back for [wallet], sorted. */
    private fun restoredUtxoTxids(wallet: ByteArray): List<String> {
        val entry = handler.onLoadWalletList().firstOrNull { it.walletId.contentEquals(wallet) }
        // Name a missing wallet rather than letting it surface as a
        // NoSuchElementException from the mapping below.
        assertNotNull("the restore must still carry this wallet", entry)
        return entry!!.utxos.map { it.prevTxid.toHex() }.sorted()
    }

    /**
     * The restore-time half of the same defect, on the state an OLDER
     * build left behind: the funding TXO is linked to a spending tx
     * stuck at MEMPOOL context (SPV block matching never matched the
     * lock, so the in-block flip never ran) while the lock row itself
     * already reads `Consumed`.
     *
     * `Consumed` is terminal — the lock never upserts again — so the
     * callback-time reconcile has no future event to repair this with.
     * Every relaunch would hand the consumed output back to Rust as
     * spendable and re-inflate the balance. The restore guard must both
     * exclude it and heal the row in place.
     */
    @Test
    fun loadSkipsAndHealsATxoConsumedByAFinalizedAssetLock() = runTest {
        val fundingTxid = ByteArray(32) { 51 }
        val lockTxid = ByteArray(32) { 52 }
        seedFundingTxoSpentByAMempoolAssetLock(
            walletId, "yLockFunder", fundingTxid, lockTxid, 30,
        )

        // Without a finalized lock row this IS the phantom UTXO: the
        // restore hands the consumed output straight back to Rust.
        assertEquals(1, restoredUtxoCount(walletId))

        seedConsumedAssetLockRow(walletId, lockTxid, vout = 0)

        assertEquals(
            "the finalized lock's funding output must not rehydrate as spendable",
            0,
            restoredUtxoCount(walletId),
        )
        assertTrue(
            "and the stale flag must be healed in place",
            db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent,
        )
    }

    /**
     * Same defect, on the outpoint shape DIP-0027 actually permits: one
     * funding transaction may carry several credit outputs, and Rust
     * persists each tracked lock under its own credit-output index
     * (`wallet/asset_lock/sync/reconstruction.rs`), so a perfectly valid
     * lock row can be keyed `<txid>:1` with no `<txid>:0` row anywhere.
     *
     * A guard that probes the synthetic vout-0 outpoint misses it and
     * hands the consumed output straight back as spendable — the finality
     * signal belongs to the transaction, not to one of its outputs.
     */
    @Test
    fun loadSkipsATxoConsumedByAFinalizedAssetLockPersistedAtANonZeroVout() = runTest {
        val fundingTxid = ByteArray(32) { 61 }
        val lockTxid = ByteArray(32) { 62 }
        seedFundingTxoSpentByAMempoolAssetLock(
            walletId, "yLockFunderVout1", fundingTxid, lockTxid, 31,
        )
        assertEquals(1, restoredUtxoCount(walletId))

        // ONLY the second credit output is persisted — no `:0` row exists.
        seedConsumedAssetLockRow(walletId, lockTxid, vout = 1)
        assertNull(
            "the fixture must not leave a vout-0 row for the guard to find",
            db.assetLockDao().getByOutPointHex(encodeOutPointHex(makeOutpoint(lockTxid, 0))),
        )

        assertEquals(
            "finality belongs to the funding transaction, not to credit output 0",
            0,
            restoredUtxoCount(walletId),
        )
        assertTrue(
            "and the stale flag must be healed in place",
            db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent,
        )
    }

    /**
     * Rebuild the fixture on a database whose statements starting with
     * [failingSqlPrefix] can be faulted mid-test. The helper factory is
     * fixed when the database is built, so this replaces the shared
     * fixture rather than decorating it. The injector comes back
     * disarmed — arm it once the seed data is in.
     */
    private fun useFaultedDatabase(failingSqlPrefix: String): SingleStatementFaultInjector {
        val faults = SingleStatementFaultInjector(failingSqlPrefix)
        db.close()
        db = Room.inMemoryDatabaseBuilder(
            ApplicationProvider.getApplicationContext<Context>(),
            DashDatabase::class.java,
        )
            .allowMainThreadQueries()
            .openHelperFactory(faults)
            .build()
        handler = newHandler()
        return faults
    }

    // ── Load failure policy ───────────────────────────────────────────
    //
    // The load slots are array-shaped, so a load reports failure the only
    // way it can: by throwing. The JNI trampoline turns the pending
    // exception into a non-zero FFI load code, reaching the host as
    // `PersisterLoadFatal` (50). Degrading to an empty array would instead
    // report a SUCCESSFUL restore of nothing, which Rust reads as a fresh
    // device — a store fault masquerading as data loss. Swift parity:
    // `loadWalletList` returns `errored = true`.

    @Test
    fun aFailingWalletFetchFailsTheLoadRatherThanRestoringNothing() = runTest {
        val faults = useFaultedDatabase("SELECT * FROM wallets")
        seedRestorableWallet(walletId, "yLoadFailFunder", ByteArray(32) { 81 }, 34)

        // Control: the wallet restores while the fetch is readable, so the
        // failure below is the injected fault and not the fixture.
        assertEquals(1, handler.onLoadWalletList().size)

        faults.armed = true
        assertThrows(
            "a failed wallet fetch must fail the load; an empty restore would " +
                "report every persisted wallet as absent",
            SQLiteException::class.java,
        ) { handler.onLoadWalletList() }
    }

    @Test
    fun aFailingShieldedNoteFetchFailsTheLoad() = runTest {
        val faults = useFaultedDatabase("SELECT * FROM shielded_notes")
        handler.onChangesetBegin(walletId)
        handler.onPersistShieldedNote(
            walletId = walletId,
            noteWalletId = walletId,
            accountIndex = 0,
            position = 3,
            cmx = ByteArray(32) { 82 },
            nullifier = ByteArray(32) { 83 },
            blockHeight = 50,
            isSpent = 0,
            value = 100_000,
            noteData = ByteArray(115) { 84 },
        )
        handler.onChangesetEnd(walletId, success = true)

        // Control, as above.
        assertEquals(1, handler.onLoadShieldedNotes().size)

        faults.armed = true
        assertThrows(
            "a failed shielded-note fetch must fail the load; an empty restore " +
                "would report the persisted notes as absent",
            SQLiteException::class.java,
        ) { handler.onLoadShieldedNotes() }
    }

    /**
     * Failure policy for the finality lookup the guard depends on.
     *
     * An unreadable asset-lock table cannot answer whether the output is
     * gone. Withholding the one candidate it could not judge would
     * under-report the wallet's funds — the same apparent data loss an
     * empty restore produces, just quieter — so the read failure fails
     * the whole load and the host retries. Swift parity:
     * `finalizedAssetLockFundingTxids` bails with `errored = true`.
     *
     * The fault is injected at the single prepared statement, not at the
     * table, because the table is read by two other restore builders
     * whose own failure modes are out of this guard's hands.
     */
    @Test
    fun aFailingFinalizedLockLookupFailsTheLoad() = runTest {
        val faults = useFaultedDatabase("SELECT MAX(statusRaw) FROM asset_locks")

        val fundingTxid = ByteArray(32) { 71 }
        val lockTxid = ByteArray(32) { 72 }
        seedFundingTxoSpentByAMempoolAssetLock(
            walletId, "yLockFunderThrow", fundingTxid, lockTxid, 32,
        )
        seedConsumedAssetLockRow(walletId, lockTxid, vout = 0)
        // Sentinel: an ordinary unspent output on the same wallet, with
        // no spender at all, so it never reaches the lookup.
        val sentinelTxid = ByteArray(32) { 73 }
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, sentinelTxid, 0, 500_000, "yLockFunderThrow",
            ByteArray(25) { 6 }, 100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Control: with the lookup readable the load succeeds, excludes
        // the consumed output and keeps the sentinel — so the failure
        // below is the injected fault, not the fixture.
        assertEquals(listOf(sentinelTxid.toHex()), restoredUtxoTxids(walletId))
        assertTrue(db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent)

        // Re-stale the healed row so the unreadable pass faces the same
        // decision the readable one just made.
        db.txoDao().upsert(
            db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.copy(isSpent = false),
        )
        faults.armed = true

        assertThrows(
            "an unanswerable finality lookup must fail the load, not silently " +
                "withhold the output it could not judge",
            SQLiteException::class.java,
        ) { handler.onLoadWalletList() }
        assertFalse(
            "an unanswerable lookup proves nothing, so it must not heal the flag",
            db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent,
        )
    }

    // ── Asset locks: Consumed is terminal ─────────────────────────────
    //
    // Swift parity with `persistAssetLocks`
    // (PlatformWalletPersistenceHandler.swift:270 upsert, :310 removal).
    // Consumed (4) is the terminal lifecycle state and the writers race:
    // the wallet-event adapter's batched drain can deliver a stale
    // reconstruction snapshot AFTER the live flow's consumption write.

    /** Upsert [outpoint] at [status] in its own committed round. */
    private fun persistAssetLock(
        outpoint: ByteArray,
        status: Byte,
        amountDuffs: Long = 100_000,
        proofBytes: ByteArray? = null,
    ) {
        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = outpoint,
            transactionBytes = ByteArray(20) { 41 },
            accountIndex = 0,
            fundingType = 0,
            identityIndex = 0,
            amountDuffs = amountDuffs,
            status = status,
            proofBytes = proofBytes,
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun shouldNotRegressAConsumedAssetLockOnAStaleNonConsumedUpsert() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 60 }, 0)
        persistAssetLock(outpoint, status = 4, proofBytes = ByteArray(8) { 1 })

        // Stale replay of an older Broadcast snapshot.
        persistAssetLock(outpoint, status = 1, amountDuffs = 999, proofBytes = null)

        val row = db.assetLockDao().getByOutPointHex(encodeOutPointHex(outpoint))!!
        assertEquals(4, row.statusRaw)
        // The whole row is skipped, not merely the status column.
        assertEquals(100_000L, row.amountDuffs)
        assertNotNull(row.proofBytes)
    }

    @Test
    fun shouldStillAcceptAnotherConsumedWriteOnAConsumedAssetLock() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 61 }, 0)
        persistAssetLock(outpoint, status = 4)
        persistAssetLock(outpoint, status = 4, amountDuffs = 250_000, proofBytes = ByteArray(4) { 7 })

        val row = db.assetLockDao().getByOutPointHex(encodeOutPointHex(outpoint))!!
        assertEquals(4, row.statusRaw)
        assertEquals(250_000L, row.amountDuffs)
        assertNotNull(row.proofBytes)
    }

    @Test
    fun shouldKeepNonConsumedAssetLockStatusesLastWriteWinsInBothDirections() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 62 }, 0)
        val hex = encodeOutPointHex(outpoint)

        persistAssetLock(outpoint, status = 1) // Broadcast
        persistAssetLock(outpoint, status = 3) // ChainLocked
        assertEquals(3, db.assetLockDao().getByOutPointHex(hex)!!.statusRaw)

        // The guard is narrow: non-terminal statuses legitimately move both
        // ways, so they stay last-write-wins.
        persistAssetLock(outpoint, status = 1)
        assertEquals(1, db.assetLockDao().getByOutPointHex(hex)!!.statusRaw)
    }

    @Test
    fun shouldRetainAConsumedAssetLockThroughAStaleRemoval() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 63 }, 0)
        persistAssetLock(outpoint, status = 4)

        handler.onChangesetBegin(walletId)
        assertEquals(0, handler.onPersistAssetLockRemoval(walletId, outpoint))
        assertEquals(0, handler.onChangesetEnd(walletId, success = true))

        // Retained for historical lookup: the only removal emitter
        // (`untrack_asset_lock`) targets rejected Built rows, so a removal
        // reaching a consumed row is by construction a stale write.
        val row = db.assetLockDao().getByOutPointHex(encodeOutPointHex(outpoint))
        assertNotNull(row)
        assertEquals(4, row!!.statusRaw)
    }

    @Test
    fun shouldStillRemoveANonConsumedAssetLock() = runTest {
        val outpoint = makeOutpoint(ByteArray(32) { 64 }, 0)
        persistAssetLock(outpoint, status = 0) // Built

        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockRemoval(walletId, outpoint)
        handler.onChangesetEnd(walletId, success = true)

        assertNull(db.assetLockDao().getByOutPointHex(encodeOutPointHex(outpoint)))
    }

    // ── Identity sweep vs marketplace column authority ────────────────
    //
    // Swift parity with `upsertDPNSNames`
    // (PlatformWalletPersistenceHandler.swift:1912-1941). The identity
    // snapshot owns `isOwned` / `acquiredAt` / `label`; the marketplace
    // reconciliation lane owns `documentId` / price / sale status /
    // counterparty. Neither may overwrite the other's columns.

    /** Commit one canonical identity snapshot carrying exactly [names]. */
    private fun persistIdentitySnapshot(identityId: ByteArray, vararg names: String) {
        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId, identityId, 1, 0, false, 0, 0, true, walletId,
            names.toList().toTypedArray(), LongArray(names.size), false, null, null, null,
            ByteArray(32), false, ByteArray(8), false, null,
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    /** Commit one marketplace row for [documentId]. */
    private fun persistMarketplaceRow(
        identityId: ByteArray,
        documentId: ByteArray,
        label: String,
        normalizedLabel: String,
        status: Byte,
        counterpartyId: ByteArray? = null,
        priceCredits: Long? = null,
    ) {
        handler.onChangesetBegin(walletId)
        handler.onPersistDpnsNameState(
            walletId = walletId,
            documentId = documentId,
            walletIdentityId = identityId,
            hasCounterparty = counterpartyId != null,
            counterpartyId = counterpartyId ?: ByteArray(32),
            label = label,
            normalizedLabel = normalizedLabel,
            normalizedParentDomainName = "dash",
            hasPrice = priceCredits != null,
            priceCredits = priceCredits ?: 0,
            status = status,
            createdAtMs = 100,
            updatedAtMs = 200,
            transferredAtMs = 300,
            lastSyncedAtMs = 400,
        )
        handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun shouldKeepAListedNameAsDepartedHistoryThroughTheIdentitySweepInsteadOfDestroyingIt() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 20 }
        val documentId = ByteArray(32) { 21 }

        persistIdentitySnapshot(identityId, "Alice")
        // Listed for sale: still owned, and marketplace-tracked.
        persistMarketplaceRow(
            identityId, documentId, "Alice", "a11ce",
            status = 0, priceCredits = 5_000,
        )
        assertTrue(db.dpnsNameDao().getByDocumentId(documentId)!!.isOwned)

        // The label leaves the canonical set before the marketplace pass can
        // classify the departure — the unclassifiable-departure round. The
        // old sweep deleted the row here, permanently destroying the only
        // record of where the name went (Android-only; Swift never did).
        persistIdentitySnapshot(identityId)

        val retained = db.dpnsNameDao().getByDocumentId(documentId)
        assertNotNull("marketplace history must survive the identity sweep", retained)
        assertFalse("a departed name must not read as owned", retained!!.isOwned)
        assertEquals(5_000L, retained.priceCredits)
        assertEquals(400L, retained.marketplaceUpdatedAt)
        // ...but owned-name queries and UI selection must not surface it.
        assertTrue(db.dpnsNameDao().observeByIdentity(identityId).first().isEmpty())
    }

    @Test
    fun shouldKeepASoldNameWithItsCounterpartyThroughTheIdentitySweep() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 22 }
        val documentId = ByteArray(32) { 23 }
        val buyerId = ByteArray(32) { 24 }

        persistIdentitySnapshot(identityId, "Bob")
        persistMarketplaceRow(
            identityId, documentId, "Bob", "b0b",
            status = 1, counterpartyId = buyerId,
        )

        persistIdentitySnapshot(identityId)

        val retained = db.dpnsNameDao().getByDocumentId(documentId)
        assertNotNull(retained)
        assertFalse(retained!!.isOwned)
        assertEquals(1, retained.saleStatusRaw)
        assertTrue(buyerId.contentEquals(retained.counterpartyIdentityId!!))
    }

    @Test
    fun shouldStillDeleteLabelCacheRowsWithNoMarketplaceHistoryOnTheIdentitySweep() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 25 }

        persistIdentitySnapshot(identityId, "Alice", "Bob")
        assertEquals(2, db.dpnsNameDao().getAllByIdentity(identityId).size)

        persistIdentitySnapshot(identityId, "Alice")

        // No documentId attached → a pure stale label-cache row, removed
        // entirely (unchanged behavior).
        assertEquals(
            listOf("Alice"),
            db.dpnsNameDao().getAllByIdentity(identityId).map { it.label },
        )
    }

    // ── Bounded tombstone lifetime ────────────────────────────────────

    /** One committed round: synced height + (optionally) chainlock bytes. */
    private fun headerRound(
        h: PlatformWalletPersistenceHandler,
        synced: Int,
        chainLockBytes: ByteArray = ByteArray(84) { 9 },
    ) {
        h.onChangesetBegin(walletId)
        h.onWalletChangesetHeader(
            walletId = walletId,
            hasSyncedHeight = true,
            syncedHeight = synced,
            hasBalance = false,
            confirmedDelta = 0,
            unconfirmedDelta = 0,
            immatureDelta = 0,
            lockedDelta = 0,
            lastAppliedChainLockBytes = chainLockBytes,
        )
        h.onChangesetEnd(walletId, success = true)
    }

    /**
     * One committed round delivering the numeric chainlock height, the way
     * the JNI bridge does — its own slot, after the header's.
     */
    private fun chainLockHeightRound(h: PlatformWalletPersistenceHandler, height: Int) {
        h.onChangesetBegin(walletId)
        h.onWalletChangesetChainLockHeight(walletId, height)
        h.onChangesetEnd(walletId, success = true)
    }

    /**
     * Record a loser spending [outpoint] (funding unknown), then sweep it
     * in the given winner context — a mined height (default 400) leaves
     * the block-context tombstone the collection tests reason about, -1
     * (an IS-locked, unmined winner) leaves the same tombstone unstamped,
     * which the collector never touches.
     */
    private fun seedSweptTombstone(
        outpoint: ByteArray,
        loser: ByteArray,
        winner: ByteArray,
        winnerMinedHeight: Int = 400,
    ) {
        recordMempoolSpend(loser, outpoint)
        sweepRound(walletId, listOf(loser), winner, winnerMinedHeight = winnerMinedHeight)
    }

    @Test
    fun aSweptTombstoneIsCollectedAtFinalityAndNotBefore() = runTest {
        // The attacker-shaped row: a swept incoming payment's foreign input
        // leaves a pending tombstone that never drains — no funding TXO
        // ever arrives — and before the collector existed it was permanent,
        // growable one row per input by repeatedly double-spending payments
        // at this wallet. The collector deletes it exactly when the
        // chainlock finality boundary min(chainlockHeight, syncedHeight)
        // reaches the WINNER'S mined height — no observation-age margin:
        // the stamp is the winner's own height, carried on the sweep event
        // itself, so nothing here guesses when the winner mined.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 71 }
        val p = makeOutpoint(fundingTxid, 0)
        seedSweptTombstone(p, ByteArray(32) { 72 }, ByteArray(32) { 73 }, winnerMinedHeight = 400)

        val tombstone = db.documentDao().getPendingInputsByOutpoint(p).single()
        assertTrue("sanity: the sweep flagged the row", tombstone.isSweptTombstone)
        assertEquals(
            "the tombstone is stamped with the winner's own mined height, " +
                "not any observation watermark",
            400, tombstone.winnerMinedHeight,
        )

        // Chainlocks race far ahead; the filter scan is one block short of
        // the winner — the boundary has not reached the spend, so the
        // funding output could still be delivered by the unscanned range.
        chainLockHeightRound(handler, 10_000)
        headerRound(handler, 399)
        assertEquals(
            "boundary min(10000, 399) = 399 is below the winner's height 400 — the hold stays",
            1, db.documentDao().getPendingInputsByOutpoint(p).size,
        )

        headerRound(handler, 400)
        assertTrue(
            "the boundary reaching the winner's height collects the row — no margin",
            db.documentDao().getPendingInputsByOutpoint(p).isEmpty(),
        )
    }

    @Test
    fun aSweptTombstoneOutlivesAnySyncProgressWithoutAChainLockHeight() = runTest {
        // Synced height alone is not finality — and neither is the mere
        // PRESENCE of chainlock bytes on the wallet row: the bincode blob
        // is opaque here, so "bytes exist" proves nothing about WHICH
        // block is final (the unsound gate the review flagged). Every
        // round below carries chainlock bytes; only the numeric height
        // delivered by onWalletChangesetChainLockHeight supplies a
        // boundary, and the moment one lands the finalized stamp collects.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        headerRound(handler, 100)

        val fundingTxid = ByteArray(32) { 74 }
        val p = makeOutpoint(fundingTxid, 0)
        seedSweptTombstone(p, ByteArray(32) { 75 }, ByteArray(32) { 76 }, winnerMinedHeight = 400)

        headerRound(handler, 100_000)
        assertEquals(
            "chainlock bytes are on record but no numeric height is — the " +
                "hold outlasts any amount of synced-height progress",
            1, db.documentDao().getPendingInputsByOutpoint(p).size,
        )

        chainLockHeightRound(handler, 100_000)
        assertTrue(
            "the first numeric chainlock height supplies the boundary and " +
                "the finalized stamp collects",
            db.documentDao().getPendingInputsByOutpoint(p).isEmpty(),
        )
    }

    @Test
    fun aDrainedClaimIsImmuneToTheCollector() = runTest {
        // The genuine claim the tombstone exists for: its funding TXO
        // arrives, the drain moves the hold onto the TXO row
        // (supersededByTxid) and deletes the pending rows — so no amount of
        // later sync progress may touch the materialised hold.
        seedWalletWithAddress(walletId, "yFundAddr")
        headerRound(handler, 100)

        val fundingTxid = ByteArray(32) { 77 }
        val p = makeOutpoint(fundingTxid, 0)
        val winner = ByteArray(32) { 79 }
        seedSweptTombstone(p, ByteArray(32) { 78 }, winner, winnerMinedHeight = 400)
        assertEquals(
            "sanity: held, undrained, stamped with the winner's height",
            400, db.documentDao().getPendingInputsByOutpoint(p).single().winnerMinedHeight,
        )

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        headerRound(handler, 10_000)
        chainLockHeightRound(handler, 10_000)

        val coin = db.txoDao().getByOutpoint(p)
        assertNotNull("the materialised claim's row survives collection", coin)
        assertTrue("still held spent by the winner's claim", coin!!.isSpent)
        assertTrue(winner.contentEquals(coin.supersededByTxid))
    }

    @Test
    fun aTombstoneWithoutAWinnerHeightIsNeverCollected() = runTest {
        // A tombstone with a NULL stamp is never collected. The
        // mempool-context sweep path writes exactly this shape — an
        // IS-locked, unmined winner has no finality horizon to stamp —
        // and legacy rows (the v10 → v11 migration leaves pre-existing
        // tombstones NULL) read identically. With no proof of finality
        // the safe reading is to hold it forever rather than guess it
        // collectible.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 80 }
        val p = makeOutpoint(fundingTxid, 0)
        // The real writer: an IS-context sweep of a loser whose funding
        // TXO never arrived.
        seedSweptTombstone(p, ByteArray(32) { 81 }, ByteArray(32) { 82 }, winnerMinedHeight = -1)

        // Two rounds, not one: a back-filling collector (the rejected
        // design) would stamp the row on the first round and collect it
        // on the second.
        chainLockHeightRound(handler, 1_000_000)
        headerRound(handler, 1_000_000)
        headerRound(handler, 1_000_010)
        val row = db.documentDao().getPendingInputsByOutpoint(p).single()
        assertNull(
            "no winner height, no proof of finality — the hold outlasts any boundary",
            row.winnerMinedHeight,
        )
        assertTrue(row.isSweptTombstone)
    }

    // ── TXO-store reconcile (the job-flower change-drop repair) ───────

    private val changeTxid = ByteArray(32) { 7 }
    private val reconcileTip = 1_536_950

    private fun engineUtxoJson(
        txidHex: String,
        vout: Int,
        amount: Long,
        address: String = "yStxXHHzhAx58JhaPBNhn3xsH93UwBM2nd",
        height: Int = 1_534_921,
    ): String =
        """{"utxos":[{"typeTag":0,"standardTag":0,"index":0,"txid":"$txidHex","vout":$vout,""" +
            """"amount":$amount,"address":"$address","scriptHex":"76a914000088ac",""" +
            """"height":$height,"isLocked":false}]}"""

    private fun ByteArray.toHexLower() = joinToString("") { "%02x".format(it) }

    @Test
    fun reconcileHealsMissingChangeTxoAndRepairsNetAmount() = runTest {
        // A send record born blind to its own change output: netAmount
        // persisted as the full input value (the job-flower 6cef55ab…
        // shape) and NO txos row for the change.
        db.transactionDao().upsert(
            org.dashfoundation.dashsdk.persistence.entities.TransactionEntity(
                txid = changeTxid,
                transactionData = byteArrayOf(1, 2, 3),
                netAmount = -1_000_010_000L,
            ),
        )

        val report = handler.reconcileFromInventory(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 1, amount = 989_009_773L),
            tipHeight = reconcileTip,
        )

        assertEquals(1, report.inserted)
        assertEquals(989_009_773L, report.insertedDuffs)
        assertEquals(1, report.netAmountSuspects)

        val row = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 1))
        assertNotNull(row)
        assertFalse(row!!.isSpent)
        assertEquals(989_009_773L, row.amount)
        assertTrue(row.isConfirmed)

        // The stored netAmount is NOT mutated: the record may already carry
        // the corrected net (a corrective callback racing this sweep), and
        // blind addition double-credits. The suspicion is logged; the event
        // pipeline owns net correctness.
        assertEquals(
            -1_000_010_000L,
            db.transactionDao().getByTxid(changeTxid)!!.netAmount,
        )
    }

    @Test
    fun aRepointedTombstoneIsRestampedToTheLaterSweep() = runTest {
        // A chained sweep that re-points a still-unfunded claim to a new
        // BLOCK-CONTEXT winner also re-stamps it with THAT winner's mined
        // height: the claim now belongs to a spend anchored at a later
        // block, and its collection horizon moves with it.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 86 }
        val p = makeOutpoint(fundingTxid, 0)
        val firstLoser = ByteArray(32) { 87 }
        val secondLoser = ByteArray(32) { 88 }
        val finalWinner = ByteArray(32) { 89 }
        seedSweptTombstone(p, firstLoser, secondLoser, winnerMinedHeight = 400)
        assertEquals(
            "sanity: stamped with the first winner's mined height",
            400, db.documentDao().getPendingInputsByOutpoint(p).single().winnerMinedHeight,
        )

        // The first winner's own record, then its sweep — mined 50 blocks
        // later — the carry-forward path that re-points the earlier
        // tombstone.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, secondLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_091,
            p, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(secondLoser), finalWinner, emptyList(), 450)
        handler.onChangesetEnd(walletId, success = true)

        val rows = db.documentDao().getPendingInputsByOutpoint(p)
        assertTrue("sanity: the claim survives the chained sweep", rows.isNotEmpty())
        for (row in rows) {
            assertTrue(row.isSweptTombstone)
            assertTrue(finalWinner.contentEquals(row.spendingTxid))
            assertEquals(
                "re-pointed ⇒ re-stamped to the later WINNER'S mined height",
                450, row.winnerMinedHeight,
            )
        }
    }

    @Test
    fun aBlockContextTombstoneOutlivesUnrelatedAdvancementBelowItsWinnersHeight() = runTest {
        // The reviewer's unrelated-advancement scenario: the chainlock can
        // run arbitrarily far ahead, but while the synced height sits
        // below the winner's mined height the boundary has not reached the
        // spend and the hold must survive — the funding output could still
        // be delivered by the unscanned range. It collects the moment the
        // scan catches up.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 111 }
        val p = makeOutpoint(fundingTxid, 0)
        seedSweptTombstone(p, ByteArray(32) { 112 }, ByteArray(32) { 113 }, winnerMinedHeight = 400)

        // Chainlocks race ahead by thousands of blocks; the filter scan
        // has only reached one block short of the winner.
        chainLockHeightRound(handler, 10_400)
        headerRound(handler, 399)
        assertEquals(
            "min(chainlock, synced) = 399 is below the winner's height 400 — any " +
                "amount of unrelated chainlock progress must not collect the hold",
            1, db.documentDao().getPendingInputsByOutpoint(p).size,
        )

        headerRound(handler, 400)
        assertTrue(
            "the scan reaching the winner's height completes the boundary and collects",
            db.documentDao().getPendingInputsByOutpoint(p).isEmpty(),
        )
    }

    @Test
    fun shouldNotClobberMarketplaceColumnsWhenAnIdentitySnapshotStillCarriesTheLabel() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 26 }
        val documentId = ByteArray(32) { 27 }
        val recipientId = ByteArray(32) { 28 }

        persistIdentitySnapshot(identityId, "Carol")
        persistMarketplaceRow(
            identityId, documentId, "Carol", "car01",
            status = 2, counterpartyId = recipientId, priceCredits = 9_000,
        )

        // A later identity flush still carries the label — the two lanes
        // disagree for a round. The canonical branch refreshes only
        // acquiredAt/label (+ isOwned); it used to blank saleStatusRaw and
        // counterpartyIdentityId on every such flush.
        persistIdentitySnapshot(identityId, "Carol")

        val row = db.dpnsNameDao().getByDocumentId(documentId)
        assertNotNull(row)
        assertEquals(2, row!!.saleStatusRaw)
        assertTrue(recipientId.contentEquals(row.counterpartyIdentityId!!))
        assertEquals(9_000L, row.priceCredits)
        assertEquals(300L, row.documentTransferredAtMs)
        assertTrue("the identity lane still asserts ownership", row.isOwned)
    }

    // ── isLocal: wallet-link promotion + load-path heal ───────────────
    //
    // Swift parity with `persistIdentities`
    // (PlatformWalletPersistenceHandler.swift:1827-1829) and
    // `healIdentityIsLocalFlags` (:4688, called from loadWalletList :4719).

    @Test
    fun shouldMarkAPersisterCreatedWalletLinkedIdentityLocal() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 30 }

        persistIdentitySnapshot(identityId)

        val row = db.identityDao().getByIdentityId(identityId)!!
        assertTrue(walletId.contentEquals(row.walletId!!))
        assertTrue("a wallet's own identity is always local", row.isLocal)
    }

    @Test
    fun shouldKeepAnObservedOutOfWalletIdentityNonLocal() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 31 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId, identityId, 1, 0, false, 0, 0, false, ByteArray(32),
            emptyArray(), longArrayOf(), false, null, null, null,
            ByteArray(32), false, ByteArray(8), false, null,
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.identityDao().getByIdentityId(identityId)!!
        assertNull(row.walletId)
        assertFalse("an observed identity is not local", row.isLocal)
    }

    @Test
    fun shouldPromoteAnAlreadyPersistedIdentityToLocalOnLaterWalletLinkage() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val identityId = ByteArray(32) { 32 }

        handler.onChangesetBegin(walletId)
        handler.onPersistIdentityUpsert(
            walletId, identityId, 1, 0, false, 0, 0, false, ByteArray(32),
            emptyArray(), longArrayOf(), false, null, null, null,
            ByteArray(32), false, ByteArray(8), false, null,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertFalse(db.identityDao().getByIdentityId(identityId)!!.isLocal)

        // The wallet relationship attaches on a later flush — promote.
        persistIdentitySnapshot(identityId)

        assertTrue(db.identityDao().getByIdentityId(identityId)!!.isLocal)
    }

    @Test
    fun shouldHealLegacyIsLocalFalseOnWalletLinkedRowsOnlyDuringLoad() = runTest {
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val legacyOwned = ByteArray(32) { 33 }
        val manualAdd = ByteArray(32) { 34 }
        val observed = ByteArray(32) { 35 }

        // Rows exactly as the pre-fix persister wrote them: a constant
        // `false` even on the wallet's own identities.
        db.identityDao().upsert(
            IdentityEntity(
                identityId = legacyOwned,
                networkRaw = testnet,
                walletId = walletId,
                isLocal = false,
            ),
        )
        db.identityDao().upsert(
            IdentityEntity(
                identityId = manualAdd,
                networkRaw = testnet,
                walletId = null,
                isLocal = true,
            ),
        )
        db.identityDao().upsert(
            IdentityEntity(
                identityId = observed,
                networkRaw = testnet,
                walletId = null,
                isLocal = false,
            ),
        )

        handler.onLoadWalletList()

        assertTrue(
            "a wallet-linked legacy row is promoted",
            db.identityDao().getByIdentityId(legacyOwned)!!.isLocal,
        )
        assertTrue(
            "a manual add (no wallet link) keeps its flag",
            db.identityDao().getByIdentityId(manualAdd)!!.isLocal,
        )
        assertFalse(
            "an observed row is never promoted",
            db.identityDao().getByIdentityId(observed)!!.isLocal,
        )

        // Promote-only and idempotent: a second pass matches nothing.
        assertEquals(0, db.identityDao().healIsLocalFlags())
    }

    @Test
    fun aMempoolContextSweepPreservesAnUnstampedTombstone() = runTest {
        // A mempool-context sweep — an InstantSend-locked winner that has
        // not mined — preserves an UNSTAMPED tombstone for every
        // held-but-unfunded input. Under DIP-10 the IS lock alone settles
        // those inputs: upstream deletes the loser and retains them in the
        // account's `spent_outpoints`, a hold with no height that no
        // record survives to rebuild (the winner need not be
        // wallet-relevant). The tombstone is that hold's only durable
        // carrier — CORE_SWEEP_REMOVAL requires every non-released input
        // to keep a durable spend claim before its funding TXO
        // materializes — and it is unstamped because an IS-locked winner
        // has no mining deadline, so no boundary may ever collect it.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        for (i in 0 until 3) {
            val p = makeOutpoint(ByteArray(32) { (114 + i).toByte() }, 0)
            seedSweptTombstone(
                p,
                ByteArray(32) { (117 + i).toByte() },
                ByteArray(32) { (120 + i).toByte() },
                winnerMinedHeight = -1,
            )
            val row = db.documentDao().getPendingInputsByOutpoint(p).single()
            assertTrue(
                "an unmined IS-locked winner must leave a held tombstone for input #$i",
                row.isSweptTombstone,
            )
            assertNull("and it carries no finality stamp", row.winnerMinedHeight)
        }
        // Arbitrary chainlock/height advancement never collects an
        // unstamped hold — two rounds, so a back-filling collector would
        // be caught too.
        chainLockHeightRound(handler, 1_000_000)
        headerRound(handler, 1_000_000)
        headerRound(handler, 1_000_010)
        assertEquals(
            "every unstamped hold outlasts any boundary — only funding " +
                "materialization, a block-context re-stamp, or a release resolves one",
            3L, db.documentDao().countPendingInputs().first(),
        )
    }

    @Test
    fun aMempoolContextSweepStillSpendMarksAMaterialisedCoin() = runTest {
        // The mempool-context sweep still spend-marks a coin that HAS
        // materialised: the row carries real funding data, so holding it
        // costs nothing an attacker controls, and the winner's eventual
        // block delivery is the durable evidence. Only the never-funded
        // tombstone is what the mempool path refuses to create.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 123 }
        val p = makeOutpoint(fundingTxid, 0)
        val loser = ByteArray(32) { 124 }
        val winner = ByteArray(32) { 125 }

        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 50_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        seedSweptTombstone(p, loser, winner, winnerMinedHeight = -1)

        val coin = db.txoDao().getByOutpoint(p)!!
        assertTrue(
            "a materialised coin is spend-marked by the IS-locked winner",
            coin.isSpent,
        )
        assertTrue(winner.contentEquals(coin.supersededByTxid))
        assertTrue(
            "and no pending tombstone rides alongside the real row",
            db.documentDao().getPendingInputsByOutpoint(p).isEmpty(),
        )
        assertTrue(handler.onLoadWalletList().single().utxos.isEmpty())
    }

    @Test
    fun aFundingOutputArrivingAfterAMempoolSweepAndRestartLandsSpent() = runTest {
        // The reviewer's named regression: an IS-locked winner sweeps on
        // the mempool path and never mines, the app restarts, chainlocks
        // and heights advance arbitrarily, and only then is the funding
        // output delivered. Under DIP-10 the IS lock already settled that
        // input — upstream deleted the loser and retained the hold in the
        // account's `spent_outpoints`, a set rebuilt from records on load
        // that no surviving record can reconstruct. The unstamped
        // tombstone is the claim's only durable carrier, so the funding
        // delivery must drain INTO it and land spent: crediting the coin
        // would hand coin selection an outpoint the network has provably
        // consumed.
        seedWalletWithAddress(walletId, "yFundAddr")

        val fundingTxid = ByteArray(32) { 126 }
        val p = makeOutpoint(fundingTxid, 0)
        val winner = ByteArray(32) { 0x7F }
        seedSweptTombstone(p, ByteArray(32) { 127 }, winner, winnerMinedHeight = -1)
        val tombstone = db.documentDao().getPendingInputsByOutpoint(p).single()
        assertTrue("sanity: the mempool-context sweep left a tombstone", tombstone.isSweptTombstone)
        assertNull("unstamped — no finality horizon exists", tombstone.winnerMinedHeight)

        // Restart: a fresh handler bound to the same underlying store —
        // this suite's restart idiom (see
        // sweptSpendBeforeFundingSurvivesRestartAndStaysSpentWhenFunded).
        val restarted = newHandler()

        // Arbitrary chainlock/height advancement while the winner stays
        // unmined — none of it may collect the unstamped hold.
        headerRound(restarted, 25_000)
        restarted.onChangesetBegin(walletId)
        restarted.onWalletChangesetChainLockHeight(walletId, 25_000)
        restarted.onChangesetEnd(walletId, success = true)
        assertEquals(
            "the unstamped hold survives the restart and every boundary",
            1, db.documentDao().getPendingInputsByOutpoint(p).size,
        )

        // The funding output is finally delivered and classified: it must
        // drain into the tombstone and stay spent.
        restarted.onChangesetBegin(walletId)
        restarted.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 50_000, "yFundAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        restarted.onChangesetEnd(walletId, success = true)

        val coin = db.txoDao().getByOutpoint(p)
        assertNotNull(coin)
        assertTrue(
            "an input the IS-locked winner consumed must never come back " +
                "spendable — the sweep's claim outlives the restart",
            coin!!.isSpent,
        )
        assertTrue(
            "held by the winner the sweep named",
            winner.contentEquals(coin.supersededByTxid),
        )
        assertTrue(
            "the claim drained into the TXO row",
            db.documentDao().getPendingInputsByOutpoint(p).isEmpty(),
        )
        assertTrue(
            "a spent coin never reaches the restored UTXO set",
            restarted.onLoadWalletList().single().utxos.isEmpty(),
        )
    }

    @Test
    fun aMempoolRepointedTombstoneKeepsItsBlockContextStamp() = runTest {
        // The IS-locked half of the chained case: an unmined winner
        // re-points the claim but must NOT disturb the earlier
        // block-context stamp — upstream's observed-spend entry is never
        // retracted by an unconfirmed conflict. Collection at the retained
        // height stays sound (the funding output is mined at or below the
        // FIRST spender's height regardless of who claims the coin now),
        // so the row still collects at that boundary.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 106 }
        val p = makeOutpoint(fundingTxid, 0)
        val firstLoser = ByteArray(32) { 107 }
        val secondLoser = ByteArray(32) { 108 }
        val finalWinner = ByteArray(32) { 109 }
        seedSweptTombstone(p, firstLoser, secondLoser, winnerMinedHeight = 400)

        // The first winner is evicted by an IS-locked, unmined conflict
        // that also claims the unfunded input.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, secondLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_092,
            p, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(secondLoser), finalWinner, emptyList(), -1)
        handler.onChangesetEnd(walletId, success = true)

        val rows = db.documentDao().getPendingInputsByOutpoint(p)
            .filter { it.isSweptTombstone }
        assertTrue("sanity: the tombstone survives the chained sweep", rows.isNotEmpty())
        for (row in rows) {
            assertTrue(
                "an unmined winner re-points the claim",
                finalWinner.contentEquals(row.spendingTxid),
            )
            assertEquals(
                "without touching the earlier block-context stamp",
                400, row.winnerMinedHeight,
            )
        }

        chainLockHeightRound(handler, 10_000)
        headerRound(handler, 400)
        assertTrue(
            "the retained stamp still bounds the row: the funding output sits at " +
                "or below the first spender's height, so the boundary reaching it " +
                "proves delivery-or-never",
            db.documentDao().getPendingInputsByOutpoint(p)
                .none { it.isSweptTombstone },
        )
    }

    @Test
    fun anUnstampedTombstoneRestampedByABlockContextSweepBecomesCollectible() = runTest {
        // The other direction of the chained case: an UNSTAMPED hold
        // (IS-context sweep) re-pointed by a later BLOCK-context sweep
        // gains that winner's stamp — the claim now belongs to a spend
        // anchored in a real block, so it enters the collectible set and
        // the boundary reaching the new winner's height collects it. One
        // of the three resolution channels that bound the unstamped
        // population.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)

        val fundingTxid = ByteArray(32) { 115 }
        val p = makeOutpoint(fundingTxid, 0)
        val firstLoser = ByteArray(32) { 116 }
        val secondLoser = ByteArray(32) { 118 }
        val finalWinner = ByteArray(32) { 119 }
        seedSweptTombstone(p, firstLoser, secondLoser, winnerMinedHeight = -1)
        assertNull(
            "sanity: held and unstamped",
            db.documentDao().getPendingInputsByOutpoint(p).single().winnerMinedHeight,
        )

        // The IS-locked first winner is itself beaten by a mined conflict
        // still claiming the unfunded input.
        handler.onChangesetBegin(walletId)
        recordTransaction(
            handler,
            walletId, secondLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_093,
            p, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        sweep(handler, walletId, listOf(secondLoser), finalWinner, emptyList(), 450)
        handler.onChangesetEnd(walletId, success = true)

        val rows = db.documentDao().getPendingInputsByOutpoint(p)
            .filter { it.isSweptTombstone }
        assertTrue("sanity: the claim survives the chained sweep", rows.isNotEmpty())
        for (row in rows) {
            assertEquals(
                "the block-context re-point stamps the previously unstamped hold",
                450, row.winnerMinedHeight,
            )
        }

        chainLockHeightRound(handler, 10_000)
        headerRound(handler, 450)
        assertTrue(
            "once stamped, the ordinary finality boundary collects the row",
            db.documentDao().getPendingInputsByOutpoint(p).none { it.isSweptTombstone },
        )
    }

    @Test
    fun onWalletChangesetChainLockHeightStoresMonotonicMaxOnTheWalletRow() = runTest {
        // The numeric chainlock height is the finality half of the
        // collection boundary, so a stale round's chainlock must never
        // lower it — monotonic max, matching the SQLite store's
        // `upsert_sync_state`.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        assertNull(
            "no height on record until the slot fires",
            db.walletDao().getByWalletId(walletId)!!.lastAppliedChainLockHeight,
        )

        chainLockHeightRound(handler, 500)
        assertEquals(500, db.walletDao().getByWalletId(walletId)!!.lastAppliedChainLockHeight)

        chainLockHeightRound(handler, 400)
        assertEquals(
            "a stale round must not lower the stored height",
            500, db.walletDao().getByWalletId(walletId)!!.lastAppliedChainLockHeight,
        )

        chainLockHeightRound(handler, 600)
        assertEquals(600, db.walletDao().getByWalletId(walletId)!!.lastAppliedChainLockHeight)
    }

    @Test
    fun reconcileIsIdempotentAndNeverDoubleCredits() = runTest {
        db.transactionDao().upsert(
            org.dashfoundation.dashsdk.persistence.entities.TransactionEntity(
                txid = changeTxid,
                transactionData = byteArrayOf(1),
                netAmount = -1_000_010_000L,
            ),
        )
        val json = engineUtxoJson(changeTxid.toHexLower(), vout = 1, amount = 989_009_773L)

        handler.reconcileFromInventory(walletId, json, tipHeight = reconcileTip)
        val second = handler.reconcileFromInventory(walletId, json, tipHeight = reconcileTip)

        assertEquals(0, second.inserted)
        assertEquals(0, second.netAmountSuspects)
        assertEquals(
            -1_000_010_000L,
            db.transactionDao().getByTxid(changeTxid)!!.netAmount,
        )
    }

    @Test
    fun reconcileSkipsImmatureOutputsAndPreservesSpentRows() = runTest {
        // Immature: inside the 100-conf gate (flags on the engine snapshot
        // can't carry coinbase/IS-lock, so fresh rows wait for a later
        // sweep) — nothing inserted.
        val fresh = handler.reconcileFromInventory(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 0, amount = 5L, height = reconcileTip - 3),
            tipHeight = reconcileTip,
        )
        assertEquals(0, fresh.inserted)
        assertEquals(1, fresh.skippedImmature)
        assertNull(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 0)))

        // A row the mirror already holds — even marked spent while the
        // engine still lists it — is left untouched: reconcile is
        // insert-only and never flips spend state.
        assertEquals(
            0,
            handler.onWalletChangesetUtxoAdded(
                walletId, changeTxid, 2, 42L, "yTestAddr", byteArrayOf(0x51), 1_500_000,
                false, true, false, false,
            ),
        )
        val seeded = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 2))!!
        db.txoDao().upsert(seeded.copy(isSpent = true))

        val report = handler.reconcileFromInventory(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 2, amount = 42L, height = 1_500_000),
            tipHeight = reconcileTip,
        )
        assertEquals(0, report.inserted)
        assertTrue(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 2))!!.isSpent)
    }

    /** Engine inventory JSON with both halves: unspent rows and spent outpoints. */
    private fun engineInventoryJson(unspent: List<Triple<String, Int, Long>>, spent: List<Pair<String, Int>>): String {
        val utxoRows = unspent.joinToString(",") { (txid, vout, amount) ->
            """{"typeTag":0,"standardTag":0,"index":0,"txid":"$txid","vout":$vout,""" +
                """"amount":$amount,"address":"yStxXHHzhAx58JhaPBNhn3xsH93UwBM2nd",""" +
                """"scriptHex":"76a914000088ac","height":1400000,"isLocked":false}"""
        }
        val spentRows = spent.joinToString(",") { (txid, vout) ->
            """{"txid":"$txid","vout":$vout}"""
        }
        return """{"utxos":[$utxoRows],"spent":[$spentRows]}"""
    }

    @Test
    fun reconcileLogsButNeverFlipsLostSpendRows() = runTest {
        // A store row still marked unspent for a coin the engine records as
        // spent (dashpay/platform#4425). The engine's spent set includes
        // MEMPOOL spends and carries no context, so persisting the flip
        // would settle an unconfirmed spend — counted and logged only.
        handler.onWalletChangesetUtxoAdded(
            walletId, changeTxid, 3, 500_000L, "yTestAddr", byteArrayOf(0x51), 1_400_000,
            false, true, false, false,
        )
        val report = handler.reconcileFromInventory(
            walletId,
            engineInventoryJson(unspent = emptyList(), spent = listOf(changeTxid.toHexLower() to 3)),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.wouldFlipSpent)
        assertEquals(500_000L, report.wouldFlipSpentDuffs)
        assertEquals(0, report.wouldRemove)
        val row = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 3))!!
        assertFalse("the row must stay unspent — the flip is log-only", row.isSpent)
        assertNull(row.spendingTxid)
    }

    @Test
    fun reconcileLogsButNeverRemovesEngineUnknownRows() = runTest {
        // A store row for a coin the engine has in NEITHER inventory —
        // residue of a swept/abandoned transaction (pre-rust-dashcore#971
        // stores). Counted and logged, NEVER removed.
        handler.onWalletChangesetUtxoAdded(
            walletId, changeTxid, 4, 250_000L, "yTestAddr", byteArrayOf(0x51), 1_400_000,
            false, true, false, false,
        )
        val report = handler.reconcileFromInventory(
            walletId,
            engineInventoryJson(unspent = emptyList(), spent = emptyList()),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.wouldRemove)
        assertEquals(250_000L, report.wouldRemoveDuffs)
        assertEquals(0, report.wouldFlipSpent)
        val row = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 4))!!
        assertFalse(row.isSpent)
        assertEquals(250_000L, row.amount)
    }

    @Test
    fun reconcileReversePassIsSilentOnConsistentStore() = runTest {
        // Rows the engine also holds unspent — including a YOUNG coin the
        // insert pass would skip as immature — are consistent, not
        // divergence. Every reverse-pass counter must be zero.
        handler.onWalletChangesetUtxoAdded(
            walletId, changeTxid, 5, 42L, "yTestAddr", byteArrayOf(0x51), reconcileTip - 3,
            false, true, false, false,
        )
        val json =
            """{"utxos":[{"typeTag":0,"standardTag":0,"index":0,""" +
                """"txid":"${changeTxid.toHexLower()}","vout":5,"amount":42,""" +
                """"address":"yTestAddr","scriptHex":"51",""" +
                """"height":${reconcileTip - 3},"isLocked":false}],"spent":[]}"""
        val report = handler.reconcileFromInventory(walletId, json, tipHeight = reconcileTip)
        assertEquals(0, report.wouldFlipSpent)
        assertEquals(0, report.wouldRemove)
        assertEquals(1, report.skippedImmature)
        assertFalse(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 5))!!.isSpent)
    }

    @Test
    fun reconcileExcludesWatchOnlyContactRowsFromReversePass() = runTest {
        // Watch-only DIP-15 contact rows are never in the engine's
        // inventories; flagging them would be a false positive on every
        // wallet with contact payments.
        db.walletDao().upsert(WalletEntity(walletId, networkRaw = Network.TESTNET.ffiValue))
        val foreignAccountId = db.accountDao().insert(
            org.dashfoundation.dashsdk.persistence.entities.AccountEntity(
                walletId = walletId,
                accountType = PlatformWalletPersistenceHandler.ACCOUNT_TYPE_TAG_DASHPAY_EXTERNAL,
                accountIndex = 0,
                accountTypeName = "DashpayExternalAccount",
            ),
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, changeTxid, 6, 1_230_000L, "yContactAddr", byteArrayOf(0x51), 1_400_000,
            false, true, false, false,
        )
        val seeded = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 6))!!
        db.txoDao().upsert(seeded.copy(accountId = foreignAccountId))

        val report = handler.reconcileFromInventory(
            walletId,
            engineInventoryJson(unspent = emptyList(), spent = emptyList()),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.skippedForeign)
        assertEquals(0, report.wouldRemove)
        assertFalse(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 6))!!.isSpent)
    }

    @Test
    fun reconcileResolvesContactOwnershipThroughCoreAddressId() = runTest {
        // Production changeset writes leave txos.accountId null and route
        // ownership through coreAddressId -> core_addresses.accountId. The
        // exclusion must resolve that path, or every contact row gets
        // classified as divergence.
        db.walletDao().upsert(WalletEntity(walletId, networkRaw = Network.TESTNET.ffiValue))
        val foreignAccountId = db.accountDao().insert(
            org.dashfoundation.dashsdk.persistence.entities.AccountEntity(
                walletId = walletId,
                accountType = PlatformWalletPersistenceHandler.ACCOUNT_TYPE_TAG_DASHPAY_EXTERNAL,
                accountIndex = 1,
                accountTypeName = "DashpayExternalAccount",
            ),
        )
        db.coreAddressDao().upsert(
            org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity(
                address = "yContactRouted",
                publicKey = ByteArray(33),
                poolTypeTag = 0,
                addressIndex = 0,
                derivationPath = "m/9'/1'/15'/0'/x/y/0",
                isUsed = true,
                accountId = foreignAccountId,
            ),
        )
        handler.onWalletChangesetUtxoAdded(
            walletId, changeTxid, 8, 990_000L, "yContactRouted", byteArrayOf(0x51), 1_400_000,
            false, true, false, false,
        )
        val seeded = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 8))!!
        assertNull("production shape: accountId is null", seeded.accountId)

        val report = handler.reconcileFromInventory(
            walletId,
            engineInventoryJson(unspent = emptyList(), spent = emptyList()),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.skippedForeign)
        assertEquals(0, report.wouldRemove)
    }

    @Test
    fun reconcileInsertPassNeverHealsContactAccountCoins() = runTest {
        // The engine's UTXO inventory export includes the watch-only DIP-15
        // external accounts' coins — it tracks them to show payments TO
        // contacts, but they are the CONTACT's money. With the foreign
        // exclusion living only on the reverse pass, a fresh restore's
        // post-backfill reconcile healed every contact-payment coin into the
        // store as an ownerless row (12 rows / 5,692,493 duffs on the
        // 2026-08-25 large-wallet validation run) while the reverse pass
        // counted the very same rows as foreign — and the mirror-reload path
        // hands such rows back to the engine on the next launch. The insert
        // pass must skip any engine UTXO whose address resolves to an
        // external account, and count it as foreign, not healed.
        db.walletDao().upsert(WalletEntity(walletId, networkRaw = Network.TESTNET.ffiValue))
        val foreignAccountId = db.accountDao().insert(
            org.dashfoundation.dashsdk.persistence.entities.AccountEntity(
                walletId = walletId,
                accountType = PlatformWalletPersistenceHandler.ACCOUNT_TYPE_TAG_DASHPAY_EXTERNAL,
                accountIndex = 2,
                accountTypeName = "DashpayExternalAccount",
            ),
        )
        db.coreAddressDao().upsert(
            org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity(
                address = "yContactPaid",
                publicKey = ByteArray(33),
                poolTypeTag = 0,
                addressIndex = 0,
                derivationPath = "m/9'/1'/15'/0'/x/y/1",
                isUsed = true,
                accountId = foreignAccountId,
            ),
        )

        val json =
            """{"utxos":[{"typeTag":0,"standardTag":0,"index":0,""" +
                """"txid":"${changeTxid.toHexLower()}","vout":9,"amount":10000,""" +
                """"address":"yContactPaid","scriptHex":"51",""" +
                """"height":1400000,"isLocked":false}],"spent":[]}"""
        val report = handler.reconcileFromInventory(walletId, json, tipHeight = reconcileTip)

        assertEquals(0, report.inserted)
        assertEquals(0L, report.insertedDuffs)
        assertEquals(1, report.skippedForeign)
        assertNull(
            "the contact's coin must not enter the store",
            db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 9)),
        )
    }

    @Test
    fun reconcileStampsResolvedAccountOnHealedRows() = runTest {
        // The blocking scenario from review: persistence lost BOTH the TXO
        // and its address row. The heal must resolve the owning Room account
        // from the inventory's account tuple and stamp it on the inserted
        // row — a healed row with neither accountId nor a resolvable address
        // is skipped by the restore loader at the next mirror-reload,
        // recreating the fund loss the heal repaired.
        db.walletDao().upsert(WalletEntity(walletId, networkRaw = Network.TESTNET.ffiValue))
        // Production shape: onPersistAccountRegistration stores the FFI's
        // 32-zero-byte identity ids verbatim (the entity ctor default of an
        // EMPTY array never occurs on persisted rows).
        val bip44Id = db.accountDao().insert(
            org.dashfoundation.dashsdk.persistence.entities.AccountEntity(
                walletId = walletId,
                accountType = 0,
                accountIndex = 0,
                accountTypeName = "standardBip44",
                userIdentityId = ByteArray(32),
                friendIdentityId = ByteArray(32),
            ),
        )
        // Deliberately NO core_addresses row for this address.
        val report = handler.reconcileFromInventory(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 11, amount = 70_000L, address = "yOrphanAddr"),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.inserted)
        assertEquals(0, report.healedUnowned)
        assertEquals(
            "the healed row must carry the account resolved from the inventory tuple",
            bip44Id, db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 11))!!.accountId,
        )
    }

    @Test
    fun reconcileCountsHealsWhoseAccountCannotBeResolved() = runTest {
        // No matching Room account row at all (a store damaged past the
        // account registrations): the heal proceeds — the address projection
        // may still attribute it — but the unresolved owner is surfaced.
        db.walletDao().upsert(WalletEntity(walletId, networkRaw = Network.TESTNET.ffiValue))
        val report = handler.reconcileFromInventory(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 12, amount = 5_000L),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.inserted)
        assertEquals(1, report.healedUnowned)
        assertNull(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 12))!!.accountId)
    }

    @Test
    fun reconcileForeignSkipKeysOffTheInventoryTagWithoutAddressRow() = runTest {
        // The tag is the authoritative foreign check: a contact's coin must
        // be skipped even when its address row never survived persistence
        // (the case the address-based fallback cannot see).
        val json =
            """{"utxos":[{"typeTag":13,"standardTag":0,"index":0,""" +
                """"userIdentityId":"${"11".repeat(32)}","friendIdentityId":"${"22".repeat(32)}",""" +
                """"txid":"${changeTxid.toHexLower()}","vout":13,"amount":10000,""" +
                """"address":"yContactNoRow","scriptHex":"51",""" +
                """"height":1400000,"isLocked":false}],"spent":[]}"""
        val report = handler.reconcileFromInventory(walletId, json, tipHeight = reconcileTip)

        assertEquals(0, report.inserted)
        assertEquals(1, report.skippedForeign)
        assertNull(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 13)))
    }

    @Test
    fun reconcileNeverUnmarksSpentRowsEvenWhenEngineDisagrees() = runTest {
        // A row marked spent while the engine lists the coin unspent: either
        // a lost release event (pre-rust-dashcore#971) or a live spend the
        // store wrote before the engine settled. Un-marking a coin
        // mid-payment would let the wallet double-spend it, so this is
        // counted and logged but NEVER changed.
        handler.onWalletChangesetUtxoAdded(
            walletId, changeTxid, 7, 77_000L, "yTestAddr", byteArrayOf(0x51), 1_400_000,
            false, true, false, false,
        )
        val seeded = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 7))!!
        db.txoDao().upsert(seeded.copy(isSpent = true))

        val report = handler.reconcileFromInventory(
            walletId,
            engineInventoryJson(
                unspent = listOf(Triple(changeTxid.toHexLower(), 7, 77_000L)),
                spent = emptyList(),
            ),
            tipHeight = reconcileTip,
        )
        assertEquals(1, report.stuckSpent)
        assertEquals(77_000L, report.stuckSpentDuffs)
        assertTrue(
            "the row must stay spent — un-marking is never done by reconciliation",
            db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 7))!!.isSpent,
        )
    }

    // ── Paged reconcile transport ─────────────────────────────────────

    @Test
    fun reconcileWalksEveryPageOfTheEngineInventory() = runTest {
        // The engine inventory is chain-controlled in size, so the sweep
        // reads it a page at a time. Every page must be applied — a sweep
        // that healed only the first one would leave most of a damaged
        // mirror unrepaired, and silently.
        val engine = FakeEngine(
            engineInventoryJson(
                unspent = (0 until 5).map { Triple(changeTxid.toHexLower(), 20 + it, 1_000L) },
                spent = emptyList(),
            ),
        )
        val report = handler.reconcileTxos(
            walletId = walletId,
            tipHeight = reconcileTip,
            pageSize = 2,
            engineUtxoPage = engine::page,
            classifyOutpoints = engine::classify,
        )!!

        assertEquals("3 pages for 5 rows at 2 per page", 3, engine.pages)
        assertEquals(5, report.engineUtxos)
        assertEquals(5, report.inserted)
        assertEquals(5_000L, report.insertedDuffs)
        for (vout in 20 until 25) {
            assertNotNull(
                "the row at vout=$vout must be healed whichever page carried it",
                db.txoDao().getByOutpoint(makeOutpoint(changeTxid, vout)),
            )
        }
    }

    @Test
    fun reconcileStopsAtAFailedPageAndKeepsWhatItAlreadyHealed() = runTest {
        // A transport that dies mid-sweep must not discard the pages that
        // already landed — the pass is insert-only and idempotent, so they
        // are already correct — and must not report a clean run either.
        val engine = FakeEngine(
            engineInventoryJson(
                unspent = (0 until 4).map { Triple(changeTxid.toHexLower(), 30 + it, 500L) },
                spent = emptyList(),
            ),
        )
        val report = handler.reconcileTxos(
            walletId = walletId,
            tipHeight = reconcileTip,
            pageSize = 2,
            engineUtxoPage = { cursor, limit ->
                if (cursor == null) engine.page(cursor, limit) else null
            },
            classifyOutpoints = engine::classify,
        )!!

        assertEquals(1, report.transportFailures)
        assertEquals("only the page that arrived", 2, report.inserted)
        assertNotNull(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 30)))
        assertNull(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 32)))
    }

    @Test
    fun reconcileClassifiesStoreRowsInBoundedBatches() = runTest {
        // The reverse direction is inverted: the STORE is paged and the
        // engine is asked about one page at a time, so neither side builds
        // a set over a whole inventory. Every batch must still be answered
        // and counted.
        for (vout in 40 until 43) {
            handler.onWalletChangesetUtxoAdded(
                walletId, changeTxid, vout, 100L, "yTestAddr", byteArrayOf(0x51), 1_400_000,
                false, true, false, false,
            )
        }
        val engine = FakeEngine(engineInventoryJson(unspent = emptyList(), spent = emptyList()))
        val report = handler.reconcileTxos(
            walletId = walletId,
            tipHeight = reconcileTip,
            pageSize = 1,
            engineUtxoPage = engine::page,
            classifyOutpoints = engine::classify,
        )!!

        assertEquals("one classification batch per store page", 3, engine.batches)
        assertEquals("every store row reached the classifier", 3, engine.classified)
        assertEquals(3, report.wouldRemove)
        assertEquals(300L, report.wouldRemoveDuffs)
    }

    @Test
    fun reconcileStopsWhenAClassificationBatchFails() = runTest {
        // No verdicts, no classification: the reverse pass stops rather
        // than guessing at rows it could not ask the engine about.
        for (vout in 50 until 53) {
            handler.onWalletChangesetUtxoAdded(
                walletId, changeTxid, vout, 100L, "yTestAddr", byteArrayOf(0x51), 1_400_000,
                false, true, false, false,
            )
        }
        val engine = FakeEngine(engineInventoryJson(unspent = emptyList(), spent = emptyList()))
        val report = handler.reconcileTxos(
            walletId = walletId,
            tipHeight = reconcileTip,
            pageSize = 1,
            engineUtxoPage = engine::page,
            classifyOutpoints = { null },
        )!!

        assertEquals(1, report.transportFailures)
        assertEquals(0, report.wouldRemove)
    }

    @Test
    fun reconcileReturnsNoReportWhenTheFirstPageIsUnavailable() = runTest {
        // No first page means nothing to reconcile against — the contract
        // callers had before the sweep was paged, now enforced inside the
        // handler rather than by a prefetch in the caller.
        var classifyCalls = 0
        val report = handler.reconcileTxos(
            walletId = walletId,
            tipHeight = reconcileTip,
            engineUtxoPage = { _, _ -> null },
            classifyOutpoints = { outpoints -> classifyCalls++; ByteArray(outpoints.size / 36) },
        )
        assertNull("a dead transport yields no report", report)
        assertEquals("and the classification pass never runs", 0, classifyCalls)
    }

    @Test
    fun reconcileRejectsAMalformedInventoryRowInsteadOfHealingIt() = runTest {
        // The transport is a typed contract on both ends. A row missing its
        // amount (or carrying a key this reader does not know) must fail the
        // decode loudly — the alternative is a defaulted row written into
        // the mirror the engine reloads from.
        val malformed = """{"utxos":[{"typeTag":0,"txid":"${changeTxid.toHexLower()}",""" +
            """"vout":40,"address":"yStxXHHzhAx58JhaPBNhn3xsH93UwBM2nd","height":1400000}],""" +
            """"cursor":null,"hasMore":false}"""
        val thrown = runCatching {
            handler.reconcileTxos(
                walletId = walletId,
                tipHeight = reconcileTip,
                engineUtxoPage = { _, _ -> malformed },
                classifyOutpoints = { outpoints -> ByteArray(outpoints.size / 36) },
            )
        }.exceptionOrNull()
        assertTrue(
            "strict decode must throw, got $thrown",
            thrown is kotlinx.serialization.SerializationException,
        )
        assertNull(
            "nothing was healed from the malformed page",
            db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 40)),
        )
    }

    /**
     * Drive the paged reconcile from one whole-inventory JSON blob — the
     * shape these tests describe an engine in, and the shape the native
     * side used to hand over in a single unbounded call.
     *
     * The blob is served the way the transport now serves it: sliced into
     * bounded pages behind an opaque cursor, with a separate positional
     * classifier for the outpoints the store asks about. [pageSize]
     * defaults to 2, so a test describing more than a couple of rows walks
     * the real cursor loop rather than a single page.
     */
    private suspend fun PlatformWalletPersistenceHandler.reconcileFromInventory(
        walletId: ByteArray,
        inventoryJson: String,
        tipHeight: Int,
        minConfirmations: Int = 100,
        pageSize: Int = 2,
    ): PlatformWalletPersistenceHandler.TxoReconcileReport {
        val engine = FakeEngine(inventoryJson)
        return checkNotNull(
            reconcileTxos(
                walletId = walletId,
                tipHeight = tipHeight,
                minConfirmations = minConfirmations,
                pageSize = pageSize,
                engineUtxoPage = engine::page,
                classifyOutpoints = engine::classify,
            ),
        ) { "the fake engine always serves a first page" }
    }

    /**
     * A stand-in for the engine's paged inventory transport, built from the
     * whole-inventory JSON a test writes out. Pages come back behind an
     * opaque ordinal cursor — the real cursor is opaque too, the handler
     * only ever hands back what it was given — and classification answers
     * positionally out of the same two inventories: 1 unspent, 2 spent, 0
     * neither.
     */
    private class FakeEngine(inventoryJson: String) {
        private val utxos: List<kotlinx.serialization.json.JsonObject>
        private val unspentKeys: Set<String>
        private val spentKeys: Set<String>

        /** Inventory pages served, classification batches answered, and
         *  outpoints classified across those batches. */
        var pages = 0
            private set
        var batches = 0
            private set
        var classified = 0
            private set

        init {
            val root = kotlinx.serialization.json.Json
                .parseToJsonElement(inventoryJson).jsonObject
            utxos = root["utxos"]?.jsonArray?.map { it.jsonObject } ?: emptyList()
            unspentKeys = utxos.map {
                key(
                    it["txid"]!!.jsonPrimitive.content,
                    it["vout"]!!.jsonPrimitive.int,
                )
            }.toSet()
            spentKeys = (root["spent"]?.jsonArray?.toList() ?: emptyList()).map {
                key(
                    it.jsonObject["txid"]!!.jsonPrimitive.content,
                    it.jsonObject["vout"]!!.jsonPrimitive.int,
                )
            }.toSet()
        }

        fun page(cursor: String?, limit: Int): String {
            pages++
            val start = cursor?.toInt() ?: 0
            val slice = utxos.drop(start).take(limit)
            val next = start + slice.size
            val hasMore = next < utxos.size
            return """{"utxos":[${slice.joinToString(",") { it.toString() }}],""" +
                """"cursor":${if (hasMore) "\"$next\"" else "null"},"hasMore":$hasMore}"""
        }

        fun classify(outpoints: ByteArray): ByteArray {
            batches++
            val count = outpoints.size / OUTPOINT_SIZE
            classified += count
            val verdicts = ByteArray(count)
            for (i in 0 until count) {
                val base = i * OUTPOINT_SIZE
                val txidHex = outpoints.copyOfRange(base, base + 32)
                    .joinToString("") { "%02x".format(it) }
                var vout = 0
                for (b in 3 downTo 0) {
                    vout = (vout shl 8) or (outpoints[base + 32 + b].toInt() and 0xFF)
                }
                val k = key(txidHex, vout)
                verdicts[i] = when {
                    k in unspentKeys -> 1
                    k in spentKeys -> 2
                    else -> 0
                }
            }
            return verdicts
        }

        private fun key(txidHex: String, vout: Int) = "$txidHex:$vout"

        private companion object {
            /** txid (32 bytes, wire order) + vout (4 bytes, little-endian). */
            const val OUTPOINT_SIZE = 36
        }
    }
}

/**
 * Room open-helper factory that fails exactly one prepared statement —
 * the one whose SQL starts with [failingSqlPrefix] — once [armed], and
 * delegates every other read and write to real SQLite.
 *
 * Faults ONE query rather than dropping its table: `asset_locks` is read
 * by three independent restore builders, so a table-level fault proves
 * nothing about the isolation of any single one of them.
 */
private class SingleStatementFaultInjector(
    private val failingSqlPrefix: String,
) : SupportSQLiteOpenHelper.Factory {
    private val real = FrameworkSQLiteOpenHelperFactory()

    /** Off while the fixture is seeded, then flipped on by the test. */
    var armed: Boolean = false

    override fun create(
        configuration: SupportSQLiteOpenHelper.Configuration,
    ): SupportSQLiteOpenHelper = Helper(real.create(configuration))

    private inner class Helper(
        private val delegate: SupportSQLiteOpenHelper,
    ) : SupportSQLiteOpenHelper by delegate {
        override val writableDatabase: SupportSQLiteDatabase
            get() = Db(delegate.writableDatabase)
        override val readableDatabase: SupportSQLiteDatabase
            get() = Db(delegate.readableDatabase)
    }

    private inner class Db(
        private val delegate: SupportSQLiteDatabase,
    ) : SupportSQLiteDatabase by delegate {
        private fun shouldFail(query: SupportSQLiteQuery) =
            armed && query.sql.startsWith(failingSqlPrefix)

        override fun query(query: SupportSQLiteQuery): Cursor =
            if (shouldFail(query)) throw SQLiteException("injected read failure: ${query.sql}")
            else delegate.query(query)

        override fun query(
            query: SupportSQLiteQuery,
            cancellationSignal: CancellationSignal?,
        ): Cursor =
            if (shouldFail(query)) throw SQLiteException("injected read failure: ${query.sql}")
            else delegate.query(query, cancellationSignal)
    }
}
