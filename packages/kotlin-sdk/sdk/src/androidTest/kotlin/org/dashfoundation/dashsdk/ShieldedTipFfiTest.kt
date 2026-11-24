package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.ShieldedTipRecipient
import org.dashfoundation.dashsdk.wallet.ShieldedTipSource
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The shielded tip calls through a real (unfunded, offline) wallet against
 * the loaded library.
 *
 * A tip started from a saved contact passes the contact's identity id. Rust
 * refuses a reviewed recipient of another identity before any wait, lookup
 * or proof, so the refusal is reached with no network: it shows the id JNI
 * read is the id the wallet compared. The wallet is left unbound on purpose:
 * were the contact's id dropped on the way, the send would stop at the
 * binding check with another error, before any name lookup. `FfiSmokeTest`
 * covers the argument refusals and an unknown manager.
 *
 * Not covered here: a review or a send that resolves a name, which needs a
 * network read (TEST_PLAN, shielded tips).
 */
@RunWith(AndroidJUnit4::class)
class ShieldedTipFfiTest {

    // BIP39 English test vector (all-zero entropy).
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    @Before
    fun setUp() {
        NativeLoader.ensureLoaded()
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
    }

    @Test
    fun shouldRefuseATipWhoseReviewedRecipientIsNotTheContact() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val storeDir = File(context.cacheDir, "shielded-tips-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, WalletStorage(context)).use { manager ->
                manager.configureShielded(File(storeDir, "shielded.sqlite").absolutePath)
                val id = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "shielded-tips",
                    createDefaultAccounts = true,
                ).walletId

                val contact = ByteArray(32) { 3 }
                val reviewed = ShieldedTipRecipient(
                    identityId = ByteArray(32) { 4 },
                    address = ByteArray(43) { 5 },
                )
                try {
                    manager.sendShieldedTip(
                        walletId = id,
                        source = ShieldedTipSource.Account(0),
                        username = "alice",
                        expectedRecipient = reviewed,
                        amount = 1L,
                        contactIdentityId = contact,
                    )
                    fail("a tip to another identity than the contact was sent")
                } catch (e: DashSdkError.PlatformWallet.ShieldedTipRecipientMismatch) {
                    assertEquals(false, e.isRetryable)
                }

                // The wallet holds no identity: the balance read says so by
                // its own error, not as a balance and not as a busy store.
                try {
                    manager.shieldedIdentityBalance(id, ByteArray(32) { 6 })
                    fail("a balance was read for an identity the wallet does not hold")
                } catch (e: DashSdkError.NotFound) {
                    assertFalse(e.isRetryable)
                }
            }
        } finally {
            storeDir.deleteRecursively()
            runCatching { db.close() }
            runCatching { sdk.close() }
        }
    }
}
