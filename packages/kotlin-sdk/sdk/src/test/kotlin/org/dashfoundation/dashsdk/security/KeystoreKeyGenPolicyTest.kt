package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.security.GeneralSecurityException
import java.security.ProviderException

/**
 * Pins the no-secure-lock-screen key-generation degradation
 * (dashpay/platform#4060). The lock-screen-bound Keystore parameters
 * (`setUnlockedDeviceRequired`, and for the auth-gated alias
 * `setUserAuthenticationRequired`) require a secure lock screen; KeyMint rejects
 * `generate_key` for them otherwise (observed on-device: ProviderException
 * "Keystore key generation failed" / internal Keystore code 4 / KeyMint 10309),
 * which used to hard-crash wallet creation on a device with no screen lock. The
 * parameter-selection and failure-classification logic is factored into pure
 * functions so it is unit-testable without an AndroidKeyStore runtime (which has
 * no Robolectric provider — see [KeySecurityPolicyTest]).
 */
class KeystoreKeyGenPolicyTest {

    @Test
    fun lockBoundParamsRequireASecureLockScreen() {
        // The whole point: apply the lock-bound params only when a secure lock
        // screen exists, so a lockless device generates a usable (degraded) key
        // instead of failing generation.
        assertTrue(KeystoreManager.lockBoundKeyParamsSupported(deviceSecure = true))
        assertFalse(KeystoreManager.lockBoundKeyParamsSupported(deviceSecure = false))
    }

    /**
     * Stand-in for `android.security.KeyStoreException`, which cannot be
     * constructed on the plain JVM. The classifier matches Keystore exceptions
     * by type-name suffix, so any type whose simple name ends in
     * `KeyStoreException` exercises the same path.
     */
    private class SimulatedKeyStoreException(message: String) :
        GeneralSecurityException(message)

    @Test
    fun classifiesWrappedKeystoreGenerationFailure() {
        // The exact on-device shape: a key-gen ProviderException wrapping a
        // Keystore system error from generate_key.
        val failure = ProviderException(
            "Keystore key generation failed",
            SimulatedKeyStoreException(
                "System error (internal Keystore code: 4 message: In generate_key. 10309)",
            ),
        )
        assertTrue(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun classifiesDirectGenerateKeyKeystoreError() {
        val failure = SimulatedKeyStoreException("Keymint error In generate_key")
        assertTrue(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun classifiesExplicitLockScreenMessage() {
        val failure = SimulatedKeyStoreException("Requires a secure lock screen to be set up")
        assertTrue(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun doesNotClassifyUnrelatedFailures() {
        // A generic runtime error is not the lock-screen signature.
        assertFalse(
            KeystoreManager.isNoSecureLockScreenKeyGenFailure(
                IllegalStateException("some unrelated error"),
            ),
        )
        // A ProviderException with no keystore cause and no gen-failed message.
        assertFalse(
            KeystoreManager.isNoSecureLockScreenKeyGenFailure(
                ProviderException("unrelated provider issue"),
            ),
        )
        // A Keystore error unrelated to generation / lock screen, not wrapped by
        // a key-gen ProviderException, must not trigger the degraded retry.
        assertFalse(
            KeystoreManager.isNoSecureLockScreenKeyGenFailure(
                SimulatedKeyStoreException("Signature verification failed"),
            ),
        )
    }
}
