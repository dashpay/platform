package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CancellationException
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
 * 3. The last-rung DEGRADATION when that schedule exhausts still
 *    false-locked (the persistent defect — an OEM unlock class that never
 *    satisfies `UNLOCKED_DEVICE_REQUIRED`): the store re-encrypts under
 *    the never-lock-bound [KeystoreManager.MASTER_ALIAS_UNBOUND], records
 *    the defect durably, and from then on writes go straight to the
 *    unbound alias, reads route by the blob's recorded alias, and
 *    still-lock-bound blobs are re-wrapped on their first successful read.
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
    private val siblingWalletId = ByteArray(32) { (it + 101).toByte() }
    private val mnemonic = "abandon abandon abandon abandon abandon abandon " +
        "abandon abandon abandon abandon abandon about"

    private lateinit var fake: FalseLockedFakeKeystoreManager
    private lateinit var storage: WalletStorage

    @Before
    fun setUp() = runBlocking {
        fake = FalseLockedFakeKeystoreManager()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), fake)
        // Isolate from any state a prior test left in the shared DataStore
        // file — including the durable false-locked defect record.
        storage.deleteAll()
    }

    // ── createWallet fail-fast pre-check ─────────────────────────────────

    @Test
    fun shouldFailFastWhenDeviceIsGenuinelyLocked() {
        // masterKeyLockBound defaults true: the key carries
        // setUnlockedDeviceRequired, so the Keystore denies the probe.
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") }
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
    fun shouldPassPreCheckWhenDeviceIsUnlocked() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") // must not throw
        // Unlocked is decided from KeyguardManager alone — prompt-free AND
        // Keystore-free (no probe).
        assertEquals(0, fake.masterEncryptCalls)
    }

    @Test
    fun shouldPassPreCheckWhenKeyguardShowsButDeviceIsNotSecurelyLocked() = runBlocking {
        // isKeyguardLocked without isDeviceLocked (e.g. a non-secure swipe
        // screen): the Keystore unlocked-device gate keys off the SECURE
        // lock, so this state must not block wallet creation.
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = true)
        storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") // must not throw
        assertEquals(0, fake.masterEncryptCalls)
    }

    @Test
    fun shouldPassPreCheckWhenDeviceIsLockedButMasterKeyIsNotLockBound() = runBlocking {
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

    @Test
    fun shouldPassPreCheckOnLockedDeviceOnceDefectIsOnRecord() = runBlocking {
        // Demonstrate the persistent defect (unlocked, denials outlast the
        // schedule) so the degradation records it...
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(walletId, mnemonic)
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
        val masterEncryptsSoFar = fake.masterEncryptCalls

        // ...then a GENUINELY locked entry must proceed with no probe at
        // all: writes target the never-lock-bound alias, which no lock
        // state can deny — there is nothing to preflight.
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)
        storage.ensureMasterKeyNotLockBlocked(operation = "createWallet") // must not throw
        assertEquals(masterEncryptsSoFar, fake.masterEncryptCalls)
    }

    // ── storeMnemonic bounded FALSE-LOCKED retry ─────────────────────────

    @Test
    fun shouldRetryFalseLockedDenialAndSucceedOnSecondAttempt() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = 1 // deny once, then heal — the observed field pattern

        storage.storeMnemonic(walletId, mnemonic)

        assertEquals(2, fake.masterEncryptCalls)
        // A transient blip must NOT record the persistent defect or touch
        // the unbound alias — the retry alone absorbed it.
        assertEquals(0, fake.unboundEncryptCalls)
        assertFalse(storage.isMasterKeyLockBindingDefectObserved())
        // The store really landed: the mnemonic round-trips.
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
    }

    @Test
    fun shouldNotRetryWhenDeviceIsGenuinelyLocked() = runBlocking {
        // The denial is CORRECT here — a 2s in-process retry cannot unlock
        // a phone, so the exception must propagate immediately, and the
        // degradation must NOT fire (a locked phone denying a lock-bound
        // key is the gate working, not the defect).
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
        assertEquals(0, fake.unboundEncryptCalls)
        assertFalse(storage.isMasterKeyLockBindingDefectObserved())
    }

    @Test
    fun shouldStoreWithoutRetryMachineryWhenKeystoreIsHealthy() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)

        storage.storeMnemonic(walletId, mnemonic)

        assertEquals(1, fake.masterEncryptCalls)
        assertEquals(0, fake.unboundEncryptCalls)
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
    }

    // ── last-rung degradation: the PERSISTENT false-locked defect ────────

    @Test
    fun shouldDegradeToUnboundAliasWhenFalseLockedRetriesExhaust() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE // never heals — the persistent defect

        storage.storeMnemonic(walletId, mnemonic)

        // Initial attempt + the full 3-retry schedule (250/750/1000ms),
        // then ONE unbound-alias encrypt instead of giving up.
        assertEquals(4, fake.masterEncryptCalls)
        assertEquals(1, fake.unboundEncryptCalls)
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
        // The store really landed, and the read routes to the recorded
        // alias (the fake rejects a blob decrypted under the wrong one).
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(1, fake.unboundDecryptCalls)
        assertEquals(0, fake.masterDecryptCalls)
    }

    @Test
    fun shouldWriteStraightToUnboundAliasOnceDefectIsOnRecord() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(walletId, mnemonic) // demonstrates + records the defect
        val masterEncryptsSoFar = fake.masterEncryptCalls

        storage.storeMnemonic(siblingWalletId, mnemonic)

        // No lock-bound attempt, no retry dance — straight to the alias
        // that works on this device.
        assertEquals(masterEncryptsSoFar, fake.masterEncryptCalls)
        assertEquals(2, fake.unboundEncryptCalls)
        assertEquals(mnemonic, storage.retrieveMnemonic(siblingWalletId))
    }

    @Test
    fun shouldPropagateOriginalDenialWhenDegradationEncryptAlsoFails() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE
        fake.failUnboundEncrypts = Int.MAX_VALUE // even the last rung fails

        var thrown: KeystoreDeviceLockedException? = null
        try {
            storage.storeMnemonic(walletId, mnemonic)
        } catch (e: KeystoreDeviceLockedException) {
            thrown = e
        }

        assertTrue("expected the typed denial to propagate", thrown != null)
        assertFalse(thrown!!.deviceReportsLocked)
        assertEquals(4, fake.masterEncryptCalls)
        assertEquals(1, fake.unboundEncryptCalls)
        // The heal failure rides along for diagnosis...
        assertTrue(thrown.suppressed.any { it is IllegalStateException })
        // ...and nothing was recorded or persisted: the failed heal must
        // not brand the device defective with no healed blob to show.
        assertFalse(storage.isMasterKeyLockBindingDefectObserved())
        assertEquals(null, storage.retrieveMnemonic(walletId))
    }

    @Test
    fun shouldRewrapLockBoundBlobOnFirstSuccessfulReadAfterDefectRecorded() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        // A pre-existing wallet stored healthily under the lock-bound alias...
        storage.storeMnemonic(walletId, mnemonic)
        // ...then a sibling wallet's store demonstrates the persistent defect.
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(siblingWalletId, mnemonic)
        fake.failMasterEncrypts = 0

        // The first successful read of the still-lock-bound blob re-wraps it
        // under the unbound alias (sibling's heal + this re-wrap = 2).
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(1, fake.masterDecryptCalls)
        assertEquals(2, fake.unboundEncryptCalls)

        // Subsequent reads route to the unbound alias — the lock-bound key
        // is no longer consulted.
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(1, fake.masterDecryptCalls)
        assertTrue(fake.unboundDecryptCalls >= 1)
    }

    @Test
    fun shouldKeepLockBoundBlobReadableWhenRewrapFails() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        storage.storeMnemonic(walletId, mnemonic)
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(siblingWalletId, mnemonic) // records the defect
        fake.failMasterEncrypts = 0

        // Re-wrap is best-effort: its failure must not fail the read or
        // corrupt the blob, and the next successful read tries again.
        fake.failUnboundEncrypts = 1
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId)) // retried re-wrap landed
        assertEquals(2, fake.masterDecryptCalls)
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(2, fake.masterDecryptCalls) // now routed to unbound
    }

    // ── retrieveMnemonicUtf8's false-locked ladder ───────────────────────

    @Test
    fun shouldFailFastWhenMnemonicReadIsDeniedOnGenuinelyLockedDevice() {
        runBlocking { storage.storeMnemonic(walletId, mnemonic) }
        fake.masterDecryptCalls = 0
        // Genuinely locked: the denial is CORRECT. Retrying cannot unlock a
        // phone, and the device is not defective — nothing may be recorded.
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrieveMnemonicUtf8(walletId) }
        }
        assertTrue(thrown.deviceReportsLocked)
        assertEquals("decrypt", thrown.operation)
        assertEquals(1, fake.masterDecryptCalls)
        assertFalse(runBlocking { storage.isMasterKeyLockBindingDefectObserved() })
    }

    @Test
    fun shouldRetryFalseLockedMnemonicReadAndSucceedOnSecondAttempt() = runBlocking {
        storage.storeMnemonic(walletId, mnemonic)
        fake.masterDecryptCalls = 0
        // One denial while KeyguardManager reports UNLOCKED — the transient
        // Keystore2 blip the schedule exists to outwait.
        fake.failMasterDecrypts = 1

        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(2, fake.masterDecryptCalls)
        // A retry that SUCCEEDS is a blip, not the persistent defect: the
        // device must not be branded, or every healthy phone that ever
        // blipped would downgrade itself permanently.
        assertFalse(storage.isMasterKeyLockBindingDefectObserved())
    }

    @Test
    fun shouldRecordDefectWhenFalseLockedMnemonicReadRetriesExhaust() = runBlocking {
        storage.storeMnemonic(walletId, mnemonic)
        fake.masterDecryptCalls = 0
        fake.failMasterDecrypts = Int.MAX_VALUE

        // The read itself still fails — a refused decrypt never obtained the
        // plaintext, so unlike storeMnemonic it cannot heal in place.
        assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrieveMnemonicUtf8(walletId) }
        }
        // One initial attempt plus the full DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS
        // schedule (3 delays).
        assertEquals(4, fake.masterDecryptCalls)
        // What it CAN do — and must — is put the device on record.
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
    }

    @Test
    fun shouldRewrapOffTheDefectiveGateAfterOnlyReadsEverObservedIt() = runBlocking {
        // The field shape (MO-972's sibling): a wallet created before the
        // degradation existed. Its blob sits under the lock-bound alias and
        // no mnemonic is EVER written again, so the write ladder never runs
        // and only a read can discover the defect.
        storage.storeMnemonic(walletId, mnemonic)
        fake.masterDecryptCalls = 0
        fake.unboundEncryptCalls = 0

        // Session 1 — the gate is jammed. The read fails, but registers.
        fake.failMasterDecrypts = Int.MAX_VALUE
        assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrieveMnemonicUtf8(walletId) }
        }
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
        // The denied read provisions the device-local witness (one unbound
        // encrypt of a probe byte) but CANNOT re-wrap the blob — it never got
        // the plaintext — so the entry is still under the lock-bound alias.
        assertEquals(1, fake.unboundEncryptCalls)
        assertEquals(0, fake.unboundDecryptCalls)

        // Session 2 — the gate lets a read through (e.g. after a credential
        // unlock). The record armed the re-wrap, which now fires.
        fake.failMasterDecrypts = 0
        fake.unboundEncryptCalls = 0
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(1, fake.unboundEncryptCalls)

        // The lock-bound key is never consulted again, so a future jam
        // cannot strand this wallet.
        val masterDecryptsBefore = fake.masterDecryptCalls
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
        assertEquals(masterDecryptsBefore, fake.masterDecryptCalls)
        assertTrue(fake.unboundDecryptCalls >= 1)
    }

    @Test
    fun shouldIgnoreADefectFlagWithoutItsDeviceLocalKeystoreWitness() = runBlocking {
        // Earn the defect record on "this" device.
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(walletId, mnemonic)
        fake.failMasterEncrypts = 0
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())

        // Now model a restore onto a DIFFERENT handset: Android can carry the
        // DataStore preference, but never the Keystore key. A healthy phone
        // must not inherit the lock-gate downgrade from a portable boolean.
        fake.unboundKeyProvisioned = false

        assertFalse(
            "a flag without its device-local witness must not be believed",
            storage.isMasterKeyLockBindingDefectObserved(),
        )

        // ...and the write path must go back to the lock-bound alias.
        fake.unboundEncryptCalls = 0
        fake.masterEncryptCalls = 0
        storage.storeMnemonic(siblingWalletId, mnemonic)
        assertEquals(1, fake.masterEncryptCalls)
        assertEquals(0, fake.unboundEncryptCalls)
    }

    // ── re-wrap must never outrun a concurrent write ─────────────────────

    /** Put the defect on record without disturbing [walletId]'s own blob. */
    private suspend fun recordDefectViaSiblingWrite() {
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(siblingWalletId, mnemonic)
        fake.failMasterEncrypts = 0
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
    }

    @Test
    fun shouldNotResurrectAMnemonicDeletedDuringTheRewrap() = runBlocking {
        storage.storeMnemonic(walletId, mnemonic)
        recordDefectViaSiblingWrite()

        // retrieveMnemonicUtf8 holds a snapshot taken before the delete.
        // Without a compare-and-set the re-wrap writes that stale ciphertext
        // back and resurrects a seed the user just destroyed.
        fake.onUnboundEncrypt = {
            fake.onUnboundEncrypt = null
            runBlocking { storage.deleteMnemonic(walletId) }
        }

        storage.retrieveMnemonic(walletId) // the read itself still succeeds

        assertFalse("a deleted mnemonic must stay deleted", storage.hasMnemonic(walletId))
    }

    @Test
    fun shouldNotClobberAMnemonicRewrittenDuringTheRewrap() = runBlocking {
        val replacement = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
        storage.storeMnemonic(walletId, mnemonic)
        recordDefectViaSiblingWrite()

        // Same window, but the racing writer stores a NEW phrase. The stale
        // re-wrap must not overwrite it with the one this read decrypted.
        fake.onUnboundEncrypt = {
            fake.onUnboundEncrypt = null
            runBlocking { storage.storeMnemonic(walletId, replacement) }
        }

        storage.retrieveMnemonic(walletId)

        assertEquals(
            "the newer mnemonic must survive the stale re-wrap",
            replacement,
            storage.retrieveMnemonic(walletId),
        )
    }

    @Test
    fun shouldScrubPlaintextWhenCancelledDuringTheRewrap() {
        runBlocking {
            storage.storeMnemonic(walletId, mnemonic)
            recordDefectViaSiblingWrite()
        }
        fake.lastMasterDecryptRef = null

        // Cancellation inside the re-wrap unwinds PAST the return, so the
        // caller never receives the buffer and can never scrub it.
        fake.onUnboundEncrypt = {
            fake.onUnboundEncrypt = null
            throw CancellationException("cancelled mid-re-wrap")
        }

        assertThrows(CancellationException::class.java) {
            runBlocking { storage.retrieveMnemonicUtf8(walletId) }
        }
        assertBufferScrubbed(fake.lastMasterDecryptRef)
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
    fun shouldScrubMnemonicBufferAfterDegradedStore() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE // the degradation path runs

        storage.storeMnemonic(walletId, mnemonic)

        assertBufferScrubbed(fake.lastMasterPlaintextRef)
        // The unbound encrypt saw the same (single) buffer — scrubbed too.
        assertBufferScrubbed(fake.lastUnboundPlaintextRef)
        assertEquals(mnemonic, storage.retrieveMnemonic(walletId))
    }

    @Test
    fun shouldScrubMnemonicBufferWhenFinalDenialPropagates() = runBlocking {
        fake.lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
        fake.failMasterEncrypts = Int.MAX_VALUE
        fake.failUnboundEncrypts = Int.MAX_VALUE // degradation fails too — it propagates

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
 * encrypts succeed regardless of lock state.
 *
 * [KeystoreManager.MASTER_ALIAS_UNBOUND] is modeled per ITS contract: never
 * lock-bound, so never denied by any lock state; [failUnboundEncrypts]
 * scripts unclassified failures for the degradation-also-fails paths. Each
 * blob's iv marks the alias that produced it and [decrypt] rejects a
 * mismatch, so the tests prove reads route to the recorded alias. Identity-
 * key aliases are out of scope here — see [WalletStorageUpgradeMatrixTest]'s
 * fake for that ladder.
 */
private class FalseLockedFakeKeystoreManager : KeystoreManager() {

    var lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)
    var failMasterEncrypts = 0
    var masterEncryptCalls = 0

    /**
     * Scripted device-locked denials for MASTER_ALIAS **decrypts** — the
     * read-side mirror of [failMasterEncrypts]. `Int.MAX_VALUE` models the
     * persistent defect (the gate stays jammed for the whole session);
     * a small count models the transient Keystore2 blip.
     */
    var failMasterDecrypts = 0
    var failUnboundEncrypts = 0
    var unboundEncryptCalls = 0
    var masterDecryptCalls = 0
    var unboundDecryptCalls = 0

    /** Whether the fake master key carries the unlocked-device requirement. */
    var masterKeyLockBound = true

    /**
     * Whether MASTER_ALIAS_UNBOUND exists in this fake's Keystore — the
     * device-local witness. Set by any unbound-alias encrypt (which
     * provisions the key for real), and clearable to model a DataStore
     * restored onto a DIFFERENT device, where the preference survives but
     * the Keystore key cannot.
     */
    var unboundKeyProvisioned = false

    /** The exact buffer reference the last master encrypt received. */
    var lastMasterPlaintextRef: ByteArray? = null

    /** Snapshot of that buffer's content AT CALL TIME (pre-scrub evidence). */
    var lastMasterPlaintextAtCall: ByteArray? = null

    /** The exact buffer reference the last unbound-alias encrypt received. */
    var lastUnboundPlaintextRef: ByteArray? = null

    /** Invoked at each master encrypt attempt (test synchronization hook). */
    var onMasterEncrypt: (() -> Unit)? = null

    /**
     * Invoked at each UNBOUND-alias encrypt, i.e. inside the re-wrap and
     * BEFORE its `store.edit` — the exact window in which a concurrent
     * delete/overwrite must be able to win.
     */
    var onUnboundEncrypt: (() -> Unit)? = null

    /** The buffer the last master decrypt handed back (scrub evidence). */
    var lastMasterDecryptRef: ByteArray? = null

    override fun sampleDeviceLockState(): DeviceLockState = lockState

    override fun hasUnboundMasterKey(): Boolean = unboundKeyProvisioned

    override fun encrypt(plaintext: ByteArray, alias: String): EncryptedBlob = when (alias) {
        MASTER_ALIAS -> {
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
            blob(MASTER_IV_MARKER, plaintext)
        }
        MASTER_ALIAS_UNBOUND -> {
            unboundEncryptCalls++
            lastUnboundPlaintextRef = plaintext
            onUnboundEncrypt?.invoke()
            unboundKeyProvisioned = true
            val scriptedFailure = failUnboundEncrypts > 0
            if (scriptedFailure) failUnboundEncrypts--
            check(!scriptedFailure) { "scripted unbound-alias encrypt failure" }
            blob(UNBOUND_IV_MARKER, plaintext)
        }
        else -> error("test fake only models the master aliases, got '$alias'")
    }

    override fun decrypt(blob: EncryptedBlob, alias: String): ByteArray {
        val expectedMarker = when (alias) {
            MASTER_ALIAS -> {
                masterDecryptCalls++
                val scriptedDenial = failMasterDecrypts > 0
                if (scriptedDenial) failMasterDecrypts--
                if (scriptedDenial || (masterKeyLockBound && lockState.isDeviceLocked)) {
                    throw KeystoreDeviceLockedException(
                        alias = alias,
                        operation = "decrypt",
                        lockState = sampleDeviceLockState(),
                    )
                }
                MASTER_IV_MARKER
            }
            MASTER_ALIAS_UNBOUND -> {
                unboundDecryptCalls++
                UNBOUND_IV_MARKER
            }
            else -> error("test fake only models the master aliases, got '$alias'")
        }
        check(blob.iv.all { it == expectedMarker }) {
            "blob was decrypted under the wrong alias: '$alias' cannot open a blob " +
                "whose iv marker is ${blob.iv.firstOrNull()}"
        }
        return blob.ciphertext.copyOf().also {
            if (alias == MASTER_ALIAS) lastMasterDecryptRef = it
        }
    }

    private fun blob(ivMarker: Byte, plaintext: ByteArray) =
        EncryptedBlob(iv = ByteArray(12) { ivMarker }, ciphertext = plaintext.copyOf())

    private companion object {
        const val MASTER_IV_MARKER: Byte = 7
        const val UNBOUND_IV_MARKER: Byte = 9
    }
}
