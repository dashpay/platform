package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.nio.ByteBuffer
import java.nio.ByteOrder

@RunWith(RobolectricTestRunner::class)
class SpentClaimsPersistenceTest {
    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler
    private val walletId = ByteArray(32) { 1 }
    private val txid = ByteArray(32) { it.toByte() }
    private val claimant = ByteArray(32) { 7 }
    private val outpoint get() = txid + ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(-1).array()

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
    }

    @After
    fun tearDown() {
        handler.close()
        db.close()
    }

    private fun register(wallet: ByteArray = walletId) {
        assertEquals(0, handler.onPersistWalletMetadata(wallet, 1, ByteArray(32), 0))
        assertEquals(0, handler.onPersistAccountRegistration(
            wallet, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 4 },
        ))
    }

    private fun claim(owner: ByteArray?, release: Boolean = false): Int = handler.onPersistSpentClaims(
        walletId, outpoint, byteArrayOf(if (owner == null) 0 else 1),
        owner ?: ByteArray(32), if (release) outpoint else ByteArray(0),
    )

    @Test
    fun shouldRestoreUnknownClaimantsAndPreserveOrderedReleaseAndReclaim() = runTest {
        register()
        assertEquals(0, handler.onChangesetBegin(walletId))
        assertEquals(0, claim(claimant, release = true))
        assertEquals(0, claim(null))
        assertEquals(0, handler.onChangesetEnd(walletId, true))
        val unknown = handler.onLoadWalletList().single().spentClaims.single()
        assertArrayEquals(txid, unknown.txid)
        assertEquals(-1, unknown.vout)
        assertNull(unknown.claimant)
        assertEquals(0, handler.onChangesetBegin(walletId))
        assertEquals(0, claim(claimant))
        assertEquals(0, handler.onChangesetEnd(walletId, true))
        assertArrayEquals(claimant, handler.onLoadWalletList().single().spentClaims.single().claimant)
        assertEquals(0, handler.onChangesetBegin(walletId))
        assertEquals(0, claim(null, release = true))
        assertEquals(0, handler.onChangesetEnd(walletId, true))
        assertTrue(handler.onLoadWalletList().single().spentClaims.isEmpty())
    }

    @Test
    fun shouldRollbackClaimsAndMetadataTogether() = runTest {
        handler.onChangesetBegin(walletId)
        register()
        claim(claimant)
        assertNull(db.walletDao().getByWalletId(walletId))
        handler.onChangesetEnd(walletId, false)
        assertNull(db.walletDao().getByWalletId(walletId))
        assertTrue(db.spentClaimDao().getAll().isEmpty())
        handler.onChangesetBegin(walletId)
        register()
        claim(null)
        handler.onChangesetEnd(walletId, true)
        assertTrue(db.walletDao().getByWalletId(walletId)!!.spentClaimsComplete)
        assertEquals(1, db.spentClaimDao().getAll().size)
        handler.onChangesetBegin(walletId)
        claim(claimant, release = true)
        handler.onChangesetEnd(walletId, false)
        assertNull(db.spentClaimDao().getAll().single().claimant)
    }

    @Test
    fun shouldRollbackWholeRoundWhenClaimWriteFails() = runTest {
        db.openHelper.writableDatabase.execSQL(
            "CREATE TRIGGER reject_known_claim BEFORE INSERT ON core_spent_claims " +
                "WHEN NEW.claimant IS NOT NULL BEGIN SELECT RAISE(ABORT, 'injected claim failure'); END",
        )
        handler.onChangesetBegin(walletId)
        register()
        assertEquals(0, claim(null))
        assertEquals(0, claim(claimant))
        assertNotEquals(0, handler.onChangesetEnd(walletId, true))
        assertNull(db.walletDao().getByWalletId(walletId))
        assertTrue(db.spentClaimDao().getAll().isEmpty())
    }

    @Test
    fun shouldNotInferAuthoritativeClaimsFromLegacySweepStamps() = runTest {
        register()
        db.txoDao().upsert(TxoEntity(
            outpoint = outpoint, vout = -1, amount = 1, address = "legacy",
            walletId = walletId, isSpent = true, supersededByTxid = claimant,
        ))
        assertTrue(handler.onLoadWalletList().single().spentClaims.isEmpty())
    }

    @Test
    fun shouldKeepLegacyWalletIncompleteAfterMetadataAndPartialClaimWrites() = runTest {
        db.walletDao().upsert(WalletEntity(walletId = walletId))
        register()
        handler.onChangesetBegin(walletId)
        claim(null)
        handler.onChangesetEnd(walletId, true)
        assertFalse(db.walletDao().getByWalletId(walletId)!!.spentClaimsComplete)
        assertThrows(Exception::class.java) { handler.onLoadWalletList() }
    }

    @Test
    fun shouldScopeGroupedClaimsByWalletAndCascadeDeletion() = runTest {
        register()
        val other = ByteArray(32) { 2 }
        register(other)
        handler.onChangesetBegin(walletId)
        claim(null)
        handler.onChangesetEnd(walletId, true)
        val loaded = handler.onLoadWalletList().associateBy { it.walletId.first() }
        assertEquals(1, loaded.getValue(1.toByte()).spentClaims.size)
        assertTrue(loaded.getValue(2.toByte()).spentClaims.isEmpty())
        db.walletDao().deleteByWalletId(walletId)
        assertTrue(db.spentClaimDao().getAll().isEmpty())
    }

    @Test
    fun shouldRejectUnbracketedAndMalformedClaimBatches() = runTest {
        register()
        assertNotEquals(0, claim(null))
        handler.onChangesetBegin(walletId)
        assertNotEquals(0, handler.onPersistSpentClaims(walletId, outpoint, byteArrayOf(1), ByteArray(0), ByteArray(0)))
        handler.onChangesetEnd(walletId, false)
        assertTrue(db.spentClaimDao().getAll().isEmpty())
    }
}
