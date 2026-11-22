package org.dashfoundation.dashsdk.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * An imported masternode voting key is signable by the real [KeystoreSigner] for its voting
 * address with key type ECDSA_HASH160 (2), the bytes and type Rust asks a vote's signer for,
 * only while a scope maps the address to the identity's entry
 * (`PlatformWalletManager.withStoredMasternodeVotingKey`), and never outside it. Offline: the
 * fixture key is stored through [MasternodeVotingKeyStore] (Rust forms the address, the
 * AndroidKeyStore wraps the ciphertext), and the signer's capability check is the same one
 * Rust's `can_sign_with` reaches through the native signer handle.
 */
@RunWith(AndroidJUnit4::class)
class MasternodeVotingKeySigningTest {

    /** The secret 1 and its voting address, as in `MasternodeVotingKeyFfiTest`. */
    private val keyOne = ByteArray(32).also { it[31] = 1 }
    private val keyOneVotingAddressHex = "751e76e8199196d454941c45d1b3a323f1433bd6"
    private val identityId = ByteArray(32) { 0x44 }

    /** DPP `KeyType::ECDSA_HASH160`. */
    private val ecdsaHash160 = 2

    private lateinit var database: DashDatabase
    private lateinit var storage: WalletStorage
    private lateinit var keys: MasternodeVotingKeyStore

    @Before
    fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        database = DashDatabase.createInMemory(context)
        storage = WalletStorage(context)
        // Deliberately no NativeLoader call: the store loads the library itself.
        keys = MasternodeVotingKeyStore(storage)
    }

    @After
    fun tearDown() = runBlocking {
        keys.deleteAll()
        database.close()
    }

    @Test
    fun shouldSignForTheVotingAddressOnlyInsideTheScope() = runBlocking {
        val identifier = keys.store(identityId, keyOne)
        val votingAddress = keys.votingAddressOf(identifier)
        assertEquals(keyOneVotingAddressHex, votingAddress.joinToString("") { "%02x".format(it) })
        assertArrayEquals(identityId, keys.identityIdOf(identifier))
        assertTrue(keys.isStored(identifier))
        // Nothing is stored under the bare address hex, the entry name a wallet key would use.
        assertFalse(storage.hasPrivateKey(keyOneVotingAddressHex))

        KeystoreSigner(
            storage = storage,
            network = Network.TESTNET,
            biometricGate = null,
            platformAddressDao = database.platformAddressDao(),
        ).use { signer ->
            assertFalse(signer.canSignWith(votingAddress, ecdsaHash160))
            signer.openScopedAlias(keyOneVotingAddressHex, identifier).use {
                assertTrue(signer.canSignWith(votingAddress, ecdsaHash160))
            }
            assertFalse(signer.canSignWith(votingAddress, ecdsaHash160))
        }
    }

    @Test
    fun shouldRewriteTheEntryWhenTheSameKeyIsStoredAgain() = runBlocking {
        val first = keys.store(identityId, keyOne)
        val second = keys.store(identityId, keyOne)
        assertEquals(first, second)
        assertTrue(keys.isStored(second))
        keys.delete(second)
        assertFalse(keys.isStored(second))
    }
}
