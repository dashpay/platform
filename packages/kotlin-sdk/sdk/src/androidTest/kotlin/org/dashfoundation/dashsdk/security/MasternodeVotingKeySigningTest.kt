package org.dashfoundation.dashsdk.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.voting.MasternodeVotingSigner
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * An imported masternode voting key signs a vote, offline: the secret-1 fixture is stored
 * through [MasternodeVotingKeyStore] (Rust forms the voting address, the AndroidKeyStore
 * wraps the ciphertext), and [PlatformWalletManager.withStoredMasternodeVotingKey] hands out
 * a signer of its own that signs for the voting address with key type ECDSA_HASH160 (2), the
 * bytes and type Rust asks a vote's signer for. The manager's own signer never can, during
 * the vote or after it, and the vote's signer cannot be used once its call ended.
 *
 * The signature goes through the dedicated signer's real sign path (lookup, Keystore
 * decrypt, Rust signature) via its test seam, and must equal the one Rust makes from the
 * fixture key directly: `dashcore::signer::sign` is RFC 6979 deterministic.
 */
@RunWith(AndroidJUnit4::class)
class MasternodeVotingKeySigningTest {

    /** The secret 1 and its voting address, as in `MasternodeVotingKeyFfiTest`. */
    private val keyOne = ByteArray(32).also { it[31] = 1 }
    private val keyOneVotingAddressHex = "751e76e8199196d454941c45d1b3a323f1433bd6"
    private val identityId = ByteArray(32) { 0x44 }
    private val payload = "masternode vote payload".toByteArray()

    /** DPP `KeyType::ECDSA_HASH160`. */
    private val ecdsaHash160 = 2

    private lateinit var database: DashDatabase
    private lateinit var storage: WalletStorage
    private lateinit var keys: MasternodeVotingKeyStore
    private lateinit var sdk: Sdk

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
        if (::sdk.isInitialized) runCatching { sdk.close() }
        database.close()
    }

    @Test
    fun shouldSignWithTheStoredKeyOnlyThroughTheVotesOwnSigner() = runBlocking {
        val identifier = keys.store(identityId, keyOne)
        val votingAddress = keys.votingAddressOf(identifier)
        assertEquals(keyOneVotingAddressHex, votingAddress.joinToString("") { "%02x".format(it) })
        assertArrayEquals(identityId, keys.identityIdOf(identifier))
        // Nothing is stored under the bare address hex, the entry name a wallet key would use.
        assertFalse(storage.hasPrivateKey(keyOneVotingAddressHex))

        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val expected = SignerNative.signWithPrivateKey(keyOne, Network.TESTNET.ffiValue, payload)
        assertNotNull(expected)
        var escaped: MasternodeVotingSigner? = null
        PlatformWalletManager(sdk, Network.TESTNET, database, storage).use { manager ->
            val managerSigner = manager.signerForTesting
            assertFalse(managerSigner.canSignWith(votingAddress, ecdsaHash160))

            manager.withStoredMasternodeVotingKey(identifier) { voter ->
                escaped = voter
                assertArrayEquals(votingAddress, voter.votingAddress)
                // The manager's signer, which concurrent wallet operations use, cannot.
                assertFalse(managerSigner.canSignWith(votingAddress, ecdsaHash160))
                assertTrue(voter.signer.canSignWith(votingAddress, ecdsaHash160))

                val completion = voter.signer.signWithStoredKeyForTesting(votingAddress, payload)
                assertEquals(completion.message, null, completion.message)
                assertArrayEquals(expected, completion.signature)

                // The manager's signer finds no key for the address.
                val refused = managerSigner.signWithStoredKeyForTesting(votingAddress, payload)
                assertEquals(null, refused.signature)
                assertEquals(SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE, refused.code)
            }
            assertFalse(managerSigner.canSignWith(votingAddress, ecdsaHash160))
        }

        // The vote's signer was freed with its call.
        assertThrows(IllegalStateException::class.java) {
            runBlocking { escaped!!.withSignerHandle { } }
        }
    }

    @Test
    fun shouldRefuseAnIdentityWithNoStoredKey() = runBlocking {
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val identifier = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { 9 })
        PlatformWalletManager(sdk, Network.TESTNET, database, storage).use { manager ->
            assertThrows(MasternodeVotingKeyNotStoredException::class.java) {
                runBlocking { manager.withStoredMasternodeVotingKey(identifier) { } }
            }
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
