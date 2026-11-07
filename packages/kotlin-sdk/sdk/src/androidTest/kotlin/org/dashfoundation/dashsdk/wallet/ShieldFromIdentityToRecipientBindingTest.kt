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
 *    lookup and comes back as `ErrorInvalidHandle`.
 *
 * The FFI encodes the memo and resolves the manager handle before it ever
 * dereferences the signer pointer, so [DUMMY_SIGNER] is never read; it only
 * has to be non-zero to pass the JNI null check.
 */
@RunWith(AndroidJUnit4::class)
class ShieldFromIdentityToRecipientBindingTest {

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
        identityId: ByteArray = ByteArray(32) { 3 },
        recipient: ByteArray = ByteArray(43) { 2 },
        amount: Long = 1_000L,
        memo: String? = null,
        signer: Long = DUMMY_SIGNER,
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
}
