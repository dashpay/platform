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
import org.dashfoundation.dashsdk.persistence.entities.PendingInputEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
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
    fun sweepSlotDefaultRefusesARoundOnlyWhenTheCapabilityIsHandDeclared() {
        // The trampoline is wired for every subclass, so "slot present"
        // proves nothing — the contract lives in the capability bit. A
        // subclass declaring CORE_SWEEP_REMOVAL without overriding the slot
        // has promised removals it would silently swallow while the
        // watermark advances; the inherited default must refuse the round
        // instead. One that declares nothing keeps the benign ignore: Rust
        // strips the watermark before its store(), and failing the round
        // would throw away its additive slots for no protection gained.
        val declaringButNotOverriding = object : NativePersistenceBridge() {
            override fun persistenceCapabilitiesBits(): Long =
                NativePersistenceBridge.CAPABILITY_CORE_SWEEP_REMOVAL
        }
        val walletId = ByteArray(32) { 1 }
        assertTrue(
            "a hand-declared capability with the inherited no-op body must fail the round",
            declaringButNotOverriding.onWalletChangesetTransactionsSwept(
                walletId, arrayOf(ByteArray(32) { 2 }), arrayOf(ByteArray(32) { 3 }), emptyArray(), 400,
            ) != 0,
        )

        val nonAttesting = object : NativePersistenceBridge() {}
        assertEquals(
            0,
            nonAttesting.onWalletChangesetTransactionsSwept(
                walletId, arrayOf(ByteArray(32) { 2 }), arrayOf(ByteArray(32) { 3 }), emptyArray(), 400,
            ),
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
    fun sweptTransactionIsDeletedAndFreesOnlyItsOwnInputs() = runTest {
        // A recorded spend that a later, final transaction beat to an input
        // can never confirm; Rust drops it and names it here. The mirror has
        // to drop it too — otherwise the row comes back on the next load and
        // re-creates a balance the wallet already corrected.
        //
        // Shape: the loser (unconfirmed, as every swept loser is) spends A
        // and B; the winner is wallet-relevant, in-block, and takes only A.
        // A must stay out of the restore set, B must return to it.
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

        val fundingTxid = ByteArray(32) { 41 }
        val sweptTxid = ByteArray(32) { 42 }
        val winnerTxid = ByteArray(32) { 44 }

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
            walletId, winnerTxid, ByteArray(10) { 6 }, 2, 102, ByteArray(32) { 9 },
            1_700_000_200, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_150,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, winnerTxid)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(sweptTxid), arrayOf(winnerTxid),
            arrayOf(makeOutpoint(fundingTxid, 1)), 400,
        )
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

        val fundingTxid = ByteArray(32) { 45 }
        val sweptTxid = ByteArray(32) { 46 }
        val irrelevantWinner = ByteArray(32) { 47 }

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
            walletId, sweptTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_050,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, sweptTxid)
        handler.onChangesetEnd(walletId, success = true)
        assertFalse(db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!.isSpent)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(sweptTxid), arrayOf(irrelevantWinner),
            // Upstream knows the winner took this coin even though it never
            // reports the winner itself, so nothing is released.
            emptyArray(), 400,
        )
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

        // A re-delivery of the funding output — what a restore-rescan does,
        // blind to the unconfirmed winner no block carries yet — must NOT
        // outrank the sweep's verdict: the coin was provably consumed, and
        // handing it back would resurrect it into the restore set on every
        // restore-from-seed until the winner confirms. Only an explicit
        // release frees a stamped hold — the same answer the SQLite store's
        // upsert valve gives to the identical event stream.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetUtxoAdded(
            walletId, fundingTxid, 0, 100_000, "yUtxoAddr", ByteArray(25) { 6 },
            100, false, true, false, false,
        )
        handler.onChangesetEnd(walletId, success = true)

        val redelivered = db.txoDao().getByOutpoint(makeOutpoint(fundingTxid, 0))!!
        assertTrue("the stamped hold survives re-delivery", redelivered.isSpent)
        assertTrue(irrelevantWinner.contentEquals(redelivered.supersededByTxid))
        assertTrue(handler.onLoadWalletList().single().utxos.isEmpty())
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

        val fundingTxid = ByteArray(32) { 56 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 57 }
        val winnerTxid = ByteArray(32) { 58 }

        // The doomed spend, before its funding output.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_050,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // The sweep holds the claim; the funding TXO then materializes it
        // as a stamped hold.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
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
        handler.onWalletChangesetTransaction(
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
        // attribution — so the hold pass never detaches F's coin and
        // `releaseByOutpoint` refuses it, while the coin only L claimed
        // still comes free in the same batch. The restore surface is the
        // restart: what `onLoadWalletList` hands back is what a relaunch
        // spends from.
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

        val fundingTxid = ByteArray(32) { 60 }
        val settledCoin = makeOutpoint(fundingTxid, 0)
        val losersOwnCoin = makeOutpoint(fundingTxid, 1)
        val attackerInput = makeOutpoint(ByteArray(32) { 61 }, 0)
        val finalizedTxid = ByteArray(32) { 62 }
        val loserTxid = ByteArray(32) { 63 }
        val winnerTxid = ByteArray(32) { 64 }

        // Fund both coins.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(winnerTxid),
            arrayOf(settledCoin, losersOwnCoin), 400,
        )
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
        // The backstop for rows written before holds named their winner: a
        // coin held spent with neither a spender nor a `supersededByTxid`
        // stamp has nothing durable behind it, so the wallet re-delivering
        // it as a UTXO — the authority on what it holds — still lifts the
        // mark. Every hold written today is stamped; this pins the migration
        // path for the ones already on disk.
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

        val fundingTxid = ByteArray(32) { 50 }
        val sweptTxid = ByteArray(32) { 51 }
        val winnerTxid = ByteArray(32) { 52 }
        val reclaimerTxid = ByteArray(32) { 53 }
        val freedCoin = makeOutpoint(fundingTxid, 1)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
            walletId, winnerTxid, ByteArray(10) { 6 }, 2, 101, ByteArray(32) { 8 },
            1_700_000_100, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_090,
            makeOutpoint(fundingTxid, 0), 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, winnerTxid)
        handler.onWalletChangesetTransaction(
            walletId, reclaimerTxid, ByteArray(10) { 7 }, 2, 102, ByteArray(32) { 9 },
            1_700_000_200, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_150,
            freedCoin, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 1, reclaimerTxid)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(sweptTxid), arrayOf(winnerTxid), arrayOf(freedCoin), 400,
        )
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

        val fundingTxid = ByteArray(32) { 70 }
        val firstLoser = ByteArray(32) { 71 }
        val secondLoser = ByteArray(32) { 72 }
        val contested = makeOutpoint(fundingTxid, 0)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
            walletId, firstLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_050,
            contested, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, firstLoser)
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, secondLoser, ByteArray(10) { 6 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -100_000, 0, false, "", 1_700_000_100,
            contested, 1,
        )
        handler.onWalletChangesetUtxoSpent(walletId, fundingTxid, 0, secondLoser)
        handler.onChangesetEnd(walletId, success = true)

        // One round, two batches, in order.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(firstLoser), arrayOf(ByteArray(32) { 73 }), arrayOf(contested), 400,
        )
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(secondLoser), arrayOf(ByteArray(32) { 74 }), emptyArray(), 400,
        )
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
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
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
        // Before the fix, whichever wallet's callback ran FIRST deleted the
        // shared loser row outright, using only its own released set to
        // decide every input on the row — including the other wallet's
        // coin. Running wallet B (which releases nothing) first used to
        // delete the row before wallet A's release of P ever landed, so
        // A's later call found nothing to update and P stayed wrongly
        // spent forever. This pins the fix: the row must survive until
        // both wallets have weighed in, and each wallet's coin must reflect
        // only that wallet's own decision.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 82 }
        val p = makeOutpoint(fundingTxid, 0)
        val q = makeOutpoint(fundingTxid, 1)

        // Wallet B first: its own released set names nothing, so its coin
        // (Q) is held rather than freed.
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(loserTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletB, success = true)

        assertNotNull(
            "wallet B alone must not delete a row wallet A still has a claim on",
            db.transactionDao().getByTxid(loserTxid),
        )
        val untouchedP = db.txoDao().getByOutpoint(p)!!
        assertFalse("wallet B's callback must not touch wallet A's coin", untouchedP.isSpent)
        assertTrue(
            "P is still linked to the loser, untouched",
            loserTxid.contentEquals(untouchedP.spendingTxid),
        )

        // Wallet A second: releases P.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(winnerTxid), arrayOf(p), 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertNull("the last wallet to run performs the delete", db.transactionDao().getByTxid(loserTxid))

        val freedP = db.txoDao().getByOutpoint(p)!!
        assertFalse("wallet A's own release must free its own coin", freedP.isSpent)
        assertNull(freedP.spendingTxid)

        val heldQ = db.txoDao().getByOutpoint(q)!!
        assertTrue(
            "wallet B's earlier decision to hold Q must survive wallet A's callback",
            heldQ.isSpent,
        )
        assertNull(heldQ.spendingTxid)
    }

    @Test
    fun sharedLoserAppliesEachWalletsOwnReleaseSetRegardlessOfOrder_walletAThenWalletB() = runTest {
        // Mirror of the ordering above: wallet A (which releases P) runs
        // first this time. The fix is meant to be order-independent, so
        // this must land on the exact same end state.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 92 }
        val p = makeOutpoint(fundingTxid, 0)
        val q = makeOutpoint(fundingTxid, 1)

        // Wallet A first: releases P.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(winnerTxid), arrayOf(p), 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertNotNull(
            "wallet A alone must not delete a row wallet B still has a claim on",
            db.transactionDao().getByTxid(loserTxid),
        )
        val untouchedQ = db.txoDao().getByOutpoint(q)!!
        assertFalse("wallet A's callback must not touch wallet B's coin", untouchedQ.isSpent)
        assertTrue(
            "Q is still linked to the loser, untouched",
            loserTxid.contentEquals(untouchedQ.spendingTxid),
        )

        // Wallet B second: releases nothing.
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(loserTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletB, success = true)

        assertNull("the last wallet to run performs the delete", db.transactionDao().getByTxid(loserTxid))

        val freedP = db.txoDao().getByOutpoint(p)!!
        assertFalse("wallet A's earlier release must survive wallet B's callback", freedP.isSpent)
        assertNull(freedP.spendingTxid)

        val heldQ = db.txoDao().getByOutpoint(q)!!
        assertTrue("wallet B's own decision to hold its coin must stick", heldQ.isSpent)
        assertNull(heldQ.spendingTxid)
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
        // The BLOCKING review finding: a shared loser's own output, and its
        // reachability through onGetCoreTxRecord, must not survive when
        // only ONE wallet's callback ever commits and the other's never
        // arrives at all — a crash, a rejection, or simply never coming.
        //
        // commit_batch calls store() once per wallet and each commits
        // independently, so before the fix wallet B alone could not delete
        // a row wallet A still had an outstanding claim on (see the
        // sharedLoserAppliesEachWalletsOwnReleaseSet* tests above) — and
        // the OUTPUT went with the row, because deletion was the only thing
        // that excluded either. If wallet A's own callback then never runs,
        // that hold is permanent: the row and its phantom output stay fully
        // live forever, so `onGetCoreTxRecord` keeps handing the dead
        // transaction back as though it were still a candidate.
        //
        // Only wallet B's callback ever runs here, and it releases nothing
        // — the worst case, since it gives the row no reason to be
        // physically deleted at all.
        val walletB = ByteArray(32) { 9 }
        val (_, loserTxid) = seedSharedLoserWithOwnOutputAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 82 }
        val phantomOutput = makeOutpoint(loserTxid, 2)

        // Only wallet B's callback ever runs, and it releases nothing —
        // wallet A's own callback (which would release P) never arrives in
        // this test at all.
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(loserTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletB, success = true)

        assertNotNull(
            "wallet A's own claim on P is still outstanding, so the row itself survives",
            db.transactionDao().getByTxid(loserTxid),
        )
        assertNull(
            "the loser's own output must not survive even a single committed callback, " +
                "regardless of which wallet's callback that was",
            db.txoDao().getByOutpoint(phantomOutput),
        )
        val row = db.transactionDao().getByTxid(loserTxid)!!
        assertTrue(
            "any callback that reaches the sweep must flag the row, not just wallet A's own",
            row.isGloballySwept,
        )

        // "Restart": a fresh handler bound to the same underlying store —
        // the same pattern `addressBalanceConflictPreservesDerivationIndicesAcrossRestart`
        // and the pending-key restart tests below use. Wallet A's own
        // callback never happens in this test, simulating a crash or a
        // rejection that stops it from ever arriving — the exact scenario
        // the finding describes.
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)

        assertNull(
            "the phantom output must not resurrect across a restart",
            db.txoDao().getByOutpoint(phantomOutput),
        )
        assertNull(
            "wallet A must not be able to read the swept loser back as a live transaction " +
                "after a restart, even though its own callback never ran",
            restarted.onGetCoreTxRecord(walletId, loserTxid),
        )
        val utxosA = restarted.onLoadWalletList().first { it.walletId.contentEquals(walletId) }.utxos
        assertFalse(
            "the phantom output must not be handed back as a restorable UTXO",
            utxosA.any { it.prevTxid.contentEquals(loserTxid) && it.vout == 2 },
        )
    }

    @Test
    fun twoWalletsReleasedPendingInputsDoNotDeadlockTheRowDelete() = runTest {
        // Port of the Swift regression of the same name. A shared loser
        // holds one unresolved pending input per wallet, and each wallet's
        // own sweep releases its own coin. Released staged rows must be
        // deleted outright: left attached they read as their wallet's claim
        // in `hasOtherWalletClaim`, so each callback would see the other's
        // row and decline the delete, and replaying either would reach the
        // same stalemate — the dead row and both pending entries stored
        // forever. The global marker keeps the funds correct either way;
        // this pins the storage half.
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
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = pA,
                inputIndex = 0,
                spendingTxid = loserTxid,
                spendingTransactionTxid = loserTxid,
                walletId = walletId,
            ),
        )
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = pB,
                inputIndex = 1,
                spendingTxid = loserTxid,
                spendingTransactionTxid = loserTxid,
                walletId = walletB,
            ),
        )

        // Each wallet's independently committed callback, each releasing
        // only its own coin.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(winnerTxid), arrayOf(pA), 400,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(loserTxid), arrayOf(winnerTxid), arrayOf(pB), 400,
        )
        handler.onChangesetEnd(walletB, success = true)

        assertNull(
            "a released pending input is not a claim once its own wallet has resolved it",
            db.transactionDao().getByTxid(loserTxid),
        )
        assertTrue(db.documentDao().getPendingInputsByOutpoint(pA).isEmpty())
        assertTrue(db.documentDao().getPendingInputsByOutpoint(pB).isEmpty())
    }

    @Test
    fun aReinstatingRecordInALaterRoundRevivesASweptTransactionAndItsOutputs() = runTest {
        // Cross-round reinstatement — the BLOCKING finding this round
        // fixes. The sweep and its reinstating record land in two
        // SEPARATE callback rounds, with wallet B's still-outstanding
        // claim keeping the shared row physically present in between,
        // exactly as
        // sharedLoserOutputAndCoreTxRecordAreExcludedAfterOnlyOneWalletsCallbackCommits
        // above establishes on its own. Before the fix,
        // onWalletChangesetTransaction bailed unconditionally on
        // isGloballySwept == true, so round 2's record — upstream's newer
        // word, per CoreChangeSet::merge's documented IS-lock-precedence
        // sequence (swept by an IS-locked conflict, then returns
        // chainlocked and sweeps that conflict in turn) — would be
        // silently discarded forever, and onWalletChangesetUtxoAdded would
        // keep rejecting its output on the strength of a tombstone nothing
        // could ever clear.
        val walletB = ByteArray(32) { 9 }
        val (fundingTxid, loserTxid) = seedSharedLoserWithOwnOutputAcrossTwoWallets(walletId, walletB)
        val winnerTxid = ByteArray(32) { 82 }
        val p = makeOutpoint(fundingTxid, 0)
        val phantomOutput = makeOutpoint(loserTxid, 2)

        // Round 1: only wallet B's own sweep callback runs, releasing
        // nothing. Wallet A's own claim on P is still outstanding, so the
        // shared row survives physically even though the global half of
        // the sweep already tombstoned it and deleted its phantom output.
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(loserTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletB, success = true)

        val tombstoned = db.transactionDao().getByTxid(loserTxid)!!
        assertTrue("sanity: the row is tombstoned after round 1", tombstoned.isGloballySwept)
        assertNull(
            "sanity: the loser's own output is gone after round 1",
            db.txoDao().getByOutpoint(phantomOutput),
        )

        // Round 2, a SEPARATE callback (not coalesced with round 1's
        // sweep — the cross-round shape the merge-level fix in
        // CoreChangeSet::merge cannot reach): the wallet returns
        // chainlocked and sweeps the erstwhile winner in turn. Arrives
        // here exactly like any freshly-detected transaction would —
        // nothing marks it as "the reinstating one" — with its own output
        // riding along in the same round, transaction before utxo per the
        // JNI bridge's account ordering.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        assertFalse(
            "a later record naming a tombstoned txid must clear the tombstone",
            reinstated.isGloballySwept,
        )
        assertEquals(200, reinstated.blockHeight)

        val revivedOutput = db.txoDao().getByOutpoint(phantomOutput)
        assertNotNull("the reinstated transaction's own output must come back", revivedOutput)
        assertEquals(60_000L, revivedOutput!!.amount)

        val reclaimedP = db.txoDao().getByOutpoint(p)!!
        assertTrue(
            "wallet A reclaims its input once its own record is live again",
            reclaimedP.isSpent,
        )
        assertTrue(loserTxid.contentEquals(reclaimedP.spendingTxid))

        assertNotNull(
            "wallet A must be able to read the reinstated transaction as live again",
            handler.onGetCoreTxRecord(walletId, loserTxid),
        )

        // "Restart": a fresh handler bound to the same underlying store —
        // the same pattern
        // sharedLoserOutputAndCoreTxRecordAreExcludedAfterOnlyOneWalletsCallbackCommits
        // above uses. The reinstatement has to be durable, not just
        // visible to the handler instance that just applied it.
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)

        val survived = db.transactionDao().getByTxid(loserTxid)!!
        assertFalse("the reinstatement must survive a restart", survived.isGloballySwept)
        assertNotNull(
            "the revived output must survive a restart",
            db.txoDao().getByOutpoint(phantomOutput),
        )
        val survivedP = db.txoDao().getByOutpoint(p)!!
        assertTrue("the reclaimed input must survive a restart", survivedP.isSpent)
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
        handler.onWalletChangesetTransaction(
            walletId, loser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -1_000, 0, false, "", 1_700_000_000,
            ByteArray(0), 0,
        )
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        val code = handler.onWalletChangesetTransactionsSwept(
            walletId,
            arrayOf(loser),
            arrayOf(ByteArray(32) { 82 }),
            released.toTypedArray(), 400,
        )
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
        handler.onWalletChangesetTransaction(
            walletId, txid, ByteArray(10) { 4 }, 2, 100, ByteArray(32) { 7 },
            1_700_000_000, 0, "Standard", 0, 100_000, 0, false, "", 1_699_999_000,
            ByteArray(0), 0,
        )
        handler.onChangesetEnd(walletId, success = true)

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(txid), arrayOf(ByteArray(32) { 44 }), emptyArray(), 400,
        )
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

        val fundingTxid = ByteArray(32) { 61 }
        val sweptTxid = ByteArray(32) { 62 }
        val winnerTxid = ByteArray(32) { 64 }

        // Changeset 1: the doomed spend arrives with no prior
        // `onWalletChangesetUtxoAdded` for `fundingTxid:0` — the funding side
        // of that outpoint has not been observed yet.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(sweptTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertNull("the loser is gone", db.transactionDao().getByTxid(sweptTxid))

        // Restart: a fresh persister loading the same on-disk store — same
        // Room database, new handler, matching this suite's own restart
        // idiom (e.g. addressBalanceConflictPreservesDerivationIndicesAcrossRestart above).
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)

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

        val fundingTxid = ByteArray(32) { 91 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 92 }
        val winnerTxid = ByteArray(32) { 93 }

        // Changeset 1: the doomed spend arrives before its funding output.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        handler.onWalletChangesetTransaction(
            walletId, winnerTxid, ByteArray(10) { 6 }, 1, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_060,
            pOutpoint, 1,
        )
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(winnerTxid), emptyArray(), 400,
        )
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
        // winner, and the recovery clear refuses stamped holds. A dead
        // parent's output is nobody's coin; the claim must be deleted with
        // the batch.
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

        val parentTxid = ByteArray(32) { 101 } // P — record never persisted
        val pOutpoint = makeOutpoint(parentTxid, 0)
        val childTxid = ByteArray(32) { 102 } // C
        val winnerTxid = ByteArray(32) { 103 } // W

        // C arrives spending the still-unfunded P:0 — parked as a pending
        // claim.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, childTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_100,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertEquals(1, db.documentDao().getPendingInputsByOutpoint(pOutpoint).size)

        // One batch removes both; upstream excludes P:0 from the released
        // set because its funder is itself a loser.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(parentTxid, childTxid),
            arrayOf(winnerTxid, winnerTxid), emptyArray(), 400,
        )
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

        val fundingTxid = ByteArray(32) { 71 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val qOutpoint = makeOutpoint(ByteArray(32) { 72 }, 0)
        val firstLoserTxid = ByteArray(32) { 73 } // L
        val secondLoserTxid = ByteArray(32) { 74 } // W
        val finalWinnerTxid = ByteArray(32) { 75 } // X

        // L spends only P, and P's funding side has never been observed.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, firstLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_070,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // First sweep: W beats L, holding P (still unfunded).
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(firstLoserTxid), arrayOf(secondLoserTxid), emptyArray(), 400,
        )
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
        handler.onWalletChangesetTransaction(
            walletId, secondLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_071,
            pOutpoint + qOutpoint, 2,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Second sweep: X beats W, releasing P this time.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(secondLoserTxid), arrayOf(finalWinnerTxid), arrayOf(pOutpoint), 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        assertTrue(
            "a released outpoint's tombstone must not survive a chained sweep",
            db.documentDao().getPendingInputsByOutpoint(pOutpoint).isEmpty(),
        )

        // P's funding TXO finally arrives.
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
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
        // time the release runs. `releaseByOutpoint` is the only writer
        // that ever clears that column — a released coin keeping its dead
        // winner's marker would turn the next hold on this outpoint
        // permanent, because the redelivery carry-over in
        // `onWalletChangesetUtxoAdded` reads a present marker as a durable
        // claim and refuses to lift `isSpent` ever again.
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

        val fundingTxid = ByteArray(32) { 96 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val loserTxid = ByteArray(32) { 97 } // L
        val intermediateWinner = ByteArray(32) { 98 } // W — never recorded here
        val finalWinner = ByteArray(32) { 99 } // X

        // L spends the still-unfunded P.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, loserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_090,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // First sweep: W beats L, holding P.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loserTxid), arrayOf(intermediateWinner), emptyArray(), 400,
        )
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
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(intermediateWinner), arrayOf(finalWinner), arrayOf(pOutpoint), 400,
        )
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

        val fundingTxid = ByteArray(32) { 81 }
        val pOutpoint = makeOutpoint(fundingTxid, 0)
        val firstLoserTxid = ByteArray(32) { 83 } // L
        val secondLoserTxid = ByteArray(32) { 84 } // W
        val finalWinnerTxid = ByteArray(32) { 85 } // X

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, firstLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_080,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // First sweep: W beats L, holding P.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(firstLoserTxid), arrayOf(secondLoserTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        // W's own record, needed by the second sweep below.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, secondLoserTxid, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_081,
            pOutpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Second sweep: X beats W, still holding the same input.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(secondLoserTxid), arrayOf(finalWinnerTxid), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletId, success = true)

        val tombstone = db.documentDao().getPendingInputsByOutpoint(pOutpoint).single()
        assertTrue(tombstone.isSweptTombstone)
        assertTrue(
            "the tombstone must be repointed at the FINAL winner, not the " +
                "intermediate one the second sweep already removed",
            finalWinnerTxid.contentEquals(tombstone.spendingTxid),
        )

        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
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
    fun sharedWinnerDeletedByAnotherWalletsCallbackStillReconcilesThisWalletsTombstones() = runTest {
        // Multi-wallet continuation of the chained-before-funding scenarios
        // above, confirming this handler is NOT exposed to the Swift-side
        // review finding on the missing-row early return: every query that
        // carries a detached tombstone forward keys on the scalar
        // `spendingTxid` (no FK — see [PendingInputEntity]) and runs
        // unconditionally in `onWalletChangesetTransactionsSwept`, so the
        // shared winner row having already been deleted by another wallet's
        // independently committed callback must change nothing about this
        // wallet's own release decision reaching its tombstones.
        val walletB = ByteArray(32) { 9 }
        handler.onPersistWalletMetadata(walletId, testnet, groupId, 0)
        handler.onPersistWalletMetadata(walletB, testnet, groupId, 0)
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 30 },
        )
        handler.onPersistAccountRegistration(
            walletB, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 31 },
        )
        val accountA = db.accountDao().observeByWallet(walletId).first().single()
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
        handler.onWalletChangesetTransaction(
            walletId, sharedLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_090,
            pA, 1,
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

        // First sweep, one independently committed callback per wallet: W
        // beats L, holding everything (nothing funded, nothing released).
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(sharedLoser), arrayOf(sharedWinner), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(sharedLoser), arrayOf(sharedWinner), emptyArray(), 400,
        )
        handler.onChangesetEnd(walletB, success = true)
        assertNull("L is gone once both wallets ran", db.transactionDao().getByTxid(sharedLoser))

        // W's own record arrives claiming all three outpoints. Each
        // `(outpoint, W)` tombstone occupies the duplicate-guard key, so no
        // new pending relationship attaches to W's row — the premise that
        // lets wallet A's callback below delete it.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, sharedWinner, ByteArray(10) { 6 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_091,
            pA + pB + rB, 3,
        )
        handler.onChangesetEnd(walletId, success = true)

        // Second sweep: X beats W. Wallet A's callback runs first, releases
        // its own coin, and — finding no attached claim of any other
        // wallet's — deletes the shared row.
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(sharedWinner), arrayOf(finalWinner), arrayOf(pA), 400,
        )
        handler.onChangesetEnd(walletId, success = true)
        assertNull(
            "sanity: wallet A's callback deleted the shared winner row — the premise " +
                "wallet B's callback below has to survive",
            db.transactionDao().getByTxid(sharedWinner),
        )

        // Wallet B's callback arrives after the row is gone, releasing one
        // of its two coins and holding the other.
        handler.onChangesetBegin(walletB)
        handler.onWalletChangesetTransactionsSwept(
            walletB, arrayOf(sharedWinner), arrayOf(finalWinner), arrayOf(rB), 400,
        )
        handler.onChangesetEnd(walletB, success = true)

        val heldTombstone = db.documentDao().getPendingInputsByOutpoint(pB).single()
        assertTrue(heldTombstone.isSweptTombstone)
        assertTrue(
            "the held tombstone must follow the chain to X even though W's row was " +
                "already deleted by wallet A's callback",
            finalWinner.contentEquals(heldTombstone.spendingTxid),
        )
        assertTrue(
            "wallet B's release decision must reach its tombstone even though W's " +
                "row was already deleted by wallet A's callback",
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

        assertFalse(
            "wallet A's released coin comes back spendable",
            db.txoDao().getByOutpoint(pA)!!.isSpent,
        )
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
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
            walletId, loser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -50_000, 0, false, "", 1_700_000_090,
            outpoint, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(loser), arrayOf(winner), emptyArray(), winnerMinedHeight,
        )
        handler.onChangesetEnd(walletId, success = true)
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
        // and legacy rows (the v12 → v13 migration leaves pre-existing
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
            """"height":$height,"isLocked":false}],"errors":[]}"""

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

        val report = handler.reconcileTxos(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 1, amount = 989_009_773L),
            tipHeight = reconcileTip,
        )

        assertEquals(1, report.inserted)
        assertEquals(989_009_773L, report.insertedDuffs)
        assertEquals(1, report.netAmountRepairs)

        val row = db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 1))
        assertNotNull(row)
        assertFalse(row!!.isSpent)
        assertEquals(989_009_773L, row.amount)
        assertTrue(row.isConfirmed)

        // -10.00010000 + 9.89009773 = -0.11000227 — history now matches
        // what the engine (and dashj) report for this send.
        assertEquals(
            -11_000_227L,
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
        handler.onWalletChangesetTransaction(
            walletId, secondLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_091,
            p, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(secondLoser), arrayOf(finalWinner), emptyArray(), 450,
        )
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

        val fundingTxid = ByteArray(32) { 123 }
        val p = makeOutpoint(fundingTxid, 0)
        val loser = ByteArray(32) { 124 }
        val winner = ByteArray(32) { 125 }

        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransaction(
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
        val restarted = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)

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
        handler.onWalletChangesetTransaction(
            walletId, secondLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_092,
            p, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(secondLoser), arrayOf(finalWinner), emptyArray(), -1,
        )
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
        handler.onWalletChangesetTransaction(
            walletId, secondLoser, ByteArray(10) { 5 }, 0, 0, ByteArray(32),
            0, 1, "Standard", 0, -40_000, 0, false, "", 1_700_000_093,
            p, 1,
        )
        handler.onChangesetEnd(walletId, success = true)
        handler.onChangesetBegin(walletId)
        handler.onWalletChangesetTransactionsSwept(
            walletId, arrayOf(secondLoser), arrayOf(finalWinner), emptyArray(), 450,
        )
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

        handler.reconcileTxos(walletId, json, tipHeight = reconcileTip)
        val second = handler.reconcileTxos(walletId, json, tipHeight = reconcileTip)

        assertEquals(0, second.inserted)
        assertEquals(0, second.netAmountRepairs)
        assertEquals(
            -11_000_227L,
            db.transactionDao().getByTxid(changeTxid)!!.netAmount,
        )
    }

    @Test
    fun reconcileSkipsImmatureOutputsAndPreservesSpentRows() = runTest {
        // Immature: inside the 100-conf gate (flags on the engine snapshot
        // can't carry coinbase/IS-lock, so fresh rows wait for a later
        // sweep) — nothing inserted.
        val fresh = handler.reconcileTxos(
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

        val report = handler.reconcileTxos(
            walletId,
            engineUtxoJson(changeTxid.toHexLower(), vout = 2, amount = 42L, height = 1_500_000),
            tipHeight = reconcileTip,
        )
        assertEquals(0, report.inserted)
        assertTrue(db.txoDao().getByOutpoint(makeOutpoint(changeTxid, 2))!!.isSpent)
    }
}
