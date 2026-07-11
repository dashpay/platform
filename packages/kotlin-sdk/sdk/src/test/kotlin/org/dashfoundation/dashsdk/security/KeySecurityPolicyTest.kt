package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Pins the dashpay/platform#4053 policy plumbing: the identity-key
 * security policy selects the Keystore alias new identity keys are wrapped
 * under, defaults to the historical [KeySecurityPolicy.AUTH_GATED]
 * behavior everywhere, and rides [WalletStorage] construction so host apps
 * with their own auth model can opt into
 * [KeySecurityPolicy.DEVICE_BOUND] without touching [KeystoreManager].
 *
 * (The Keystore key-generation semantics themselves — auth-gated vs not —
 * can't run on the JVM: AndroidKeyStore has no Robolectric provider. The
 * alias split is the load-bearing part: Keystore auth parameters are fixed
 * at key generation, so the policy MUST resolve to distinct aliases.)
 */
@RunWith(RobolectricTestRunner::class)
class KeySecurityPolicyTest {

    @Test
    fun keystoreManagerDefaultsToAuthGated() {
        val manager = KeystoreManager()
        assertEquals(KeySecurityPolicy.AUTH_GATED, manager.keySecurityPolicy)
        // The AUTH_GATED RSA keypair lives at its own alias — NOT the legacy
        // KEYS_ALIAS, whose AES key must survive the upgrade so old blobs stay
        // recoverable.
        assertEquals(KeystoreManager.KEYS_ALIAS_AUTH_GATED, manager.keysAlias)
        assertFalse(manager.keysAlias == KeystoreManager.KEYS_ALIAS)
    }

    @Test
    fun deviceBoundPolicySelectsTheDedicatedAlias() {
        val manager = KeystoreManager(KeySecurityPolicy.DEVICE_BOUND)
        assertEquals(KeySecurityPolicy.DEVICE_BOUND, manager.keySecurityPolicy)
        assertEquals(KeystoreManager.KEYS_ALIAS_DEVICE_BOUND, manager.keysAlias)
    }

    @Test
    fun policiesResolveToDistinctAliases() {
        // Keystore auth parameters are immutable post-generation — sharing one
        // alias across policies would silently keep the first policy. All three
        // identity aliases (legacy AES + the two RSA) must also stay distinct so
        // the legacy key is never regenerated or deleted.
        val aliases = setOf(
            KeystoreManager.KEYS_ALIAS,
            KeystoreManager.KEYS_ALIAS_AUTH_GATED,
            KeystoreManager.KEYS_ALIAS_DEVICE_BOUND,
        )
        assertEquals(3, aliases.size)
    }

    @Test
    fun onlyTheRsaIdentityAliasesAreRecognized() {
        assertTrue(KeystoreManager.isIdentityKeysAlias(KeystoreManager.KEYS_ALIAS_AUTH_GATED))
        assertTrue(KeystoreManager.isIdentityKeysAlias(KeystoreManager.KEYS_ALIAS_DEVICE_BOUND))
        // The legacy alias is read-only (migration fallback), never an RSA
        // encrypt/decrypt target.
        assertFalse(KeystoreManager.isIdentityKeysAlias(KeystoreManager.KEYS_ALIAS))
        assertFalse(KeystoreManager.isIdentityKeysAlias(KeystoreManager.MASTER_ALIAS))
    }

    @Test
    fun blobTypeDiscriminationSeparatesRsaFromLegacy() {
        // Construction is inert (AndroidKeyStore access is lazy); the blob-type
        // checks are pure structural predicates.
        val km = KeystoreManager()

        // An RSA blob carries no iv and exactly one 2048-bit block.
        val rsaBlob = KeystoreManager.EncryptedBlob(
            iv = ByteArray(0),
            ciphertext = ByteArray(2048 / 8),
        )
        assertTrue(km.isKeysBlobDecryptable(rsaBlob))
        assertFalse(km.isLegacyKeysBlob(rsaBlob))

        // A legacy AES-GCM blob carries a 12-byte GCM nonce.
        val legacyBlob = KeystoreManager.EncryptedBlob(
            iv = ByteArray(12) { 7 },
            ciphertext = ByteArray(48),
        )
        assertFalse(km.isKeysBlobDecryptable(legacyBlob))
        assertTrue(km.isLegacyKeysBlob(legacyBlob))
    }

    @Test
    fun walletStoragePlumbsThePolicyThrough() {
        val storage = WalletStorage(
            ApplicationProvider.getApplicationContext(),
            KeySecurityPolicy.DEVICE_BOUND,
        )
        assertEquals(KeySecurityPolicy.DEVICE_BOUND, storage.keySecurityPolicy)
    }

    @Test
    fun walletStorageDefaultStaysAuthGated() {
        val storage = WalletStorage(ApplicationProvider.getApplicationContext())
        assertEquals(KeySecurityPolicy.AUTH_GATED, storage.keySecurityPolicy)
    }
}
