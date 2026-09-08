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
    fun malformedTypedDetailFailsClosed() {
        val error = DashSdkError.fromNative(
            DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 39, "not-json"),
        )
        assertTrue(error is DashSdkError.PlatformWallet.Generic)
        assertEquals(39, (error as DashSdkError.PlatformWallet.Generic).nativeCode)
    }
}
