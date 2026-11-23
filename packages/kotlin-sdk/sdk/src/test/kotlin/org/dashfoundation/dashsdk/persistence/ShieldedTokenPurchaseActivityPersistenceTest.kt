package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * A shielded token purchase (transition type 28) is recorded by Rust as a
 * pending activity row of kind 10 whose counterparty is the token's id, and
 * stays pending until a shielded sync finds it. That row is the buyer's
 * only trace of a purchase in flight, so it has to come back unchanged from
 * a new handler over the same database (the process restart), and a later
 * status for the same entry has to replace it, not add a second row.
 */
@RunWith(RobolectricTestRunner::class)
class ShieldedTokenPurchaseActivityPersistenceTest {

    private lateinit var db: DashDatabase

    private val walletId = ByteArray(32) { 1 }
    private val entryId = ByteArray(32) { 7 }
    private val tokenId = ByteArray(32) { 9 }
    private val changeCmx = ByteArray(32) { 5 }
    private val spentNullifiers = ByteArray(64) { 6 }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun handler() = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)

    private fun persistPurchase(handler: PlatformWalletPersistenceHandler, status: Int): Int {
        handler.onChangesetBegin(walletId)
        handler.onPersistShieldedActivity(
            walletId = walletId,
            noteWalletId = walletId,
            accountIndex = 0,
            entryId = entryId,
            kindTag = KIND_TOKEN_PURCHASE,
            direction = 1,
            status = status.toByte(),
            amount = 250_000,
            fee = 114_140_000,
            hasFee = true,
            blockHeight = 0,
            hasBlockHeight = false,
            createdAtMs = 1_700_000_000_000,
            identityId = ByteArray(0),
            hasIdentityId = false,
            counterparty = tokenId,
            memo = ByteArray(0),
            noteCmxs = changeCmx,
            spentNullifiers = spentNullifiers,
        )
        return handler.onChangesetEnd(walletId, success = true)
    }

    @Test
    fun shouldRestoreAPendingTokenPurchaseAfterARestart() = runTest {
        assertEquals(0, persistPurchase(handler(), status = 0))

        // A new handler over the same database: what Rust loads on relaunch.
        val restored = handler().onLoadShieldedActivity().single()

        assertEquals(KIND_TOKEN_PURCHASE, restored.kindTag)
        assertEquals("still pending", 0.toByte(), restored.status)
        assertArrayEquals("the token's id rides as the counterparty", tokenId, restored.counterparty)
        assertArrayEquals(entryId, restored.entryId)
        assertArrayEquals("the change note that confirms it", changeCmx, restored.noteCmxs)
        assertArrayEquals("the notes it holds reserved", spentNullifiers, restored.spentNullifiers)
        assertEquals(250_000L, restored.amount)
        assertEquals(114_140_000L, restored.fee)
    }

    @Test
    fun shouldReplaceThePendingRowWhenTheScanConfirmsThePurchase() = runTest {
        val handler = handler()
        persistPurchase(handler, status = 0)
        assertEquals(0, persistPurchase(handler, status = 1))

        val rows = db.shieldedDao().getAllActivity()
        assertEquals("one row per purchase", 1, rows.size)
        assertEquals(KIND_TOKEN_PURCHASE.toInt(), rows.single().kindTag)
        assertEquals(1, rows.single().status)
        assertArrayEquals(tokenId, rows.single().counterparty)
    }

    private companion object {
        /** `ShieldedActivityKind::TokenPurchase`. */
        const val KIND_TOKEN_PURCHASE: Byte = 10
    }
}
