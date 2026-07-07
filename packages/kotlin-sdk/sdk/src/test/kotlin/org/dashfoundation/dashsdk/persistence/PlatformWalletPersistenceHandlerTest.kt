package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
    }

    @After
    fun tearDown() {
        db.close()
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
        handler.onWalletChangesetTransaction(
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

        val row = db.platformAddressDao().getByAddressHash(hash)
        assertNotNull(row)
        assertEquals(12_345, row!!.balance)
        assertEquals(3, row.nonce)
        assertEquals(777, row.lastSeenHeight)
        assertTrue(row.isUsed)
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

        assertNull(db.platformAddressDao().getByAddressHash(ByteArray(20) { 44 }))
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
        override fun deriveAndStore(
            walletId: ByteArray,
            publicKeyData: ByteArray,
            identityIndex: Int,
            keyIndex: Int,
        ): String? {
            calls.add(Triple(walletId, identityIndex, keyIndex))
            lastPublicKey = publicKeyData
            return id
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)

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

    @Test
    fun loadWalletListRestoresUnspentUtxosAndExcludesConfirmedSpends() = runTest {
        // CORE-06 regression: persisted unspent TXOs must come back on
        // the restore row (routed to their owning account through
        // core_addresses — Android txos carry no accountId FK), and a
        // TXO whose spend has confirmed must NOT rehydrate as spendable.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val account = db.accountDao().observeByWallet(walletId).first().single()
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = "yUtxoAddr",
                poolTypeTag = 0,
                addressIndex = 0,
                derivationPath = "m/44'/1'/0'/0/0",
                accountId = account.id,
            ),
        )

        val fundingTxid = ByteArray(32) { 21 }
        val spendingTxid = ByteArray(32) { 22 }
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, fundingTxid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
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
        handler.onWalletChangesetTransaction(
            walletId, spendingTxid, ByteArray(10) { 5 }, 1, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_100,
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
        handler.onWalletChangesetTransaction(
            walletId, spendingTxid, ByteArray(10) { 5 }, 2, 101, ByteArray(32) { 8 },
            1_700_000_200, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_100,
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
}
