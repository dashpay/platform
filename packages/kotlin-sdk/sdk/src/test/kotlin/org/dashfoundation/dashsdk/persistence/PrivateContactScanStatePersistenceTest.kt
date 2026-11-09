package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The private contact request scan state (read positions and private
 * contacts' shielded addresses) round-trips through Room in the packed rows
 * the JNI trampolines exchange. Port of
 * `PrivateContactScanStatePersistenceTests.swift`.
 */
@RunWith(RobolectricTestRunner::class)
class PrivateContactScanStatePersistenceTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler

    private val walletId = ByteArray(32) { 42 }
    private val owner = ByteArray(32) { 7 }
    private val contact = ByteArray(32) { 5 }
    private val otherContact = ByteArray(32) { 6 }
    private val fingerprint = ByteArray(32) { 3 }

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

    private fun position(identityId: ByteArray, position: Long): ByteArray =
        java.nio.ByteBuffer.allocate(NativePersistenceBridge.PRIVATE_POSITION_ENTRY_SIZE)
            .order(java.nio.ByteOrder.LITTLE_ENDIAN)
            .put(identityId).putLong(position).put(fingerprint)
            .array()

    private fun address(ownerId: ByteArray, contactId: ByteArray, fill: Int): ByteArray =
        ownerId + contactId + ByteArray(43) { fill.toByte() }

    private fun round(
        positions: ByteArray = ByteArray(0),
        addresses: ByteArray = ByteArray(0),
        removed: ByteArray = ByteArray(0),
        cleared: ByteArray = ByteArray(0),
        success: Boolean = true,
    ) {
        assertEquals(0, handler.onChangesetBegin(walletId))
        assertEquals(
            0,
            handler.onPersistPrivateContactScanState(walletId, positions, addresses, removed, cleared),
        )
        assertEquals(0, handler.onChangesetEnd(walletId, success))
    }

    private fun loaded(): Pair<List<Pair<String, Long>>, List<Pair<String, Int>>> {
        val (positions, addresses) = handler.onLoadPrivateContactScanState(walletId)
        val positionRows = unpackPrivatePositions(walletId, positions)
            .map { it.identityId.toHex() to it.position }
        val addressRows = unpackPrivateAddresses(walletId, addresses)
            .map { it.contactId.toHex() to it.address[0].toInt() }
        return positionRows to addressRows
    }

    @Test
    fun shouldStoreReplaceAndRemoveTheScanStateWithItsRound() {
        round(
            positions = position(owner, 12),
            addresses = address(owner, contact, 9) + address(owner, otherContact, 8),
        )
        var (positions, addresses) = loaded()
        assertEquals(listOf(owner.toHex() to 12L), positions)
        assertEquals(setOf(contact.toHex() to 9, otherContact.toHex() to 8), addresses.toSet())

        // A rewind writes the lower position; a tombstone removes one address.
        round(positions = position(owner, 0), removed = owner + otherContact)
        loaded().let { (p, a) -> positions = p; addresses = a }
        assertEquals(listOf(owner.toHex() to 0L), positions)
        assertEquals(listOf(contact.toHex() to 9), addresses)

        // A rolled-back round leaves the stored state as it was.
        round(positions = position(owner, 40), success = false)
        assertEquals(listOf(owner.toHex() to 0L), loaded().first)

        // A full-width position survives the signed column.
        round(positions = position(owner, -1L))
        assertEquals(listOf(owner.toHex() to -1L), loaded().first)
        val (packed, _) = handler.onLoadPrivateContactScanState(walletId)
        assertArrayEquals(position(owner, -1L), packed)
    }

    @Test
    fun shouldClearTheScanStateOfARemovedIdentity() {
        val survivor = ByteArray(32) { 1 }
        round(
            positions = position(owner, 12) + position(survivor, 4),
            addresses = address(owner, contact, 9) + address(survivor, contact, 1),
        )
        round(cleared = owner)
        val (positions, addresses) = loaded()
        assertEquals(listOf(survivor.toHex() to 4L), positions)
        assertEquals(listOf(contact.toHex() to 1), addresses)
    }

    @Test
    fun shouldScopeTheScanStateByWalletAndPurgeItWithTheWallet() = runTest {
        round(positions = position(owner, 12), addresses = address(owner, contact, 9))
        val (otherPositions, otherAddresses) =
            handler.onLoadPrivateContactScanState(ByteArray(32) { 43 })
        assertEquals(0, otherPositions.size)
        assertEquals(0, otherAddresses.size)

        handler.deleteWalletData(walletId)
        val (positions, addresses) = handler.onLoadPrivateContactScanState(walletId)
        assertEquals(0, positions.size)
        assertEquals(0, addresses.size)
    }

    /**
     * A stored row that cannot cross the ABI fails the load instead of
     * handing back the rest without it: a valid position beside a 42-byte
     * address would otherwise resume past the entry that carried it.
     */
    @Test
    fun shouldFailTheLoadOnAStoredRowThatCannotCrossTheAbi() = runTest {
        round(positions = position(owner, 12))
        db.privateContactScanStateDao().upsertAddress(
            org.dashfoundation.dashsdk.persistence.entities.PrivateShieldedAddressEntity(
                walletId = walletId,
                ownerId = owner,
                contactId = contact,
                address = ByteArray(42) { 9 },
            ),
        )
        assertThrows(IllegalArgumentException::class.java) {
            handler.onLoadPrivateContactScanState(walletId)
        }
    }

    /** A partial row fails the callback instead of storing a misread row. */
    @Test
    fun shouldRefuseARoundWithAPartialRow() {
        assertEquals(0, handler.onChangesetBegin(walletId))
        assertNotEquals(
            0,
            handler.onPersistPrivateContactScanState(
                walletId, ByteArray(NativePersistenceBridge.PRIVATE_POSITION_ENTRY_SIZE - 1),
                ByteArray(0), ByteArray(0), ByteArray(0),
            ),
        )
        assertEquals(0, handler.onChangesetEnd(walletId, false))
        assertTrue(loaded().first.isEmpty())
        assertThrows(IllegalArgumentException::class.java) {
            unpackPrivateAddresses(walletId, ByteArray(NativePersistenceBridge.PRIVATE_ADDRESS_ENTRY_SIZE + 1))
        }
    }

    /** The inherited bridge stores nothing and loads two empty arrays. */
    @Test
    fun aBridgeWithoutTheScanStateStoresNothingAndLoadsEmpty() {
        val noOp = object : NativePersistenceBridge() {}
        assertEquals(0L, noOp.persistenceCapabilitiesBits())
        assertEquals(
            0,
            noOp.onPersistPrivateContactScanState(
                walletId, position(owner, 1), ByteArray(0), ByteArray(0), ByteArray(0),
            ),
        )
        val (positions, addresses) = noOp.onLoadPrivateContactScanState(walletId)
        assertEquals(0, positions.size + addresses.size)
    }
}
