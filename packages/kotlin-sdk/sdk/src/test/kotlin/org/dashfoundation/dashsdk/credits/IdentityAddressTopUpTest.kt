package org.dashfoundation.dashsdk.credits

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.wallet.TeardownGate
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentityAddressTopUpTest {

    /**
     * The top-up of any identity hands its arguments, the funding rows
     * encoded, to its own JNI bridge and returns the balance it reports;
     * whether the identity is the wallet's own is not decided here.
     */
    @Test
    fun `top up of any identity marshals to its own bridge and returns its balance`() = runTest {
        val calls = mutableListOf<List<Any>>()
        val credits = IdentityCredits(
            addressTopUpNative = AddressTopUpNativeCall { wallet, identity, blob, signer ->
                calls += listOf(wallet, identity, blob, signer)
                777L
            },
        )
        val identityId = ByteArray(32) { 9 }
        val inputs = listOf(
            FundingInput(addressType = 0, hash = ByteArray(20) { 7 }, credits = 1_000),
            FundingInput(addressType = 1, hash = ByteArray(20) { 8 }, credits = 7),
        )

        val balance = credits.topUpIdentityFromAddresses(
            walletHandle = 42,
            identityId = identityId,
            inputs = inputs,
            signerHandle = 5,
        )

        assertEquals(777L, balance)
        assertEquals(1, calls.size)
        val (wallet, identity, blob, signer) = calls.single()
        assertEquals(42L, wallet)
        assertArrayEquals(identityId, identity as ByteArray)
        assertArrayEquals(FundingInput.encode(inputs), blob as ByteArray)
        assertEquals(5L, signer)
    }

    /**
     * A top-up that completed natively after its caller was cancelled still
     * hands the balance back, so the caller can tell the credits moved and
     * does not retry the payment. Holds for the deprecated managed-only
     * route too.
     */
    @Suppress("DEPRECATION")
    @Test
    fun `a cancelled caller still receives a completed top-up's balance`() = runBlocking {
        for (route in listOf(TopUpRoute.ANY_IDENTITY, TopUpRoute.MANAGED_DEPRECATED))
        for (gate in listOf(TeardownGate(), null)) {
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            val received = CompletableDeferred<Long>()
            val native = AddressTopUpNativeCall { _, _, _, _ ->
                entered.countDown()
                // The blocking JNI call does not observe cancellation; it
                // completes only after the caller has been cancelled.
                release.await()
                777L
            }
            val unused = AddressTopUpNativeCall { _, _, _, _ -> error("wrong bridge") }
            val credits = when (route) {
                TopUpRoute.ANY_IDENTITY ->
                    IdentityCredits(gate = gate, addressTopUpNative = native, managedAddressTopUpNative = unused)
                TopUpRoute.MANAGED_DEPRECATED ->
                    IdentityCredits(gate = gate, addressTopUpNative = unused, managedAddressTopUpNative = native)
            }
            val identityId = ByteArray(32) { 9 }
            val inputs = listOf(FundingInput(addressType = 0, hash = ByteArray(20) { 7 }, credits = 1_000))

            val op = launch(Dispatchers.Default) {
                val balance = when (route) {
                    TopUpRoute.ANY_IDENTITY ->
                        credits.topUpIdentityFromAddresses(42, identityId, inputs, 5)
                    TopUpRoute.MANAGED_DEPRECATED ->
                        credits.topUpFromAddresses(42, identityId, inputs, 5)
                }
                received.complete(balance)
            }
            try {
                assertTrue("native top-up never started", entered.await(5, TimeUnit.SECONDS))
            } finally {
                // Always release the simulated native call: a failed start
                // assertion would otherwise leave it blocked and hang runBlocking.
                op.cancel()
                release.countDown()
            }
            withTimeout(5_000) { op.join() }

            assertTrue(op.isCancelled)
            assertTrue("$route: completed top-up balance was dropped", received.isCompleted)
            assertEquals(777L, received.await())
            gate?.let { withTimeout(5_000) { it.closeAndAwait() } }
        }
    }

    /**
     * A funding row whose address type does not fit the wire's one byte is
     * refused where it is built, so it never reaches either bridge truncated
     * to another type; every byte value is passed through for Rust to judge.
     */
    @Test
    fun `an address type outside a byte never reaches the native bridge`() {
        for (outOfRange in listOf(-1, 256, 257, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) {
                FundingInput(addressType = outOfRange, hash = ByteArray(20) { 7 }, credits = 1_000)
            }
        }
        val edges = listOf(0, 255).map { FundingInput(addressType = it, hash = ByteArray(20) { 7 }, credits = 1_000) }
        val blob = FundingInput.encode(edges)
        assertEquals(0, blob[4].toInt() and 0xFF)
        assertEquals(255, blob[4 + 29].toInt() and 0xFF)
    }

    /**
     * The outcomes the top-up's KDoc tells callers apart reach them as those
     * types: an unknown identity and a failed lookup were refused before
     * anything was sent (the lookup retryable), an unproved top-up must stay
     * pending and never be retried, and a removed wallet sent nothing.
     */
    @Test
    fun `the top-up of any identity reports each native outcome as its documented type`() = runTest {
        val unknown = failureFor(TopUpRoute.ANY_IDENTITY, 7) // ErrorIdentityNotFound
        assertTrue("7: got $unknown", unknown is DashSdkError.NotFound)

        val lookup = failureFor(TopUpRoute.ANY_IDENTITY, 80) // ErrorIdentityLookupFailed
        assertTrue("80: got $lookup", lookup is DashSdkError.PlatformWallet.IdentityLookupFailed)
        assertTrue("80 must be retryable", (lookup as DashSdkError).isRetryable)

        val unconfirmed = failureFor(TopUpRoute.ANY_IDENTITY, 20) // ErrorTransactionBroadcastUnconfirmed
        assertTrue(
            "20: got $unconfirmed",
            unconfirmed is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
        )
        assertTrue("20 must not be retryable", !(unconfirmed as DashSdkError).isRetryable)

        val removed = failureFor(TopUpRoute.ANY_IDENTITY, 98) // NotFound: wallet removed before send
        assertTrue("98: got $removed", removed is DashSdkError.PlatformWallet.NotFound)
    }

    /**
     * The deprecated managed-only route reports Platform not showing the
     * wallet's identity yet as the retryable balance-unavailable outcome.
     */
    @Test
    fun `the deprecated managed top-up reports an unavailable balance as retryable`() = runTest {
        val failure = failureFor(TopUpRoute.MANAGED_DEPRECATED, 58) // ErrorIdentityBalanceUnavailable

        assertTrue("58: got $failure", failure is DashSdkError.PlatformWallet.IdentityBalanceUnavailable)
        assertTrue("58 must be retryable", (failure as DashSdkError).isRetryable)
    }

    /** Empty inputs are Rust's to refuse; Kotlin passes them through. */
    @Test
    fun `empty inputs reach the native bridge for Rust to judge`() = runTest {
        var blob: ByteArray? = null
        val credits = IdentityCredits(
            addressTopUpNative = AddressTopUpNativeCall { _, _, b, _ ->
                blob = b
                0L
            },
        )

        credits.topUpIdentityFromAddresses(42, ByteArray(32) { 9 }, emptyList(), 5)

        assertArrayEquals(FundingInput.encode(emptyList()), blob)
    }

    @Suppress("DEPRECATION")
    private suspend fun failureFor(route: TopUpRoute, platformWalletCode: Int): Throwable? {
        val failing = AddressTopUpNativeCall { _, _, _, _ ->
            throw DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + platformWalletCode,
                "native failure $platformWalletCode",
            )
        }
        val unused = AddressTopUpNativeCall { _, _, _, _ -> error("wrong bridge") }
        val identityId = ByteArray(32) { 9 }
        val inputs = listOf(FundingInput(addressType = 0, hash = ByteArray(20) { 7 }, credits = 1_000))
        return runCatching {
            when (route) {
                TopUpRoute.ANY_IDENTITY ->
                    IdentityCredits(addressTopUpNative = failing, managedAddressTopUpNative = unused)
                        .topUpIdentityFromAddresses(42, identityId, inputs, 5)
                TopUpRoute.MANAGED_DEPRECATED ->
                    IdentityCredits(addressTopUpNative = unused, managedAddressTopUpNative = failing)
                        .topUpFromAddresses(42, identityId, inputs, 5)
            }
        }.exceptionOrNull()
    }

    private enum class TopUpRoute { ANY_IDENTITY, MANAGED_DEPRECATED }
}
