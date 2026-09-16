package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Binding-level coverage for the `coreWalletSetGapLimit` JNI export — the
 * same no-wallet discipline as [CoreTxBuilderOpReturnBindingTest]: prove the
 * Kotlin external declaration, the generated JNI symbol, and the parameter
 * descriptor stay in lockstep (a naming/signature mismatch surfaces here as
 * `UnsatisfiedLinkError`, not in production), and pin each trampoline
 * validation branch to the exception it throws. No network, no wallet, no
 * funds.
 *
 * Two rejection layers are asserted apart:
 *  - the JNI trampoline's own parameter validation throws
 *    [DashSDKException] with RAW code 1 (the rs-sdk-ffi InvalidParameter
 *    code) and a branch-naming message, BEFORE any FFI call;
 *  - a well-formed call with a dead handle crosses into
 *    `core_wallet_set_gap_limit`, whose storage miss comes back translated
 *    into the platform-wallet code range
 *    (>= [DashSdkError.PLATFORM_WALLET_CODE_OFFSET]) — proof the JNI
 *    validations passed and execution reached the underlying FFI's
 *    invalid-handle path.
 *
 * The branch-naming message assertions double as parameter-order pins: the
 * three ints share one JNI descriptor slot type, so a swapped argument
 * order in either declaration would misroute a probe into the wrong
 * validation branch and fail the message check.
 */
@RunWith(AndroidJUnit4::class)
class CoreWalletSetGapLimitBindingTest {

    private fun callExpectingThrow(
        handle: Long,
        accountType: Int,
        accountIndex: Int,
        gapLimit: Int,
    ): DashSDKException {
        NativeLoader.ensureLoaded()
        return assertThrows(DashSDKException::class.java) {
            WalletManagerNative.coreWalletSetGapLimit(handle, accountType, accountIndex, gapLimit)
        }
    }

    @Test
    fun allSpendableAggregateIsRejectedBeforeTheFfi() {
        // 3 = AllSpendable: it pools several accounts and has no address
        // pool of its own, so the trampoline rejects it up front rather
        // than letting the per-account FFI fail opaquely.
        val e = callExpectingThrow(0L, accountType = 3, accountIndex = 0, gapLimit = 100)
        assertEquals("JNI-side parameter rejection carries raw code 1", 1, e.code)
        assertTrue(
            "the rejection must name the accountType branch, got: ${e.message}",
            e.message.orEmpty().contains("accountType"),
        )
    }

    @Test
    fun unknownAccountTypeIsRejectedBeforeTheFfi() {
        // Outside the mapped range entirely — the mapping's `None` arm
        // shares the AllSpendable rejection.
        val e = callExpectingThrow(0L, accountType = 42, accountIndex = 0, gapLimit = 100)
        assertEquals(1, e.code)
        assertTrue(
            "the rejection must name the accountType branch, got: ${e.message}",
            e.message.orEmpty().contains("accountType"),
        )
    }

    @Test
    fun negativeAccountIndexIsRejectedBeforeTheFfi() {
        val e = callExpectingThrow(0L, accountType = 0, accountIndex = -1, gapLimit = 100)
        assertEquals(1, e.code)
        assertTrue(
            "the rejection must name the accountIndex branch, got: ${e.message}",
            e.message.orEmpty().contains("accountIndex"),
        )
    }

    @Test
    fun nonPositiveGapLimitIsRejectedBeforeTheFfi() {
        // 0 would freeze the address frontier and a negative jint would
        // otherwise bit-cast to a huge u32 — both stop at the boundary.
        for (gap in intArrayOf(0, -1)) {
            val e = callExpectingThrow(0L, accountType = 0, accountIndex = 0, gapLimit = gap)
            assertEquals("gapLimit $gap", 1, e.code)
            assertTrue(
                "the rejection must name the gapLimit branch for $gap, got: ${e.message}",
                e.message.orEmpty().contains("gapLimit"),
            )
        }
    }

    @Test
    fun concreteAccountTypesReachTheFfiInvalidHandlePath() {
        // 0 BIP44, 1 BIP32, 2 CoinJoin — every concrete arm of the
        // trampoline's account-type mapping must pass validation and cross
        // into `core_wallet_set_gap_limit`, where handle 0 can never be a
        // live core wallet. The FFI's miss comes back translated into the
        // platform-wallet code range — NOT the trampoline's raw code 1 —
        // which proves the call left the JNI layer and the concrete arms
        // are wired through.
        for (accountType in intArrayOf(0, 1, 2)) {
            val e = callExpectingThrow(0L, accountType, accountIndex = 0, gapLimit = 100)
            assertTrue(
                "type $accountType must fail inside the FFI (translated code >= " +
                    "${DashSdkError.PLATFORM_WALLET_CODE_OFFSET}), got ${e.code}: ${e.message}",
                e.code >= DashSdkError.PLATFORM_WALLET_CODE_OFFSET,
            )
        }
    }
}
