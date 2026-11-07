package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.FundingNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Binding-level coverage for the `shieldedShieldToRecipient` JNI export, with
 * the same no-wallet discipline as [CoreWalletSetGapLimitBindingTest]. It
 * proves the Kotlin `external` declaration, the generated JNI symbol and the
 * parameter descriptor stay in lockstep (a mismatch surfaces here as
 * `UnsatisfiedLinkError`), and pins each rejection to its layer. No network,
 * no wallet, no funds.
 *
 * Three layers are asserted apart:
 *  - the JNI trampoline's own boundary checks throw [DashSDKException] with
 *    RAW code 1 and a message naming the argument, BEFORE any FFI call. The
 *    message checks double as parameter-order pins: the two account ints share
 *    a descriptor slot type, so a swap would hit the wrong branch;
 *  - an over-limit memo crosses into the FFI and is rejected by Rust's
 *    `encode_memo_text` (the single owner of the 32-byte rule) as
 *    `ErrorInvalidParameter`, which the FFI checks before resolving the handle;
 *  - a well-formed call with dead manager handle 0 reaches the FFI's
 *    handle lookup and comes back as `ErrorInvalidHandle`.
 *
 * The FFI encodes the memo and resolves the manager handle before it ever
 * dereferences the signer pointer, so [DUMMY_SIGNER] is never read; it only
 * has to be non-zero to pass the JNI null check.
 */
@RunWith(AndroidJUnit4::class)
class ShieldToRecipientBindingTest {

    private companion object {
        const val DEAD_HANDLE = 0L
        const val DUMMY_SIGNER = 1L

        /** `PlatformWalletFFIResultCode` values, shifted into the Kotlin range. */
        const val INVALID_HANDLE = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 1
        const val INVALID_PARAMETER = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 2
    }

    private fun callExpectingThrow(
        walletId: ByteArray = ByteArray(32) { 1 },
        shieldedAccount: Int = 0,
        paymentAccount: Int = 0,
        recipient: ByteArray = ByteArray(43) { 2 },
        amount: Long = 1_000L,
        memo: String? = null,
        signer: Long = DUMMY_SIGNER,
    ): DashSDKException {
        NativeLoader.ensureLoaded()
        return assertThrows(DashSDKException::class.java) {
            FundingNative.shieldedShieldToRecipient(
                DEAD_HANDLE,
                walletId,
                shieldedAccount,
                paymentAccount,
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
    fun shouldRejectNonPositiveAmountBeforeTheFfi() {
        for (amount in longArrayOf(0L, -1L)) {
            assertJniRejection(callExpectingThrow(amount = amount), "amount")
        }
    }

    @Test
    fun shouldRejectNegativeAccountsBeforeTheFfi() {
        assertJniRejection(callExpectingThrow(shieldedAccount = -1), "shieldedAccount")
        assertJniRejection(callExpectingThrow(paymentAccount = -1), "paymentAccount")
    }

    @Test
    fun shouldRejectANullSignerBeforeTheFfi() {
        assertJniRejection(callExpectingThrow(signer = 0L), "signerAddressHandle")
    }

    @Test
    fun shouldRejectMisSizedIdsBeforeTheFfi() {
        assertJniRejection(callExpectingThrow(walletId = ByteArray(31)), "walletId")
        assertJniRejection(callExpectingThrow(recipient = ByteArray(42)), "recipientRaw43")
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
}
