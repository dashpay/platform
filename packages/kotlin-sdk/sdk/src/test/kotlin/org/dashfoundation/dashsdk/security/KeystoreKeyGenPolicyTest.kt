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
    private open class SimulatedKeyStoreException(message: String) :
        GeneralSecurityException(message)

    /**
     * Numeric-code-carrying stand-in: the classifier reads
     * `getNumericErrorCode()` reflectively (the real method is
     * `android.security.KeyStoreException`'s, API 33+), so declaring the
     * same method here exercises the numeric path on the JVM.
     */
    private class SimulatedNumericKeyStoreException(
        message: String,
        private val numericErrorCode: Int,
    ) : SimulatedKeyStoreException(message) {
        @Suppress("unused") // reflectively invoked by the classifier
        fun getNumericErrorCode(): Int = numericErrorCode
    }

    @Test
    fun classifiesWrappedKeystoreGenerationFailure() {
        // The exact on-device shape: a key-gen ProviderException wrapping a
        // Keystore system error carrying the lock-screen rejection numeric code
        // (the real android.security.KeyStoreException exposes it via
        // getNumericErrorCode(), API 33+). It is the numeric code — not the
        // incidental "generate_key" mention in the text — that classifies
        // (dashpay/platform#4060 blocker 2).
        val failure = ProviderException(
            "Keystore key generation failed",
            SimulatedNumericKeyStoreException(
                "System error (internal Keystore code: 4 message: In generate_key. 10309)",
                numericErrorCode = 4,
            ),
        )
        assertTrue(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun doesNotClassifyBareGenerateKeyWithoutLockScreenSignal() {
        // Blocker 2: a bare "generate_key" mention is NOT a lock-screen signal.
        // A transient KeyMint generation failure names generate_key too; if it
        // classified, an AUTH_GATED key would be permanently downgraded to
        // DEVICE_BOUND instead of being retried.
        val failure = SimulatedKeyStoreException("Keymint error In generate_key")
        assertFalse(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun doesNotClassifyTransientGenerateKeyUnderKeyGenProviderException() {
        // Blocker 2, the dangerous shape: a key-gen ProviderException wrapping a
        // transient generate_key Keystore failure on a device that HAS a lock
        // screen — no lock-screen text, no rejection numeric code. It must NOT
        // classify, so the AUTH_GATED write is retried, not silently and
        // permanently degraded to DEVICE_BOUND.
        val failure = ProviderException(
            "Keystore key generation failed",
            SimulatedNumericKeyStoreException(
                "System error: In generate_key. Failed to generate key (transient)",
                numericErrorCode = -8, // KeyMint SECURE_HW_COMMUNICATION_FAILED-style transient, not the lock-screen code
            ),
        )
        assertFalse(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun classifiesExplicitLockScreenMessage() {
        val failure = SimulatedKeyStoreException("Requires a secure lock screen to be set up")
        assertTrue(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun classifiesNumericRejectionCodeUnderAKeyGenProviderException() {
        // Some OEM builds report the rejection with an opaque message; the
        // structured numeric code (internal Keystore 4 / KeyMint 10309) is
        // then the only evidence. It counts ONLY inside a key-gen
        // ProviderException's cause chain.
        listOf(4, 10309).forEach { code ->
            val failure = ProviderException(
                "Keystore key generation failed",
                SimulatedNumericKeyStoreException("System error", code),
            )
            assertTrue(
                "numeric code $code must classify",
                KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure),
            )
        }
    }

    @Test
    fun doesNotClassifyUnrelatedNumericCodes() {
        val failure = ProviderException(
            "Keystore key generation failed",
            SimulatedNumericKeyStoreException("System error", 7),
        )
        assertFalse(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun doesNotClassifyANestedKeystoreErrorWithAnUnrelatedMessage() {
        // Regression guard for the over-broad classifier: previously, ANY
        // nested KeyStoreException classified once a generic "key generation
        // failed" ProviderException had been seen anywhere earlier in the
        // chain walk. A generation failure whose underlying Keystore error is
        // unrelated to the lock screen (no generate_key / lock-screen message,
        // no rejection code) must NOT trigger the degraded retry.
        val failure = ProviderException(
            "Keystore key generation failed",
            SimulatedKeyStoreException("Signature verification failed"),
        )
        assertFalse(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
    }

    @Test
    fun doesNotClassifyAKeystoreErrorOutsideTheKeyGenSubtree() {
        // The lock-screen-message match stands alone, but the bare-system-
        // error match requires the key-gen ProviderException ABOVE it in the
        // cause chain — an unrelated wrapper does not open the window.
        val failure = ProviderException(
            "unrelated provider issue",
            SimulatedNumericKeyStoreException("System error", 4),
        )
        assertFalse(KeystoreManager.isNoSecureLockScreenKeyGenFailure(failure))
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
