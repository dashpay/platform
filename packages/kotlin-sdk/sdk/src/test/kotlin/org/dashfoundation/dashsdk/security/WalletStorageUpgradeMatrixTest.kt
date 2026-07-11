package org.dashfoundation.dashsdk.security

import android.security.keystore.UserNotAuthenticatedException
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.security.GeneralSecurityException
import javax.crypto.BadPaddingException

/**
 * Full upgrade-matrix coverage for [WalletStorage]'s identity-key recovery
 * (dashpay/platform#4060). Exercises every stored-blob shape an upgraded
 * install can hold — legacy AES-GCM, pre-alias-split RSA under
 * [KeystoreManager.KEYS_ALIAS], and current policy-alias RSA — plus the
 * stranded and auth-propagation cases.
 *
 * The real AndroidKeyStore crypto cannot run on the JVM (no Robolectric
 * provider — see [KeySecurityPolicyTest]), so a [FakeKeystoreManager]
 * substitutes deterministic in-memory "crypto" through the `open` seams on
 * [KeystoreManager]. It models the load-bearing invariants the production code
 * relies on: an empty-IV blob only decrypts under the alias that produced it,
 * [KeystoreManager.KEYS_ALIAS] holds at most one former key (AES XOR RSA), and
 * a public-key encrypt provisions the policy alias. This pins the ROUTING and
 * migration behavior; the concrete keystore crypto is out of unit-test reach by
 * construction.
 */
@RunWith(RobolectricTestRunner::class)
class WalletStorageUpgradeMatrixTest {

    private val pub = "02aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"
    private val secret = ByteArray(32) { (it + 1).toByte() }

    private lateinit var fake: FakeKeystoreManager
    private lateinit var storage: WalletStorage

    @Before
    fun setUp() = runBlocking {
        fake = FakeKeystoreManager()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), fake)
        // Isolate from any state a prior test left in the shared DataStore file.
        storage.deleteAll()
    }

    // ── Blob-shape / routing matrix ──────────────────────────────────────

    /** New-alias (current-scheme) blob: decrypts under the policy alias, no migration. */
    @Test
    fun currentPolicyAliasBlobDecryptsDirectly() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)

        assertTrue(storage.isPrivateKeyDecryptable(pub))
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        // No fallback path was taken.
        assertEquals(0, fake.legacyRsaFallbackCalls)
    }

    /** Legacy AES-GCM blob: recovered via the retained AES key and migrated forward. */
    @Test
    fun legacyAesBlobIsRecoveredAndMigrated() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.AES
        fake.scheme = FakeKeystoreManager.Scheme.LEGACY_AES
        storage.storePrivateKey(pub, secret) // writes a non-empty-IV AES blob

        // Health check sees the retained AES key.
        assertTrue(storage.isPrivateKeyDecryptable(pub))

        // Migration re-encrypts under the current alias.
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))

        // The stored blob is now a current-alias blob: a second read no longer
        // touches the legacy AES path.
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.NONE
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
    }

    /** Legacy AES key already deleted by an older build → stranded, reported undecryptable. */
    @Test
    fun legacyAesBlobWithDeletedKeyIsStranded() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.AES
        fake.scheme = FakeKeystoreManager.Scheme.LEGACY_AES
        storage.storePrivateKey(pub, secret)

        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.NONE // key gone
        assertFalse(storage.isPrivateKeyDecryptable(pub))
        // decryptLegacyKeysBlob returns null → retrieve yields null, not a wrong value.
        assertNull(storage.retrievePrivateKey(pub))
    }

    /**
     * Pre-alias-split RSA blob under KEYS_ALIAS with an empty policy alias:
     * the upgrade fast path recovers it with the former RSA key and migrates.
     */
    @Test
    fun formerRsaBlobEmptyPolicyTakesFastPathAndMigrates() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        storage.storePrivateKey(pub, secret) // former-RSA blob; policy alias not provisioned

        assertTrue(storage.isPrivateKeyDecryptable(pub)) // recoverable via former RSA key

        fake.scheme = FakeKeystoreManager.Scheme.CURRENT // migration writes a policy blob
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertTrue(fake.legacyRsaFallbackCalls > 0)

        // Migrated: the next read decrypts straight under the (now provisioned)
        // policy alias without another former-RSA fallback.
        val before = fake.legacyRsaFallbackCalls
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertEquals(0, fake.legacyRsaFallbackCalls - before)
    }

    /**
     * Mixed window: the policy alias already holds a key (so the fast path is
     * skipped) while a former-RSA blob lingers. The policy decrypt fails with a
     * wrong-key crypto error and the code falls back to the former RSA key.
     */
    @Test
    fun formerRsaBlobProvisionedPolicyTakesCatchFallback() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        storage.storePrivateKey(pub, secret)

        // Simulate a sibling key already provisioned at the policy alias.
        fake.policyKeyProvisioned = true
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT

        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertTrue(fake.legacyRsaFallbackCalls > 0) // reached the catch-branch fallback
    }

    /** Former-RSA blob whose KEYS_ALIAS key is gone → stranded (no wrong-value recovery). */
    @Test
    fun formerRsaBlobWithDeletedKeyIsStranded() {
        runBlocking {
            fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
            fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
            storage.storePrivateKey(pub, secret)

            fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.NONE // former RSA key gone
            fake.scheme = FakeKeystoreManager.Scheme.CURRENT
            assertFalse(storage.isPrivateKeyDecryptable(pub))
        }
        // Policy decrypt fails (wrong key) and there is no former key to fall
        // back to → the crypto failure surfaces rather than a bogus plaintext.
        assertThrows(GeneralSecurityException::class.java) {
            runBlocking { storage.retrievePrivateKey(pub) }
        }
    }

    /**
     * A closed auth window must NOT be mistaken for a wrong key: the
     * UserNotAuthenticatedException propagates so KeystoreSigner can prompt,
     * instead of being swallowed into the former-RSA fallback.
     */
    @Test
    fun authFailureOnPolicyDecryptPropagates() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret) // provisions the policy alias

        fake.throwAuthOnPolicyDecrypt = true
        assertThrows(UserNotAuthenticatedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pub) }
        }
        assertEquals(0, fake.legacyRsaFallbackCalls) // never fell through to the fallback
    }

}

/**
 * Deterministic in-memory stand-in for [KeystoreManager] used only by
 * [WalletStorageUpgradeMatrixTest]. RSA-shaped blobs are 256-byte, empty-IV
 * ciphertexts tagged with the producing alias; legacy AES blobs carry a
 * non-empty IV. Only the alias that produced an RSA blob can decrypt it — every
 * other combination raises [BadPaddingException], mirroring the JCE contract the
 * production routing depends on. The pure structural predicates
 * ([isLegacyKeysBlob], [isKeysBlobDecryptable]) are inherited unchanged.
 */
private class FakeKeystoreManager :
    KeystoreManager(KeySecurityPolicy.AUTH_GATED) {

    enum class Scheme { CURRENT, FORMER_RSA, LEGACY_AES }

    enum class KeysAliasKind { NONE, AES, RSA }

    var scheme: Scheme = Scheme.CURRENT
    var keysAliasKind: KeysAliasKind = KeysAliasKind.NONE
    var policyKeyProvisioned: Boolean = false
    var throwAuthOnPolicyDecrypt: Boolean = false
    var legacyRsaFallbackCalls: Int = 0

    override val keysAlias: String get() = POLICY_ALIAS

    override fun encrypt(plaintext: ByteArray, alias: String): EncryptedBlob = when (scheme) {
        Scheme.CURRENT -> {
            policyKeyProvisioned = true // public-key encrypt provisions the alias
            rsaBlob(TAG_POLICY, plaintext)
        }
        Scheme.FORMER_RSA -> rsaBlob(TAG_FORMER_RSA, plaintext) // does NOT provision policy
        Scheme.LEGACY_AES -> aesBlob(plaintext)
    }

    override fun decrypt(blob: EncryptedBlob, alias: String): ByteArray {
        if (throwAuthOnPolicyDecrypt) throw UserNotAuthenticatedException()
        require(alias == POLICY_ALIAS) { "test only decrypts under the policy alias" }
        if (blob.ciphertext[0] == TAG_POLICY && policyKeyProvisioned) return plaintextOfRsa(blob)
        throw BadPaddingException("wrong key for $alias")
    }

    override fun decryptLegacyKeysBlob(blob: EncryptedBlob): ByteArray? =
        if (keysAliasKind == KeysAliasKind.AES) plaintextOfAes(blob) else null

    override fun decryptLegacyRsaKeysBlob(blob: EncryptedBlob): ByteArray? {
        legacyRsaFallbackCalls++
        if (keysAliasKind != KeysAliasKind.RSA) return null
        if (blob.ciphertext[0] == TAG_FORMER_RSA) return plaintextOfRsa(blob)
        throw BadPaddingException("former RSA key cannot open this blob")
    }

    override fun hasLegacyKeysKey(): Boolean = keysAliasKind == KeysAliasKind.AES

    override fun hasLegacyRsaKeysKey(): Boolean = keysAliasKind == KeysAliasKind.RSA

    override fun hasIdentityKeysKey(alias: String): Boolean =
        alias == POLICY_ALIAS && policyKeyProvisioned

    private fun rsaBlob(tag: Byte, plain: ByteArray): EncryptedBlob {
        val ct = ByteArray(RSA_BLOB_BYTES)
        ct[0] = tag
        ct[1] = plain.size.toByte()
        plain.copyInto(ct, 2)
        return EncryptedBlob(iv = ByteArray(0), ciphertext = ct)
    }

    private fun plaintextOfRsa(blob: EncryptedBlob): ByteArray {
        val len = blob.ciphertext[1].toInt() and 0xFF
        return blob.ciphertext.copyOfRange(2, 2 + len)
    }

    private fun aesBlob(plain: ByteArray): EncryptedBlob {
        val ct = ByteArray(1 + plain.size)
        ct[0] = plain.size.toByte()
        plain.copyInto(ct, 1)
        return EncryptedBlob(iv = ByteArray(12) { 0xAA.toByte() }, ciphertext = ct)
    }

    private fun plaintextOfAes(blob: EncryptedBlob): ByteArray {
        val len = blob.ciphertext[0].toInt() and 0xFF
        return blob.ciphertext.copyOfRange(1, 1 + len)
    }

    private companion object {
        const val POLICY_ALIAS = KeystoreManager.KEYS_ALIAS_AUTH_GATED
        const val RSA_BLOB_BYTES = 2048 / 8
        const val TAG_POLICY: Byte = 0
        const val TAG_FORMER_RSA: Byte = 2
    }
}
