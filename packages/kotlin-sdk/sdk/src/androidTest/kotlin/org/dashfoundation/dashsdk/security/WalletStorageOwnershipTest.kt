package org.dashfoundation.dashsdk.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Instrumented coverage for the [WalletStorage] cross-wallet ownership /
 * tombstone additions (PR #3999 review findings @ `PlatformWalletManager.kt:778`
 * and `:783`) — requires real Android Keystore, same tier as
 * [org.dashfoundation.dashsdk.wallet.WalletManagerRoundTripTest]. Do NOT run
 * in the JVM unit suite; `AndroidKeyStore` has no JVM/Robolectric provider
 * (confirmed empirically: `KeyStore.getInstance("AndroidKeyStore")` throws
 * `NoSuchAlgorithmException` there).
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
    fun storeIfAbsentRejectsATombstonedWallet() = runBlocking {
        storage.withPrivateKeyExclusion { tombstoneWallet(walletB) }

        assertThrows(WalletTombstonedException::class.java) {
            runBlocking {
                storage.storeIfAbsent("11223344", ownerWalletId = walletB) { ByteArray(32) { 2 } }
            }
        }
    }
}
