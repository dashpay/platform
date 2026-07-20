package org.dashfoundation.dashsdk.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.security.KeyStore

/**
 * Instrumented coverage for the [WalletStorage] cross-wallet ownership /
 * tombstone additions (PR #3999 review findings @ `PlatformWalletManager.kt:778`
 * and `:783`) — requires real Android Keystore, same tier as
 * [org.dashfoundation.dashsdk.wallet.WalletManagerRoundTripTest]. Do NOT run
 * in the JVM unit suite; `AndroidKeyStore` has no JVM/Robolectric provider
 * (confirmed empirically: `KeyStore.getInstance("AndroidKeyStore")` throws
 * `NoSuchAlgorithmException` there).
 *
 * Any test here that reaches [WalletStorage.storePrivateKey] generates the
 * `KEYS_ALIAS` RSA keypair via [KeystoreManager.ensureKeysKeyPair], which
 * requires `setUserAuthenticationRequired(true)` — Android Keystore
 * refuses to create that key without a secure lock screen enrolled on the
 * device. This is the first androidTest in this module to exercise that
 * code path (every existing androidTest avoids identity-key storage), so
 * CI's emulator setup now enrolls one (`adb shell locksettings set-pin`
 * in `.github/workflows/kotlin-sdk-build.yml`) before running this suite.
 */
@RunWith(AndroidJUnit4::class)
class WalletStorageOwnershipTest {

    private lateinit var storage: WalletStorage
    private val walletA = ByteArray(32) { 0xA0.toByte() }
    private val walletB = ByteArray(32) { 0xB0.toByte() }

    @Before
    fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        storage = WalletStorage(context)
    }

    @Test
    fun isOwnedByAnotherWalletSeesASiblingsDurableOwnerIndexEntry() = runBlocking {
        // Wallet B pre-stores an alias (e.g. an in-flight registration key)
        // before any public_keys row for it commits — exactly the state
        // removeWallet's committed-row-only check used to miss.
        storage.storePrivateKey("cafef00d", ByteArray(32) { 7 }, ownerWalletId = walletB)

        val ownedFromA = storage.withPrivateKeyExclusion {
            isOwnedByAnotherWallet("cafef00d", excludingWalletId = walletA)
        }
        assertTrue("wallet A must see B's durable claim", ownedFromA)

        val ownedFromB = storage.withPrivateKeyExclusion {
            isOwnedByAnotherWallet("cafef00d", excludingWalletId = walletB)
        }
        assertFalse("a wallet's own claim must not count as 'another wallet'", ownedFromB)
    }

    @Test
    fun isOwnedByAnotherWalletIsFalseWhenNoOwnerIndexClaimsTheAlias() = runBlocking {
        val owned = storage.withPrivateKeyExclusion {
            isOwnedByAnotherWallet("neverstored", excludingWalletId = walletA)
        }
        assertFalse(owned)
    }

    @Test
    fun storeIfAbsentDerivesOnceThenOnlyRecordsOwnershipForASibling() = runBlocking {
        var deriveCalls = 0
        val scalar = ByteArray(32) { 3 }

        val firstResult = storage.storeIfAbsent("beefcafe", ownerWalletId = walletA) {
            deriveCalls++
            scalar
        }
        assertTrue("first call has nothing stored yet, must derive", firstResult)
        assertEquals(1, deriveCalls)

        val secondResult = storage.storeIfAbsent("beefcafe", ownerWalletId = walletB) {
            deriveCalls++
            scalar
        }
        assertFalse("second call finds an existing decryptable entry", secondResult)
        assertEquals("derive must not run again", 1, deriveCalls)

        // Both wallets are now discoverable owners of the shared alias.
        assertTrue(storage.ownedPrivateKeyAliases(walletA).contains("beefcafe"))
        assertTrue(storage.ownedPrivateKeyAliases(walletB).contains("beefcafe"))
    }

    @Test
    fun storeIfAbsentDiscardsTheLosingDerivationWhenAnotherWriterWinsTheRace() = runBlocking {
        // storeIfAbsent's own derive lambda stores the SAME alias for a
        // different owner before returning — simulates a second caller
        // winning the derive race while this one was still deriving. The
        // second store's recheck must see it and discard this call's bytes.
        storage.storePrivateKey("racealias", ByteArray(32) { 8 }, ownerWalletId = walletB)

        val result = storage.storeIfAbsent("racealias", ownerWalletId = walletA) {
            ByteArray(32) { 9 } // would-be derived bytes, never actually stored
        }

        assertFalse("the winner's copy stands; this call only records ownership", result)
        assertTrue(storage.ownedPrivateKeyAliases(walletA).contains("racealias"))
        assertTrue(storage.ownedPrivateKeyAliases(walletB).contains("racealias"))
    }

    // NOTE: the "present but undecryptable legacy blob → treated as absent,
    // re-derived" branch of storeIfAbsent (addOwnerIfUsableLocked's
    // isPrivateKeyDecryptable check) is intentionally NOT covered here.
    // WalletStorage's public API has no seam to plant a raw legacy-shaped
    // blob (storePrivateKeyEntryLocked hardcodes the current RSA scheme) —
    // exercising it would need a test-only internal hook into fund/key-
    // custody code, which wasn't added without a sync. The boundary this
    // branch relies on (KeystoreManager.isKeysBlobDecryptable's structural
    // shape check) is covered directly by KeystoreManagerTest instead.

    @Test
    fun storePrivateKeyRejectsATombstonedWalletUntilCleared() = runBlocking {
        storage.withPrivateKeyExclusion { tombstoneWallet(walletA) }

        assertThrows(WalletTombstonedException::class.java) {
            runBlocking {
                storage.storePrivateKey("aa11bb22", ByteArray(32) { 5 }, ownerWalletId = walletA)
            }
        }
        assertFalse(
            "the rejected store must not have written anything",
            storage.hasPrivateKey("aa11bb22"),
        )

        storage.clearTombstone(walletA)
        // Re-import of the same (deterministic) wallet id must work again.
        storage.storePrivateKey("aa11bb22", ByteArray(32) { 5 }, ownerWalletId = walletA)
        assertTrue(storage.hasPrivateKey("aa11bb22"))
    }

    @Test
    fun storeIfAbsentRederivesWhenTheKeysAliasKeypairWasReplaced() = runBlocking {
        storage.storePrivateKey("d15ca4d3", ByteArray(32) { 4 }, ownerWalletId = walletA)
        assertTrue(storage.isPrivateKeyDecryptable("d15ca4d3"))

        // Simulate KEYS_ALIAS being replaced (Keystore data loss + a fresh
        // key generated on next use, or a DataStore-only backup restore
        // reintroducing this exact blob onto a device with its own key) —
        // delete the entry directly so the old RSA-shaped blob now sits
        // under a keypair that never encrypted it.
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        keyStore.deleteEntry(KeystoreManager.KEYS_ALIAS)

        assertFalse(
            "an RSA-shaped blob encrypted under a replaced keypair must not be trusted by shape alone",
            storage.isPrivateKeyDecryptable("d15ca4d3"),
        )

        var deriveCalls = 0
        val stored = storage.storeIfAbsent("d15ca4d3", ownerWalletId = walletA) {
            deriveCalls++
            ByteArray(32) { 6 }
        }
        assertTrue("the stale blob must be treated as absent and re-derived", stored)
        assertEquals(1, deriveCalls)
    }

    @Test
    fun retrievePrivateKeyRejectsBlobFromReplacedKeysAlias() = runBlocking {
        storage.storePrivateKey("57a1eb10", ByteArray(32) { 5 }, ownerWalletId = walletA)

        // The ciphertext remains RSA-shaped after alias replacement, but it
        // belongs to the deleted keypair. A stable mismatch is a re-derive
        // signal, not an invitation to attempt OAEP with the replacement
        // private key and surface a provider-specific decryption failure.
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        keyStore.deleteEntry(KeystoreManager.KEYS_ALIAS)

        assertNull(storage.retrievePrivateKey("57a1eb10"))
    }

    @Test
    fun keysAliasEncryptionCarriesTheFingerprintOfItsEncryptionKey() {
        val keystore = KeystoreManager()

        val encrypted = keystore.encrypt(
            ByteArray(32) { 6 },
            alias = KeystoreManager.KEYS_ALIAS,
        )

        // The returned fingerprint is a snapshot of the exact key the
        // ciphertext was produced with, not a live re-read of the alias.
        assertEquals(keystore.keysAliasFingerprint(), encrypted.keyFingerprint)

        // Rotating KEYS_ALIAS afterward must not retroactively change what the
        // blob claims: were the fingerprint re-derived from the current alias
        // instead of captured at encrypt time, this would still match the new
        // key and the mislabel race (old-key ciphertext, new-key fingerprint)
        // would be reachable. It must stay bound to the retired key.
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        keyStore.deleteEntry(KeystoreManager.KEYS_ALIAS)

        assertNotEquals(keystore.keysAliasFingerprint(), encrypted.keyFingerprint)
    }

    @Test
    fun storeIfAbsentRejectsATombstonedWallet() {
        // Block body, not `= runBlocking { ... }`: assertThrows returns the
        // caught exception (not Unit), so an expression-body function ending
        // in it infers a non-Unit return type — JUnit's runtime validator
        // rejects @Test methods that aren't void (caught this the hard way:
        // it compiles fine but fails at connectedDebugAndroidTest time).
        runBlocking {
            storage.withPrivateKeyExclusion { tombstoneWallet(walletB) }

            assertThrows(WalletTombstonedException::class.java) {
                runBlocking {
                    storage.storeIfAbsent("11223344", ownerWalletId = walletB) { ByteArray(32) { 2 } }
                }
            }
        }
    }
}
