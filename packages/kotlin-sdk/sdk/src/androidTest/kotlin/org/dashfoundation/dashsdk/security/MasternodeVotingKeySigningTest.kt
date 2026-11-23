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
 * bytes and type Rust asks a vote's signer for, and nothing else: an ordinary wallet key in
 * the same storage is refused. The manager's own signer never signs for the address, during
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

    /** DPP `KeyType::ECDSA_SECP256K1`, an ordinary wallet identity key. */
    private val ecdsaSecp256k1 = 0

    /** The platform-address dispatch tag of the signer vtable. */
    private val platformAddressKeyType = 0xFF

    /** An ordinary wallet identity key, stored under its public key hex as wallets do. */
    private val walletPublicKey = ByteArray(33) { index -> if (index == 0) 0x02 else (index + 7).toByte() }
    private val walletPublicKeyHex = walletPublicKey.joinToString("") { "%02x".format(it) }

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
    fun tearDown() = runBlocking<Unit> {
        keys.deleteAll()
        storage.deletePrivateKey(walletPublicKeyHex)
        if (::sdk.isInitialized) runCatching { sdk.close() }
        database.close()
    }

    @Test
    fun shouldSignWithTheStoredKeyOnlyThroughTheVotesOwnSigner() = runBlocking<Unit> {
        val identifier = keys.store(identityId, keyOne)
        val votingAddress = keys.votingAddressOf(identifier)
        assertEquals(keyOneVotingAddressHex, votingAddress.joinToString("") { "%02x".format(it) })
        assertArrayEquals(identityId, keys.identityIdOf(identifier))
        // Nothing is stored under the bare address hex, the entry name a wallet key would use.
        assertFalse(storage.hasPrivateKey(keyOneVotingAddressHex))
        // A wallet key the vote's signer must not reach although it is in the same storage.
        storage.storePrivateKey(walletPublicKeyHex, ByteArray(32) { 7 })

        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val expected = SignerNative.signWithPrivateKey(keyOne, Network.TESTNET.ffiValue, payload)
        assertNotNull(expected)
        var escaped: MasternodeVotingSigner? = null
        PlatformWalletManager(sdk, Network.TESTNET, database, storage).use { manager ->
            val managerSigner = manager.signerForTesting
            assertFalse(managerSigner.canSignWith(votingAddress, ecdsaHash160))
            assertTrue(managerSigner.canSignWith(walletPublicKey, ecdsaSecp256k1))

            manager.withStoredMasternodeVotingKey(identifier) { voter ->
                escaped = voter
                assertArrayEquals(votingAddress, voter.votingAddress)
                // The manager's signer, which concurrent wallet operations use, cannot.
                assertFalse(managerSigner.canSignWith(votingAddress, ecdsaHash160))
                assertTrue(voter.signer.canSignWith(votingAddress, ecdsaHash160))

                val completion =
                    voter.signer.signWithStoredKeyForTesting(votingAddress, ecdsaHash160, payload)
                assertEquals(completion.message, null, completion.message)
                assertArrayEquals(expected, completion.signature)

                // The manager's signer finds no key for the address.
                val refused =
                    managerSigner.signWithStoredKeyForTesting(votingAddress, ecdsaHash160, payload)
                assertEquals(null, refused.signature)
                assertEquals(SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE, refused.code)

                // The vote's signer serves only its own key: not the wallet key in the same
                // storage, not the address under another key type, no platform address.
                assertFalse(voter.signer.canSignWith(walletPublicKey, ecdsaSecp256k1))
                assertFalse(voter.signer.canSignWith(votingAddress, ecdsaSecp256k1))
                assertFalse(voter.signer.canSignWith(votingAddress, platformAddressKeyType))
                val unrelated =
                    voter.signer.signWithStoredKeyForTesting(walletPublicKey, ecdsaSecp256k1, payload)
                assertEquals(null, unrelated.signature)
                assertEquals(SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE, unrelated.code)
            }
            assertFalse(managerSigner.canSignWith(votingAddress, ecdsaHash160))
        }

        // The vote's signer was freed with its call.
        assertThrows(IllegalStateException::class.java) {
            runBlocking { escaped!!.withSignerHandle { } }
        }
    }

    @Test
    fun shouldRefuseAnIdentityWithNoStoredKey() = runBlocking<Unit> {
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val identifier = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { 9 })
        PlatformWalletManager(sdk, Network.TESTNET, database, storage).use { manager ->
            assertThrows(MasternodeVotingKeyNotStoredException::class.java) {
                runBlocking { manager.withStoredMasternodeVotingKey(identifier) { } }
            }
        }
    }

    @Test
    fun shouldRewriteTheEntryWhenTheSameKeyIsStoredAgain() = runBlocking<Unit> {
        val first = keys.store(identityId, keyOne)
        val second = keys.store(identityId, keyOne)
        assertEquals(first, second)
        assertTrue(keys.isStored(second))
        keys.delete(second)
        assertFalse(keys.isStored(second))
    }
}
