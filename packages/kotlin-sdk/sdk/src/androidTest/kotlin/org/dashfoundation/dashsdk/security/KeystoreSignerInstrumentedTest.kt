package org.dashfoundation.dashsdk.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.security.KeyStore

/**
 * Native/device coverage for [KeystoreSigner]'s synchronous capability
 * boundary. Constructing the signer creates a Rust handle immediately, and
 * identity-key health depends on AndroidKeyStore, so a local JVM fixture
 * would hide both dependencies this check must coexist with in production.
 */
@RunWith(AndroidJUnit4::class)
class KeystoreSignerInstrumentedTest {

    private lateinit var database: DashDatabase
    private lateinit var storage: WalletStorage

    @Before
    fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        database = DashDatabase.createInMemory(context)
        storage = WalletStorage(context)
        NativeLoader.ensureLoaded()
    }

    @After
    fun tearDown() {
        database.close()
    }

    @Test
    fun canSignWithRejectsAKeyEncryptedUnderAReplacedKeysAlias() = runBlocking {
        val publicKey = ByteArray(33) { index -> if (index == 0) 0x02 else index.toByte() }
        val publicKeyHex = publicKey.joinToString("") { "%02x".format(it) }
        storage.storePrivateKey(publicKeyHex, ByteArray(32) { 7 })

        KeystoreSigner(
            storage = storage,
            network = Network.TESTNET,
            biometricGate = null,
            platformAddressDao = database.platformAddressDao(),
        ).use { signer ->
            assertTrue(signer.canSignWith(publicKey, keyType = 0))

            // The DataStore entry still exists after a Keystore replacement,
            // but the replacement private key can never decrypt it. Capability
            // must describe signability, not mere ciphertext presence. The
            // default-policy storage writes under KEYS_ALIAS_AUTH_GATED.
            val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            keyStore.deleteEntry(KeystoreManager.KEYS_ALIAS_AUTH_GATED)

            assertFalse(signer.canSignWith(publicKey, keyType = 0))
        }
    }
}
