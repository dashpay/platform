package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.PendingInputEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionAccountInvolvementEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Robolectric round-trip tests over
 * [PlatformWalletPersistenceHandler.deleteWalletData] + in-memory
 * [DashDatabase] — Kotlin port of the feasible (device-independent)
 * assertions in `SwiftExampleAppTests/WalletDeletionTests.swift`.
 *
 * Covers the Room-cascade contract of a wallet wipe: the wallet's own
 * rows and every walletId-keyed child table are removed, SET_NULL
 * children (identities, token balances) are deleted explicitly, a
 * transaction still referenced by a surviving TXO is preserved, an
 * orphaned transaction is swept, a sibling wallet's asset locks survive,
 * and the shared network sync-state row is dropped only when the last
 * wallet on that network is gone.
 */
@RunWith(RobolectricTestRunner::class)
class WalletDeletionTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

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
    fun deleteWalletDataRemovesFootprintAndLastNetworkSyncState() = runTest {
        val walletId = ByteArray(32) { 0x44 }
        val identityId = ByteArray(32) { 0x55 }

        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        db.identityDao().upsert(
            IdentityEntity(identityId = identityId, networkRaw = testnet, walletId = walletId),
        )
        db.tokenDao().insertBalance(
            TokenBalanceEntity(
                tokenId = "token-a",
                identityId = identityId,
                balance = UInt64Value(10u),
                networkRaw = testnet,
            ),
        )
        // An identity key row — CASCADEs from the identity via identityIdData.
        db.publicKeyDao().insert(
            PublicKeyEntity(
                keyId = 0,
                purpose = "AUTHENTICATION",
                securityLevel = "HIGH",
                keyType = "ECDSA_SECP256K1",
                publicKeyData = ByteArray(33) { 0xAB.toByte() },
                identityId = identityId.toBase58String(),
                identityIdData = identityId,
            ),
        )

        // A pending input (walletId-keyed, no wallet FK) pointing at a
        // spending transaction.
        val pendingTxid = ByteArray(32) { 0x66 }
        db.transactionDao().upsert(
            TransactionEntity(txid = pendingTxid, transactionData = byteArrayOf(0x01)),
        )
        db.documentDao().upsertPendingInput(
            PendingInputEntity(
                outpoint = ByteArray(36) { 0x67 },
                inputIndex = 0,
                spendingTxid = pendingTxid,
                spendingTransactionTxid = pendingTxid,
                walletId = walletId,
            ),
        )

        // An orphan transaction referenced by nothing.
        db.transactionDao().upsert(
            TransactionEntity(txid = ByteArray(32) { 0x77 }, transactionData = byteArrayOf(0x02)),
        )

        // A live transaction kept alive by a surviving TXO NOT owned by
        // this wallet.
        val liveTxid = ByteArray(32) { 0x88.toByte() }
        db.transactionDao().upsert(
            TransactionEntity(txid = liveTxid, transactionData = byteArrayOf(0x03)),
        )
        db.txoDao().upsert(
            TxoEntity(
                outpoint = makeOutpoint(liveTxid, 0),
                vout = 0,
                amount = 1,
                address = "yLive",
                walletId = ByteArray(32) { 0x33 },
                txid = liveTxid,
            ),
        )

        db.platformAddressDao().upsertSyncState(
            PlatformAddressesSyncStateEntity(
                walletId = syncStateScopeId(testnet),
                networkRaw = testnet,
                syncHeight = 10,
                syncTimestamp = 20,
                lastKnownRecentBlock = 30,
            ),
        )

        // Idempotent: two passes.
        handler.deleteWalletData(walletId)
        handler.deleteWalletData(walletId)

        assertNull(db.walletDao().getByWalletId(walletId))
        assertTrue(db.identityDao().observeByWallet(walletId).first().isEmpty())
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertTrue(db.tokenDao().observeBalancesByIdentity(identityId).first().isEmpty())
        assertTrue(db.publicKeyDao().getByIdentityId(identityId.toBase58String()).isEmpty())
        assertTrue(db.documentDao().observePendingInputsByWallet(walletId).first().isEmpty())
        assertNull(db.platformAddressDao().getSyncState(syncStateScopeId(testnet)))

        // Only the live transaction survives (pending + orphan swept).
        val transactions = db.transactionDao().observeAll().first()
        assertEquals(1, transactions.size)
        assertTrue(liveTxid.contentEquals(transactions.single().txid))
    }

    @Test
    fun deleteWalletDataRemovesAssetLocksAndPreservesOtherWallets() = runTest {
        val walletId = ByteArray(32) { 0xA1.toByte() }
        val siblingId = ByteArray(32) { 0xB2.toByte() }

        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        db.walletDao().upsert(WalletEntity(walletId = siblingId, networkRaw = testnet))

        db.assetLockDao().upsert(
            AssetLockEntity(
                outPointHex = "deadbeef:0",
                walletId = walletId,
                transactionBytes = byteArrayOf(0x01, 0x02, 0x03),
                fundingTypeRaw = 0,
                identityIndexRaw = 0,
                amountDuffs = 100_000_000,
                statusRaw = 1,
            ),
        )
        db.assetLockDao().upsert(
            AssetLockEntity(
                outPointHex = "cafebabe:1",
                walletId = siblingId,
                transactionBytes = byteArrayOf(0x04, 0x05, 0x06),
                fundingTypeRaw = 0,
                identityIndexRaw = 0,
                amountDuffs = 200_000_000,
                statusRaw = 2,
            ),
        )

        handler.deleteWalletData(walletId)

        val remaining = db.assetLockDao().observeByWallet(siblingId).first()
        assertEquals(1, remaining.size)
        assertTrue(siblingId.contentEquals(remaining.single().walletId))
        assertEquals("cafebabe:1", remaining.single().outPointHex)
        assertTrue(db.assetLockDao().observeByWallet(walletId).first().isEmpty())
    }

    @Test
    fun deleteWalletDataRemovesOnlyThatWalletsOrchardViewingKeys() = runTest {
        val walletId = ByteArray(32) { 0x41 }
        val siblingId = ByteArray(32) { 0x42 }
        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        db.walletDao().upsert(WalletEntity(walletId = siblingId, networkRaw = testnet))
        db.shieldedDao().upsertViewingKey(
            ShieldedViewingKeyEntity(walletId, 0, ByteArray(96) { 1 }),
        )
        db.shieldedDao().upsertViewingKey(
            ShieldedViewingKeyEntity(siblingId, 0, ByteArray(96) { 2 }),
        )

        handler.deleteWalletData(walletId)

        assertTrue(db.shieldedDao().observeViewingKeysByWallet(walletId).first().isEmpty())
        val sibling = db.shieldedDao().observeViewingKeysByWallet(siblingId).first()
        assertEquals(1, sibling.size)
        assertEquals(2.toByte(), sibling.single().fvkBytes.first())
    }

    @Test
    fun deleteWalletDataCascadesProviderMembershipAndSweepsOnlyItsTransaction() = runTest {
        val walletId = ByteArray(32) { 0x71 }
        val siblingId = ByteArray(32) { 0x72 }
        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        db.walletDao().upsert(WalletEntity(walletId = siblingId, networkRaw = testnet))
        db.accountDao().insert(
            AccountEntity(
                walletId = walletId, accountType = 9, accountIndex = 0,
                accountTypeName = "providerOwnerKeys",
            ),
        )
        db.accountDao().insert(
            AccountEntity(
                walletId = siblingId, accountType = 9, accountIndex = 0,
                accountTypeName = "providerOwnerKeys",
            ),
        )
        val account = db.accountDao().observeByWallet(walletId).first().single()
        val siblingAccount = db.accountDao().observeByWallet(siblingId).first().single()
        val txid = ByteArray(32) { 0x73 }
        val siblingTxid = ByteArray(32) { 0x74 }
        val sharedTxid = ByteArray(32) { 0x75 }
        db.transactionDao().upsert(
            TransactionEntity(txid = txid, transactionData = byteArrayOf(1), transactionTypeKind = 2),
        )
        db.transactionDao().upsert(
            TransactionEntity(
                txid = siblingTxid,
                transactionData = byteArrayOf(2),
                transactionTypeKind = 3,
            ),
        )
        db.transactionDao().upsert(
            TransactionEntity(
                txid = sharedTxid,
                transactionData = byteArrayOf(3),
                transactionTypeKind = 4,
            ),
        )
        db.transactionDao().upsertInvolvement(TransactionAccountInvolvementEntity(txid, account.id))
        db.transactionDao().upsertInvolvement(
            TransactionAccountInvolvementEntity(siblingTxid, siblingAccount.id),
        )
        db.transactionDao().upsertInvolvement(
            TransactionAccountInvolvementEntity(sharedTxid, account.id),
        )
        db.transactionDao().upsertInvolvement(
            TransactionAccountInvolvementEntity(sharedTxid, siblingAccount.id),
        )

        handler.deleteWalletData(walletId)

        assertNull(db.transactionDao().getByTxid(txid))
        assertEquals(0, db.transactionDao().countInvolvements(txid))
        assertTrue(db.transactionDao().getByTxid(siblingTxid) != null)
        assertEquals(1, db.transactionDao().countInvolvements(siblingTxid))
        assertTrue(db.transactionDao().getByTxid(sharedTxid) != null)
        assertEquals(1, db.transactionDao().countInvolvements(sharedTxid))

        handler.deleteWalletData(siblingId)

        assertNull(db.transactionDao().getByTxid(sharedTxid))
        assertEquals(0, db.transactionDao().countInvolvements(sharedTxid))
    }

    @Test
    fun deleteWalletDataKeepsNetworkSyncStateWhenSiblingWalletRemains() = runTest {
        val walletId = ByteArray(32) { 0x99.toByte() }
        val siblingId = ByteArray(32) { 0xAA.toByte() }

        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        db.walletDao().upsert(WalletEntity(walletId = siblingId, networkRaw = testnet))
        db.platformAddressDao().upsertSyncState(
            PlatformAddressesSyncStateEntity(
                walletId = syncStateScopeId(testnet),
                networkRaw = testnet,
                syncHeight = 10,
                syncTimestamp = 20,
                lastKnownRecentBlock = 30,
            ),
        )

        handler.deleteWalletData(walletId)

        val wallets = db.walletDao().getByNetwork(testnet)
        assertEquals(1, wallets.size)
        assertTrue(siblingId.contentEquals(wallets.single().walletId))
        // The shared sync-state row survives while a sibling remains.
        assertNull(db.walletDao().getByWalletId(walletId))
        assertTrue(db.platformAddressDao().getSyncState(syncStateScopeId(testnet)) != null)
    }
}
