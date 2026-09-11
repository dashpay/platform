package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Pins MO-972: identity-key **signing** dying on a device whose Keystore
 * denies lock-bound keys while `KeyguardManager` reports it unlocked.
 *
 * [KeySecurityPolicy.DEVICE_BOUND] exists precisely so a host with its own
 * PIN never meets an authentication gate, and the production wallet adopted
 * it for that reason. But [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND] still
 * carries `setUnlockedDeviceRequired`, and Android reports THAT denial with
 * the very same `UserNotAuthenticatedException` as a closed auth window — so
 * on the defective device the failure came back as "User not authenticated"
 * from a policy with no auth window, one second after a successful biometric.
 *
 * The storage-side contract this fixes, in the order a device meets it:
 *  1. a genuinely-locked denial still fails fast, unretried and unrecorded;
 *  2. a denial that outlasts the bounded false-locked schedule records the
 *     device, and the read still fails truthfully;
 *  3. once recorded, new identity-key writes go to the never-lock-bound
 *     [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND_UNBOUND];
 *  4. the first read that DOES get through re-wraps the stranded blob onto
 *     that alias, after which the defective gate is out of the signing path;
 *  5. throughout, a lock denial is never mistaken for a wrong key — the
 *     recovery ladder must not report an intact key as needing a re-derive.
 *
 * The classification itself (which aliases may map the ambiguous exception)
 * is pinned separately and prompt-free in [KeystoreDeviceLockedDenialTest];
 * the fake here raises the typed exception directly, exactly as a classified
 * [KeystoreManager.decrypt] would.
 */
@RunWith(RobolectricTestRunner::class)
class WalletStorageIdentityKeyLockDefectTest {

    private val pubkeyHex = "02" + "ab".repeat(32)
    private val privateKey = ByteArray(32) { (it + 3).toByte() }
    private val walletId = ByteArray(32) { (it + 11).toByte() }
    private val mnemonic = "abandon abandon abandon abandon abandon abandon " +
        "abandon abandon abandon abandon abandon about"

    private lateinit var fake: DeviceBoundLockDefectFakeKeystore
    private lateinit var storage: WalletStorage

    @Before
    fun setUp() = runBlocking {
        fake = DeviceBoundLockDefectFakeKeystore()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), fake)
        // Shared DataStore file — clear prior state, the defect record included.
        storage.deleteAll()
    }

    /** Drive the master-alias write ladder until the defect is on record. */
    private suspend fun recordDefectViaMnemonicWrite() {
        fake.failMasterEncrypts = Int.MAX_VALUE
        storage.storeMnemonic(walletId, mnemonic)
        fake.failMasterEncrypts = 0
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
    }

    @Test
    fun shouldFailFastWhenIdentityKeyReadIsDeniedOnGenuinelyLockedDevice() {
        runBlocking { storage.storePrivateKey(pubkeyHex, privateKey) }
        fake.deviceBoundDecryptCalls = 0
        fake.lockState = DeviceLockState(isDeviceLocked = true, isKeyguardLocked = true)
        fake.failDeviceBoundDecrypts = Int.MAX_VALUE

        val thrown = assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pubkeyHex) }
        }
        assertTrue(thrown.deviceReportsLocked)
        assertEquals(KeystoreManager.KEYS_ALIAS_DEVICE_BOUND, thrown.alias)
        // Genuinely locked is a CORRECT denial: no retry, and the device is
        // not branded defective.
        assertEquals(1, fake.deviceBoundDecryptCalls)
        assertFalse(runBlocking { storage.isMasterKeyLockBindingDefectObserved() })
    }

    @Test
    fun shouldNotTreatALockDenialAsAWrongKey() = runBlocking {
        storage.storePrivateKey(pubkeyHex, privateKey)
        fake.failDeviceBoundDecrypts = Int.MAX_VALUE

        // KeystoreDeviceLockedException IS a GeneralSecurityException, so
        // without an explicit clause it would fall into the recovery ladder
        // and surface as `null` — which the signer and key-health paths read
        // as "this key is stranded, re-derive it". The key is intact; only
        // the gate is shut.
        assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pubkeyHex) }
        }
        // Same reasoning for the health probe: an intact key behind a shut
        // gate must not be offered for repair.
        assertTrue(storage.probeIdentityKeyRecoverability(pubkeyHex))
    }

    @Test
    fun shouldRetryFalseLockedIdentityReadAndSucceedWithoutBrandingTheDevice() = runBlocking {
        storage.storePrivateKey(pubkeyHex, privateKey)
        fake.deviceBoundDecryptCalls = 0
        fake.failDeviceBoundDecrypts = 1 // one transient blip

        assertArrayEquals(privateKey, storage.retrievePrivateKey(pubkeyHex))
        assertEquals(2, fake.deviceBoundDecryptCalls)
        assertFalse(storage.isMasterKeyLockBindingDefectObserved())
        assertEquals(0, fake.unboundEncryptCalls)
    }

    @Test
    fun shouldRecordDefectWhenFalseLockedIdentityReadRetriesExhaust() = runBlocking {
        storage.storePrivateKey(pubkeyHex, privateKey)
        fake.deviceBoundDecryptCalls = 0
        fake.failDeviceBoundDecrypts = Int.MAX_VALUE

        assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pubkeyHex) }
        }
        // One attempt plus the full DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS
        // schedule (3 delays).
        assertEquals(4, fake.deviceBoundDecryptCalls)
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
    }

    @Test
    fun shouldWriteNewIdentityKeysToTheUnboundAliasOnceDefectIsOnRecord() = runBlocking {
        recordDefectViaMnemonicWrite()
        fake.unboundEncryptCalls = 0
        fake.deviceBoundEncryptCalls = 0

        storage.storePrivateKey(pubkeyHex, privateKey)

        // The lock-bound alias is not consulted at all, so this key can never
        // be taken hostage by the defective gate.
        assertEquals(1, fake.unboundEncryptCalls)
        assertEquals(0, fake.deviceBoundEncryptCalls)
        assertArrayEquals(privateKey, storage.retrievePrivateKey(pubkeyHex))
        assertEquals(0, fake.deviceBoundDecryptCalls)
    }

    @Test
    fun shouldRewrapStrandedIdentityKeyOffTheDefectiveGateOnFirstReadThatSucceeds() = runBlocking {
        // The MO-972 field shape: the key was stored BEFORE the device's
        // defect was known, so it sits under the lock-bound alias.
        storage.storePrivateKey(pubkeyHex, privateKey)
        assertEquals(1, fake.deviceBoundEncryptCalls)
        fake.unboundEncryptCalls = 0

        // Session 1 — the gate is jammed. Signing fails, but registers the
        // device. Only a read can discover this: the key is already stored,
        // so nothing writes an identity key again.
        fake.failDeviceBoundDecrypts = Int.MAX_VALUE
        assertThrows(KeystoreDeviceLockedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pubkeyHex) }
        }
        assertTrue(storage.isMasterKeyLockBindingDefectObserved())
        assertEquals(0, fake.unboundEncryptCalls)

        // Session 2 — the gate lets a read through. The record armed the
        // re-wrap, which moves the blob to the never-lock-bound alias.
        fake.failDeviceBoundDecrypts = 0
        assertArrayEquals(privateKey, storage.retrievePrivateKey(pubkeyHex))
        assertEquals(1, fake.unboundEncryptCalls)

        // From here signing never touches the defective gate again — even
        // while it is jammed solid.
        fake.deviceBoundDecryptCalls = 0
        fake.failDeviceBoundDecrypts = Int.MAX_VALUE
        assertArrayEquals(privateKey, storage.retrievePrivateKey(pubkeyHex))
        assertEquals(0, fake.deviceBoundDecryptCalls)
        assertTrue(fake.unboundDecryptCalls >= 1)
    }

    @Test
    fun shouldScrubIdentityKeyPlaintextWhenCancelledDuringMigration() {
        runBlocking {
            storage.storePrivateKey(pubkeyHex, privateKey)
            recordDefectViaMnemonicWrite()
        }
        fake.lastIdentityDecryptRef = null

        // migrateToPolicyAlias rethrows CancellationException by design, so a
        // cancellation inside it unwinds PAST retrievePrivateKey's return and
        // the owner never gets the buffer to scrub. Covers the legacy
        // migration and recovery ladder too — they share the helper.
        fake.onUnboundIdentityEncrypt = {
            fake.onUnboundIdentityEncrypt = null
            throw kotlin.coroutines.cancellation.CancellationException("cancelled mid-migration")
        }

        assertThrows(kotlin.coroutines.cancellation.CancellationException::class.java) {
            runBlocking { storage.retrievePrivateKey(pubkeyHex) }
        }
        val buf = fake.lastIdentityDecryptRef
        assertNotNull("expected the identity-key plaintext to have been captured", buf)
        assertTrue(
            "decrypted identity key must be zeroed when it cannot be returned",
            buf!!.all { it == 0.toByte() },
        )
    }

    @Test
    fun shouldKeepStrandedKeyIntactWhenRewrapFails() = runBlocking {
        storage.storePrivateKey(pubkeyHex, privateKey)
        recordDefectViaMnemonicWrite()
        fake.unboundEncryptCalls = 0

        // Re-wrap is best-effort: its failure must neither fail the read nor
        // corrupt the blob, and the next read tries again.
        fake.failUnboundEncrypts = 1
        assertArrayEquals(privateKey, storage.retrievePrivateKey(pubkeyHex))
        assertNotNull(storage.retrievePrivateKey(pubkeyHex)) // retried re-wrap lands

        fake.deviceBoundDecryptCalls = 0
        assertArrayEquals(privateKey, storage.retrievePrivateKey(pubkeyHex))
        assertEquals(0, fake.deviceBoundDecryptCalls) // now on the unbound alias
    }
}

/**
 * Fake [KeystoreManager] for the DEVICE_BOUND identity aliases plus the
 * master alias (needed only to drive the write ladder that records the
 * defect).
 *
 * [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND] is modeled per its real
 * contract — lock-bound, so deniable — and raises the TYPED
 * [KeystoreDeviceLockedException] the production `decrypt` now produces for
 * it. [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND_UNBOUND] is modeled per ITS
 * contract: never lock-bound, so never denied by any lock state. Each blob's
 * leading byte marks the alias that produced it and [decrypt] rejects a
 * mismatch, so the tests prove reads route to the recorded alias.
 */
private class DeviceBoundLockDefectFakeKeystore :
    KeystoreManager(KeySecurityPolicy.DEVICE_BOUND) {

    var lockState = DeviceLockState(isDeviceLocked = false, isKeyguardLocked = false)

    /** Scripted device-locked denials for the LOCK-BOUND identity alias. */
    var failDeviceBoundDecrypts = 0

    /** Scripted device-locked denials for the master-alias encrypt. */
    var failMasterEncrypts = 0

    /** Scripted unclassified failures of the unbound-alias encrypt. */
    var failUnboundEncrypts = 0

    var deviceBoundDecryptCalls = 0
    var unboundDecryptCalls = 0
    var deviceBoundEncryptCalls = 0
    var unboundEncryptCalls = 0

    override fun sampleDeviceLockState(): DeviceLockState = lockState

    /** The device-local witness: provisioned by any unbound-alias encrypt. */
    var unboundMasterKeyProvisioned = false

    /**
     * Invoked inside the UNBOUND identity-alias encrypt — i.e. inside
     * `migrateToPolicyAlias`, after the plaintext is in hand and before the
     * caller can return it.
     */
    var onUnboundIdentityEncrypt: (() -> Unit)? = null

    /** The buffer the last identity decrypt handed back (scrub evidence). */
    var lastIdentityDecryptRef: ByteArray? = null

    override fun hasUnboundMasterKey(): Boolean = unboundMasterKeyProvisioned

    override fun effectiveKeySecurityPolicy(): KeySecurityPolicy = keySecurityPolicy

    override fun hasIdentityKeysKey(alias: String): Boolean = isIdentityKeysAlias(alias)

    override fun keysAliasFingerprintOrNull(alias: String): String? =
        if (isIdentityKeysAlias(alias)) fpOf(alias) else null

    override fun keysAliasFingerprint(alias: String): String = fpOf(alias)

    override fun hasLegacyKeysKey(): Boolean = false

    override fun hasLegacyRsaKeysKey(): Boolean = false

    override fun decryptLegacyKeysBlob(blob: EncryptedBlob): ByteArray? = null

    override fun decryptLegacyRsaKeysBlob(blob: EncryptedBlob): ByteArray? = null

    override fun opensUnderNonGatedDeviceBoundSibling(blob: EncryptedBlob): Boolean = false

    override fun encryptForIdentityKeys(plaintext: ByteArray): KeysAliasEncryptedBlob =
        encryptForIdentityKeysAlias(KEYS_ALIAS_DEVICE_BOUND, plaintext)

    override fun encryptForIdentityKeysAlias(
        alias: String,
        plaintext: ByteArray,
    ): KeysAliasEncryptedBlob {
        when (alias) {
            KEYS_ALIAS_DEVICE_BOUND -> deviceBoundEncryptCalls++
            KEYS_ALIAS_DEVICE_BOUND_UNBOUND -> {
                unboundEncryptCalls++
                onUnboundIdentityEncrypt?.invoke()
                val scripted = failUnboundEncrypts > 0
                if (scripted) failUnboundEncrypts--
                check(!scripted) { "scripted unbound-alias encrypt failure" }
            }
            else -> error("fake models only the DEVICE_BOUND identity aliases, got '$alias'")
        }
        return KeysAliasEncryptedBlob(rsaBlob(alias, plaintext), fpOf(alias), alias)
    }

    override fun encrypt(plaintext: ByteArray, alias: String): EncryptedBlob = when (alias) {
        MASTER_ALIAS -> {
            val scripted = failMasterEncrypts > 0
            if (scripted) failMasterEncrypts--
            if (scripted || lockState.isDeviceLocked) {
                throw KeystoreDeviceLockedException(
                    alias = alias,
                    operation = "encrypt",
                    lockState = sampleDeviceLockState(),
                )
            }
            EncryptedBlob(iv = ByteArray(12) { 9 }, ciphertext = plaintext.copyOf())
        }
        MASTER_ALIAS_UNBOUND -> {
            unboundMasterKeyProvisioned = true
            EncryptedBlob(iv = ByteArray(12) { 8 }, ciphertext = plaintext.copyOf())
        }
        else -> error("fake models only the master aliases for AES, got '$alias'")
    }

    override fun decrypt(blob: EncryptedBlob, alias: String): ByteArray {
        when (alias) {
            KEYS_ALIAS_DEVICE_BOUND -> {
                deviceBoundDecryptCalls++
                val scripted = failDeviceBoundDecrypts > 0
                if (scripted) failDeviceBoundDecrypts--
                if (scripted || lockState.isDeviceLocked) {
                    // What the production decrypt now raises for this alias:
                    // it is lock-bound and NOT auth-gated, so Keystore's
                    // "user not authenticated" can only be the lock gate.
                    throw KeystoreDeviceLockedException(
                        alias = alias,
                        operation = "decrypt",
                        lockState = sampleDeviceLockState(),
                    )
                }
            }
            KEYS_ALIAS_DEVICE_BOUND_UNBOUND -> unboundDecryptCalls++
            MASTER_ALIAS, MASTER_ALIAS_UNBOUND -> return blob.ciphertext.copyOf()
            else -> error("fake cannot decrypt under '$alias'")
        }
        check(blob.ciphertext[0] == aliasTag(alias)) {
            "blob was decrypted under the wrong alias: '$alias' cannot open a blob " +
                "produced by tag ${blob.ciphertext[0]}"
        }
        val len = blob.ciphertext[1].toInt() and 0xFF
        return blob.ciphertext.copyOfRange(2, 2 + len).also { lastIdentityDecryptRef = it }
    }

    private fun fpOf(alias: String): String = "fake-fp-$alias"

    private fun aliasTag(alias: String): Byte =
        if (alias == KEYS_ALIAS_DEVICE_BOUND) 1 else 2

    private fun rsaBlob(alias: String, plain: ByteArray): EncryptedBlob {
        val ct = ByteArray(RSA_BLOB_BYTES)
        ct[0] = aliasTag(alias)
        ct[1] = plain.size.toByte()
        plain.copyInto(ct, 2)
        return EncryptedBlob(iv = ByteArray(0), ciphertext = ct)
    }

    private companion object {
        const val RSA_BLOB_BYTES = 2048 / 8
    }
}
