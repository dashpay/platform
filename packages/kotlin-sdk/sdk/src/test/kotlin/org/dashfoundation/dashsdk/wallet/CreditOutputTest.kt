package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.CreditOutput
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

/**
 * Pins the host-side shape check on a Platform-address transfer recipient:
 * only what the wire cannot carry is refused here. Which address types are
 * supported is platform-wallet's decision.
 */
class CreditOutputTest {

    @Test
    fun shouldNotConstructAnOutputWhoseAddressTypeDoesNotFitAByte() {
        // The outputs blob carries the type as one byte: 256 would reach
        // Rust as 0 (P2PKH) and send the credits to another address.
        listOf(-1, 256, 257, Int.MAX_VALUE).forEach { type ->
            assertThrows(IllegalArgumentException::class.java) {
                CreditOutput(addressType = type, hash = ByteArray(20), credits = 500)
            }
        }
    }

    @Test
    fun shouldConstructAnOutputForEveryByteAddressType() {
        listOf(0, 1, 255).forEach { type ->
            assertEquals(type, CreditOutput(addressType = type, hash = ByteArray(20), credits = 500).addressType)
        }
    }
}
