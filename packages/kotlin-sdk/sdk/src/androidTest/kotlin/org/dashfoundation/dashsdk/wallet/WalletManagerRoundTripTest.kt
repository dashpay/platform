package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Instrumented round-trip: create a manager, create a wallet from a fixed
 * test mnemonic OFFLINE (the FFI derives locally — `createTrusted` only
 * builds the client, it does not connect), assert Room rows landed via the
 * persistence callbacks, then build a NEW manager against the same DB and
 * [PlatformWalletManager.loadPersistedWallets] to confirm the wallet is
 * restored watch-only.
 *
 * Requires an emulator/device (native lib + Android Keystore). Do NOT run
 * in the JVM unit suite — it is the wallet-manager analog of
 * [org.dashfoundation.dashsdk.FfiSmokeTest]. Orchestrator note: gate this
 * behind `connectedDebugAndroidTest`.
 */
@RunWith(AndroidJUnit4::class)
class WalletManagerRoundTripTest {

    // BIP39 English test vector (all-zero entropy).
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    private lateinit var db: DashDatabase
    private lateinit var walletStorage: WalletStorage
    private lateinit var sdk: Sdk

    @Before
    fun setUp() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        db = DashDatabase.createInMemory(context)
        walletStorage = WalletStorage(context)
        // Testnet, no overrides → offline client build (no connection made).
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
    }

    @After
    fun tearDown() {
        runCatching { db.close() }
        runCatching { sdk.close() }
    }

    @Test
    fun walletCreateAndReloadRoundTrip() = runBlocking {
        val walletId: ByteArray

        // ── First manager: create the wallet from the fixed mnemonic ──
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            val created = manager.createWallet(
                mnemonic = testMnemonic,
                name = "round-trip",
                createDefaultAccounts = true,
            )
            walletId = created.walletId
            assertEquals("wallet id is 32 bytes", 32, walletId.size)

            // The map is keyed by walletId hex.
            assertNotNull(manager.wallet(forWalletId = walletId))

            // Persistence callbacks (fired synchronously from create) must
            // have written the wallet row + at least one account with an xpub.
            val walletRow = db.walletDao().getByWalletId(walletId)
            assertNotNull("wallet row persisted", walletRow)
            assertEquals(Network.TESTNET.ffiValue, walletRow!!.networkRaw)

            val accounts = db.accountDao().observeByWallet(walletId).first()
            assertTrue("at least one account persisted", accounts.isNotEmpty())
            assertTrue(
                "at least one account carries an xpub (restorable)",
                accounts.any { it.accountExtendedPubKeyBytes?.isNotEmpty() == true },
            )

            // The mnemonic must be retrievable keyed by the derived id.
            assertEquals(testMnemonic, walletStorage.retrieveMnemonic(walletId))
        }

        // ── Second manager: reload from persistence (watch-only) ──
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
            val restored = reloaded.loadPersistedWallets()
            assertTrue("at least one wallet restored", restored.isNotEmpty())
            val match = reloaded.wallet(forWalletId = walletId)
            assertNotNull("the created wallet is restored by id", match)
            assertTrue(walletId.contentEquals(match!!.walletId))
        }
    }
}
