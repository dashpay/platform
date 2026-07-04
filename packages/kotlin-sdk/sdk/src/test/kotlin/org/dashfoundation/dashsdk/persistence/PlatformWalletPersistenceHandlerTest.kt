package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
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
        )
        handler.onChangesetEnd(walletId, success = true)

        val row = db.platformAddressDao().getByAddressHash(hash)
        assertNotNull(row)
        assertEquals(12_345, row!!.balance)
        assertEquals(3, row.nonce)
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
