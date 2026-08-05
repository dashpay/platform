package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityIndexStateEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.services.DataManager
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.fail
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class IdentityIndexReservationStoreTest {
    private lateinit var database: DashDatabase
    private lateinit var store: RoomIdentityIndexReservationStore
    private val walletId = ByteArray(32) { 7 }

    @Before
    fun setUp() {
        database = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        store = RoomIdentityIndexReservationStore(database)
    }

    @After
    fun tearDown() {
        database.close()
    }

    @Test
    fun nativeSuccessWithoutIdentityRowNeverReusesIssuedSlot() = runTest {
        assertEquals(0, store.reserveNext(walletId))

        // Models native success followed by the Rust best-effort Room write
        // being absent: allocation durability must not depend on an identity row.
        assertEquals(1, store.reserveNext(walletId))
        assertEquals(1, database.identityIndexStateDao().get(walletId)?.lastIssuedIndex)
    }

    @Test
    fun clearingWalletRowsDoesNotMakeAnIssuedIdentitySlotReusable() = runTest {
        assertEquals(0, store.reserveNext(walletId))

        DataManager(database).clear(DataManager.Category.WALLETS)

        assertEquals(1, store.reserveNext(walletId))
    }

    @Test
    fun simultaneousReservationsAreDistinct() = runTest {
        val issued = listOf(
            async { store.reserveNext(walletId) },
            async { store.reserveNext(walletId) },
        ).awaitAll()

        assertEquals(setOf(0, 1), issued.toSet())
    }

    @Test
    fun committedIdentityAndDurableFloorBothParticipate() = runTest {
        database.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = 1))
        database.identityDao().upsert(
            IdentityEntity(
                identityId = ByteArray(32) { 9 },
                networkRaw = 1,
                walletId = walletId,
                identityIndex = 4,
            ),
        )
        database.identityIndexStateDao().upsert(IdentityIndexStateEntity(walletId, 7))

        assertEquals(8, store.reserveNext(walletId))
    }

    @Test
    fun freshExactSlotBelowFloorIsRejectedButTrackedResumeCanReuseItsOwnSlot() = runTest {
        database.identityIndexStateDao().upsert(IdentityIndexStateEntity(walletId, 5))

        try {
            store.reserveFreshExact(walletId, 3)
            fail("an issued identity index must not be reusable")
        } catch (_: IllegalStateException) {
        }
        store.reserveResumeExact(walletId, 3)
        assertEquals(5, database.identityIndexStateDao().get(walletId)?.lastIssuedIndex)
    }

    @Test
    fun nextReservationFailsAtIntMaxInsteadOfOverflowing() = runTest {
        database.identityIndexStateDao().upsert(
            IdentityIndexStateEntity(walletId, Int.MAX_VALUE),
        )

        try {
            store.reserveNext(walletId)
            fail("Int.MAX_VALUE must fail rather than overflow")
        } catch (_: IllegalStateException) {
        }
    }
}
