package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.FundingNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.ShieldedTokenPurchaseNative
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The `ShieldedTokenPurchaseNative` JNI surface against the real library:
 * both exports bind with the declared signatures, an argument that cannot
 * be read is refused before any wallet code runs, a call that cannot reach
 * a manager throws the mapped error instead of crashing, and a purchase the
 * wallet refuses on its first check (a token count of zero) is reported as
 * a refusal that sent nothing, never as a spend that may have gone through.
 *
 * Not covered here: a purchase Platform accepts, which needs a funded
 * shielded balance and a token with a shielded pool (TEST_PLAN SH-24).
 */
@RunWith(AndroidJUnit4::class)
class ShieldedTokenPurchaseFfiTest {

    private val walletId = ByteArray(32) { 1 }
    private val contractId = ByteArray(32) { 2 }

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
    fun shouldThrowFromBothExportsWhenTheManagerHandleIsInvalid() {
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.availability(0L, contractId, 0)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.purchaseFromPool(0L, walletId, 1L, 0, contractId, 0, 1, 1)
        }
    }

    @Test
    fun shouldRefuseArgumentsItCannotReadBeforeAnyWalletCodeRuns() {
        // A contract id of the wrong size.
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.availability(0L, ByteArray(31), 0)
        }
        // A token position outside 0..65535.
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.availability(0L, contractId, 65_536)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.availability(0L, contractId, -1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.purchaseFromPool(0L, ByteArray(31), 1L, 0, contractId, 0, 1, 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.purchaseFromPool(0L, walletId, 1L, -1, contractId, 0, 1, 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPurchaseNative.purchaseFromPool(0L, walletId, 1L, 0, contractId, 70_000, 1, 1)
        }
    }

    @Test
    fun shouldRefuseTheTokenPurchaseFeeKindOnlyForAnInvalidManager() {
        // Kind 5 reaches the manager lookup; a kind Rust does not know is
        // refused whatever the handle. Both throw, with different errors.
        assertThrows(DashSDKException::class.java) {
            FundingNative.estimateShieldedFee(0L, ShieldedProver.FeeKind.TokenPurchaseFromPool.raw, 2)
        }
    }

    /**
     * Through a real manager and a real (unfunded) wallet: the fee estimate
     * for the purchase kind is a positive number, and a purchase of zero
     * tokens is refused by the wallet's first check, before any note is
     * reserved or anything is sent. The refusal must not be the
     * spend-unconfirmed error, which would tell the buyer it may have gone
     * through.
     */
    @Test
    fun shouldEstimateTheFeeAndRefuseAPurchaseOfZeroTokensAsNothingSent() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val storeDir = File(context.cacheDir, "token-purchase-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, WalletStorage(context)).use { manager ->
                manager.configureShielded(File(storeDir, "shielded.sqlite").absolutePath)
                val id = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "token-purchase",
                    createDefaultAccounts = true,
                ).walletId
                manager.bindShielded(id, listOf(0))

                val fee = manager.estimateShieldedFee(ShieldedProver.FeeKind.TokenPurchaseFromPool)
                assertTrue("the purchase fee is a positive number of credits: $fee", fee > 0)

                try {
                    manager.shieldedTokenPurchaseFromPool(
                        walletId = id,
                        contractId = contractId,
                        tokenPosition = 0,
                        tokenCount = 0uL,
                        totalAgreedPrice = 1_000uL,
                    )
                    fail("a purchase of zero tokens must be refused")
                } catch (e: DashSdkError) {
                    assertFalse(
                        "a refusal that sent nothing is not a spend that may have gone through: $e",
                        e is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed ||
                            e is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
                    )
                }
                assertEquals(
                    "nothing was recorded for the refused purchase",
                    0,
                    db.shieldedDao().getAllActivity().size,
                )
            }
        } finally {
            storeDir.deleteRecursively()
            runCatching { db.close() }
            runCatching { sdk.close() }
        }
    }
}
