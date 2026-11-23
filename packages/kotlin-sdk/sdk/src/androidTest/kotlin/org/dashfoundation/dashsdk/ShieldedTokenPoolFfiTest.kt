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
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The `ShieldedTokenPoolNative` JNI surface against the real library.
 *
 * - An argument JNI cannot read is refused by JNI itself (code 1, naming the
 *   argument) before any wallet code runs.
 * - With every argument valid (a real Orchard address, a live resolver), a
 *   handle that names no manager reaches the FFI's manager lookup and is
 *   refused there (invalid handle, code 1001), for every export.
 * - Through a real (unfunded) wallet the balance list is empty, and a spend
 *   or preview of zero tokens reaches the wallet's own amount check and is
 *   refused as a refusal that sent nothing, never as a spend that may have
 *   gone through.
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

    // BIP39 English test vector (all-zero entropy).
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    @Before
    fun setUp() {
        NativeLoader.ensureLoaded()
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
    }

    private fun assertJniRefusal(field: String, call: () -> Unit) {
        val e = assertThrows(DashSDKException::class.java) { call() }
        assertEquals("JNI's own argument refusal: ${e.message}", JNI_ARGUMENT_CODE, e.code)
        assertTrue(e.message, e.message!!.contains(field))
    }

    @Test
    fun shouldRefuseArgumentsJniCannotReadBeforeAnyWalletCodeRuns() {
        val recipient = ByteArray(43)
        assertJniRefusal("walletId") { ShieldedTokenPoolNative.watch(0L, ByteArray(31), contractId, 0) }
        assertJniRefusal("tokenPosition") { ShieldedTokenPoolNative.watch(0L, walletId, contractId, 65_536) }
        assertJniRefusal("tokenId") { ShieldedTokenPoolNative.unwatch(0L, walletId, ByteArray(33)) }
        assertJniRefusal("walletId") { ShieldedTokenPoolNative.balances(0L, ByteArray(0)) }
        assertJniRefusal("recipientRaw43") {
            ShieldedTokenPoolNative.transfer(0L, walletId, 0L, 0, contractId, 0, ByteArray(42), 1)
        }
        assertJniRefusal("account") {
            ShieldedTokenPoolNative.transfer(0L, walletId, 0L, -1, contractId, 0, recipient, 1)
        }
        assertJniRefusal("recipientIdentityId") {
            ShieldedTokenPoolNative.unshield(0L, walletId, 0L, 0, contractId, 0, ByteArray(31), 1)
        }
        assertJniRefusal("tokenPosition") {
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
                // A real Orchard address, so no refusal below is about the recipient.
                val recipient = manager.shieldedDefaultAddress(id, 0)
                assertNotNull("the bound account has a default address", recipient)
                recipient!!
                val resolver = manager.mnemonicResolverHandle

                // Every argument valid, the handle names no manager: refused by
                // the FFI's manager lookup.
                val invalidHandle = listOf<() -> Unit>(
                    { ShieldedTokenPoolNative.watch(0L, id, contractId, 0) },
                    { ShieldedTokenPoolNative.unwatch(0L, id, contractId) },
                    { ShieldedTokenPoolNative.balances(0L, id) },
                    { ShieldedTokenPoolNative.transfer(0L, id, resolver, 0, contractId, 0, recipient, 1) },
                    { ShieldedTokenPoolNative.unshield(0L, id, resolver, 0, contractId, 0, id, 1) },
                    { ShieldedTokenPoolNative.spendPreview(0L, id, 0, contractId, 0, 0, 1) },
                )
                for ((index, call) in invalidHandle.withIndex()) {
                    val e = assertThrows(DashSDKException::class.java) { call() }
                    assertEquals("export $index: ${e.message}", INVALID_HANDLE_CODE, e.code)
                }

                assertEquals(emptyList<Any>(), manager.shieldedTokenPoolBalances(id))
                // Unwatching a pool that is not watched changes nothing.
                manager.shieldedTokenPoolUnwatch(id, contractId)

                for (kind in ShieldedTokenPoolSpendKind.entries) {
                    assertZeroAmountRefusal("preview ($kind)") {
                        manager.shieldedTokenSpendPreview(id, contractId, 0, kind, 0uL)
                    }
                }
                assertZeroAmountRefusal("transfer") {
                    manager.shieldedTokenTransfer(id, contractId, 0, recipient, 0uL)
                }
                assertZeroAmountRefusal("unshield") {
                    manager.shieldedTokenUnshield(id, contractId, 0, id, 0uL)
                }
                assertEquals(
                    "nothing was recorded for the refused spends",
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

    /**
     * [call] must fail with the wallet's own zero-amount refusal: not an
     * earlier refusal of another argument, and never the error that says a
     * spend may have gone through.
     */
    private suspend fun assertZeroAmountRefusal(what: String, call: suspend () -> Unit) {
        try {
            call()
            fail("a $what of zero tokens must be refused")
        } catch (e: DashSdkError) {
            assertFalse(
                "a refusal that sent nothing is not a spend that may have gone through: $e",
                e is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed ||
                    e is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
            )
            assertTrue("$what: ${e.message}", e.message.orEmpty().contains("amount must be > 0"))
        }
    }

    private companion object {
        /** `throw_sdk_exception(env, 1, …)`: JNI could not read an argument. */
        const val JNI_ARGUMENT_CODE = 1

        /** `PWFFI_CODE_OFFSET` (1000) + `ErrorInvalidHandle` (1). */
        const val INVALID_HANDLE_CODE = 1001
    }
}
