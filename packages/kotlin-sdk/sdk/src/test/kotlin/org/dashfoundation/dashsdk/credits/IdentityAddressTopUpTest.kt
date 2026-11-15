package org.dashfoundation.dashsdk.credits

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.wallet.TeardownGate
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
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
     * does not retry the payment.
     */
    @Test
    fun `a cancelled caller still receives a completed top-up's balance`() = runBlocking {
        for (gate in listOf(TeardownGate(), null)) {
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            val received = CompletableDeferred<Long>()
            val credits = IdentityCredits(
                gate = gate,
                addressTopUpNative = AddressTopUpNativeCall { _, _, _, _ ->
                    entered.countDown()
                    // The blocking JNI call does not observe cancellation; it
                    // completes only after the caller has been cancelled.
                    release.await()
                    777L
                },
            )

            val op = launch(Dispatchers.Default) {
                val balance = credits.topUpIdentityFromAddresses(
                    walletHandle = 42,
                    identityId = ByteArray(32) { 9 },
                    inputs = listOf(FundingInput(addressType = 0, hash = ByteArray(20) { 7 }, credits = 1_000)),
                    signerHandle = 5,
                )
                received.complete(balance)
            }
            assertTrue("native top-up never started", entered.await(5, TimeUnit.SECONDS))
            op.cancel()
            release.countDown()
            withTimeout(5_000) { op.join() }

            assertTrue(op.isCancelled)
            assertTrue("completed top-up balance was dropped", received.isCompleted)
            assertEquals(777L, received.await())
            gate?.let { withTimeout(5_000) { it.closeAndAwait() } }
        }
    }
}
