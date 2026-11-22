package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The parts of [MasternodeVotingKeyStore] that need no native library: reading an
 * identifier back as a voting address, and the lookups of a key that was never stored.
 * Forming the address and storing a key go through `libdash_sdk_jni` and are covered by
 * `MasternodeVotingKeyFfiTest` (androidTest).
 */
@RunWith(RobolectricTestRunner::class)
class MasternodeVotingKeyStoreTest {

    private val store = MasternodeVotingKeyStore(
        WalletStorage(ApplicationProvider.getApplicationContext()),
    )

    /** hash160 of the compressed public key of the secret 1. */
    private val addressHex = "751e76e8199196d454941c45d1b3a323f1433bd6"

    @Test
    fun shouldReadAnIdentifierBackAsItsTwentyByteVotingAddress() {
        val expected = ByteArray(20) { addressHex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
        assertArrayEquals(expected, store.votingAddressOf(addressHex))
        assertArrayEquals(expected, store.votingAddressOf(addressHex.uppercase()))
    }

    @Test
    fun shouldRefuseAnIdentifierThatIsNotFortyHexCharacters() {
        assertThrows(IllegalArgumentException::class.java) { store.votingAddressOf("") }
        assertThrows(IllegalArgumentException::class.java) {
            store.votingAddressOf(addressHex.dropLast(2))
        }
        assertThrows(IllegalArgumentException::class.java) { store.votingAddressOf(addressHex + "00") }
        assertThrows(IllegalArgumentException::class.java) {
            store.votingAddressOf("zz" + addressHex.drop(2))
        }
    }

    @Test
    fun shouldReportAKeyNeverStoredAsAbsentAndDeleteItQuietly() = runBlocking {
        val neverStored = "00".repeat(20)
        assertFalse(store.isStored(neverStored))
        store.delete(neverStored)
        assertFalse(store.isStored(neverStored))
    }

    @Test
    fun shouldRefuseAVotingKeyThatIsNot32BytesBeforeTheNativeCall() {
        assertThrows(IllegalArgumentException::class.java) { store.votingAddress(ByteArray(31)) }
    }
}
