package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
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
 * 1. [WalletStorage.ensureMasterKeyNotLockBlocked] — the `createWallet`
 *    fail-fast pre-check: throws the typed, retryable
 *    [KeystoreDeviceLockedException] BEFORE any native wallet exists when
 *    the device is locked AND the master key is actually lock-bound —
 *    decided by a preflight master-alias probe encrypt, because a key
 *    generated on a then-lockless device carries no lock binding and must
 *    not block creation.
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
        // masterKeyLockBound defaults true: the key carries
        // setUnlockedDeviceRequired, so the Keystore denies the probe.
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            storage.ensureMasterKeyNotLockBlocked(operation = "createWallet")
        }
        assertEquals("createWallet", thrown.operation)
        assertEquals(KeystoreManager.MASTER_ALIAS, thrown.alias)
        assertTrue(thrown.deviceReportsLocked)
        // The verdict came from the Keystore itself: exactly one preflight
        // probe encrypt, whose classified denial rides along as the cause.
        assertEquals(1, fake.masterEncryptCalls)
        assertTrue(thrown.cause is KeystoreDeviceLockedException)
    }

    @Test
    fun shouldPassPreCheckWhenDeviceIsUnlocked() {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") // must not throw
        // Unlocked is decided from KeyguardManager alone — prompt-free AND
        // Keystore-free (no probe).
        assertEquals(0, fake.masterEncryptCalls)
    }

    @Test
    fun shouldPassPreCheckWhenKeyguardShowsButDeviceIsNotSecurelyLocked() {
        // isKeyguardLocked without isDeviceLocked (e.g. a non-secure swipe
        // screen): the Keystore unlocked-device gate keys off the SECURE
        // lock, so this state must not block wallet creation.
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = true)
        storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") // must not throw
        assertEquals(0, fake.masterEncryptCalls)
    }

    @Test
    fun shouldPassPreCheckWhenDeviceIsLockedButMasterKeyIsNotLockBound() {
        // A master key generated while the device had NO secure lock screen
        // carries no setUnlockedDeviceRequired
        // ([KeystoreManager]'s generateWithLockScreenDegradation) and existing
        // keys are never regenerated — so after the user later enrolls a PIN,
        // master-alias crypto still succeeds on the locked device and wallet
        // creation must proceed. KeyguardManager.isDeviceLocked alone cannot
        // decide this; only the Keystore can.
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)
        fake.masterKeyLockBound = false

        storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") // must not throw

        // The verdict came from the Keystore itself: exactly one preflight
        // probe encrypt (discarded, nothing persisted).
        assertEquals(1, fake.masterEncryptCalls)
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

    // ── storeMnemonic plaintext-buffer scrubbing ─────────────────────────

    @Test
    fun shouldScrubMnemonicBufferAfterSuccessfulStore() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)

        storage.storeMnemonic(walletId, mnemonic)

        // The encrypt really saw the phrase...
        assertArrayEquals(mnemonic.encodeToByteArray(), fake.lastMasterPlaintextAtCall)
        // ...and the retained plaintext copy was zeroed before returning.
        assertBufferScrubbed(fake.lastMasterPlaintextRef)
        // Scrubbing the input buffer must not corrupt what was stored.
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
    }

    @Test
    fun shouldScrubMnemonicBufferWhenFinalDenialPropagates() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE // never heals — the schedule exhausts

        var thrown = false
        try {
            storage.storeMnemonic(walletId, mnemonic)
        } catch (e: KeystoreDeviceLockedException) {
            thrown = true
        }

        assertTrue("expected the typed denial to propagate", thrown)
        assertBufferScrubbed(fake.lastMasterPlaintextRef)
    }

    @Test
    fun shouldScrubMnemonicBufferWhenCancelledDuringRetryBackoff() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE // park storeMnemonic in its backoff delay
        val firstAttempt = CompletableDeferred<Unit>()
        fake.onMasterEncrypt = { firstAttempt.complete(Unit) }

        val job = launch { storage.storeMnemonic(walletId, mnemonic) }
        firstAttempt.await()
        // The first denial has happened; storeMnemonic is in (or headed into)
        // its backoff delay — the retry loop's only suspension point, where
        // this cancellation lands. join returns only after the coroutine has
        // fully completed, finally blocks included.
        job.cancelAndJoin()

        assertBufferScrubbed(fake.lastMasterPlaintextRef)
    }

    private fun assertBufferScrubbed(buffer: ByteArray?) {
        assertTrue("expected the plaintext buffer to have been captured", buffer != null)
        assertTrue(
            "expected the retained mnemonic plaintext buffer to be zeroed",
            buffer!!.all { it == 0.toByte() },
        )
    }
}

/**
 * Master-alias-focused fake: scripts [failMasterEncrypts] device-locked
 * denials (each carrying [lockState] sampled "at throw time", as the real
 * mapping does) before letting encrypts succeed with a trivially reversible
 * blob. [masterKeyLockBound] models the key's effective policy: when true
 * (the default — a key generated on a lock-screen device carries
 * `setUnlockedDeviceRequired`), any encrypt while [lockState] reports the
 * device locked is denied, exactly as the real Keystore gate behaves; when
 * false (a key generated on a then-lockless device, never regenerated),
 * encrypts succeed regardless of lock state. Identity-key aliases are out of
 * scope here — see [WalletStorageUpgradeMatrixTest]'s fake for that ladder.
 */
private class FalseLockedFakeKeystoreManager : KeystoreManager() {

    var lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
    var failMasterEncrypts = 0
    var masterEncryptCalls = 0

    /** Whether the fake master key carries the unlocked-device requirement. */
    var masterKeyLockBound = true

    /** The exact buffer reference the last master encrypt received. */
    var lastMasterPlaintextRef: ByteArray? = null

    /** Snapshot of that buffer's content AT CALL TIME (pre-scrub evidence). */
    var lastMasterPlaintextAtCall: ByteArray? = null

    /** Invoked at each master encrypt attempt (test synchronization hook). */
    var onMasterEncrypt: (() -> Unit)? = null

    override fun sampleDeviceLockState(): DeviceLockState = lockState

    override fun encrypt(plaintext: ByteArray, alias: String): EncryptedBlob {
        check(alias == MASTER_ALIAS) { "test fake only models the master alias" }
        masterEncryptCalls++
        lastMasterPlaintextRef = plaintext
        lastMasterPlaintextAtCall = plaintext.copyOf()
        onMasterEncrypt?.invoke()
        val scriptedDenial = failMasterEncrypts > 0
        if (scriptedDenial) failMasterEncrypts--
        if (scriptedDenial || (masterKeyLockBound && lockState.isDeviceLocked)) {
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
