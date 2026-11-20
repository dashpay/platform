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

class ManagedIdentityTopUpTest {

    private val identityId = ByteArray(32) { 5 }
    private val inputs = listOf(
        FundingInput(addressType = 0, hash = ByteArray(20) { 3 }, credits = 40_000L),
    )

    @Test
    fun `should pass the handles, id and encoded inputs to the managed export and return its balance`() = runTest {
        var seen: List<Any>? = null
        val credits = IdentityCredits(
            topUpNative = AddressTopUpNativeCall { wallet, id, blob, signer ->
                seen = listOf(wallet, id, blob, signer)
                7_000L
            },
        )

        val balance = credits.topUpFromAddresses(11L, identityId, inputs, 22L)

        assertEquals(7_000L, balance)
        val call = checkNotNull(seen)
        assertEquals(11L, call[0])
        assertArrayEquals(identityId, call[1] as ByteArray)
        assertArrayEquals(FundingInput.encode(inputs), call[2] as ByteArray)
        assertEquals(22L, call[3])
    }

    @Test
    fun `should map an unavailable managed balance to a retryable IdentityBalanceUnavailable`() = runTest {
        val credits = IdentityCredits(
            topUpNative = AddressTopUpNativeCall { _, _, _, _ ->
                throw DashSDKException(
                    DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 58, // ErrorIdentityBalanceUnavailable
                    "no balance",
                )
            },
        )

        val failure = runCatching {
            credits.topUpFromAddresses(1L, identityId, inputs, 2L)
        }.exceptionOrNull()

        assertTrue(
            "expected IdentityBalanceUnavailable, got $failure",
            failure is DashSdkError.PlatformWallet.IdentityBalanceUnavailable,
        )
        assertTrue((failure as DashSdkError).isRetryable)
    }

    @Test
    fun `should leave empty inputs to Rust rather than refusing them in Kotlin`() = runTest {
        var blob: ByteArray? = null
        val credits = IdentityCredits(
            topUpNative = AddressTopUpNativeCall { _, _, b, _ ->
                blob = b
                0L
            },
        )

        credits.topUpFromAddresses(1L, identityId, emptyList(), 2L)

        assertArrayEquals(FundingInput.encode(emptyList()), blob)
    }

    @Test
    fun `should return a managed top-up that completes after the caller was cancelled`() = runBlocking {
        for (gate in listOf(TeardownGate(), null)) {
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            val received = CompletableDeferred<Long>()
            val credits = IdentityCredits(
                gate = gate,
                topUpNative = AddressTopUpNativeCall { _, _, _, _ ->
                    entered.countDown()
                    // The native top-up does not observe cancellation; it pays
                    // the identity only after the caller has been cancelled.
                    release.await()
                    7_000L
                },
            )

            val call = launch(Dispatchers.Default) {
                received.complete(credits.topUpFromAddresses(1L, identityId, inputs, 2L))
            }
            assertTrue("native top-up never started", entered.await(5, TimeUnit.SECONDS))
            call.cancel()
            release.countDown()
            withTimeout(5_000) { call.join() }

            assertTrue("completed top-up was dropped", received.isCompleted)
            assertEquals(7_000L, received.await())
            gate?.let { withTimeout(5_000) { it.closeAndAwait() } }
        }
    }
}
