package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.FundingNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.NativeSignerBridge
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Binding-level coverage for the `shieldedShieldFromIdentityToRecipient` JNI
 * export, with the same no-wallet discipline as [ShieldToRecipientBindingTest].
 * It proves the Kotlin `external` declaration, the generated JNI symbol and
 * the parameter descriptor stay in lockstep (a mismatch surfaces here as
 * `UnsatisfiedLinkError`), and pins each rejection to its layer. No network,
 * no wallet, no funds.
 *
 * Three layers are asserted apart:
 *  - the JNI trampoline's marshalling checks (sign checks before the unsigned
 *    casts, fixed-size arrays, non-null signer) throw [DashSDKException] with
 *    RAW code 1 and a message naming the argument, BEFORE any FFI call. The
 *    two 32-byte ids share a descriptor slot type, so the message checks
 *    double as parameter-order pins;
 *  - a zero amount and an over-limit memo are not checked there. Zero passes
 *    the JNI layer and fails at the dead handle; the memo is rejected inside
 *    the FFI by Rust's `encode_memo_text` (the single owner of the 32-byte
 *    rule) as `ErrorInvalidParameter`, before the handle is resolved;
 *  - a well-formed call with dead manager handle 0 reaches the FFI's handle
 *    lookup and comes back as `ErrorInvalidHandle`;
 *  - through the public [PlatformWalletManager] method on a live, empty
 *    manager, the FFI's wallet lookup names the wallet id it received, which
 *    pins that the two 32-byte ids are not swapped anywhere between Kotlin
 *    and the FFI (the width checks cannot tell them apart).
 *
 * The signer is a real native handle over a bridge that holds no keys, so
 * the calls stay memory-safe even if the FFI ever borrows the signer before
 * it resolves the manager handle. It never signs: every call here fails
 * before a transition is built.
 */
@RunWith(AndroidJUnit4::class)
class ShieldFromIdentityToRecipientBindingTest {

    private companion object {
        const val DEAD_HANDLE = 0L

        /** `PlatformWalletFFIResultCode` values, shifted into the Kotlin range. */
        const val INVALID_HANDLE = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 1
        const val INVALID_PARAMETER = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 2
    }

    /** Holds no keys: refuses every signature request. */
    private class NoKeySigner : NativeSignerBridge() {
        override fun signAsync(
            pubkeyBytes: ByteArray,
            keyType: Int,
            data: ByteArray,
            completionToken: Long,
        ) {
            SignerNative.completeSign(
                completionToken,
                null,
                SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE,
                "test signer holds no keys",
            )
        }

        override fun canSignWith(pubkeyBytes: ByteArray, keyType: Int): Boolean = false
    }

    private var signerHandle = 0L

    @Before
    fun createSigner() {
        NativeLoader.ensureLoaded()
        signerHandle = SignerNative.createSigner(NoKeySigner())
        assertTrue("createSigner returned a null handle", signerHandle != 0L)
    }

    @After
    fun destroySigner() {
        SignerNative.destroySigner(signerHandle)
        signerHandle = 0L
    }

    private fun callExpectingThrow(
        walletId: ByteArray = ByteArray(32) { 1 },
        shieldedAccount: Int = 0,
        identityId: ByteArray = ByteArray(32) { 3 },
        recipient: ByteArray = ByteArray(43) { 2 },
        amount: Long = 1_000L,
        memo: String? = null,
        signer: Long = signerHandle,
    ): DashSDKException {
        NativeLoader.ensureLoaded()
        return assertThrows(DashSDKException::class.java) {
            FundingNative.shieldedShieldFromIdentityToRecipient(
                DEAD_HANDLE,
                walletId,
                shieldedAccount,
                identityId,
                recipient,
                amount,
                memo,
                signer,
            )
        }
    }

    private fun assertJniRejection(e: DashSDKException, argument: String) {
        assertEquals("JNI-side rejection carries raw code 1: ${e.message}", 1, e.code)
        assertTrue(
            "the rejection must name $argument, got: ${e.message}",
            e.message.orEmpty().contains(argument),
        )
    }

    @Test
    fun shouldRejectNegativeAmountAndAccountBeforeTheFfi() {
        assertJniRejection(callExpectingThrow(amount = -1L), "amount")
        assertJniRejection(callExpectingThrow(shieldedAccount = -1), "shieldedAccount")
    }

    @Test
    fun shouldRejectANullSignerBeforeTheFfi() {
        assertJniRejection(callExpectingThrow(signer = 0L), "signerIdentityHandle")
    }

    @Test
    fun shouldRejectMisSizedIdsBeforeTheFfi() {
        assertJniRejection(callExpectingThrow(walletId = ByteArray(31)), "walletId")
        assertJniRejection(callExpectingThrow(identityId = ByteArray(33)), "identityId")
        assertJniRejection(callExpectingThrow(recipient = ByteArray(42)), "recipientRaw43")
    }

    @Test
    fun shouldLeaveAZeroAmountToRust() {
        // The JNI layer only blocks negatives; zero crosses into the FFI and
        // here fails at the dead handle, before the wallet's own > 0 check.
        val e = callExpectingThrow(amount = 0L)
        assertEquals("zero amount: ${e.message}", INVALID_HANDLE, e.code)
    }

    @Test
    fun shouldLeaveTheMemoLimitToRust() {
        // 20 × "€" = 60 UTF-8 bytes. Neither Kotlin nor the JNI layer checks
        // the length; Rust's encode_memo_text rejects it inside the FFI.
        val e = callExpectingThrow(memo = "€".repeat(20))
        assertEquals("over-limit memo: ${e.message}", INVALID_PARAMETER, e.code)
    }

    @Test
    fun shouldReachTheFfiInvalidHandlePathForWellFormedCalls() {
        // null, empty and a multibyte memo within 32 UTF-8 bytes (10 × "€" =
        // 30 bytes) all pass every check and fail only at the handle lookup.
        for (memo in listOf(null, "", "€".repeat(10))) {
            val e = callExpectingThrow(memo = memo)
            assertEquals("memo=${memo?.length}: ${e.message}", INVALID_HANDLE, e.code)
        }
    }

    @Test
    fun shouldForwardTheWalletIdToTheWalletSlot() = runBlocking {
        // A live manager with no wallets, offline: the FFI resolves the wallet
        // first and fails with "wallet not found: <hex wallet id>" before it
        // needs a shielded coordinator, the signer or the network. Distinct
        // fill bytes show which id reached the wallet slot.
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, WalletStorage(context)).use { manager ->
                val e = assertThrows(DashSdkError::class.java) {
                    runBlocking {
                        manager.shieldedShieldFromIdentityToRecipient(
                            walletId = ByteArray(32) { 0x11 },
                            identityId = ByteArray(32) { 0x22 },
                            recipientRaw43 = ByteArray(43) { 0x33 },
                            amount = 1_000L,
                        )
                    }
                }
                val message = e.message.orEmpty()
                assertTrue(
                    "the wallet lookup must name the wallet id, got: $message",
                    message.contains("wallet not found: " + "11".repeat(32)),
                )
                assertFalse(
                    "the identity id must not reach the wallet slot, got: $message",
                    message.contains("22".repeat(32)),
                )
            }
        } finally {
            runCatching { db.close() }
            runCatching { sdk.close() }
        }
    }
}
