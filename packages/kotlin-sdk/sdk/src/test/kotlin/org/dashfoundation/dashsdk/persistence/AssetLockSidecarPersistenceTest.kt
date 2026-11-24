package org.dashfoundation.dashsdk.persistence

import android.database.sqlite.SQLiteException
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The three asset-lock sidecars [PlatformWalletPersistenceHandler] keeps on
 * `asset_locks` — operation UUID (16 bytes), shielded recipient (43) and
 * identity recipient (32) — against the contract of the `PersistAssetLock*Fn`
 * / `LoadAssetLock*Fn` typedefs in
 * `packages/rs-platform-wallet-ffi/src/persistence.rs`: written after the
 * row's upsert in the same round, immutable once stored, kept through later
 * upserts, and a load of a missing or malformed row fails instead of reading
 * as "none". Parity with Swift `PlatformWalletPersistenceHandler`'s
 * `persistAssetLockOperationId` / `persistAssetLockShieldedRecipient` and the
 * SQLite store's V019, V020 and V023 triggers.
 */
@RunWith(RobolectricTestRunner::class)
class AssetLockSidecarPersistenceTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

    private val walletId = ByteArray(32) { 1 }
    private val operationId = ByteArray(16) { 0x0A }
    private val shieldedRecipient = ByteArray(43) { 0x0B }
    private val identityRecipient = ByteArray(32) { 0x0C }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun outpoint(seed: Int, vout: Int = 0) = makeOutpoint(ByteArray(32) { seed.toByte() }, vout)

    private fun upsert(outPoint: ByteArray, status: Byte = 1, fundingType: Byte = 2): Int =
        handler.onPersistAssetLockUpsert(
            walletId = walletId,
            outPoint = outPoint,
            transactionBytes = ByteArray(20) { 41 },
            accountIndex = 0,
            fundingType = fundingType,
            identityIndex = 0,
            amountDuffs = 100_000,
            status = status,
            proofBytes = null,
        )

    /** One committed round: [body] runs between the begin and end callbacks. */
    private fun round(body: () -> Unit): Int {
        handler.onChangesetBegin(walletId)
        body()
        return handler.onChangesetEnd(walletId, success = true)
    }

    private suspend fun row(outPoint: ByteArray) =
        db.assetLockDao().getByWalletAndOutPointHex(walletId, encodeOutPointHex(outPoint))

    @Test
    fun shouldStoreEachSidecarAfterItsUpsertInTheSameRoundAndLoadItBack() = runTest {
        val op = outpoint(1)
        assertEquals(
            0,
            round {
                assertEquals(0, upsert(op))
                assertEquals(0, handler.onPersistAssetLockOperationId(walletId, op, operationId))
                assertEquals(0, handler.onPersistAssetLockShieldedRecipient(walletId, op, shieldedRecipient))
                assertEquals(0, handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient))
            },
        )

        assertArrayEquals(operationId, handler.onLoadAssetLockOperationId(walletId, op))
        assertArrayEquals(shieldedRecipient, handler.onLoadAssetLockShieldedRecipient(walletId, op))
        assertArrayEquals(identityRecipient, handler.onLoadAssetLockIdentityRecipient(walletId, op))
    }

    @Test
    fun shouldLoadNullForARowRecordingNone() {
        val op = outpoint(2)
        round { upsert(op) }

        assertNull(handler.onLoadAssetLockOperationId(walletId, op))
        assertNull(handler.onLoadAssetLockShieldedRecipient(walletId, op))
        assertNull(handler.onLoadAssetLockIdentityRecipient(walletId, op))
    }

    @Test
    fun shouldFailTheLoadOfAMissingRowRatherThanReadNone() {
        val missing = outpoint(3)
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockOperationId(walletId, missing)
        }
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockShieldedRecipient(walletId, missing)
        }
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockIdentityRecipient(walletId, missing)
        }
        // Another wallet's row is not this wallet's.
        round { upsert(missing) }
        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockIdentityRecipient(ByteArray(32) { 9 }, missing)
        }
    }

    @Test
    fun shouldFailTheLoadOfAStoredValueOfTheWrongWidth() = runTest {
        val op = outpoint(4)
        round { upsert(op) }
        val stored = requireNotNull(row(op))
        db.assetLockDao().upsert(stored.copy(identityRecipientId = ByteArray(31)))

        assertThrows(IllegalStateException::class.java) {
            handler.onLoadAssetLockIdentityRecipient(walletId, op)
        }
    }

    @Test
    fun shouldKeepTheSidecarsThroughALaterUpsertOfTheRow() = runTest {
        val op = outpoint(5)
        round {
            upsert(op, status = 1)
            handler.onPersistAssetLockOperationId(walletId, op, operationId)
            handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient)
        }
        // The status advance native sends next carries no sidecar.
        assertEquals(0, round { upsert(op, status = 3) })

        val stored = requireNotNull(row(op))
        assertEquals(3, stored.statusRaw)
        assertArrayEquals(operationId, stored.operationId)
        assertArrayEquals(identityRecipient, stored.identityRecipientId)
    }

    /**
     * An assignment arrives as a round of its own carrying the identity
     * recipient alone, with no upsert of the row: it records the recipient on
     * the stored row and leaves everything else as stored, including a status
     * the row reached after the wallet read it.
     */
    @Test
    fun shouldRecordAnAssignmentRoundOnTheStoredRowAlone() = runTest {
        val op = outpoint(13)
        round { upsert(op, status = 1) }
        round { upsert(op, status = 4) }

        assertEquals(
            0,
            round {
                assertEquals(0, handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient))
            },
        )

        val stored = requireNotNull(row(op))
        assertEquals(4, stored.statusRaw)
        assertArrayEquals(identityRecipient, stored.identityRecipientId)
    }

    @Test
    fun shouldAcceptTheSameValueAgainAndRefuseADifferentOne() = runTest {
        val op = outpoint(6)
        round {
            upsert(op)
            handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient)
        }

        assertEquals(
            0,
            round {
                assertEquals(0, handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient))
            },
        )
        handler.onChangesetBegin(walletId)
        val refused = handler.onPersistAssetLockIdentityRecipient(walletId, op, ByteArray(32) { 0x0D })
        handler.onChangesetEnd(walletId, success = false)

        assertEquals(NativePersistenceBridge.PERSIST_RC_CONSTRAINT, refused)
        assertArrayEquals(identityRecipient, requireNotNull(row(op)).identityRecipientId)
    }

    /**
     * A consumed lock pays no one again, so a snapshot naming another identity
     * is stale: skipped, as the SQLite store skips it, instead of failing the
     * round. The shielded recipient keeps the strict rule.
     */
    @Test
    fun shouldSkipADifferentIdentityRecipientForAConsumedRow() = runTest {
        for (terminal in listOf<Byte>(4, 6)) {
            val op = outpoint(20 + terminal)
            round {
                upsert(op, status = terminal)
                handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient)
                handler.onPersistAssetLockShieldedRecipient(walletId, op, shieldedRecipient)
            }

            assertEquals(
                0,
                round {
                    assertEquals(
                        0,
                        handler.onPersistAssetLockIdentityRecipient(walletId, op, ByteArray(32) { 0x0D }),
                    )
                },
            )
            assertArrayEquals(identityRecipient, requireNotNull(row(op)).identityRecipientId)

            handler.onChangesetBegin(walletId)
            val refused = handler.onPersistAssetLockShieldedRecipient(walletId, op, ByteArray(43) { 0x0E })
            handler.onChangesetEnd(walletId, success = false)
            assertEquals(NativePersistenceBridge.PERSIST_RC_CONSTRAINT, refused)
        }
    }

    @Test
    fun shouldFailTheRoundWhenItRecordsTwoDifferentValuesForOneRow() = runTest {
        val op = outpoint(7)
        val end = round {
            upsert(op)
            handler.onPersistAssetLockShieldedRecipient(walletId, op, shieldedRecipient)
            handler.onPersistAssetLockShieldedRecipient(walletId, op, ByteArray(43) { 0x0E })
        }

        assertNotEquals(0, end)
        assertNull("the whole round rolls back", row(op))
    }

    @Test
    fun shouldRefuseAnOperationIdAnotherRowOfTheWalletHolds() = runTest {
        val first = outpoint(8)
        val second = outpoint(9)
        round {
            upsert(first)
            handler.onPersistAssetLockOperationId(walletId, first, operationId)
            upsert(second)
        }

        handler.onChangesetBegin(walletId)
        val refused = handler.onPersistAssetLockOperationId(walletId, second, operationId)
        handler.onChangesetEnd(walletId, success = false)

        assertEquals(NativePersistenceBridge.PERSIST_RC_CONSTRAINT, refused)
        assertNull(requireNotNull(row(second)).operationId)
    }

    @Test
    fun shouldRefuseASidecarOutsideAChangesetRound() = runTest {
        val op = outpoint(10)
        round { upsert(op) }

        assertNotEquals(0, handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient))
        assertNull(requireNotNull(row(op)).identityRecipientId)
    }

    @Test
    fun shouldFailTheRoundWhenTheRowDoesNotExist() = runTest {
        val op = outpoint(11)
        val end = round {
            assertEquals(0, handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient))
        }

        assertNotEquals(0, end)
        assertNull(row(op))
    }

    @Test
    fun shouldRefuseAValueOfTheWrongWidth() {
        val op = outpoint(12)
        round { upsert(op) }
        handler.onChangesetBegin(walletId)
        assertEquals(
            NativePersistenceBridge.PERSIST_RC_CONSTRAINT,
            handler.onPersistAssetLockOperationId(walletId, op, ByteArray(15)),
        )
        assertEquals(
            NativePersistenceBridge.PERSIST_RC_CONSTRAINT,
            handler.onPersistAssetLockShieldedRecipient(walletId, op, ByteArray(32)),
        )
        assertEquals(
            NativePersistenceBridge.PERSIST_RC_CONSTRAINT,
            handler.onPersistAssetLockIdentityRecipient(walletId, op, ByteArray(43)),
        )
        handler.onChangesetEnd(walletId, success = false)
    }

    @Test
    fun shouldRefuseAChangedSidecarAtTheSqlLayerToo() = runTest {
        val op = outpoint(13)
        round {
            upsert(op)
            handler.onPersistAssetLockOperationId(walletId, op, operationId)
            handler.onPersistAssetLockShieldedRecipient(walletId, op, shieldedRecipient)
            handler.onPersistAssetLockIdentityRecipient(walletId, op, identityRecipient)
        }
        val stored = requireNotNull(row(op))

        for (changed in listOf(
            stored.copy(operationId = ByteArray(16) { 0x7F }),
            stored.copy(shieldedRecipientRaw43 = ByteArray(43) { 0x7F }),
            stored.copy(identityRecipientId = ByteArray(32) { 0x7F }),
            stored.copy(identityRecipientId = null),
        )) {
            assertThrows(SQLiteException::class.java) {
                runBlocking { db.assetLockDao().upsert(changed) }
            }
        }
        // The same values again are not a change.
        db.assetLockDao().upsert(stored.copy(statusRaw = 3))
        assertEquals(3, requireNotNull(row(op)).statusRaw)
    }

    @Test
    fun shouldKeepADrainCompletedRowThroughAConsumedUpsertAndARemoval() = runTest {
        val op = outpoint(14)
        round { upsert(op, status = 6) }

        round { upsert(op, status = 4) }
        assertEquals(6, requireNotNull(row(op)).statusRaw)

        round { handler.onPersistAssetLockRemoval(walletId, op) }
        assertNotNull(row(op))

        // A Consumed row may still advance to DrainCompleted.
        val consumed = outpoint(15)
        round { upsert(consumed, status = 4) }
        round { upsert(consumed, status = 6) }
        assertEquals(6, requireNotNull(row(consumed)).statusRaw)
    }
}
