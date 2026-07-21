package org.dashfoundation.dashsdk.security

import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.UserNotAuthenticatedException
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import javax.crypto.BadPaddingException

/**
 * Full upgrade-matrix coverage for [WalletStorage]'s layered identity-key
 * retrieve ladder (dashpay/platform#4060). Exercises every stored-blob shape
 * an upgraded install can hold — legacy AES-GCM, pre-alias-split RSA under
 * [KeystoreManager.KEYS_ALIAS], current policy-alias RSA, and
 * DEVICE_BOUND-tagged degradation blobs — plus the stranded, invalidated-key,
 * and auth-propagation cases, and the cheap-vs-probing capability split.
 *
 * The real AndroidKeyStore crypto cannot run on the JVM (no Robolectric
 * provider — see [KeySecurityPolicyTest]), so a [FakeKeystoreManager]
 * substitutes deterministic in-memory "crypto" through the `open` seams on
 * [KeystoreManager]. It models the load-bearing invariants the production code
 * relies on: an empty-IV blob only decrypts under the alias that produced it,
 * [KeystoreManager.KEYS_ALIAS] holds at most one former key (AES XOR RSA), a
 * write-time encrypt captures blob + fingerprint + producing alias in one
 * lookup, and an invalidated policy key runs the generation-checked cleanup
 * then rethrows. This pins the ROUTING and migration behavior; the concrete
 * keystore crypto is out of unit-test reach by construction.
 */
@RunWith(RobolectricTestRunner::class)
class WalletStorageUpgradeMatrixTest {

    private val pub = "02aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"
    private val secret = ByteArray(32) { (it + 1).toByte() }

    private lateinit var fake: FakeKeystoreManager
    private lateinit var storage: WalletStorage

    @Before
    fun setUp() = runBlocking {
        fake = FakeKeystoreManager()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), fake)
        // Isolate from any state a prior test left in the shared DataStore file.
        storage.deleteAll()
    }

    // ── Blob-shape / routing matrix ──────────────────────────────────────

    /** New-alias (current-scheme) blob: fingerprint fast path, no recovery rung. */
    @Test
    fun currentPolicyAliasBlobDecryptsDirectly() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)

        assertTrue(storage.isPrivateKeyDecryptable(pub))
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        // No fallback path was taken.
        assertEquals(0, fake.legacyRsaFallbackCalls)
    }

    /** Legacy AES-GCM blob: recovered via the retained AES key and migrated forward. */
    @Test
    fun legacyAesBlobIsRecoveredAndMigrated() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.AES
        fake.scheme = FakeKeystoreManager.Scheme.LEGACY_AES
        storage.storePrivateKey(pub, secret) // writes a non-empty-IV AES blob

        // The cheap capability check sees the retained AES key (presence only).
        assertTrue(storage.isPrivateKeyDecryptable(pub))
        // So does the probing health check (real decrypt).
        assertTrue(storage.probeIdentityKeyRecoverability(pub))

        // Migration re-encrypts under the current alias.
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))

        // The stored blob is now a current-alias blob: a second read no longer
        // touches the legacy AES path.
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.NONE
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
    }

    /** Legacy AES key already deleted by an older build → stranded, reported undecryptable. */
    @Test
    fun legacyAesBlobWithDeletedKeyIsStranded() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.AES
        fake.scheme = FakeKeystoreManager.Scheme.LEGACY_AES
        storage.storePrivateKey(pub, secret)

        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.NONE // key gone
        assertFalse(storage.isPrivateKeyDecryptable(pub))
        assertFalse(storage.probeIdentityKeyRecoverability(pub))
        // decryptLegacyKeysBlob returns null → retrieve yields null, not a wrong value.
        assertNull(storage.retrievePrivateKey(pub))
    }

    /**
     * Pre-alias-split RSA blob (the former key's fingerprint never matches the
     * policy alias): the fingerprint gate ROUTES to the former-RSA recovery
     * ladder — not an immediate null — and the value is migrated forward.
     */
    @Test
    fun formerRsaBlobRoutesThroughRecoveryLadderAndMigrates() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        storage.storePrivateKey(pub, secret) // former-RSA blob; policy alias not provisioned

        assertTrue(storage.isPrivateKeyDecryptable(pub)) // recoverable via former RSA key
        assertTrue(storage.probeIdentityKeyRecoverability(pub))

        fake.scheme = FakeKeystoreManager.Scheme.CURRENT // migration writes a policy blob
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertTrue(fake.legacyRsaFallbackCalls > 0)

        // Migrated: the next read decrypts straight under the (now provisioned)
        // policy alias without another former-RSA fallback.
        val before = fake.legacyRsaFallbackCalls
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertEquals(0, fake.legacyRsaFallbackCalls - before)
    }

    /**
     * Mixed window: the policy alias already holds a key while a former-RSA
     * blob lingers. The fingerprint mismatch routes STRAIGHT to the recovery
     * ladder — the policy alias's key is never even tried against a blob it
     * cannot own (no OAEP attempt with an unrelated key on a read path).
     */
    @Test
    fun formerRsaBlobWithProvisionedPolicySkipsThePolicyKeyEntirely() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        storage.storePrivateKey(pub, secret)

        // Simulate a sibling key already provisioned at the policy alias.
        fake.policyKeyProvisioned = true
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT

        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertTrue(fake.legacyRsaFallbackCalls > 0) // reached the recovery ladder
        assertEquals(0, fake.policyDecryptCalls) // wrong key never attempted
    }

    /**
     * Former-RSA blob whose KEYS_ALIAS key is gone → stranded. No present key can
     * open it, so recovery yields null (a re-derive signal, like the legacy-AES
     * stranded path) rather than a bogus plaintext — and, critically, rather than
     * an uncaught crypto exception into KeystoreSigner (dashpay/platform#4060).
     */
    @Test
    fun formerRsaBlobWithDeletedKeyIsStranded() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        storage.storePrivateKey(pub, secret)

        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.NONE // former RSA key gone
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        assertFalse(storage.isPrivateKeyDecryptable(pub))
        assertFalse(storage.probeIdentityKeyRecoverability(pub))
        // Unrecoverable → null (not a wrong value, not a thrown BadPaddingException).
        assertNull(storage.retrievePrivateKey(pub))
    }

    /**
     * Regression (dashpay/platform#4060): an un-tagged blob written by the
     * DEVICE_BOUND sibling while an UNRELATED former RSA key still lingers at
     * KEYS_ALIAS. The recovery ladder tries the former key on presence — but
     * that key did not write this blob, so decryptLegacyRsaKeysBlob raises
     * BadPaddingException. That wrong-key failure must be absorbed ("not this
     * key") and reported as unrecoverable (null) so the host can re-derive,
     * NOT escape uncaught into KeystoreSigner (which only catches
     * UserNotAuthenticatedException).
     */
    @Test
    fun siblingAliasBlobUnderUnprovisionedPolicyDoesNotThrow() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA // unrelated former RSA key present
        fake.scheme = FakeKeystoreManager.Scheme.SIBLING_POLICY
        storage.storePrivateKey(pub, secret) // sibling-written blob, tag says policy alias

        // The former-RSA recovery is attempted (presence-based) but cannot open
        // the blob; the wrong-key failure is absorbed and surfaces as null.
        assertNull(storage.retrievePrivateKey(pub))
        assertTrue(fake.legacyRsaFallbackCalls > 0) // recovery was tried, not skipped

        // And the key-health PROBE must NOT report the stranded blob
        // recoverable (finding e17e265dc680): the former RSA key is present but
        // does not open this sibling-alias blob, so probing it yields
        // BadPadding -> false, which lets WalletKeyHealthSheet offer the
        // re-derive/repair path. Before the fix this returned true on bare key
        // presence.
        assertFalse(storage.probeIdentityKeyRecoverability(pub))
    }

    /**
     * Key-health probes actual decryptability, not presence — but an auth-gated
     * key with a closed window is RECOVERABLE, not stranded (finding e17e265dc680).
     * A provisioned policy-alias blob whose decrypt throws
     * UserNotAuthenticatedException (window closed) must report recoverable: the
     * key is present and the value recovers once the user authenticates, and the
     * probe must not prompt. (Contrast siblingAlias above, where the key is present
     * but genuinely wrong -> BadPadding -> not recoverable.)
     */
    @Test
    fun authGatedPolicyKeyReportsRecoverableWithoutPrompting() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret) // provisions the policy alias

        fake.throwAuthOnPolicyDecrypt = true // closed auth window
        assertTrue(storage.probeIdentityKeyRecoverability(pub))
        // The cheap check agrees via the fingerprint match (no decrypt at all).
        assertTrue(storage.isPrivateKeyDecryptable(pub))
    }

    /**
     * #4060 round-2 finding: a REPLACED auth-gated alias (Keystore loss +
     * regeneration) whose window is closed — the normal state, the window is
     * only ~30 s — throws UserNotAuthenticatedException at cipher.init,
     * before the ciphertext is touched. Without the fingerprint disproof the
     * probe reported the unopenable blob "healthy" (no repair offered) while
     * pendingIdentityKeys listed the same key. The stored fingerprint no
     * longer matches the replacement key, which disproves ownership
     * prompt-free → not recoverable → the sheet offers repair.
     */
    @Test
    fun replacedAliasBlobWithClosedAuthWindowIsNotReportedRecoverable() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)
        assertTrue(storage.probeIdentityKeyRecoverability(pub))

        // Keystore loss + regeneration: fresh keypair at the policy alias…
        fake.policyFingerprintSuffix = "-replacement"
        // …auth-gated, so its window is closed in the steady state.
        fake.throwAuthOnPolicyDecrypt = true

        assertFalse(storage.probeIdentityKeyRecoverability(pub))
        // The cheap capability surface agrees (fingerprint mismatch).
        assertFalse(storage.isPrivateKeyDecryptable(pub))
    }

    /**
     * The complement guard: a fingerprint-MATCHED blob behind a closed auth
     * window stays recoverable — it genuinely just needs authentication, and
     * the probe must not prompt (also pinned by
     * [authGatedPolicyKeyReportsRecoverableWithoutPrompting]).
     */
    @Test
    fun fingerprintMatchedBlobWithClosedAuthWindowStaysRecoverable() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)

        fake.throwAuthOnPolicyDecrypt = true // same key, window merely closed

        assertTrue(storage.probeIdentityKeyRecoverability(pub))
    }

    /**
     * The same auth-gated semantics on the former-RSA recovery path: a present but
     * auth-gated KEYS_ALIAS RSA key that would open the blob after auth reports
     * recoverable when the window is closed (UserNotAuth), rather than stranded.
     */
    @Test
    fun authGatedFormerRsaKeyReportsRecoverable() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        storage.storePrivateKey(pub, secret) // former-RSA blob; policy alias unprovisioned

        fake.throwAuthOnLegacyRsaDecrypt = true // closed window on the former RSA key
        assertTrue(storage.probeIdentityKeyRecoverability(pub))
    }

    /**
     * Regression (dashpay/platform#4060, finding b80a15c93339): the
     * "provisioned + locked + WRONG alias" case. The current AUTH_GATED policy
     * alias is provisioned from an earlier period and its auth window is closed,
     * but the (un-tagged) blob was actually written by the DEVICE_BOUND sibling.
     * The locked policy alias throws UserNotAuthenticatedException at
     * cipher.init — before the ciphertext is examined — so a bare catch would
     * mis-report it "recoverable". The prompt-free DEVICE_BOUND sibling opens
     * the blob, proving the policy alias does not own it; since
     * retrievePrivateKey never falls back to the sibling for an un-tagged blob,
     * it is genuinely strandable and key-health must report it unrecoverable so
     * the repair path fires.
     */
    @Test
    fun provisionedLockedWrongAliasBlobDisprovedByPromptFreeSibling() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.SIBLING_POLICY
        storage.storePrivateKey(pub, secret) // sibling-written blob, tag says policy alias

        fake.policyKeyProvisioned = true // AUTH_GATED alias provisioned earlier...
        fake.throwAuthOnPolicyDecrypt = true // ...and its auth window is closed
        fake.deviceBoundKeyPresent = true // DEVICE_BOUND actually wrote it, key present

        assertFalse(storage.probeIdentityKeyRecoverability(pub))
    }

    /**
     * The disproof must NOT fire when the sibling can't open the blob: a locked
     * auth-gated policy alias that legitimately owns its blob still reports
     * recoverable even with an unrelated DEVICE_BOUND key present (which raises
     * BadPadding on this policy-written blob, so it proves nothing). Guards
     * against the b80a15c93339 fix regressing the legitimate locked-owner case.
     */
    @Test
    fun lockedPolicyOwnerNotDisprovedWhenSiblingCannotOpen() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret) // policy-alias-owned blob (TAG_POLICY)

        fake.throwAuthOnPolicyDecrypt = true // locked window
        fake.deviceBoundKeyPresent = true // sibling present but cannot open a TAG_POLICY blob

        assertTrue(storage.probeIdentityKeyRecoverability(pub))
    }

    /**
     * Alias-tag routing after a lockless AUTH_GATED→DEVICE_BOUND write
     * degradation (dashpay/platform#4060 finding 4): the blob was written —
     * and TAGGED — under the DEVICE_BOUND alias while the device had no lock
     * screen. After a lock screen is enrolled (and the auth-gated alias is
     * provisioned for NEW writes), reads must still decrypt this blob under
     * its RECORDED alias, and neither capability surface may misreport it.
     */
    @Test
    fun aliasTagRoutesReadsAfterLocklessDegradation() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        fake.degradeWritesToDeviceBound = true
        storage.storePrivateKey(pub, secret) // written + tagged devicebound

        // Lock enrolled later; gated alias provisioned for new writes.
        fake.degradeWritesToDeviceBound = false
        fake.policyKeyProvisioned = true

        assertTrue(storage.isPrivateKeyDecryptable(pub))
        assertTrue(storage.probeIdentityKeyRecoverability(pub))
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        assertEquals(0, fake.legacyRsaFallbackCalls) // routed by tag, not recovery
    }

    /**
     * Pins finding 3 (the cheap/probing split): the signer-facing capability
     * check must NEVER decrypt — it runs under runBlocking on the Rust
     * callback thread. Presence and fingerprint reads only, across every blob
     * shape.
     */
    @Test
    fun cheapCapabilityCheckNeverDecrypts() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)
        assertTrue(storage.isPrivateKeyDecryptable(pub))

        val former = "02" + "11".repeat(32)
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        storage.storePrivateKey(former, secret)
        assertTrue(storage.isPrivateKeyDecryptable(former))

        val legacy = "02" + "22".repeat(32)
        fake.scheme = FakeKeystoreManager.Scheme.LEGACY_AES
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.AES
        storage.storePrivateKey(legacy, secret)
        assertTrue(storage.isPrivateKeyDecryptable(legacy))

        assertEquals(0, fake.policyDecryptCalls)
        assertEquals(0, fake.deviceBoundDecryptCalls)
        assertEquals(0, fake.legacyAesDecryptCalls)
        assertEquals(0, fake.legacyRsaFallbackCalls)
    }

    /**
     * #4172's invalidation recovery survives the alias split (finding 1): a
     * KeyPermanentlyInvalidatedException at the policy alias runs the
     * generation-checked alias deletion and RETHROWS the typed signal (never
     * swallowed into the recovery ladder — the signer must suppress its
     * biometric retry). After the deletion the stored fingerprint can no
     * longer match, so the capability check reports the blob non-current and
     * the repair path re-derives instead of trusting a stale certificate
     * (the brick-loop fix).
     */
    @Test
    fun invalidatedPolicyKeyRunsGenerationCheckedCleanupAndRethrows() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)
        assertTrue(storage.isPrivateKeyDecryptable(pub))

        fake.throwInvalidatedOnPolicyDecrypt = true
        assertThrows(KeyPermanentlyInvalidatedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pub) }
        }
        assertTrue("cleanup must have deleted the invalidated generation", fake.invalidatedCleanupRan)
        assertEquals(0, fake.legacyRsaFallbackCalls) // typed signal, not recovery

        // Alias gone → fingerprint unreadable → blob non-current → repair path.
        fake.throwInvalidatedOnPolicyDecrypt = false
        assertFalse(storage.isPrivateKeyDecryptable(pub))
        assertNull(storage.retrievePrivateKey(pub))
    }

    /**
     * Defense in depth on the fast path: a fingerprint-matched blob whose
     * decrypt still fails with an unexpected crypto error (rotation race /
     * provider quirk) falls to the recovery ladder — and, with no former key
     * present, surfaces as null rather than an escaped exception.
     */
    @Test
    fun fastPathCryptoFailureFallsToRecoveryLadder() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret)

        fake.throwBadPaddingOnPolicyDecrypt = true
        assertNull(storage.retrievePrivateKey(pub))
        assertTrue(fake.legacyRsaFallbackCalls > 0) // ladder was consulted
    }

    /**
     * Regression (dashpay/platform#4060, finding 1049be675782): the legacy
     * migration must not resurrect a private key a concurrent wallet deletion
     * just removed. [WalletStorage.retrievePrivateKey] reads and recovers the
     * former-RSA blob WITHOUT holding the private-key mutex; a removeWallet
     * sweep can win `withPrivateKeyExclusion` between that read and
     * `migrateToPolicyAlias`'s rewrite, delete the alias plus its owner-index
     * entry, and cascade the Room rows. The rewrite must then be SKIPPED — an
     * unconditional edit recreated `privkey.<pubkeyHex>` as undiscoverable
     * ciphertext (no owner index, no database row) behind a "successful" wipe.
     */
    @Test
    fun migrationDoesNotResurrectAKeyDeletedMidRecovery() = runBlocking {
        fake.keysAliasKind = FakeKeystoreManager.KeysAliasKind.RSA
        fake.scheme = FakeKeystoreManager.Scheme.FORMER_RSA
        val owner = ByteArray(32) { 4 }
        storage.storePrivateKey(pub, secret, ownerWalletId = owner)

        // The migration's policy-alias re-encrypt is the window between the
        // caller's read and the conditional rewrite: model the concurrent
        // deletion sweep winning it.
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        fake.onNextPolicyEncrypt = {
            runBlocking {
                storage.withPrivateKeyExclusion {
                    deletePrivateKeys(listOf(pub))
                    deleteOwnerIndex(owner)
                }
            }
        }

        // The caller still gets the value it legitimately recovered…
        assertArrayEquals(secret, storage.retrievePrivateKey(pub))
        // …but the swept entry and owner index must NOT be re-created.
        assertFalse(storage.hasPrivateKey(pub))
        assertTrue(storage.ownedPrivateKeyAliases(owner).isEmpty())
        assertNull(storage.retrievePrivateKey(pub))
    }

    /**
     * Finding 6 (forced replacement): a blob that passes the shape +
     * fingerprint usability check but does not actually decrypt must NOT
     * short-circuit the repair path. replacePrivateKey overwrites the entry
     * unconditionally (blob + fingerprint + alias tag in one edit), while
     * storeIfAbsent — the idempotent persist path — keeps its short-circuit
     * and would have skipped the re-derive.
     */
    @Test
    fun forcedReplaceOverwritesAFingerprintValidButUndecryptableBlob() = runBlocking {
        val owner = ByteArray(32) { 6 }
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret, ownerWalletId = owner)

        // The blob still LOOKS current (shape + fingerprint) but the key no
        // longer opens it — the stale-but-matching corner.
        fake.throwBadPaddingOnPolicyDecrypt = true
        assertTrue(storage.isPrivateKeyDecryptable(pub)) // cheap check fooled
        assertFalse(storage.probeIdentityKeyRecoverability(pub)) // probe not fooled

        // storeIfAbsent short-circuits on the fingerprint match — no derive.
        var storeIfAbsentDerives = 0
        val skipped = storage.storeIfAbsent(pub, ownerWalletId = owner) {
            storeIfAbsentDerives++
            secret
        }
        assertFalse(skipped)
        assertEquals(0, storeIfAbsentDerives)

        // replacePrivateKey derives unconditionally and replaces the entry.
        val replacement = ByteArray(32) { (it + 100).toByte() }
        var replaceDerives = 0
        storage.replacePrivateKey(pub, ownerWalletId = owner) {
            replaceDerives++
            replacement
        }
        assertEquals(1, replaceDerives)

        fake.throwBadPaddingOnPolicyDecrypt = false
        assertArrayEquals(replacement, storage.retrievePrivateKey(pub))
        assertTrue(storage.probeIdentityKeyRecoverability(pub))
        assertTrue(storage.ownedPrivateKeyAliases(owner).contains(pub))
    }

    /** replacePrivateKey honors wallet tombstones like every other store. */
    @Test
    fun forcedReplaceRejectsATombstonedWallet() {
        runBlocking {
            val owner = ByteArray(32) { 9 }
            storage.withPrivateKeyExclusion { tombstoneWallet(owner) }
            org.junit.Assert.assertThrows(WalletTombstonedException::class.java) {
                runBlocking {
                    storage.replacePrivateKey(pub, ownerWalletId = owner) { secret }
                }
            }
            assertFalse(storage.hasPrivateKey(pub))
        }
    }

    /**
     * A closed auth window must NOT be mistaken for a wrong key: the
     * UserNotAuthenticatedException propagates so KeystoreSigner can prompt,
     * instead of being swallowed into the former-RSA fallback.
     */
    @Test
    fun authFailureOnPolicyDecryptPropagates() = runBlocking {
        fake.scheme = FakeKeystoreManager.Scheme.CURRENT
        storage.storePrivateKey(pub, secret) // provisions the policy alias

        fake.throwAuthOnPolicyDecrypt = true
        assertThrows(UserNotAuthenticatedException::class.java) {
            runBlocking { storage.retrievePrivateKey(pub) }
        }
        assertEquals(0, fake.legacyRsaFallbackCalls) // never fell through to the fallback
    }
}

/**
 * Deterministic in-memory stand-in for [KeystoreManager] used only by
 * [WalletStorageUpgradeMatrixTest]. RSA-shaped blobs are 256-byte, empty-IV
 * ciphertexts tagged with the producing key; legacy AES blobs carry a
 * non-empty IV. Only the key that produced an RSA blob can decrypt it — every
 * other combination raises [BadPaddingException], mirroring the JCE contract the
 * production routing depends on. The pure structural predicates
 * ([KeystoreManager.isLegacyKeysBlob], [KeystoreManager.isKeysBlobDecryptable])
 * are inherited unchanged.
 *
 * Fingerprint model: each present key has the deterministic fingerprint
 * `fake-fp-<alias>`; the write-time capture ([encryptForIdentityKeys])
 * returns the fingerprint of the key that actually produced the blob, so the
 * production fingerprint gate routes exactly as it would against real keys
 * (a FORMER_RSA blob carries the former key's fingerprint, which never
 * matches a policy alias).
 */
private class FakeKeystoreManager :
    KeystoreManager(KeySecurityPolicy.AUTH_GATED) {

    enum class Scheme { CURRENT, FORMER_RSA, LEGACY_AES, SIBLING_POLICY }

    enum class KeysAliasKind { NONE, AES, RSA }

    var scheme: Scheme = Scheme.CURRENT
    var keysAliasKind: KeysAliasKind = KeysAliasKind.NONE
    var policyKeyProvisioned: Boolean = false
    var deviceBoundKeyPresent: Boolean = false
    var degradeWritesToDeviceBound: Boolean = false
    var throwAuthOnPolicyDecrypt: Boolean = false

    /**
     * Models a Keystore-loss + regeneration of the policy alias: the CURRENT
     * key's fingerprint diverges from every previously captured one while
     * the alias stays present (and, being auth-gated, usually locked).
     */
    var policyFingerprintSuffix: String = ""
    var throwAuthOnLegacyRsaDecrypt: Boolean = false
    var throwInvalidatedOnPolicyDecrypt: Boolean = false
    var throwBadPaddingOnPolicyDecrypt: Boolean = false
    var invalidatedCleanupRan: Boolean = false

    var policyDecryptCalls: Int = 0
    var deviceBoundDecryptCalls: Int = 0
    var legacyAesDecryptCalls: Int = 0
    var legacyRsaFallbackCalls: Int = 0

    /**
     * One-shot hook fired at the next policy-alias encrypt — the exact
     * window between a legacy recovery's read/decrypt and
     * `migrateToPolicyAlias`'s rewrite, where a concurrent wallet deletion
     * can interleave (finding 1049be675782).
     */
    var onNextPolicyEncrypt: (() -> Unit)? = null

    override val keysAlias: String get() = POLICY_ALIAS

    override fun effectiveKeySecurityPolicy(): KeySecurityPolicy = keySecurityPolicy

    override fun keysAliasFingerprint(alias: String): String = fpOf(alias)

    // The real implementation hashes an AndroidKeyStore public key, which
    // cannot exist on the JVM; presence-driven per-alias values instead.
    override fun keysAliasFingerprintOrNull(alias: String): String? = when (alias) {
        POLICY_ALIAS ->
            if (policyKeyProvisioned) fpOf(POLICY_ALIAS) + policyFingerprintSuffix else null
        KEYS_ALIAS_DEVICE_BOUND -> if (deviceBoundKeyPresent) fpOf(KEYS_ALIAS_DEVICE_BOUND) else null
        else -> null
    }

    override fun hasIdentityKeysKey(alias: String): Boolean = when (alias) {
        POLICY_ALIAS -> policyKeyProvisioned
        KEYS_ALIAS_DEVICE_BOUND -> deviceBoundKeyPresent
        else -> false
    }

    override fun hasLegacyKeysKey(): Boolean = keysAliasKind == KeysAliasKind.AES

    override fun hasLegacyRsaKeysKey(): Boolean = keysAliasKind == KeysAliasKind.RSA

    override fun encryptForIdentityKeys(plaintext: ByteArray): KeysAliasEncryptedBlob =
        when (scheme) {
            Scheme.CURRENT -> {
                onNextPolicyEncrypt?.let { hook ->
                    onNextPolicyEncrypt = null
                    hook()
                }
                if (degradeWritesToDeviceBound) {
                    // Lockless degradation: the write lands on (and tags) the
                    // DEVICE_BOUND alias, provisioning it.
                    deviceBoundKeyPresent = true
                    KeysAliasEncryptedBlob(
                        rsaBlob(TAG_DEVICE_BOUND, plaintext),
                        fpOf(KEYS_ALIAS_DEVICE_BOUND),
                        KEYS_ALIAS_DEVICE_BOUND,
                    )
                } else {
                    policyKeyProvisioned = true // public-key encrypt provisions the alias
                    KeysAliasEncryptedBlob(rsaBlob(TAG_POLICY, plaintext), fpOf(POLICY_ALIAS), POLICY_ALIAS)
                }
            }
            // Pre-alias-split write: the blob carries the FORMER key's
            // fingerprint and (having predated the tag) resolves to the policy
            // alias on read. Does NOT provision the policy alias.
            Scheme.FORMER_RSA -> KeysAliasEncryptedBlob(
                rsaBlob(TAG_FORMER_RSA, plaintext),
                FP_FORMER_RSA,
                POLICY_ALIAS,
            )
            // A blob produced by the DEVICE_BOUND sibling but NOT tagged as
            // such (pre-tag data / policy switch): neither the policy alias key
            // nor the former RSA key can open it (dashpay/platform#4060).
            Scheme.SIBLING_POLICY -> KeysAliasEncryptedBlob(
                rsaBlob(TAG_DEVICE_BOUND, plaintext),
                fpOf(KEYS_ALIAS_DEVICE_BOUND),
                POLICY_ALIAS,
            )
            Scheme.LEGACY_AES -> KeysAliasEncryptedBlob(aesBlob(plaintext), FP_LEGACY_AES, POLICY_ALIAS)
        }

    override fun decrypt(blob: EncryptedBlob, alias: String): ByteArray = when (alias) {
        POLICY_ALIAS -> {
            policyDecryptCalls++
            if (throwInvalidatedOnPolicyDecrypt) {
                // Mirror the production KeystoreManager.decrypt contract:
                // generation-checked deletion of the invalidated alias, THEN
                // the typed rethrow.
                invalidatedCleanupRan =
                    deleteIdentityKeysAliasIfCurrentGeneration(POLICY_ALIAS, fpOf(POLICY_ALIAS))
                throw KeyPermanentlyInvalidatedException()
            }
            if (throwAuthOnPolicyDecrypt) throw UserNotAuthenticatedException()
            if (throwBadPaddingOnPolicyDecrypt) throw BadPaddingException("simulated rotation race")
            if (policyKeyProvisioned && blob.ciphertext[0] == TAG_POLICY) {
                plaintextOfRsa(blob)
            } else {
                throw BadPaddingException("wrong key for $alias")
            }
        }
        KEYS_ALIAS_DEVICE_BOUND -> {
            deviceBoundDecryptCalls++
            if (deviceBoundKeyPresent && blob.ciphertext[0] == TAG_DEVICE_BOUND) {
                plaintextOfRsa(blob)
            } else {
                throw BadPaddingException("wrong key for $alias")
            }
        }
        else -> throw IllegalArgumentException("test only decrypts under the RSA policy aliases")
    }

    override fun deleteIdentityKeysAliasIfCurrentGeneration(
        alias: String,
        expectedFingerprint: String,
    ): Boolean {
        if (alias != POLICY_ALIAS || !policyKeyProvisioned) return false
        if (expectedFingerprint != fpOf(POLICY_ALIAS)) return false
        policyKeyProvisioned = false
        return true
    }

    override fun opensUnderNonGatedDeviceBoundSibling(blob: EncryptedBlob): Boolean =
        deviceBoundKeyPresent && blob.iv.isEmpty() && blob.ciphertext[0] == TAG_DEVICE_BOUND

    override fun decryptLegacyKeysBlob(blob: EncryptedBlob): ByteArray? {
        if (keysAliasKind != KeysAliasKind.AES) return null
        legacyAesDecryptCalls++
        return plaintextOfAes(blob)
    }

    override fun decryptLegacyRsaKeysBlob(blob: EncryptedBlob): ByteArray? {
        legacyRsaFallbackCalls++
        if (keysAliasKind != KeysAliasKind.RSA) return null
        // Auth-gated former RSA key with a closed window throws before the padding
        // check (parity with AndroidKeyStore), independent of blob match.
        if (throwAuthOnLegacyRsaDecrypt) throw UserNotAuthenticatedException()
        if (blob.ciphertext[0] == TAG_FORMER_RSA) return plaintextOfRsa(blob)
        throw BadPaddingException("former RSA key cannot open this blob")
    }

    private fun fpOf(alias: String): String = "fake-fp-$alias"

    private fun rsaBlob(tag: Byte, plain: ByteArray): EncryptedBlob {
        val ct = ByteArray(RSA_BLOB_BYTES)
        ct[0] = tag
        ct[1] = plain.size.toByte()
        plain.copyInto(ct, 2)
        return EncryptedBlob(iv = ByteArray(0), ciphertext = ct)
    }

    private fun plaintextOfRsa(blob: EncryptedBlob): ByteArray {
        val len = blob.ciphertext[1].toInt() and 0xFF
        return blob.ciphertext.copyOfRange(2, 2 + len)
    }

    private fun aesBlob(plain: ByteArray): EncryptedBlob {
        val ct = ByteArray(1 + plain.size)
        ct[0] = plain.size.toByte()
        plain.copyInto(ct, 1)
        return EncryptedBlob(iv = ByteArray(12) { 0xAA.toByte() }, ciphertext = ct)
    }

    private fun plaintextOfAes(blob: EncryptedBlob): ByteArray {
        val len = blob.ciphertext[0].toInt() and 0xFF
        return blob.ciphertext.copyOfRange(1, 1 + len)
    }

    private companion object {
        const val POLICY_ALIAS = KeystoreManager.KEYS_ALIAS_AUTH_GATED
        const val RSA_BLOB_BYTES = 2048 / 8
        const val TAG_POLICY: Byte = 0
        const val TAG_FORMER_RSA: Byte = 2
        const val TAG_DEVICE_BOUND: Byte = 3
        const val FP_FORMER_RSA = "fake-fp-former-keys-alias"
        const val FP_LEGACY_AES = "fake-fp-legacy-aes"
    }
}
