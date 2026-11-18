package org.dashfoundation.example.ui.funding

import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * The resume form of an external asset lock lists the recipients the lock
 * recorded before the user confirms: every payee with its explicit credits,
 * and the remainder that returns to this wallet, in the recorded order.
 */
class RecordedPayeeLinesTest {

    @Test
    fun listsEachPayeeWithItsAmountAndTheRemainder() {
        val payee = ManagedPlatformWallet.FundRecipient(0, ByteArray(20) { 0x11 }, 5_000L)
        val remainder = ManagedPlatformWallet.FundRecipient(0, ByteArray(20) { 0x22 }, null)

        assertEquals(
            listOf(
                "1111111111111111… — 5000 credits",
                "2222222222222222… — remainder (this wallet)",
            ),
            recordedPayeeLines(listOf(payee, remainder)),
        )
    }
}
