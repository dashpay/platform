package org.dashfoundation.dashsdk.persistence

import android.content.Context
import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.wallet.PlatformWalletPersistenceCapabilities
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The Orchard recipient Rust records on a shielded asset-lock funding
 * (`TrackedAssetLock::shielded_recipient`) and hands to the host through
 * `onPersistAssetLockShieldedRecipient` / `onLoadAssetLockShieldedRecipient`,
 * which is what lets this store attest `ASSET_LOCK_SHIELDED_RECIPIENTS`: a
 * resume after a restart pays the recorded recipient and refuses another.
 * Mirrors Swift `persistAssetLockShieldedRecipient` /
 * `loadAssetLockShieldedRecipient` in
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletPersistenceHandler.swift`.
 */
@RunWith(RobolectricTestRunner::class)
class AssetLockShieldedRecipientPersistenceTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

    private val walletId = ByteArray(32) { 1 }
    private val otherWalletId = ByteArray(32) { 9 }
    private val groupId = ByteArray(32) { 2 }
    private val recipient = ByteArray(43) { (0x20 + it).toByte() }
    private val otherRecipient = ByteArray(43) { (0x60 + it).toByte() }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
    }

    @After
    fun tearDown() {
        db.close()
    }

    /** A shielded top-up lock (funding type 5) at [status]. */
    private fun PlatformWalletPersistenceHandler.upsertLock(
        outPoint: ByteArray,
        status: Int,
        wallet: ByteArray = walletId,
    ): Int = onPersistAssetLockUpsert(
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

    /** One round: the lock at [status], then its recipient, as Rust sends them. */
    private fun PlatformWalletPersistenceHandler.storeLockWithRecipient(
        outPoint: ByteArray,
        status: Int,
        raw: ByteArray = recipient,
        wallet: ByteArray = walletId,
    ): Int {
        onChangesetBegin(wallet)
        upsertLock(outPoint, status, wallet)
        onPersistAssetLockShieldedRecipient(wallet, outPoint, raw)
        return onChangesetEnd(wallet, success = true)
    }

    private suspend fun storedLock(outPoint: ByteArray, database: DashDatabase = db) =
        database.assetLockDao().getByOutPointHex(encodeOutPointHex(outPoint))

    /** A wallet the load restores: metadata plus one registered account. */
    private fun PlatformWalletPersistenceHandler.registerRestorableWallet() {
        onPersistWalletMetadata(walletId, 1, groupId, 0)
        onPersistAccountRegistration(
            walletId, 0, 0, 0, 0, 0, ByteArray(0), ByteArray(0), ByteArray(78) { 30 },
        )
    }

    @Test
    fun shouldAttestTheShieldedRecipientCapability() {
        val bits = handler.persistenceCapabilitiesBits()
        assertEquals(0x4000L, NativePersistenceBridge.CAPABILITY_ASSET_LOCK_SHIELDED_RECIPIENTS)
        assertEquals(
            NativePersistenceBridge.CAPABILITY_ASSET_LOCK_SHIELDED_RECIPIENTS,
            bits and NativePersistenceBridge.CAPABILITY_ASSET_LOCK_SHIELDED_RECIPIENTS,
        )
        assertTrue(
            PlatformWalletPersistenceCapabilities(1, bits)
                .contains(PlatformWalletPersistenceCapabilities.ASSET_LOCK_SHIELDED_RECIPIENTS),
        )
    }

    @Test
    fun shouldStoreARecipientWithItsLockAndLoadItBack() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 40 }, 1)
        assertEquals(0, handler.storeLockWithRecipient(outPoint, status = 0))

        assertArrayEquals(recipient, storedLock(outPoint)!!.shieldedRecipientRaw43)
        assertArrayEquals(recipient, handler.onLoadAssetLockShieldedRecipient(walletId, outPoint))
    }

    @Test
    fun shouldKeepTheRecipientThroughLaterStatusUpsertsAndOnTheTerminalRow() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 41 }, 0)
        handler.storeLockWithRecipient(outPoint, status = 0)

        // Rust re-sends the recipient with every upsert of a recipient-bearing lock.
        for (status in listOf(1, 2, 4)) {
            assertEquals(0, handler.storeLockWithRecipient(outPoint, status))
            assertEquals(status, storedLock(outPoint)!!.statusRaw)
            assertArrayEquals(recipient, storedLock(outPoint)!!.shieldedRecipientRaw43)
        }
        // A status upsert that arrives without the recipient callback keeps it too.
        val bare = makeOutpoint(ByteArray(32) { 42 }, 0)
        handler.storeLockWithRecipient(bare, status = 0, raw = otherRecipient)
        handler.onChangesetBegin(walletId)
        handler.upsertLock(bare, status = 1)
        assertEquals(0, handler.onChangesetEnd(walletId, success = true))
        assertEquals(1, storedLock(bare)!!.statusRaw)
        assertArrayEquals(otherRecipient, handler.onLoadAssetLockShieldedRecipient(walletId, bare))
    }

    @Test
    fun shouldLoadNoRecipientForALockThatHasNone() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 43 }, 0)
        handler.onChangesetBegin(walletId)
        handler.upsertLock(outPoint, status = 1)
        handler.onChangesetEnd(walletId, success = true)

        assertNull(handler.onLoadAssetLockShieldedRecipient(walletId, outPoint))
    }

    @Test
    fun shouldFailTheLoadOfARecipientWhenItsLockIsNotStored() = runTest {
        val stored = makeOutpoint(ByteArray(32) { 44 }, 0)
        handler.storeLockWithRecipient(stored, status = 1)

        // No such lock: not evidence that the lock has no recipient.
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockShieldedRecipient(walletId, makeOutpoint(ByteArray(32) { 45 }, 0))
        }
        // The lock of another wallet is not this wallet's lock.
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockShieldedRecipient(otherWalletId, stored)
        }
        assertThrows(IllegalArgumentException::class.java) {
            handler.onLoadAssetLockShieldedRecipient(walletId, ByteArray(35))
        }
    }

    @Test
    fun shouldFailTheLoadOnAStoredRecipientOfTheWrongSize() = runTest {
        handler.registerRestorableWallet()
        val outPoint = makeOutpoint(ByteArray(32) { 46 }, 0)
        handler.storeLockWithRecipient(outPoint, status = 1)
        assertEquals(1, handler.onLoadWalletList().single().trackedAssetLocks.size)

        db.assetLockDao().upsert(storedLock(outPoint)!!.copy(shieldedRecipientRaw43 = ByteArray(42)))

        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockShieldedRecipient(walletId, outPoint)
        }
        // The wallet load fails too, as it does on the other asset-lock columns.
        assertThrows(IllegalStateException::class.java) { handler.onLoadWalletList() }
    }

    @Test
    fun shouldRefuseADifferentRecipientForTheSameLock() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 47 }, 0)
        handler.storeLockWithRecipient(outPoint, status = 0)

        assertNotEquals(0, handler.storeLockWithRecipient(outPoint, status = 1, raw = otherRecipient))

        val row = storedLock(outPoint)!!
        assertArrayEquals("a recorded recipient never changes", recipient, row.shieldedRecipientRaw43)
        assertEquals("the refused round's status upsert rolled back with it", 0, row.statusRaw)
    }

    @Test
    fun shouldFailTheRoundWhenARecipientHasNoLockToRideWith() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 48 }, 0)
        val other = makeOutpoint(ByteArray(32) { 49 }, 0)
        handler.onChangesetBegin(walletId)
        handler.upsertLock(other, status = 0)
        assertEquals(0, handler.onPersistAssetLockShieldedRecipient(walletId, outPoint, recipient))

        assertNotEquals(
            "a recipient whose lock was never upserted must fail the commit",
            0,
            handler.onChangesetEnd(walletId, success = true),
        )
        assertNull("the round rolls back whole", storedLock(other))
    }

    @Test
    fun shouldRefuseARecipientOutsideARoundOrOfTheWrongSize() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 50 }, 0)
        handler.onChangesetBegin(walletId)
        handler.upsertLock(outPoint, status = 0)
        handler.onChangesetEnd(walletId, success = true)

        assertNotEquals(
            "outside a round there is no upsert for the recipient to commit with",
            0,
            handler.onPersistAssetLockShieldedRecipient(walletId, outPoint, recipient),
        )
        assertNull(storedLock(outPoint)!!.shieldedRecipientRaw43)

        handler.onChangesetBegin(walletId)
        assertNotEquals(0, handler.onPersistAssetLockShieldedRecipient(walletId, outPoint, ByteArray(42)))
        assertNotEquals(0, handler.onPersistAssetLockShieldedRecipient(walletId, ByteArray(35), recipient))
        handler.onChangesetEnd(walletId, success = false)
    }

    @Test
    fun shouldLeaveNoRecipientBehindWhenTheRoundRollsBack() = runTest {
        val outPoint = makeOutpoint(ByteArray(32) { 51 }, 0)
        handler.onChangesetBegin(walletId)
        handler.upsertLock(outPoint, status = 0)
        handler.onPersistAssetLockShieldedRecipient(walletId, outPoint, recipient)
        assertEquals(0, handler.onChangesetEnd(walletId, success = false))

        assertNull(storedLock(outPoint))
    }

    /**
     * The restart: a file-backed database is written, closed and opened
     * again by a new handler, which restores the lock and hands back the
     * recipient it was built to pay.
     */
    @Test
    fun shouldRestoreTheRecipientAfterTheDatabaseIsClosedAndReopened() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val name = "shielded-recipient-restart-${System.nanoTime()}.db"
        fun open() = Room.databaseBuilder(context, DashDatabase::class.java, name)
            .addMigrations(DashDatabase.MIGRATION_19_20)
            .allowMainThreadQueries()
            .build()
        val outPoint = makeOutpoint(ByteArray(32) { 52 }, 0)
        val legacy = makeOutpoint(ByteArray(32) { 53 }, 0)
        try {
            val before = open()
            try {
                val writer = PlatformWalletPersistenceHandler(before, Dispatchers.Unconfined)
                writer.registerRestorableWallet()
                assertEquals(0, writer.storeLockWithRecipient(outPoint, status = 2))
                writer.onChangesetBegin(walletId)
                writer.upsertLock(legacy, status = 1)
                assertEquals(0, writer.onChangesetEnd(walletId, success = true))
            } finally {
                before.close()
            }

            val after = open()
            try {
                val reader = PlatformWalletPersistenceHandler(after, Dispatchers.Unconfined)
                val restored = reader.onLoadWalletList().single().trackedAssetLocks
                assertEquals(2, restored.size)
                // What the JNI load asks once per restored lock.
                for (lock in restored) {
                    val loaded = reader.onLoadAssetLockShieldedRecipient(walletId, lock.outPoint)
                    when {
                        lock.outPoint.contentEquals(outPoint) -> assertArrayEquals(recipient, loaded)
                        else -> assertNull("a lock stored without a recipient has none", loaded)
                    }
                }
                assertArrayEquals(recipient, storedLock(outPoint, after)!!.shieldedRecipientRaw43)
            } finally {
                after.close()
            }
        } finally {
            context.deleteDatabase(name)
        }
    }
}
