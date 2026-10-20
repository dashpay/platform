package org.dashfoundation.dashsdk.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.security.KeyStore
import java.util.concurrent.Callable
import java.util.concurrent.CyclicBarrier
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/**
 * Real-Keystore counterpart of [KeystoreAliasProvisioningTest]: races
 * first-use provisioning of the lazily created
 * [KeystoreManager.MASTER_ALIAS_UNBOUND] AES key across concurrent
 * encrypts from SEPARATE [KeystoreManager] instances (the per-`WalletStorage`
 * shape) and checks that every ciphertext produced during the race still
 * opens under the surviving key. Unserialized provisioning let a losing
 * caller's `generateKey` replace the winner's key after the winner had
 * already encrypted with it, orphaning a persisted mnemonic
 * (dashpay/platform#4643 review). The alias carries neither an auth gate
 * nor lock binding, so this needs no prompt and no lock screen.
 */
@RunWith(AndroidJUnit4::class)
class KeystoreUnboundMasterKeyProvisioningTest {

    private val alias = KeystoreManager.MASTER_ALIAS_UNBOUND

    @Before
    fun clearAlias() = deleteAlias()

    @After
    fun tearDown() = deleteAlias()

    private fun deleteAlias() {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        if (keyStore.containsAlias(alias)) keyStore.deleteEntry(alias)
    }

    @Test
    fun concurrentFirstUseEncryptsAllOpenUnderTheSurvivingKey() {
        val callers = 8
        val plaintexts = List(callers) { i -> ByteArray(32) { (i * 7 + it).toByte() } }
        val start = CyclicBarrier(callers)
        val pool = Executors.newFixedThreadPool(callers)
        val blobs = try {
            pool.invokeAll(
                plaintexts.map { plain ->
                    Callable {
                        // One manager per caller, as WalletStorage instances do.
                        val manager = KeystoreManager()
                        start.await(10, TimeUnit.SECONDS)
                        manager.encrypt(plain, alias)
                    }
                },
            ).map { it.get(30, TimeUnit.SECONDS) }
        } finally {
            pool.shutdownNow()
        }

        val reader = KeystoreManager()
        assertTrue("the raced alias must exist afterwards", reader.hasUnboundMasterKey())
        blobs.forEachIndexed { i, blob ->
            assertArrayEquals(
                "ciphertext $i must open under the surviving key",
                plaintexts[i],
                reader.decrypt(blob, alias),
            )
        }
    }
}
