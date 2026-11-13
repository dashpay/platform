package org.dashfoundation.dashsdk.credits

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ExternalIdentityTopUpTest {

    private val identityId = ByteArray(32) { 7 }
    private val inputs = listOf(
        FundingInput(addressType = 0, hash = ByteArray(20) { 1 }, credits = 50_000L),
        FundingInput(addressType = 1, hash = ByteArray(20) { 2 }, credits = 25_000L),
    )

    @Test
    fun `should pass the handles, id and encoded inputs to the external export and return its balance`() = runTest {
        var seen: List<Any>? = null
        val credits = IdentityCredits(
            externalTopUpNative = AddressTopUpNativeCall { wallet, id, blob, signer ->
                seen = listOf(wallet, id, blob, signer)
                9_000L
            },
        )

        val balance = credits.topUpExternalIdentityFromAddresses(11L, identityId, inputs, 22L)

        assertEquals(9_000L, balance)
        val call = checkNotNull(seen)
        assertEquals(11L, call[0])
        assertArrayEquals(identityId, call[1] as ByteArray)
        assertArrayEquals(FundingInput.encode(inputs), call[2] as ByteArray)
        assertEquals(22L, call[3])
    }

    @Test
    fun `should map an unknown identity to NotFound`() = runTest {
        val credits = IdentityCredits(
            externalTopUpNative = AddressTopUpNativeCall { _, _, _, _ ->
                throw DashSDKException(
                    DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 7, // ErrorIdentityNotFound
                    "identity not found",
                )
            },
        )

        val failure = runCatching {
            credits.topUpExternalIdentityFromAddresses(1L, identityId, inputs, 2L)
        }.exceptionOrNull()

        assertTrue("expected NotFound, got $failure", failure is DashSdkError.NotFound)
    }

    @Test
    fun `should leave empty inputs to Rust rather than refusing them in Kotlin`() = runTest {
        var blob: ByteArray? = null
        val credits = IdentityCredits(
            externalTopUpNative = AddressTopUpNativeCall { _, _, b, _ ->
                blob = b
                0L
            },
        )

        credits.topUpExternalIdentityFromAddresses(1L, identityId, emptyList(), 2L)

        assertArrayEquals(FundingInput.encode(emptyList()), blob)
    }
}
