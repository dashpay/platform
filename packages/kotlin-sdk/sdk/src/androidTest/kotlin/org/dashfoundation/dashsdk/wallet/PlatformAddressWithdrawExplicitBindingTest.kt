package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Binding-level coverage for the `walletPlatformAddressWithdrawExplicit` JNI
 * export, with the same no-wallet discipline as
 * [CoreWalletSetGapLimitBindingTest]: prove the Kotlin external declaration,
 * the generated JNI symbol and the parameter descriptor stay in lockstep (a
 * mismatch surfaces here as `UnsatisfiedLinkError`), and pin each trampoline
 * rejection to the exception it throws. No network, no wallet, no funds.
 *
 * Two rejection layers are asserted apart:
 *  - the trampoline's own checks throw [DashSDKException] with raw code 1 and
 *    a message naming the rejected parameter, before any FFI call;
 *  - a well-formed call with a dead wallet handle crosses into the FFI, whose
 *    miss comes back in the platform-wallet code range
 *    (>= [DashSdkError.PLATFORM_WALLET_CODE_OFFSET]).
 *
 * One test goes further, through the public `withdrawCredits` overload with
 * an offline wallet and its real signer, into the platform wallet's
 * withdraw: it pins that the trampoline selects EXPLICIT inputs, not AUTO.
 *
 * The message checks double as parameter-order pins: `accountIndex` and
 * `coreFeePerByte` share the `I` descriptor slot type, so swapping them in
 * either declaration would misroute a probe and fail the message check.
 */
@RunWith(AndroidJUnit4::class)
class PlatformAddressWithdrawExplicitBindingTest {

    private val oneInput = encodeWithdrawalInputs(
        listOf(ManagedPlatformWallet.WithdrawalInput(0, ByteArray(20), 1_000_000L)),
    )

    private fun callExpectingThrow(
        handle: Long = 0L,
        accountIndex: Int = 0,
        inputsBlob: ByteArray = oneInput,
        // Never parsed here: the wallet-handle lookup fails first.
        coreAddress: String = "unchecked-core-address",
        coreFeePerByte: Int = 1,
    ): DashSDKException {
        NativeLoader.ensureLoaded()
        return assertThrows(DashSDKException::class.java) {
            WalletManagerNative.walletPlatformAddressWithdrawExplicit(
                walletHandle = handle,
                accountIndex = accountIndex,
                inputsBlob = inputsBlob,
                coreAddress = coreAddress,
                coreFeePerByte = coreFeePerByte,
                signerHandle = 0L,
            )
        }
    }

    private fun assertRejectedBeforeTheFfi(e: DashSDKException, names: String) {
        assertEquals("JNI-side parameter rejection carries raw code 1", 1, e.code)
        assertTrue(
            "the rejection must name $names, got: ${e.message}",
            e.message.orEmpty().contains(names),
        )
    }

    @Test
    fun negativeAccountIndexIsRejectedBeforeTheFfi() {
        assertRejectedBeforeTheFfi(callExpectingThrow(accountIndex = -1), "accountIndex")
    }

    @Test
    fun negativeCoreFeePerByteIsRejectedBeforeTheFfi() {
        assertRejectedBeforeTheFfi(callExpectingThrow(coreFeePerByte = -1), "coreFeePerByte")
    }

    @Test
    fun malformedInputsBlobIsRejectedBeforeTheFfi() {
        // Truncated header, and a row count that disagrees with the body.
        for (blob in listOf(ByteArray(3), oneInput.copyOf(oneInput.size - 1))) {
            assertRejectedBeforeTheFfi(
                callExpectingThrow(inputsBlob = blob),
                "withdrawal inputs blob",
            )
        }
    }

    @Test
    fun negativeInputCreditsAreRejectedBeforeTheFfi() {
        val blob = encodeWithdrawalInputs(
            listOf(ManagedPlatformWallet.WithdrawalInput(0, ByteArray(20), -1L)),
        )
        assertRejectedBeforeTheFfi(callExpectingThrow(inputsBlob = blob), "out of range")
    }

    @Test
    fun emptyCoreAddressIsRejectedBeforeTheFfi() {
        assertRejectedBeforeTheFfi(callExpectingThrow(coreAddress = ""), "core_address")
    }

    @Test
    fun wellFormedRequestReachesTheFfiInvalidHandlePath() {
        // Handle 0 can never be a live wallet. The FFI's miss comes back
        // translated into the platform-wallet code range, NOT the
        // trampoline's raw code 1, which proves every JNI check passed and
        // the call left the JNI layer.
        val e = callExpectingThrow(handle = 0L)
        assertTrue(
            "a well-formed request must fail inside the FFI (translated code >= " +
                "${DashSdkError.PLATFORM_WALLET_CODE_OFFSET}), got ${e.code}: ${e.message}",
            e.code >= DashSdkError.PLATFORM_WALLET_CODE_OFFSET,
        )
    }

    @Test
    fun explicitSelectionReachesThePlatformWalletWithdraw() = runBlocking {
        // An offline wallet (no connection is made) and the manager's real
        // signer get the call past every JNI and FFI check and into the
        // platform wallet's `withdraw`. With an empty input list, EXPLICIT
        // selection fails there with its own message before any network
        // call. AUTO selection would instead fail on the fresh wallet with
        // "No funded addresses available", so a trampoline that passed the
        // wrong selection type fails this test.
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, WalletStorage(context)).use { manager ->
                val walletId = manager.createWallet(
                    mnemonic = TEST_MNEMONIC,
                    name = "withdraw-explicit-binding",
                    createDefaultAccounts = true,
                ).walletId
                val wallet = manager.wallet(forWalletId = walletId)
                assertNotNull("the created wallet is managed", wallet)

                val e = assertThrows(DashSdkError::class.java) {
                    runBlocking {
                        requireNotNull(wallet).withdrawCredits(
                            coreAddress = TESTNET_CORE_ADDRESS,
                            inputs = emptyList(),
                            coreFeePerByte = 1,
                            signerHandle = manager.signerHandle,
                        )
                    }
                }
                assertTrue(
                    "an empty EXPLICIT withdrawal must fail in the platform wallet, got: ${e.message}",
                    e.message.orEmpty().contains("at least one input address"),
                )
            }
        } finally {
            runCatching { db.close() }
            runCatching { sdk.close() }
        }
    }

    private companion object {
        // BIP39 English test vector (all-zero entropy).
        const val TEST_MNEMONIC =
            "abandon abandon abandon abandon abandon abandon abandon abandon " +
                "abandon abandon abandon about"

        // Testnet P2PKH address of the all-zero hash: valid, owned by no one.
        const val TESTNET_CORE_ADDRESS = "yLKSrCjLQFsfVgX8RjdctZ797d54atPjnV"
    }
}
