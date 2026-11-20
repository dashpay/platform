package org.dashfoundation.dashsdk.credits

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.wallet.TeardownGate
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
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
        val failure = failureFor(7) // ErrorIdentityNotFound

        assertTrue("expected NotFound, got $failure", failure is DashSdkError.NotFound)
    }

    @Test
    fun `should map a failed identity lookup to a retryable IdentityLookupFailed`() = runTest {
        val failure = failureFor(80) // ErrorIdentityLookupFailed

        assertTrue(
            "expected IdentityLookupFailed, got $failure",
            failure is DashSdkError.PlatformWallet.IdentityLookupFailed,
        )
        assertTrue((failure as DashSdkError).isRetryable)
    }

    @Test
    fun `should map an unproved result to TransactionBroadcastUnconfirmed`() = runTest {
        val failure = failureFor(20) // ErrorTransactionBroadcastUnconfirmed

        assertTrue(
            "expected TransactionBroadcastUnconfirmed, got $failure",
            failure is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
        )
    }

    @Test
    fun `should return a top-up that completes after the caller was cancelled`() = runBlocking {
        for (gate in listOf(TeardownGate(), null)) {
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            val received = CompletableDeferred<Long>()
            val credits = IdentityCredits(
                gate = gate,
                externalTopUpNative = AddressTopUpNativeCall { _, _, _, _ ->
                    entered.countDown()
                    // The native top-up does not observe cancellation; it pays
                    // the identity only after the caller has been cancelled.
                    release.await()
                    9_000L
                },
            )

            val call = launch(Dispatchers.Default) {
                received.complete(
                    credits.topUpExternalIdentityFromAddresses(1L, identityId, inputs, 2L),
                )
            }
            assertTrue("native top-up never started", entered.await(5, TimeUnit.SECONDS))
            call.cancel()
            release.countDown()
            withTimeout(5_000) { call.join() }

            assertTrue("completed top-up was dropped", received.isCompleted)
            assertEquals(9_000L, received.await())
            gate?.let { withTimeout(5_000) { it.closeAndAwait() } }
        }
    }

    private suspend fun failureFor(platformWalletCode: Int): Throwable? {
        val credits = IdentityCredits(
            externalTopUpNative = AddressTopUpNativeCall { _, _, _, _ ->
                throw DashSDKException(
                    DashSdkError.PLATFORM_WALLET_CODE_OFFSET + platformWalletCode,
                    "native failure $platformWalletCode",
                )
            },
        )
        return runCatching {
            credits.topUpExternalIdentityFromAddresses(1L, identityId, inputs, 2L)
        }.exceptionOrNull()
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
