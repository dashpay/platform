package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * A top-up of an identity from the shielded pool (transition type 22) is
 * recorded by Rust as a pending activity row of kind 9 carrying the identity
 * it tops up. The top-up screen lists that identity's pending rows so an
 * interrupted top-up is visible when it reopens; the row has to come back
 * from a new handler over the same database (the process restart), and stop
 * being listed once a shielded sync confirms it.
 */
@RunWith(RobolectricTestRunner::class)
class ShieldedIdentityTopUpActivityPersistenceTest {

    private lateinit var db: DashDatabase

    private val walletId = ByteArray(32) { 1 }
    private val entryId = ByteArray(32) { 7 }
    private val identityId = ByteArray(32) { 3 }
    private val otherIdentityId = ByteArray(32) { 4 }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun handler() = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)

    private fun persistTopUp(handler: PlatformWalletPersistenceHandler, status: Int): Int {
        handler.onChangesetBegin(walletId)
        handler.onPersistShieldedActivity(
            walletId = walletId,
            noteWalletId = walletId,
            accountIndex = 0,
            entryId = entryId,
            kindTag = KIND_IDENTITY_TOP_UP,
            direction = 1,
            status = status.toByte(),
            amount = 50_000_000_000,
            fee = 114_140_000,
            hasFee = true,
            blockHeight = 0,
            hasBlockHeight = false,
            createdAtMs = 1_700_000_000_000,
            identityId = identityId,
            hasIdentityId = true,
            counterparty = ByteArray(0),
            memo = ByteArray(0),
            noteCmxs = ByteArray(32) { 5 },
            spentNullifiers = ByteArray(32) { 6 },
        )
        return handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun shouldListAPendingShieldedTopUpOfTheIdentityAfterARestart() = runTest {
        assertEquals(0, persistTopUp(handler(), status = 0))

        // A new handler over the same database: what the app reads on relaunch.
        val restored = handler().onLoadShieldedActivity().single()
        assertEquals(KIND_IDENTITY_TOP_UP, restored.kindTag)
        assertEquals("still pending", 0.toByte(), restored.status)
        assertArrayEquals("the identity it tops up", identityId, restored.identityId)

        val listed = db.shieldedDao().observePendingIdentityTopUps(walletId, identityId).first()
        assertEquals(1, listed.size)
        assertEquals(50_000_000_000L, listed.single().amount)
        assertTrue(
            "another identity's screen does not list it",
            db.shieldedDao().observePendingIdentityTopUps(walletId, otherIdentityId).first().isEmpty(),
        )
    }

    @Test
    fun shouldStopListingTheTopUpOnceTheScanConfirmsIt() = runTest {
        val handler = handler()
        persistTopUp(handler, status = 0)
        assertEquals(0, persistTopUp(handler, status = 1))

        assertTrue(db.shieldedDao().observePendingIdentityTopUps(walletId, identityId).first().isEmpty())
        assertEquals("one row per top-up", 1, db.shieldedDao().getAllActivity().size)
    }

    private companion object {
        /** `ShieldedActivityKind::IdentityTopUp`. */
        const val KIND_IDENTITY_TOP_UP: Byte = 9
    }
}
