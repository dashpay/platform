package org.dashfoundation.example.ui.shielded

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolBalance
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolSpendKind
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * What the Shielded Tokens screens show and read (SH-25, SH-26, SH-27), apart
 * from Compose. Ports the cases the Swift screens settled in
 * dashpay/platform#6116, #6132 and #6133: amounts are whole numbers of the
 * token's smallest unit passed unchanged, recipients are read as the wallet
 * calls take them, and an accepted spend is reported as submitted, never as
 * complete.
 */
class ShieldedTokenPoolsTest {

    private fun balance(
        token: Byte,
        account: Int,
        scanned: ULong? = null,
        pool: ULong? = null,
        height: ULong? = null,
    ) = ShieldedTokenPoolBalance(
        tokenId = ByteArray(32) { token },
        dataContractId = ByteArray(32) { 4 },
        tokenContractPosition = 0,
        accountIndex = account,
        balance = 10uL,
        unspentNoteCount = 1u,
        reservedBalance = 0uL,
        scannedNoteCount = scanned,
        poolNoteCount = pool,
        scannedHeight = height,
        lastError = null,
    )

    @Test
    fun `groups rows by token in the order Rust listed them`() {
        val groups = groupShieldedTokenBalances(
            listOf(balance(2, 0), balance(1, 0), balance(2, 1)),
        )

        assertEquals(2, groups.size)
        assertArrayEquals(ByteArray(32) { 2 }, groups[0].tokenId)
        assertEquals(listOf(0, 1), groups[0].accounts.map { it.accountIndex })
        assertArrayEquals(ByteArray(32) { 1 }, groups[1].tokenId)
        assertTrue(groupShieldedTokenBalances(emptyList()).isEmpty())
    }

    @Test
    fun `describes the scan state as Rust reports it`() {
        assertEquals("Not scanned yet", shieldedTokenScanText(balance(1, 0)))
        assertEquals("Scanned 3 notes", shieldedTokenScanText(balance(1, 0, scanned = 3uL)))
        assertEquals(
            "Scanned 3 of 8 notes (height 120)",
            shieldedTokenScanText(balance(1, 0, scanned = 3uL, pool = 8uL, height = 120uL)),
        )
        assertEquals("1 unspent note", shieldedTokenNoteCountText(1u))
        assertEquals("2 unspent notes", shieldedTokenNoteCountText(2u))
        assertEquals("7 (smallest units)", shieldedTokenAmountText(7uL, token = null))
    }

    @Test
    fun `reads only a whole number of smallest units, unchanged`() {
        assertEquals(1_500uL, shieldedTokenSmallestUnitAmount("1500"))
        assertEquals(ULong.MAX_VALUE, shieldedTokenSmallestUnitAmount("18446744073709551615"))
        for (refused in listOf("", "1.5", "-1", "+1", "1e3", "1 000", "١٢", "18446744073709551616")) {
            assertNull("'$refused'", shieldedTokenSmallestUnitAmount(refused))
        }
    }

    @Test
    fun `reads an Orchard recipient only on its own network`() {
        val raw = ByteArray(43) { (it + 1).toByte() }
        val testnet = DashAddress.encodeOrchard(raw, Network.TESTNET) ?: error("encodable")
        val mainnet = DashAddress.encodeOrchard(raw, Network.MAINNET) ?: error("encodable")

        assertArrayEquals(raw, shieldedTokenOrchardRecipient("  $testnet\n", Network.TESTNET))
        assertNull(shieldedTokenOrchardRecipient(mainnet, Network.TESTNET))
        assertNull(shieldedTokenOrchardRecipient("", Network.TESTNET))
        assertNull(shieldedTokenOrchardRecipient("not an address", Network.TESTNET))
    }

    @Test
    fun `reads a Base58 identity id of 32 bytes and nothing else`() {
        val id = ByteArray(32) { (it * 7).toByte() }
        val encoded = Base58.encode(id)

        assertArrayEquals(id, shieldedTokenIdentityRecipient(" $encoded "))
        assertNull(shieldedTokenIdentityRecipient(""))
        assertNull("too short", shieldedTokenIdentityRecipient(Base58.encode(ByteArray(20) { 1 })))
        assertNull("not Base58", shieldedTokenIdentityRecipient("0OIl"))
        assertNull("longer than 44 characters", shieldedTokenIdentityRecipient(encoded + "1111"))
    }

    @Test
    fun `reports an accepted spend as submitted, never as complete`() = runTest {
        for (kind in ShieldedTokenPoolSpendKind.entries) {
            val result = shieldedTokenSpendResult(kind) { }

            assertFalse(result.mayHaveGoneThrough)
            assertSame(SHIELDED_TOKEN_SPEND_SUBMITTED, result.message)
            assertTrue(result.message.startsWith("Submitted, pending until the next shielded sync"))
            assertFalse(result.message.contains("complete", ignoreCase = true))
            assertFalse(result.message.contains("confirmed", ignoreCase = true))
        }
    }

    @Test
    fun `reports a spend that may have reached a node without inviting a retry`() = runTest {
        val transfer = shieldedTokenSpendResult(ShieldedTokenPoolSpendKind.ShieldedTransfer) {
            throw DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("result not proved")
        }
        val unshield = shieldedTokenSpendResult(ShieldedTokenPoolSpendKind.Unshield) {
            throw DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("no answer")
        }

        assertTrue(transfer.mayHaveGoneThrough)
        assertTrue(transfer.message.startsWith("The transfer may have gone through"))
        assertTrue(transfer.message.contains("Do not retry"))
        assertTrue(unshield.mayHaveGoneThrough)
        assertTrue(unshield.message.startsWith("The unshield may have gone through"))
    }

    @Test
    fun `passes on an error that means nothing was sent`() = runTest {
        val refused = DashSdkError.PlatformWallet.WalletOperation("insufficient shielded token balance")
        try {
            shieldedTokenSpendResult(ShieldedTokenPoolSpendKind.ShieldedTransfer) { throw refused }
            fail("an error that sent nothing must reach the form's error line")
        } catch (e: DashSdkError) {
            assertSame(refused, e)
        }
    }

    @Test
    fun `describes the fee preview's notes`() {
        assertEquals(
            "Spends 1 token note; 1 credit note pays the fee.",
            shieldedTokenFeeNotesText(1u, 1u),
        )
        assertEquals(
            "Spends 2 token notes; 3 credit notes pay the fee.",
            shieldedTokenFeeNotesText(2u, 3u),
        )
    }

    @Test
    fun `token transfer and unshield rows say what happened only once confirmed`() {
        assertEquals("Token Transfer", kindLabel(KIND_TOKEN_TRANSFER))
        assertEquals("Token Unshield", kindLabel(KIND_TOKEN_UNSHIELD))
        assertEquals(11, KIND_TOKEN_TRANSFER)
        assertEquals(12, KIND_TOKEN_UNSHIELD)

        assertTrue(tokenTransferNote(1).startsWith("The tokens were sent"))
        assertTrue(tokenUnshieldNote(1).startsWith("The tokens were added"))
        for (unsettled in listOf(0, 2, 7)) {
            assertFalse(tokenTransferNote(unsettled).startsWith("The tokens were sent"))
            assertFalse(tokenUnshieldNote(unsettled).startsWith("The tokens were added"))
        }
        assertTrue(supportingText(KIND_TOKEN_TRANSFER, 0).startsWith("Pending. Not confirmed yet"))
        assertTrue(supportingText(KIND_TOKEN_UNSHIELD, 2).startsWith("Failed. The unshield did not go through"))
    }

    @Test
    fun `token rows show their amount in token units, credit rows in DASH`() {
        assertEquals("−1500 token units", signedAmount(KIND_TOKEN_TRANSFER, direction = 1, amount = 1_500))
        assertEquals("−7 token units", signedAmount(KIND_TOKEN_UNSHIELD, direction = 1, amount = 7))
        assertEquals("−1.0000 DASH", signedAmount(KIND_TOKEN_PURCHASE, direction = 1, amount = 100_000_000_000))
        assertTrue(amountIsTokens(KIND_TOKEN_TRANSFER))
        assertFalse(amountIsTokens(KIND_TOKEN_PURCHASE))
    }
}
