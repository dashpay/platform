package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.ShieldedTokenPoolNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolSpendKind
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
 * The `ShieldedTokenPoolNative` JNI surface against the real library: every
 * export binds with its declared signature, an argument that cannot be read
 * is refused before any wallet code runs, a call that cannot reach a manager
 * throws instead of crashing, and through a real (unfunded) wallet the
 * balance list is empty and a spend of zero tokens is refused as a refusal
 * that sent nothing, never as a spend that may have gone through.
 *
 * Not covered here: a watched pool with notes, which needs a network read of
 * a token with a shielded pool (TEST_PLAN SH-25..SH-27). The balance row's
 * constructor descriptor is checked against the Kotlin class by the JVM test
 * `ShieldedTokenPoolBalanceTest`.
 */
@RunWith(AndroidJUnit4::class)
class ShieldedTokenPoolFfiTest {

    private val walletId = ByteArray(32) { 1 }
    private val contractId = ByteArray(32) { 2 }
    private val recipient43 = ByteArray(43) { 3 }

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
    fun shouldThrowFromEveryExportWhenTheManagerHandleIsInvalid() {
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.watch(0L, walletId, contractId, 0)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.unwatch(0L, walletId, contractId)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.balances(0L, walletId)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.transfer(0L, walletId, 1L, 0, contractId, 0, recipient43, 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.unshield(0L, walletId, 1L, 0, contractId, 0, walletId, 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.spendPreview(0L, walletId, 0, contractId, 0, 0, 1)
        }
    }

    @Test
    fun shouldRefuseArgumentsItCannotReadBeforeAnyWalletCodeRuns() {
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.watch(0L, ByteArray(31), contractId, 0)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.watch(0L, walletId, contractId, 65_536)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.transfer(0L, walletId, 1L, 0, contractId, 0, ByteArray(42), 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.transfer(0L, walletId, 1L, -1, contractId, 0, recipient43, 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.unshield(0L, walletId, 1L, 0, contractId, 0, ByteArray(31), 1)
        }
        assertThrows(DashSDKException::class.java) {
            ShieldedTokenPoolNative.spendPreview(0L, walletId, 0, contractId, -1, 0, 1)
        }
    }

    @Test
    fun shouldListNothingAndRefuseAZeroSpendAsNothingSent() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val storeDir = File(context.cacheDir, "token-pools-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, WalletStorage(context)).use { manager ->
                manager.configureShielded(File(storeDir, "shielded.sqlite").absolutePath)
                val id = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "token-pools",
                    createDefaultAccounts = true,
                ).walletId
                manager.bindShielded(id, listOf(0))

                assertEquals(emptyList<Any>(), manager.shieldedTokenPoolBalances(id))
                // Unwatching a pool that is not watched changes nothing.
                manager.shieldedTokenPoolUnwatch(id, contractId)

                for (kind in ShieldedTokenPoolSpendKind.entries) {
                    try {
                        manager.shieldedTokenSpendPreview(id, contractId, 0, kind, 0uL)
                        fail("a preview of zero tokens must be refused ($kind)")
                    } catch (e: DashSdkError) {
                        // Rust's refusal, shown by the form in place of the fee.
                    }
                }
                try {
                    manager.shieldedTokenTransfer(id, contractId, 0, recipient43, 0uL)
                    fail("a transfer of zero tokens must be refused")
                } catch (e: DashSdkError) {
                    assertFalse(
                        "a refusal that sent nothing is not a spend that may have gone through: $e",
                        e is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed ||
                            e is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
                    )
                }
                assertEquals(
                    "nothing was recorded for the refused transfer",
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
