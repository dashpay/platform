package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.wallet.PlatformWalletPersistenceCapabilities
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
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

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
    }

    @After
    fun tearDown() {
        db.close()
    }

    @Test
    fun persistenceCapabilitiesAreExplicitAndFailClosedByDefault() {
        val noOpBridge = object : NativePersistenceBridge() {}
        assertEquals(0, noOpBridge.persistenceCapabilitiesVersion())
        assertEquals(0L, noOpBridge.persistenceCapabilitiesBits())

        assertEquals(1, handler.persistenceCapabilitiesVersion())
        assertEquals(0x1bfL, handler.persistenceCapabilitiesBits())
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
            handler.onWalletChangesetTransaction(
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
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
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

    @Test
    fun rolledBackRoundScrubsDeriverWrittenAliases() = runTest {
        val deriver = FakeDeriver()
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())

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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
        val identityId = ByteArray(32) { 21 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 12 }
        upsertIdentityKey(pubkey, identityId) // derive fails → watch-only + breadcrumbs

        // Model a process restart: a fresh handler starts with an empty
        // in-memory map, then rebuilds it from the durable rows.
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, FakeDeriver())
        val identityId = ByteArray(32) { 22 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 13 }
        upsertIdentityKey(pubkey, identityId)
        assertTrue(handler.pendingIdentityKeys.value.isEmpty()) // healthy at persist time

        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { false }, // blob stranded
        )
        assertNotNull(restarted.pendingIdentityKeys.value[pubkey.toHex()])
    }

    @Test
    fun reconstructionSkipsHealthyRows() = runTest {
        // Identifier recorded AND the blob still decrypts: nothing to repair,
        // so a restart must not fabricate pending state.
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, FakeDeriver())
        val identityId = ByteArray(32) { 23 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 14 }
        upsertIdentityKey(pubkey, identityId)

        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
        val identityId = ByteArray(32) { 24 }
        seedIdentity(identityId)
        val pubkey = ByteArray(33) { 16 }
        upsertIdentityKey(pubkey, identityId)

        val row = db.publicKeyDao().getByIdentityAndKeyId(identityId.toBase58String(), 0)!!
        assertNull(row.privateKeyKeychainIdentifier)
        db.publicKeyDao().update(
            row.copy(privateKeyKeychainIdentifier = "privkey." + pubkey.toHex()),
        )

        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, FakeDeriver())
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
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
        restarted.reconstructPendingIdentityKeysFromPersistence(
            isPrivateKeyDecryptable = { true },
        )
        assertNotNull(restarted.pendingIdentityKeys.value[pubkey.toHex()])
    }

    @Test
    fun reconstructionNeverOverwritesALiveEntry() = runTest {
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, ThrowingDeriver())
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)
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
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined, deriver)
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
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
    fun spendBeforeFundingReconcilesViaPendingInputAndExcludesFromRestore() = runTest {
        // CORE-06, out-of-order arrival: an in-block spending tx is persisted
        // BEFORE its funding TXO is known (Rust's utxos_spent slice is empty
        // because the previous output wasn't classified yet). The spend must
        // not be lost — `inputOutpoints` stages a pending-input row that the
        // funding TXO's later upsert drains, so the consumed output is excluded
        // from the restore set instead of being handed back to Rust as
        // spendable. 1:1 mirror of Swift resolveInputOutpoint + upsertUtxo drain.
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        val xpub = ByteArray(78) { 30 }
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), xpub,
        )
        val account = db.accountDao().observeByWallet(walletId).first().single()
        db.coreAddressDao().upsert(
            CoreAddressEntity(
                address = "yFundAddr",
                poolTypeTag = 0,
                addressIndex = 0,
                derivationPath = "m/44'/1'/0'/0/0",
                accountId = account.id,
            ),
        )

        val fundingTxid = ByteArray(32) { 41 }
        val spendingTxid = ByteArray(32) { 42 }

        // Changeset 1: the in-block spending tx arrives first. Its funding TXO
        // is unknown, so a pending-input row is staged (no utxos_spent fires).
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
}
