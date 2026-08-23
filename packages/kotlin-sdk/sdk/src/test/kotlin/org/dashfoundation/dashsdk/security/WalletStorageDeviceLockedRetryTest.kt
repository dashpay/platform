package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Pins the device-locked hardening around wallet creation's mnemonic
 * persistence (the QA field failure — Keystore denied the lock-bound
 * [KeystoreManager.MASTER_ALIAS] encrypt as device-locked during
 * `createWallet`, on devices that were demonstrably unlocked):
 *
 * 1. [WalletStorage.ensureDeviceUnlocked] — the `createWallet` fail-fast
 *    pre-check: throws the typed, retryable [KeystoreDeviceLockedException]
 *    on a genuinely locked device BEFORE any native wallet exists.
 * 2. [WalletStorage.storeMnemonic]'s bounded FALSE-LOCKED retry: a denial
 *    whose sampled `KeyguardManager` state says the device is NOT locked
 *    (the Keystore2 misreporting defect) is retried up to 3 times; a
 *    genuinely-locked denial fails fast with no retry.
 *
 * The real AndroidKeyStore crypto cannot run on the JVM (see
 * [KeySecurityPolicyTest]), so a fake [KeystoreManager] scripts the
 * master-alias encrypt outcomes through the class's `open` test seams,
 * exactly as [WalletStorageUpgradeMatrixTest] does for the identity-key
 * ladder.
 */
@RunWith(RobolectricTestRunner::class)
class WalletStorageDeviceLockedRetryTest {

    private val walletId = ByteArray(32) { (it + 1).toByte() }
    private val mnemonic = "abandon abandon abandon abandon abandon abandon " +
        "abandon abandon abandon abandon abandon about"

    private lateinit var fake: FalseLockedFakeKeystoreManager
    private lateinit var storage: WalletStorage

    @Before
    fun setUp() = runBlocking {
        fake = FalseLockedFakeKeystoreManager()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), fake)
        // Isolate from any state a prior test left in the shared DataStore file.
        storage.deleteAll()
    }

    // ── createWallet fail-fast pre-check ─────────────────────────────────

    @Test
    fun shouldFailFastWhenDeviceIsGenuinelyLocked() {
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            storage.ensureDeviceUnlocked(operation = "createWallet")
        }
        assertEquals("createWallet", thrown.operation)
        assertEquals(KeystoreManager.MASTER_ALIAS, thrown.alias)
        assertTrue(thrown.deviceReportsLocked)
        // Prompt-free and Keystore-free: the pre-check must never touch the
        // master key (nothing to retry, nothing to roll back).
        assertEquals(0, fake.masterEncryptCalls)
    }

    @Test
    fun shouldPassPreCheckWhenDeviceIsUnlocked() {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        storage.ensureDeviceUnlocked(operation = "createWallet") // must not throw
    }

    @Test
    fun shouldPassPreCheckWhenKeyguardShowsButDeviceIsNotSecurelyLocked() {
        // isKeyguardLocked without isDeviceLocked (e.g. a non-secure swipe
        // screen): the Keystore unlocked-device gate keys off the SECURE
        // lock, so this state must not block wallet creation.
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = true)
        storage.ensureDeviceUnlocked(operation = "createWallet") // must not throw
    }

    // ── storeMnemonic bounded FALSE-LOCKED retry ─────────────────────────

    @Test
    fun shouldRetryFalseLockedDenialAndSucceedOnSecondAttempt() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = 1 // deny once, then heal — the observed field pattern

        storage.storeMnemonic(walletId, mnemonic)

        assertEquals(2, fake.masterEncryptCalls)
        // The store really landed: the mnemonic round-trips.
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
    }

    @Test
    fun shouldGiveUpAfterThreeFalseLockedRetries() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE // never heals

        var thrown: KeystoreDeviceLockedException? = null
        try {
            storage.storeMnemonic(walletId, mnemonic)
        } catch (e: KeystoreDeviceLockedException) {
            thrown = e
        }

        assertTrue("expected the typed denial to propagate", thrown != null)
        assertFalse(thrown!!.deviceReportsLocked)
        // Initial attempt + the full 3-retry schedule (250/750/1000ms),
        // then give up.
        assertEquals(4, fake.masterEncryptCalls)
        assertEquals(null, storage.retrieveMnemonic(walletId))
    }

    @Test
    fun shouldNotRetryWhenDeviceIsGenuinelyLocked() = runBlocking {
        // The denial is CORRECT here — a 2s in-process retry cannot unlock
        // a phone, so the exception must propagate immediately.
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)
        fake.failMasterEncrypts = Int.MAX_VALUE

        var thrown: KeystoreDeviceLockedException? = null
        try {
            storage.storeMnemonic(walletId, mnemonic)
        } catch (e: KeystoreDeviceLockedException) {
            thrown = e
        }

        assertTrue("expected the typed denial to propagate", thrown != null)
        assertTrue(thrown!!.deviceReportsLocked)
        assertEquals(1, fake.masterEncryptCalls)
    }

    @Test
    fun shouldStoreWithoutRetryMachineryWhenKeystoreIsHealthy() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)

        storage.storeMnemonic(walletId, mnemonic)

        assertEquals(1, fake.masterEncryptCalls)
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
    }
}

/**
 * Master-alias-focused fake: scripts [failMasterEncrypts] device-locked
 * denials (each carrying [lockState] sampled "at throw time", as the real
 * mapping does) before letting encrypts succeed with a trivially reversible
 * blob. Identity-key aliases are out of scope here — see
 * [WalletStorageUpgradeMatrixTest]'s fake for that ladder.
 */
private class FalseLockedFakeKeystoreManager : KeystoreManager() {

    var lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
    var failMasterEncrypts = 0
    var masterEncryptCalls = 0

    override fun sampleDeviceLockState(): DeviceLockState = lockState

    override fun encrypt(plaintext: ByteArray, alias: String): EncryptedBlob {
        check(alias == MASTER_ALIAS) { "test fake only models the master alias" }
        masterEncryptCalls++
        if (failMasterEncrypts > 0) {
            failMasterEncrypts--
            throw KeystoreDeviceLockedException(
                alias = alias,
                operation = "encrypt",
                lockState = sampleDeviceLockState(),
            )
        }
        return EncryptedBlob(iv = ByteArray(12) { 7 }, ciphertext = plaintext.copyOf())
    }

    override fun decrypt(blob: EncryptedBlob, alias: String): ByteArray {
        check(alias == MASTER_ALIAS) { "test fake only models the master alias" }
        return blob.ciphertext.copyOf()
    }
}
