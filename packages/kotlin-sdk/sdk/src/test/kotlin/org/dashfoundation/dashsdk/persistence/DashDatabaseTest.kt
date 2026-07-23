package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletManagerMetadataEntity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Round-trip tests over the in-memory [DashDatabase] — validates the Room
 * schema (entities, FKs, converters) that transcribes the SwiftData models.
 */
@RunWith(RobolectricTestRunner::class)
class DashDatabaseTest {

    private lateinit var db: DashDatabase

    private val walletId = ByteArray(32) { 1 }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        db.close()
    }

    @Test
    fun walletRoundTripsThroughUpsertAndFlow() = runTest {
        val wallet = WalletEntity(
            walletId = walletId,
            networkRaw = 1,
            name = "Test wallet",
            birthHeight = 1_000_000,
        )
        db.walletDao().upsert(wallet)

        val loaded = db.walletDao().getByWalletId(walletId)
        assertNotNull(loaded)
        assertEquals("Test wallet", loaded!!.name)
        assertEquals(1_000_000, loaded.birthHeight)

        val byNetwork = db.walletDao().observeByNetwork(1).first()
        assertEquals(1, byNetwork.size)

        // Date converter round-trip (epoch millis).
        assertEquals(wallet.createdAt.time, loaded.createdAt.time)
    }

    @Test
    fun deletingWalletCascadesToAccounts() = runTest {
        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = 1))
        db.accountDao().insert(
            AccountEntity(
                walletId = walletId,
                accountType = 0,
                accountIndex = 0,
                accountTypeName = "standardBip44",
            ),
        )
        assertEquals(1, db.accountDao().observeByWallet(walletId).first().size)

        db.walletDao().deleteByWalletId(walletId)

        // FK CASCADE mirrors SwiftData's `.cascade` delete rule on accounts.
        assertTrue(db.accountDao().observeByWallet(walletId).first().isEmpty())
    }

    @Test
    fun tokenCapabilityColumnsDriveThePredicateQueries() = runTest {
        val contractId = ByteArray(32) { 7 }
        // Tokens FK-reference their contract (CASCADE), so the parent row
        // must exist first — same shape as the Swift relationship.
        db.dataContractDao().upsert(
            org.dashfoundation.dashsdk.persistence.entities.DataContractEntity(
                id = contractId,
                name = "test-contract",
                serializedContract = ByteArray(8),
                networkRaw = 1,
            ),
        )
        fun token(position: Int, mint: Boolean, paused: Boolean) = TokenEntity(
            id = ByteArray(32) { position.toByte() },
            contractId = contractId,
            position = position,
            name = "token-$position",
            baseSupply = "1000000",
            canManuallyMint = mint,
            isPaused = paused,
        )
        db.tokenDao().upsertToken(token(position = 0, mint = true, paused = false))
        db.tokenDao().upsertToken(token(position = 1, mint = false, paused = true))

        // Mirror of PersistentToken.mintableTokensPredicate().
        val mintable = db.tokenDao().observeMintableTokens().first()
        assertEquals(1, mintable.size)
        assertEquals(0, mintable[0].position)

        val byContract = db.tokenDao().observeTokensByContract(contractId).first()
        assertEquals(2, byContract.size)
    }

    @Test
    fun tokenBalancesRoundTripAndSortAcrossFullUnsignedRange() = runTest {
        val values = listOf(ULong.MAX_VALUE, 0uL, 1uL shl 63, Long.MAX_VALUE.toULong())
        values.forEachIndexed { index, value ->
            db.tokenDao().insertBalance(
                TokenBalanceEntity(
                    tokenId = "token-$index",
                    identityId = byteArrayOf(index.toByte()),
                    balance = UInt64Value(value),
                    networkRaw = 1,
                ),
            )
        }

        assertEquals(
            listOf(0uL, Long.MAX_VALUE.toULong(), 1uL shl 63, ULong.MAX_VALUE),
            db.tokenDao().getBalancesOrderedByAmount().map { it.balance.value },
        )
        assertEquals(
            setOf(ULong.MAX_VALUE, 1uL shl 63, Long.MAX_VALUE.toULong()),
            db.tokenDao().observeNonZeroBalances().first().map { it.balance.value }.toSet(),
        )
    }

    @Test
    fun orchardViewingKeysUpsertByWalletAndAccountWithoutCrossWalletCollisions() = runTest {
        val walletA = ByteArray(32) { 0x11 }
        val walletB = ByteArray(32) { 0x22 }
        fun key(walletId: ByteArray, account: Int, marker: Byte) =
            ShieldedViewingKeyEntity(walletId, account, ByteArray(96) { marker })

        db.shieldedDao().upsertViewingKey(key(walletA, 0, 1))
        db.shieldedDao().upsertViewingKey(key(walletA, 1, 2))
        db.shieldedDao().upsertViewingKey(key(walletB, 0, 3))
        // Same composite key replaces its row rather than creating a duplicate.
        db.shieldedDao().upsertViewingKey(key(walletA, 0, 4))

        val a = db.shieldedDao().observeViewingKeysByWallet(walletA).first()
        val b = db.shieldedDao().observeViewingKeysByWallet(walletB).first()
        assertEquals(listOf(0, 1), a.map { it.accountIndex })
        assertEquals(4.toByte(), a.first().fvkBytes.first())
        assertEquals(1, b.size)
        assertEquals(3.toByte(), b.single().fvkBytes.first())
        assertEquals(3, db.shieldedDao().getAllViewingKeys().size)
    }

    @Test
    fun orchardViewingKeyEntityRejectsMalformedFixedFields() {
        assertThrows(IllegalArgumentException::class.java) {
            ShieldedViewingKeyEntity(ByteArray(31), 0, ByteArray(96))
        }
        assertThrows(IllegalArgumentException::class.java) {
            ShieldedViewingKeyEntity(ByteArray(32), 0, ByteArray(95))
        }
        assertThrows(IllegalArgumentException::class.java) {
            ShieldedViewingKeyEntity(ByteArray(32), 0, ByteArray(97))
        }
    }

    @Test
    fun identityRoundTripsAndIsNetworkScoped() = runTest {
        val identityId = ByteArray(32) { 9 }
        db.identityDao().upsert(
            IdentityEntity(
                identityId = identityId,
                networkRaw = 1,
                balance = 42_000,
                alias = "alice",
            ),
        )

        val loaded = db.identityDao().getByIdentityId(identityId)
        assertNotNull(loaded)
        assertEquals(42_000, loaded!!.balance)
        assertEquals("alice", loaded.alias)

        assertTrue(db.identityDao().observeByNetwork(0).first().isEmpty())
        assertEquals(1, db.identityDao().observeByNetwork(1).first().size)
    }

    @Test
    fun walletManagerMetadataIsSingletonPerNetwork() = runTest {
        db.walletManagerMetadataDao().upsert(
            WalletManagerMetadataEntity(networkRaw = 1, walletCount = 2),
        )
        db.walletManagerMetadataDao().upsert(
            WalletManagerMetadataEntity(networkRaw = 1, walletCount = 3),
        )

        val row = db.walletManagerMetadataDao().getByNetwork(1)
        assertEquals(3, row!!.walletCount)
        assertEquals(1L, db.walletManagerMetadataDao().count().first())
        assertNull(db.walletManagerMetadataDao().getByNetwork(0))
    }

    @Test
    fun updateNameStampsTheNameOntoAnExistingRow() = runTest {
        // The persister writes rows without a user-facing label; the
        // manager stamps it post-create (B-M2 wallet-name persist fix,
        // ← CreateWalletView.swift's label write).
        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = 1))

        val updated = db.walletDao().updateName(walletId, "Groceries", 42_000L)
        assertEquals(1, updated)

        val row = db.walletDao().getByWalletId(walletId)!!
        assertEquals("Groceries", row.name)
        assertEquals(42_000L, row.lastUpdated.time)

        // Missing row → 0 updates, no insert-by-side-effect.
        val missing = db.walletDao().updateName(ByteArray(32) { 9 }, "Nope", 1L)
        assertEquals(0, missing)
        assertEquals(1L, db.walletDao().count().first())
    }

    @Test
    fun storageCountsCoverEveryTable() = runTest {
        val counts = db.storageCountsDao()
        assertEquals(0L, counts.countWallets().first())
        assertEquals(0L, counts.countTokens().first())
        assertEquals(0L, counts.countShieldedNotes().first())
        assertEquals(0L, counts.countShieldedViewingKeys().first())

        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = 1))
        assertEquals(1L, counts.countWallets().first())
    }
}
