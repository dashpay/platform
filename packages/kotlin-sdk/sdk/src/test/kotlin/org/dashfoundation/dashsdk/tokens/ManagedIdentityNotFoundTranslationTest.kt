package org.dashfoundation.dashsdk.tokens

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertEquals
import org.junit.Assert.fail
import org.junit.Test

/**
 * Pins the dashpay/platform#4051 fix: `TokensNative.getManagedIdentity`
 * reports an identity the wallet does not manage as a platform-wallet
 * NotFound error (code 98, via the FFI's blanket `Option → result`
 * conversion), which [translateManagedIdentityNotFoundToZero] turns into a
 * zero handle so [Dashpay.syncState] / [Dashpay.payments] /
 * [Dashpay.contacts] return null / empty instead of throwing a typed
 * `DashSdkError.NotFound("…ManagedIdentity not found")`.
 */
class ManagedIdentityNotFoundTranslationTest {

    private val notFoundCode =
        DashSdkError.PLATFORM_WALLET_CODE_OFFSET + DashSdkError.PLATFORM_WALLET_NOT_FOUND_CODE

    @Test
    fun notFoundBecomesAZeroHandle() {
        val handle = translateManagedIdentityNotFoundToZero {
            throw DashSDKException(
                notFoundCode,
                "requested platform_wallet::identity::ManagedIdentity not found",
            )
        }
        assertEquals(0L, handle)
    }

    @Test
    fun aRealHandlePassesThrough() {
        assertEquals(42L, translateManagedIdentityNotFoundToZero { 42L })
    }

    @Test
    fun otherNativeErrorsAreRethrownUntouched() {
        val other = DashSDKException(
            DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 1, // ErrorInvalidHandle
            "stale wallet handle",
        )
        try {
            translateManagedIdentityNotFoundToZero { throw other }
            fail("expected the non-NotFound error to be rethrown")
        } catch (e: DashSDKException) {
            assertEquals(other.code, e.code)
            assertEquals(other.message, e.message)
        }
    }
}
