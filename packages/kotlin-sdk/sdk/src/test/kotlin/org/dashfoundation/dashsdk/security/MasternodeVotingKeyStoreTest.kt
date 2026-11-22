package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The parts of [MasternodeVotingKeyStore] that need no native library: the identifier format
 * and reading it back, the lookups of a key that was never stored, and deleting every voting
 * entry. Forming the address and storing a key go through `libdash_sdk_jni` and are covered
 * by `MasternodeVotingKeyFfiTest` and `MasternodeVotingKeySigningTest` (androidTest).
 */
@RunWith(RobolectricTestRunner::class)
class MasternodeVotingKeyStoreTest {

    private lateinit var storage: WalletStorage
    private lateinit var store: MasternodeVotingKeyStore

    /** hash160 of the compressed public key of the secret 1. */
    private val addressHex = "751e76e8199196d454941c45d1b3a323f1433bd6"
    private val address = hex(addressHex)
    private val identityId = ByteArray(32) { (it + 1).toByte() }
    private val identityIdHex = identityId.joinToString("") { "%02x".format(it) }
    private val identifier = "mnvoting.$identityIdHex.$addressHex"

    @Before
    fun setUp() = runBlocking {
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), PlaintextIdentityKeysKeystore())
        storage.deleteAll()
        store = MasternodeVotingKeyStore(storage)
    }

    @Test
    fun shouldNameAnEntryByItsIdentityAndVotingAddress() {
        assertEquals(identifier, MasternodeVotingKeyStore.identifierFor(identityId, address))
    }

    @Test
    fun shouldReadAnIdentifierBackAsItsIdentityAndVotingAddress() {
        assertArrayEquals(address, store.votingAddressOf(identifier))
        assertArrayEquals(identityId, store.identityIdOf(identifier))
        assertArrayEquals(address, store.votingAddressOf(identifier.uppercase()))
    }

    @Test
    fun shouldRefuseAnythingThatIsNotAVotingKeyIdentifier() {
        val refused = listOf(
            "",
            // The address alone, the format of an ordinary identity key entry.
            addressHex,
            "mnvoting.$identityIdHex",
            "mnvoting.$identityIdHex.${addressHex.dropLast(2)}",
            "mnvoting.${identityIdHex.drop(2)}.$addressHex",
            "mnvoting.$identityIdHex.$addressHex.00",
            "other.$identityIdHex.$addressHex",
            "mnvoting.$identityIdHex.zz${addressHex.drop(2)}",
        )
        for (text in refused) {
            assertThrows(text, IllegalArgumentException::class.java) { store.votingAddressOf(text) }
            assertThrows(text, IllegalArgumentException::class.java) { store.identityIdOf(text) }
        }
    }

    @Test
    fun shouldRefuseIdsAndAddressesOfTheWrongWidth() {
        assertThrows(IllegalArgumentException::class.java) {
            MasternodeVotingKeyStore.identifierFor(ByteArray(31), address)
        }
        assertThrows(IllegalArgumentException::class.java) {
            MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(21))
        }
    }

    @Test
    fun shouldReportAKeyNeverStoredAsAbsentAndDeleteItQuietly() = runBlocking {
        assertFalse(store.isStored(identifier))
        store.delete(identifier)
        assertFalse(store.isStored(identifier))
    }

    @Test
    fun shouldRefuseAVotingKeyThatIsNot32BytesBeforeTheNativeCall() {
        assertThrows(IllegalArgumentException::class.java) { store.votingAddress(ByteArray(31)) }
    }

    /** deleteAll removes every voting entry and nothing else, such as a wallet's key. */
    @Test
    fun shouldDeleteEveryVotingEntryAndKeepIdentityKeys() = runBlocking {
        val other = MasternodeVotingKeyStore.identifierFor(ByteArray(32) { 9 }, address)
        storage.storePrivateKey(identifier, ByteArray(32) { 1 })
        storage.storePrivateKey(other, ByteArray(32) { 1 })
        // A wallet identity's ECDSA_HASH160 key is stored under the same address hex.
        storage.storePrivateKey(addressHex, ByteArray(32) { 2 }, ownerWalletId = ByteArray(32) { 3 })

        store.deleteAll()

        assertFalse(storage.hasPrivateKey(identifier))
        assertFalse(storage.hasPrivateKey(other))
        assertTrue(storage.hasPrivateKey(addressHex))
    }

    private fun hex(value: String): ByteArray =
        ByteArray(value.length / 2) { value.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
}
