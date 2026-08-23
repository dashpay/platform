package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.security.GeneralSecurityException
import java.security.InvalidKeyException
import javax.crypto.BadPaddingException

/**
 * Pins the typed device-locked Keystore denial mapping (the QA field
 * failure: `UserNotAuthenticatedException` from the lock-bound, NON-auth-
 * gated [KeystoreManager.MASTER_ALIAS] AES key during wallet creation —
 * sometimes while the device was demonstrably unlocked, i.e. Keystore2
 * lock-state misreporting). The classifier and the mapping are pure /
 * probe-injected, so — like [KeystoreKeyGenPolicyTest] — they run on the
 * plain JVM with type-name stand-ins for the Android Keystore exceptions
 * (which cannot be constructed here).
 */
class KeystoreDeviceLockedDenialTest {

    /**
     * Stand-in for `android.security.keystore.UserNotAuthenticatedException`;
     * the classifier matches by type-name suffix (the
     * [KeystoreManager.isNoSecureLockScreenKeyGenFailure] discipline).
     */
    private class SimulatedUserNotAuthenticatedException :
        GeneralSecurityException("User not authenticated")

    /** Stand-in for `android.security.KeyStoreException`. */
    private class SimulatedKeyStoreException(message: String) :
        GeneralSecurityException(message)

    // ── Classifier ───────────────────────────────────────────────────────

    @Test
    fun shouldClassifyUserNotAuthenticatedAsDeviceLockedDenial() {
        // The field shape: the master-alias AES key carries NO
        // setUserAuthenticationRequired gate, so "user not authenticated"
        // from it can only be the setUnlockedDeviceRequired denial.
        assertTrue(
            KeystoreManager.isDeviceLockedKeystoreDenial(
                SimulatedUserNotAuthenticatedException(),
            ),
        )
    }

    @Test
    fun shouldClassifyWrappedUserNotAuthenticatedInCauseChain() {
        // Some API levels wrap the denial (e.g. cipher.init throwing
        // InvalidKeyException whose cause is the Keystore denial).
        val wrapped = InvalidKeyException(
            "Keystore operation failed",
            SimulatedUserNotAuthenticatedException(),
        )
        assertTrue(KeystoreManager.isDeviceLockedKeystoreDenial(wrapped))
    }

    @Test
    fun shouldClassifyKeyStoreExceptionNamingTheLockedDevice() {
        assertTrue(
            KeystoreManager.isDeviceLockedKeystoreDenial(
                SimulatedKeyStoreException("Keystore operation failed: device locked"),
            ),
        )
        assertTrue(
            KeystoreManager.isDeviceLockedKeystoreDenial(
                InvalidKeyException("unlocked device required"),
            ),
        )
    }

    @Test
    fun shouldNotClassifyUnrelatedCryptoOrKeystoreFailures() {
        // A GCM tag failure, a generic Keystore fault, a transient internal
        // error: none of these are the device-locked gate and none may be
        // mapped to the retryable exception.
        assertFalse(
            KeystoreManager.isDeviceLockedKeystoreDenial(BadPaddingException("mac check failed")),
        )
        assertFalse(
            KeystoreManager.isDeviceLockedKeystoreDenial(
                SimulatedKeyStoreException("System error (internal Keystore code: 4)"),
            ),
        )
        assertFalse(
            KeystoreManager.isDeviceLockedKeystoreDenial(
                InvalidKeyException("no key material"),
            ),
        )
    }

    // ── Mapping + at-throw-time lock-state sampling ──────────────────────

    private fun managerSampling(state: DeviceLockState): KeystoreManager =
        KeystoreManager(deviceLockStateProbe = { state })

    @Test
    fun shouldMapDenialToTypedExceptionSamplingFalseLockedState() {
        // The defect class: Keystore denies as device-locked while
        // KeyguardManager says the device is NOT locked. The sampled state
        // must ride the exception so logs (and the storeMnemonic retry) can
        // tell this apart from a genuine lock.
        val manager = managerSampling(
            DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false),
        )
        val denial = SimulatedUserNotAuthenticatedException()

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            manager.rethrowClassifyingDeviceLockedDenial(
                denial,
                KeystoreManager.MASTER_ALIAS,
                operation = "encrypt",
            )
        }
        assertEquals(KeystoreManager.MASTER_ALIAS, thrown.alias)
        assertEquals("encrypt", thrown.operation)
        assertFalse(thrown.deviceReportsLocked)
        assertFalse(thrown.lockState.isDeviceLocked)
        assertSame(denial, thrown.cause)
        assertTrue(thrown.message.orEmpty().contains("FALSE-LOCKED"))
    }

    @Test
    fun shouldMapDenialToTypedExceptionSamplingGenuinelyLockedState() {
        val manager = managerSampling(
            DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true),
        )

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            manager.rethrowClassifyingDeviceLockedDenial(
                SimulatedUserNotAuthenticatedException(),
                KeystoreManager.MASTER_ALIAS,
                operation = "decrypt",
            )
        }
        assertEquals("decrypt", thrown.operation)
        assertTrue(thrown.deviceReportsLocked)
        assertTrue(thrown.lockState.isKeyguardLocked)
        assertTrue(thrown.message.orEmpty().contains("genuinely locked"))
    }

    @Test
    fun shouldRethrowNonDenialExceptionsUnchanged() {
        val manager = managerSampling(
            DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false),
        )
        val unrelated = BadPaddingException("mac check failed")

        val thrown = assertThrows(BadPaddingException::class.java) {
            manager.rethrowClassifyingDeviceLockedDenial(
                unrelated,
                KeystoreManager.MASTER_ALIAS,
                operation = "decrypt",
            )
        }
        assertSame(unrelated, thrown)
    }
}
