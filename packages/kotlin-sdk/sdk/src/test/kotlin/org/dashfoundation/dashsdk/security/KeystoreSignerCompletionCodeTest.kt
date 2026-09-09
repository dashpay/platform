package org.dashfoundation.dashsdk.security

import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.UserNotAuthenticatedException
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import javax.crypto.BadPaddingException

/**
 * Pins the sign-path completion-code classification (#4060 round-2 finding
 * 2): a `KeyPermanentlyInvalidatedException` — the key was invalidated by
 * biometric/credential re-enrollment and cannot sign until re-derived —
 * must surface as the TYPED `SIGNER_ERROR_CODE_KEY_UNAVAILABLE` on the
 * FIRST attempt (→ platform-wallet code 31 →
 * `DashSdkError.PlatformWallet.SigningKeyUnavailable`), never as an opaque
 * generic failure. Classification is a pure companion function because the
 * full [KeystoreSigner] cannot be constructed on the JVM (its constructor
 * creates a native signer handle); Robolectric supplies the
 * `android.security.keystore` exception types.
 */
@RunWith(RobolectricTestRunner::class)
class KeystoreSignerCompletionCodeTest {

    @Test
    fun keyPermanentlyInvalidatedClassifiesAsKeyUnavailable() {
        assertEquals(
            SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE,
            KeystoreSigner.completionErrorCodeFor(KeyPermanentlyInvalidatedException()),
        )
    }

    @Test
    fun otherSignFailuresStayGeneric() {
        assertEquals(
            SignerNative.SIGNER_ERROR_CODE_GENERIC,
            KeystoreSigner.completionErrorCodeFor(IllegalStateException("boom")),
        )
        assertEquals(
            SignerNative.SIGNER_ERROR_CODE_GENERIC,
            KeystoreSigner.completionErrorCodeFor(BadPaddingException("wrong key")),
        )
        // A closed auth window is handled by the biometric-gate retry; an
        // unhandled one is a generic failure, NOT a missing key — the key
        // exists and opens after auth.
        assertEquals(
            SignerNative.SIGNER_ERROR_CODE_GENERIC,
            KeystoreSigner.completionErrorCodeFor(UserNotAuthenticatedException()),
        )
    }
}
