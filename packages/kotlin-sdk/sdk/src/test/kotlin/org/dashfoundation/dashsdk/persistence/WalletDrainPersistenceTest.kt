package org.dashfoundation.dashsdk.persistence

import android.database.sqlite.SQLiteConstraintException
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.persistence.entities.WalletDrainOperationEntity
import org.dashfoundation.dashsdk.wallet.PlatformWalletPersistenceCapabilities
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The persistence a Core to shielded "Max" (a wallet drain) relies on: the
 * operation id Rust stores on the asset lock it builds, the terminal
 * DrainCompleted status, the fail-closed lock load, and the table of ids a
 * host stores before it calls the drain. Mirrors the Swift
 * `WalletDrainTests` persistence cases in
 * `packages/swift-sdk/SwiftTests/SwiftDashSDKTests/WalletDrainTests.swift`.
 */
@RunWith(RobolectricTestRunner::class)
class WalletDrainPersistenceTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

    private val walletId = ByteArray(32) { 1 }
    private val otherWalletId = ByteArray(32) { 9 }
    private val groupId = ByteArray(32) { 2 }
    private val operationId = ByteArray(16) { (0x10 + it).toByte() }
    private val otherOperationId = ByteArray(16) { (0x40 + it).toByte() }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
    }

    @After
    fun tearDown() {
        db.close()
    }

    /** A shielded top-up lock (funding type 5) at [status], as a drain builds. */
    private fun upsertLock(
        outPoint: ByteArray,
        status: Int,
        wallet: ByteArray = walletId,
    ): Int = handler.onPersistAssetLockUpsert(
        walletId = wallet,
        outPoint = outPoint,
        transactionBytes = ByteArray(20) { 41 },
        accountIndex = 0,
        fundingType = 5,
        identityIndex = 0,
        amountDuffs = 100_000,
        status = status.toByte(),
        proofBytes = null,
    )

    /** One round: the lock at [status], then its operation id, as Rust sends them. */
    private fun storeLockWithId(
        outPoint: ByteArray,
        status: Int,
        id: ByteArray = operationId,
        wallet: ByteArray = walletId,
    ): Int {
        handler.onChangesetBegin(wallet)
        upsertLock(outPoint, status, wallet)
        handler.onPersistAssetLockOperationId(wallet, outPoint, id)
        return handler.onChangesetEnd(wallet, success = true)
    }

    private suspend fun storedLock(outPoint: ByteArray) =
        db.assetLockDao().getByOutPointHex(encodeOutPointHex(outPoint))

    @Test
    fun shouldAttestTheOperationIdCapability() {
        val bits = handler.persistenceCapabilitiesBits()
        assertEquals(0x2000L, NativePersistenceBridge.CAPABILITY_ASSET_LOCK_OPERATION_IDS)
        assertNotEquals(0L, bits and NativePersistenceBridge.CAPABILITY_ASSET_LOCK_OPERATION_IDS)
        assertTrue(
            PlatformWalletPersistenceCapabilities(1, bits)
                .contains(PlatformWalletPersistenceCapabilities.ASSET_LOCK_OPERATION_IDS),
        )
        // `PersistenceCapabilities::WALLET_DRAIN` (0x2285): atomic changesets,
        // funding indices, wallet restore, tracked asset locks, operation ids.
        assertEquals(0x2285L, bits and 0x2285L)
    }

    @Test
    fun shouldStoreAnOperationIdWithItsLockAndLoadItBack() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 40 }, 1)
        assertEquals(0, storeLockWithId(outPoint, status = 0))

        assertArrayEquals(operationId, storedLock(outPoint)!!.operationId)
        assertArrayEquals(operationId, handler.onLoadAssetLockOperationId(walletId, outPoint))
    }

    @Test
    fun shouldKeepTheOperationIdThroughLaterStatusUpsertsAndOnTheTerminalRow() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 42 }, 0)
        storeLockWithId(outPoint, status = 0)

        // Rust re-sends the id with every upsert of a drain's lock.
        for (status in listOf(1, 2, 6)) {
            assertEquals(0, storeLockWithId(outPoint, status))
            assertEquals(status, storedLock(outPoint)!!.statusRaw)
            assertArrayEquals(operationId, storedLock(outPoint)!!.operationId)
        }
        // A status upsert that arrives without the id callback keeps it too.
        val bare = makeOutpoint(ByteArray(32) { 43 }, 0)
        storeLockWithId(bare, status = 0, id = otherOperationId)
        handler.onChangesetBegin(walletId)
        upsertLock(bare, status = 1)
        assertEquals(0, handler.onChangesetEnd(walletId, success = true))
        assertEquals(1, storedLock(bare)!!.statusRaw)
        assertArrayEquals(otherOperationId, storedLock(bare)!!.operationId)
        assertArrayEquals(otherOperationId, handler.onLoadAssetLockOperationId(walletId, bare))
    }

    @Test
    fun shouldLoadNoOperationIdForALockThatHasNone() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 44 }, 0)
        handler.onChangesetBegin(walletId)
        upsertLock(outPoint, status = 1)
        handler.onChangesetEnd(walletId, success = true)

        assertNull(handler.onLoadAssetLockOperationId(walletId, outPoint))
    }

    @Test
    fun shouldFailTheLoadOfAnOperationIdWhenItsLockIsNotStored() = runTest {
        val stored = makeOutpoint(ByteArray(32) { 45 }, 0)
        storeLockWithId(stored, status = 1)

        // No such lock: not evidence that the lock has no id.
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockOperationId(walletId, makeOutpoint(ByteArray(32) { 46 }, 0))
        }
        // The lock of another wallet is not this wallet's lock.
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockOperationId(otherWalletId, stored)
        }
        assertThrows(IllegalArgumentException::class.java) {
            handler.onLoadAssetLockOperationId(walletId, ByteArray(35))
        }
    }

    @Test
    fun shouldFailTheLoadOfAnOperationIdOfTheWrongSize() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 47 }, 0)
        storeLockWithId(outPoint, status = 1)
        db.assetLockDao().upsert(storedLock(outPoint)!!.copy(operationId = ByteArray(15)))

        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockOperationId(walletId, outPoint)
        }
    }

    @Test
    fun shouldFailTheRoundWhenAnOperationIdHasNoLockToRideWith() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 48 }, 0)
        val other = makeOutpoint(ByteArray(32) { 49 }, 0)
        handler.onChangesetBegin(walletId)
        upsertLock(other, status = 0)
        assertEquals(0, handler.onPersistAssetLockOperationId(walletId, outPoint, operationId))

        assertNotEquals(
            "an id whose lock was never upserted must fail the commit",
            0,
            handler.onChangesetEnd(walletId, success = true),
        )
        assertNull("the round rolls back whole", storedLock(other))
    }

    @Test
    fun shouldRefuseADifferentOperationIdForTheSameLock() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 50 }, 0)
        storeLockWithId(outPoint, status = 0)

        assertNotEquals(0, storeLockWithId(outPoint, status = 1, id = otherOperationId))

        val row = storedLock(outPoint)!!
        assertArrayEquals("an operation id never changes", operationId, row.operationId)
        assertEquals("the refused round's status upsert rolled back with it", 0, row.statusRaw)
    }

    @Test
    fun shouldRefuseTheSameOperationIdOnASecondLockOfTheWallet() = runTest {
        val first = makeOutpoint(ByteArray(32) { 51 }, 0)
        val second = makeOutpoint(ByteArray(32) { 52 }, 0)
        storeLockWithId(first, status = 0)

        assertNotEquals(0, storeLockWithId(second, status = 0))
        assertNull("the second lock did not land", storedLock(second))

        // Another wallet may carry the same bytes: an id names one lock per wallet.
        assertEquals(0, storeLockWithId(second, status = 0, wallet = otherWalletId))
        assertArrayEquals(operationId, storedLock(second)!!.operationId)
    }

    @Test
    fun shouldRefuseAnOperationIdOutsideARoundOrOfTheWrongSize() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 53 }, 0)
        handler.onChangesetBegin(walletId)
        upsertLock(outPoint, status = 0)
        handler.onChangesetEnd(walletId, success = true)

        assertNotEquals(
            "outside a round there is no upsert for the id to commit with",
            0,
            handler.onPersistAssetLockOperationId(walletId, outPoint, operationId),
        )
        assertNull(storedLock(outPoint)!!.operationId)

        handler.onChangesetBegin(walletId)
        assertNotEquals(0, handler.onPersistAssetLockOperationId(walletId, outPoint, ByteArray(15)))
        assertNotEquals(0, handler.onPersistAssetLockOperationId(walletId, ByteArray(35), operationId))
        handler.onChangesetEnd(walletId, success = false)
    }

    @Test
    fun shouldLeaveNoOperationIdBehindWhenTheRoundRollsBack() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 54 }, 0)
        handler.onChangesetBegin(walletId)
        upsertLock(outPoint, status = 0)
        handler.onPersistAssetLockOperationId(walletId, outPoint, operationId)
        assertEquals(0, handler.onChangesetEnd(walletId, success = false))

        assertNull(storedLock(outPoint))
    }

    @Test
    fun shouldTreatDrainCompletedAsTerminalAndLetConsumedReachIt() = runTest {
        for ((existing, incoming, regresses) in listOf(
            Triple(6, 6, false),
            Triple(6, 4, true),
            Triple(6, 1, true),
            Triple(4, 6, false),
            Triple(4, 4, false),
            Triple(4, 3, true),
            Triple(3, 1, false),
            Triple(5, 2, false),
        )) {
            assertEquals(
                "existing $existing, incoming $incoming",
                regresses,
                PlatformWalletPersistenceHandler.assetLockUpsertRegressesTerminal(existing, incoming),
            )
        }

        val outPoint = makeOutpoint(ByteArray(32) { 55 }, 0)
        storeLockWithId(outPoint, status = 4)
        storeLockWithId(outPoint, status = 6)
        assertEquals("Consumed may become DrainCompleted", 6, storedLock(outPoint)!!.statusRaw)

        storeLockWithId(outPoint, status = 4)
        storeLockWithId(outPoint, status = 1)
        assertEquals("nothing replaces DrainCompleted", 6, storedLock(outPoint)!!.statusRaw)

        handler.onChangesetBegin(walletId)
        handler.onPersistAssetLockRemoval(walletId, outPoint)
        handler.onChangesetEnd(walletId, success = true)
        assertNotNull("a removal never takes the drain's receipt", storedLock(outPoint))
    }

    /** A wallet the load restores: metadata plus one registered account. */
    private fun registerRestorableWallet() {
        handler.onPersistWalletMetadata(walletId, 1, groupId, 0)
        handler.onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 30 },
        )
    }

    @Test
    fun shouldRestoreADrainsLockAtEveryStatusIncludingTheTerminalOne() = runTest {
        registerRestorableWallet()
        val built = makeOutpoint(ByteArray(32) { 56 }, 0)
        val completed = makeOutpoint(ByteArray(32) { 57 }, 0)
        storeLockWithId(built, status = 0)
        storeLockWithId(completed, status = 6, id = otherOperationId)

        val restored = handler.onLoadWalletList().single().trackedAssetLocks
        assertEquals(
            setOf(0.toByte(), 6.toByte()),
            restored.map { it.status }.toSet(),
        )
    }

    @Test
    fun shouldFailTheWalletLoadOnAnAssetLockRowItCannotHandBack() = runTest {
        registerRestorableWallet()
        val outPoint = makeOutpoint(ByteArray(32) { 58 }, 0)
        storeLockWithId(outPoint, status = 1)
        assertEquals(1, handler.onLoadWalletList().single().trackedAssetLocks.size)

        // A row with no transaction bytes cannot be restored. Dropping it
        // would hand Rust a wallet without the drain's lock, which a status
        // read reports as "not sent".
        db.assetLockDao().upsert(storedLock(outPoint)!!.copy(transactionBytes = ByteArray(0)))
        assertThrows(IllegalStateException::class.java) { handler.onLoadWalletList() }

        db.assetLockDao().upsert(
            storedLock(outPoint)!!.copy(transactionBytes = ByteArray(20) { 41 }, statusRaw = 300),
        )
        assertThrows(IllegalStateException::class.java) { handler.onLoadWalletList() }
    }

    private fun drainOperation(
        id: ByteArray,
        wallet: ByteArray = walletId,
        createdAtMillis: Long = 1_000,
    ) = WalletDrainOperationEntity(
        operationId = id,
        walletId = wallet,
        networkRaw = 1,
        fundingAccountIndex = 0,
        recipientRaw43 = ByteArray(43) { 7 },
        createdAtMillis = createdAtMillis,
    )

    @Test
    fun shouldKeepStoredDrainOperationsOldestFirstAndRemoveThemById() = runTest {
        val dao = db.walletDrainOperationDao()
        val newer = drainOperation(otherOperationId, createdAtMillis = 2_000)
        val older = drainOperation(operationId, createdAtMillis = 1_000)
        dao.insert(newer)
        dao.insert(older)

        assertEquals(listOf(older, newer), dao.getAll())
        assertEquals(listOf(older, newer), dao.observeAll().first())
        assertEquals(2L, dao.count().first())

        assertEquals(1, dao.deleteByOperationId(operationId))
        assertEquals("removing an id that is gone is a no-op", 0, dao.deleteByOperationId(operationId))
        assertEquals(listOf(newer), dao.getAll())
    }

    @Test
    fun shouldRefuseToStoreTheSameDrainOperationIdTwice() = runTest {
        val dao = db.walletDrainOperationDao()
        dao.insert(drainOperation(operationId))

        // REPLACE would silently rewrite the recipient a resume pays.
        var refused = false
        try {
            dao.insert(drainOperation(operationId, wallet = otherWalletId))
        } catch (e: SQLiteConstraintException) {
            refused = true
        }
        assertTrue(refused)
        assertTrue(dao.getAll().single().walletId.contentEquals(walletId))
    }

    @Test
    fun shouldRemoveAWalletsStoredDrainOperationsWhenTheWalletIsDeleted() = runTest {
        val dao = db.walletDrainOperationDao()
        registerRestorableWallet()
        dao.insert(drainOperation(operationId))
        dao.insert(drainOperation(otherOperationId, wallet = otherWalletId))

        handler.deleteWalletData(walletId)

        val left = dao.getAll()
        assertEquals(1, left.size)
        assertFalse(left.single().walletId.contentEquals(walletId))
    }
}
