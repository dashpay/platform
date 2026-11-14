package org.dashfoundation.dashsdk.credits

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
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
}
