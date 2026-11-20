package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.persistence.entities.IdentitySlotLeaseEntity
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The invitation-claim slot leases round-trip through Room in the packed
 * rows the JNI trampolines exchange. Port of
 * `IdentitySlotLeasePersistenceTests.swift`.
 */
@RunWith(RobolectricTestRunner::class)
class IdentitySlotLeasePersistenceTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

    private val walletId = ByteArray(32) { 42 }
    private val invitation = outPoint(1, 3)
    private val otherInvitation = outPoint(2, 0)

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
        assertEquals(0, handler.onPersistWalletMetadata(walletId, 1, ByteArray(32) { 2 }, 0))
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun outPoint(fill: Int, vout: Int): ByteArray =
        ByteArray(32) { fill.toByte() } +
            java.nio.ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(vout).array()

    private fun lease(outPoint: ByteArray, identityIndex: Int, phase: Int): ByteArray =
        java.nio.ByteBuffer.allocate(NativePersistenceBridge.IDENTITY_SLOT_LEASE_ENTRY_SIZE)
            .order(java.nio.ByteOrder.LITTLE_ENDIAN)
            .put(outPoint).putInt(identityIndex).put(phase.toByte())
            .array()

    private fun round(
        leases: ByteArray = ByteArray(0),
        released: ByteArray = ByteArray(0),
        wallet: ByteArray = walletId,
        success: Boolean = true,
    ) {
        assertEquals(0, handler.onChangesetBegin(wallet))
        assertEquals(0, handler.onPersistIdentitySlotLeases(wallet, leases, released))
        assertEquals(0, handler.onChangesetEnd(wallet, success))
    }

    /** `(outpoint hex, identity index, phase)` of each stored lease. */
    private fun loaded(wallet: ByteArray = walletId): Set<Triple<String, Int, Int>> =
        unpackIdentitySlotLeases(wallet, handler.onLoadIdentitySlotLeases(wallet))
            .map { Triple(it.invitationOutPoint.toHex(), it.identityIndex, it.phase) }
            .toSet()

    @Test
    fun shouldStoreUpdateAndReleaseTheLeasesWithTheirRound() {
        round(leases = lease(invitation, 4, 0) + lease(otherInvitation, 5, 0))
        assertEquals(
            setOf(Triple(invitation.toHex(), 4, 0), Triple(otherInvitation.toHex(), 5, 0)),
            loaded(),
        )

        // A submitted claim replaces its lease; a landed one releases it.
        round(leases = lease(invitation, 4, 1), released = otherInvitation)
        assertEquals(setOf(Triple(invitation.toHex(), 4, 1)), loaded())

        // A rolled-back round leaves the stored leases as they were.
        round(leases = lease(otherInvitation, 6, 0), released = invitation, success = false)
        assertEquals(setOf(Triple(invitation.toHex(), 4, 1)), loaded())

        // A full-width identity index and vout survive, and the row is handed
        // back byte for byte.
        val wide = outPoint(9, -1)
        round(leases = lease(wide, -1, 1), released = invitation)
        assertArrayEquals(lease(wide, -1, 1), handler.onLoadIdentitySlotLeases(walletId))
    }

    /** The phase is native's to judge: a code this build does not name is stored as it came. */
    @Test
    fun shouldHandBackThePhaseAsItCame() {
        round(leases = lease(invitation, 4, 0xFE))
        assertArrayEquals(lease(invitation, 4, 0xFE), handler.onLoadIdentitySlotLeases(walletId))
    }

    @Test
    fun shouldScopeTheLeasesByWalletAndPurgeThemWithTheWallet() = runTest {
        val otherWallet = ByteArray(32) { 43 }
        assertEquals(0, handler.onPersistWalletMetadata(otherWallet, 1, ByteArray(32) { 3 }, 0))
        round(leases = lease(invitation, 4, 0))
        round(leases = lease(invitation, 7, 1), wallet = otherWallet)
        assertEquals(setOf(Triple(invitation.toHex(), 4, 0)), loaded())
        assertEquals(setOf(Triple(invitation.toHex(), 7, 1)), loaded(otherWallet))

        handler.deleteWalletData(walletId)
        assertEquals(0, handler.onLoadIdentitySlotLeases(walletId).size)
        assertEquals(setOf(Triple(invitation.toHex(), 7, 1)), loaded(otherWallet))
    }

    /**
     * A lease the handler cannot store fails the callback instead of being
     * acknowledged: outside a round, or for a wallet with no row.
     */
    @Test
    fun shouldFailALeaseItCannotStore() {
        val invitation = outPoint(7, 0)
        assertNotEquals(0, handler.onPersistIdentitySlotLeases(walletId, lease(invitation, 1, 0), ByteArray(0)))

        val unknownWallet = ByteArray(32) { 43 }
        assertEquals(0, handler.onChangesetBegin(unknownWallet))
        assertEquals(0, handler.onPersistIdentitySlotLeases(unknownWallet, lease(invitation, 1, 0), ByteArray(0)))
        assertNotEquals(0, handler.onChangesetEnd(unknownWallet, true))
        assertEquals(0, handler.onLoadIdentitySlotLeases(unknownWallet).size)
    }

    /** A partial row fails the callback instead of storing a misread row. */
    @Test
    fun shouldRefuseARoundWithAPartialRow() {
        assertEquals(0, handler.onChangesetBegin(walletId))
        assertNotEquals(
            0,
            handler.onPersistIdentitySlotLeases(
                walletId, ByteArray(NativePersistenceBridge.IDENTITY_SLOT_LEASE_ENTRY_SIZE - 1), ByteArray(0),
            ),
        )
        assertNotEquals(
            0,
            handler.onPersistIdentitySlotLeases(
                walletId, ByteArray(0), ByteArray(NativePersistenceBridge.IDENTITY_SLOT_LEASE_RELEASE_ENTRY_SIZE + 1),
            ),
        )
        assertEquals(0, handler.onChangesetEnd(walletId, false))
        assertEquals(0, handler.onLoadIdentitySlotLeases(walletId).size)
    }

    /**
     * A stored row that cannot cross the ABI fails the load instead of
     * handing back the rest without it: a dropped lease would free a slot a
     * claim already took.
     */
    @Test
    fun shouldFailTheLoadOnAStoredRowThatCannotCrossTheAbi() = runTest {
        round(leases = lease(invitation, 4, 0))
        db.identitySlotLeaseDao().upsert(
            IdentitySlotLeaseEntity(
                walletId = walletId,
                invitationOutPoint = ByteArray(35) { 9 },
                identityIndex = 5,
                phase = 0,
            ),
        )
        assertThrows(IllegalArgumentException::class.java) {
            handler.onLoadIdentitySlotLeases(walletId)
        }
    }

    /** The inherited bridge stores nothing and loads empty. */
    @Test
    fun aBridgeWithoutTheLeasesStoresNothingAndLoadsEmpty() {
        val noOp = object : NativePersistenceBridge() {}
        assertEquals(0L, noOp.persistenceCapabilitiesBits())
        assertEquals(0, noOp.onPersistIdentitySlotLeases(walletId, lease(invitation, 1, 0), ByteArray(0)))
        assertEquals(0, noOp.onLoadIdentitySlotLeases(walletId).size)
    }
}
