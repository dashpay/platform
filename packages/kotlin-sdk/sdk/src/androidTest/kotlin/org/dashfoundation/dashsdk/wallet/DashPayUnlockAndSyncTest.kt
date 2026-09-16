package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.toHex
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Instrumented coverage for the K2 seedless-unlock topology and the
 * DashPay sync-service lifecycle (KOTLIN_MIGRATION_SPEC.md §K2), through
 * the real native lib:
 *
 * - The **happy-path unlock is the end-to-end proof of the out-buffer
 *   mnemonic resolver**: `loadPersistedWallets` auto-runs
 *   `unlockWalletFromKeystore`, whose `verify_seed_binds_to_wallet`
 *   derives the BIP44 account-0 xpub through the
 *   `resolveMnemonicInto(byte[], byte[])` vtable — a broken copy/zero/
 *   length in the new no-JVM-String contract fails the verify.
 * - The **wrong-seed leg** pins the seed-mismatch contract: a foreign
 *   (valid) mnemonic stored under the wallet's id must publish
 *   `seedMismatch = true` on [PlatformWalletManager.dashPayUnlockStatus]
 *   and never sign.
 * - The **sync-service leg** pins the manager-owned lifecycle:
 *   start → running, idempotent double-start, stop → stopped, and a
 *   second manager on the same DB can start it again after close
 *   (the manager-swap disposal path). `dashPaySyncNow` is deliberately
 *   NOT exercised — it is a live network sweep, `-Ptestnet=true` tier.
 */
@RunWith(AndroidJUnit4::class)
class DashPayUnlockAndSyncTest {

    // BIP39 English test vectors.
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"
    private val foreignMnemonic =
        "legal winner thank year wave sausage worth useful legal winner " +
            "thank yellow"

    private lateinit var db: DashDatabase
    private lateinit var walletStorage: WalletStorage
    private lateinit var sdk: Sdk

    @Before
    fun setUp() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        db = DashDatabase.createInMemory(context)
        walletStorage = WalletStorage(context)
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
    }

    @After
    fun tearDown() {
        runCatching { db.close() }
        runCatching { sdk.close() }
    }

    private suspend fun createWallet(manager: PlatformWalletManager): ByteArray =
        manager.createWallet(
            mnemonic = testMnemonic,
            name = "unlock-test",
            createDefaultAccounts = true,
        ).walletId

    /** Await until [predicate] holds on the unlock status map, or fail. */
    private suspend fun PlatformWalletManager.awaitUnlockStatus(
        key: String,
        what: String,
        predicate: (DashPayUnlockStatus?) -> Boolean,
    ) = withTimeout(20_000) {
        while (!predicate(dashPayUnlockStatus.value[key])) delay(50)
        assertTrue(what, predicate(dashPayUnlockStatus.value[key]))
    }

    @Test
    fun loadAutoUnlocksVerifiesSeedAndDrains() = runBlocking {
        val walletId: ByteArray
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            walletId = createWallet(manager)
            // Storage discipline: existence check + raw-bytes read both
            // work, and the bytes are the exact phrase UTF-8.
            assertTrue(walletStorage.hasMnemonic(walletId))
            val utf8 = walletStorage.retrieveMnemonicUtf8(walletId)
            assertNotNull(utf8)
            assertEquals(testMnemonic, utf8!!.decodeToString())
            utf8.fill(0)
        }

        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
            reloaded.loadPersistedWallets()
            val key = walletId.toHex()

            // The auto-unlock published a status entry: the seed verified
            // (via the out-buffer resolver) and the drain ran (no pending
            // entries on a fresh wallet) and cleared its flag.
            reloaded.awaitUnlockStatus(key, "unlock status published") { it != null }
            assertFalse(
                "seed must verify against its own wallet",
                reloaded.dashPayUnlockStatus.value[key]!!.seedMismatch,
            )
            reloaded.awaitUnlockStatus(key, "drain completes") { it?.draining == false }
            assertEquals(0, reloaded.contactCryptoPendingCount(walletId))
        }
    }

    @Test
    fun wrongSeedPublishesSeedMismatchAndStaysWatchOnly() = runBlocking {
        val walletId: ByteArray
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            walletId = createWallet(manager)
        }
        // Mis-map the Keystore slot: a DIFFERENT valid mnemonic under this
        // wallet's id. The binding verify derives a different BIP44
        // account-0 xpub and must reject.
        walletStorage.storeMnemonic(walletId, foreignMnemonic)

        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
            val restored = reloaded.loadPersistedWallets()
            assertTrue("restore itself must not fail", restored.isNotEmpty())
            val key = walletId.toHex()
            reloaded.awaitUnlockStatus(key, "seed mismatch published") {
                it?.seedMismatch == true
            }
            assertFalse(
                "no drain may run on a mismatched seed",
                reloaded.dashPayUnlockStatus.value[key]!!.draining,
            )
        }
    }

    @Test
    fun dashPaySyncLifecycleSurvivesManagerSwap() = runBlocking {
        val walletId: ByteArray
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            walletId = createWallet(manager)

            assertFalse(manager.isDashPaySyncRunning())
            manager.startDashPaySync()
            assertTrue(manager.isDashPaySyncRunning())
            // Idempotent double-start.
            manager.startDashPaySync()
            assertTrue(manager.isDashPaySyncRunning())

            manager.setDashPaySyncInterval(300)

            manager.stopDashPaySync()
            assertFalse(manager.isDashPaySyncRunning())

            // Leave it running into close() — the manager-swap disposal
            // path must not leak the loop into the next manager.
            manager.startDashPaySync()
            assertTrue(manager.isDashPaySyncRunning())
        }

        // Fresh manager on the same DB: its own sweep starts cleanly.
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { second ->
            second.loadPersistedWallets()
            assertNotNull(second.wallet(forWalletId = walletId))
            assertFalse(second.isDashPaySyncRunning())
            second.startDashPaySync()
            assertTrue(second.isDashPaySyncRunning())
            second.stopDashPaySync()
        }
    }
}
