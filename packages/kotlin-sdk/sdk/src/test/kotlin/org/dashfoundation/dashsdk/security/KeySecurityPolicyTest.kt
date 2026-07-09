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
        assertEquals(KeystoreManager.KEYS_ALIAS, manager.keysAlias)
    }

    @Test
    fun deviceBoundPolicySelectsTheDedicatedAlias() {
        val manager = KeystoreManager(KeySecurityPolicy.DEVICE_BOUND)
        assertEquals(KeySecurityPolicy.DEVICE_BOUND, manager.keySecurityPolicy)
        assertEquals(KeystoreManager.KEYS_ALIAS_DEVICE_BOUND, manager.keysAlias)
    }

    @Test
    fun policiesResolveToDistinctAliases() {
        // Keystore auth parameters are immutable post-generation — sharing
        // one alias across policies would silently keep the first policy.
        assertFalse(
            KeystoreManager.KEYS_ALIAS == KeystoreManager.KEYS_ALIAS_DEVICE_BOUND,
        )
    }

    @Test
    fun bothIdentityKeyAliasesAreRecognized() {
        assertTrue(KeystoreManager.isIdentityKeysAlias(KeystoreManager.KEYS_ALIAS))
        assertTrue(KeystoreManager.isIdentityKeysAlias(KeystoreManager.KEYS_ALIAS_DEVICE_BOUND))
        assertFalse(KeystoreManager.isIdentityKeysAlias(KeystoreManager.MASTER_ALIAS))
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
