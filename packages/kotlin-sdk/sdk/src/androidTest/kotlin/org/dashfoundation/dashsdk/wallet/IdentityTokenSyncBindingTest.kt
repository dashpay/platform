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
 * Binding-level coverage for the `identitySyncRegisterIdentity` and
 * `identitySyncSyncNow` JNI exports, with the no-wallet discipline of
 * [CoreWalletSetGapLimitBindingTest]: a symbol or descriptor mismatch fails
 * here as `UnsatisfiedLinkError`, each trampoline validation throws raw code
 * 1 naming its argument, and a well-formed call with a dead handle reaches
 * the FFI's invalid-handle path (a code in the platform-wallet range). No
 * network, no wallet.
 */
@RunWith(AndroidJUnit4::class)
class IdentityTokenSyncBindingTest {

    private fun register(identityId: ByteArray, tokenIds: ByteArray): DashSDKException {
        NativeLoader.ensureLoaded()
        return assertThrows(DashSDKException::class.java) {
            WalletManagerNative.identitySyncRegisterIdentity(0L, identityId, tokenIds)
        }
    }

    @Test
    fun shortIdentityIdIsRejectedBeforeTheFfi() {
        val e = register(ByteArray(31), ByteArray(32))
        assertEquals(1, e.code)
        assertTrue(
            "the rejection must name identityId, got: ${e.message}",
            e.message.orEmpty().contains("identityId"),
        )
    }

    @Test
    fun tokenIdsNotAMultipleOf32AreRejectedBeforeTheFfi() {
        val e = register(ByteArray(32), ByteArray(33))
        assertEquals(1, e.code)
        assertTrue(
            "the rejection must name tokenIds, got: ${e.message}",
            e.message.orEmpty().contains("tokenIds"),
        )
    }

    @Test
    fun wellFormedRegistrationReachesTheFfiInvalidHandlePath() {
        // Two token ids and none: both pass the trampoline and fail only on
        // the dead manager handle inside the FFI.
        for (tokenIds in listOf(ByteArray(64), ByteArray(0))) {
            val e = register(ByteArray(32), tokenIds)
            assertTrue(
                "${tokenIds.size / 32} token ids must fail inside the FFI (code >= " +
                    "${DashSdkError.PLATFORM_WALLET_CODE_OFFSET}), got ${e.code}: ${e.message}",
                e.code >= DashSdkError.PLATFORM_WALLET_CODE_OFFSET,
            )
        }
    }

    @Test
    fun syncNowReachesTheFfiInvalidHandlePath() {
        NativeLoader.ensureLoaded()
        val e = assertThrows(DashSDKException::class.java) {
            WalletManagerNative.identitySyncSyncNow(0L)
        }
        assertTrue(
            "syncNow must fail inside the FFI (code >= " +
                "${DashSdkError.PLATFORM_WALLET_CODE_OFFSET}), got ${e.code}: ${e.message}",
            e.code >= DashSdkError.PLATFORM_WALLET_CODE_OFFSET,
        )
    }
}
