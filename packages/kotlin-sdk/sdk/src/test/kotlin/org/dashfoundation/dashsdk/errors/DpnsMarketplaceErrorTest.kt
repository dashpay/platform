package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class DpnsMarketplaceErrorTest {
    @Test
    fun mapsPriceChangedDetail() {
        val error = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 38,
                """{"documentId":"abc","expected":1000,"actual":2000}""",
            ),
        )

        assertTrue(error is DashSdkError.PlatformWallet.DocumentPriceChanged)
        error as DashSdkError.PlatformWallet.DocumentPriceChanged
        assertEquals("abc", error.documentId)
        assertEquals(1_000uL, error.expectedCredits)
        assertEquals(2_000uL, error.actualCredits)
    }

    @Test
    fun shouldDecodeInsufficientCreditsAboveTheSignedRangeExactly() {
        // Unquoted u64::MAX, as Rust's serde_json writes it. Android's org.json
        // read this as a Double, and the mapping fell back to Generic.
        val error = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 39,
                """{"identityId":"id","required":18446744073709551615,"available":9223372036854775808}""",
            ),
        )

        assertTrue(error is DashSdkError.PlatformWallet.InsufficientIdentityCredits)
        error as DashSdkError.PlatformWallet.InsufficientIdentityCredits
        assertEquals("id", error.identityId)
        assertEquals(ULong.MAX_VALUE, error.requiredCredits)
        assertEquals(9_223_372_036_854_775_808uL, error.availableCredits)
    }

    @Test
    fun shouldMapContestedNameDetail() {
        val error = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 40,
                """{"label":"alice","endsAtMs":1700000000000}""",
            ),
        )

        assertTrue(error is DashSdkError.PlatformWallet.ContestedNameNotTradable)
        error as DashSdkError.PlatformWallet.ContestedNameNotTradable
        assertEquals("alice", error.label)
        assertEquals(1_700_000_000_000L, error.endsAtMs)
    }

    @Test
    fun shouldFailClosedWhenATypedDetailStringIsNotAString() {
        val error = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 39,
                """{"identityId":null,"required":1,"available":0}""",
            ),
        )
        assertTrue(error is DashSdkError.PlatformWallet.Generic)
    }

    @Test
    fun malformedTypedDetailFailsClosed() {
        val error = DashSdkError.fromNative(
            DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 39, "not-json"),
        )
        assertTrue(error is DashSdkError.PlatformWallet.Generic)
        assertEquals(39, (error as DashSdkError.PlatformWallet.Generic).nativeCode)
    }
}
