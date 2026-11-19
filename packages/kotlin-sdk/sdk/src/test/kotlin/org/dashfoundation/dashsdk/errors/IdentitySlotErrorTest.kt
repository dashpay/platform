package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** Codes 77-78 from the identity-slot lease (`rs-platform-wallet-ffi/src/error.rs`). */
@RunWith(RobolectricTestRunner::class)
class IdentitySlotErrorTest {
    private fun map(code: Int, message: String) =
        DashSdkError.fromNative(DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + code, message))

    @Test
    fun shouldReportASlotNotHeldWithItsIndex() {
        val notHeld = map(77, "{\"identityIndex\":3}")
        assertTrue(notHeld is DashSdkError.PlatformWallet.IdentitySlotNotHeld)
        assertEquals(3, (notHeld as DashSdkError.PlatformWallet.IdentitySlotNotHeld).identityIndex)
        assertFalse(notHeld.isRetryable)
    }

    @Test
    fun shouldReportASlotTakenOnChainWithItsIndexAndIdentity() {
        val id = "8eTDkBhpQjHeqgbVeriwLeZr1tCa6yBGw76SckvD1cwc"
        val taken = map(78, "{\"identityIndex\":2,\"identityId\":\"$id\"}")
        assertTrue(taken is DashSdkError.PlatformWallet.IdentitySlotTakenOnChain)
        taken as DashSdkError.PlatformWallet.IdentitySlotTakenOnChain
        assertEquals(2, taken.identityIndex)
        assertEquals(id, taken.identityId)
        assertFalse("the same index is never retried", taken.isRetryable)
    }

    @Test
    fun shouldKeepAMalformedSlotPayloadGeneric() {
        assertTrue(map(77, "not json") is DashSdkError.PlatformWallet.Generic)
        assertTrue(map(77, "{\"identityIndex\":4294967295}") is DashSdkError.PlatformWallet.Generic)
        assertTrue(map(78, "{\"identityIndex\":2}") is DashSdkError.PlatformWallet.Generic)
        assertTrue(map(78, "{\"identityIndex\":2,\"identityId\":null}") is DashSdkError.PlatformWallet.Generic)
    }
}
