package org.dashfoundation.example.ui.tokens

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPurchaseAvailability
import org.dashfoundation.example.ui.shielded.KIND_TOKEN_PURCHASE
import org.dashfoundation.example.ui.shielded.kindLabel
import org.dashfoundation.example.ui.shielded.supportingText
import org.dashfoundation.example.ui.shielded.tokenPurchaseNote
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * What the Direct Purchase form shows and reports when paying from the
 * shielded pool (transition type 28). Port of the shielded cases the Swift
 * form's review settled in dashpay/platform#6090: the shielded payment is
 * offered only on Rust's "available" answer, and a purchase is never
 * reported as complete.
 */
class ShieldedTokenPurchaseTest {

    private fun loaded(availability: ShieldedTokenPurchaseAvailability) =
        ShieldedPurchaseAvailabilityState.Loaded(availability)

    @Test
    fun `offers the shielded payment only on Rust's available answer`() {
        assertTrue(loaded(ShieldedTokenPurchaseAvailability.Available).offersShieldedPayment)
        assertFalse(loaded(ShieldedTokenPurchaseAvailability.NoShieldedPool).offersShieldedPayment)
        assertFalse(loaded(ShieldedTokenPurchaseAvailability.ConfigurationUnreadable).offersShieldedPayment)
        assertFalse(ShieldedPurchaseAvailabilityState.Loading.offersShieldedPayment)
        assertFalse(ShieldedPurchaseAvailabilityState.Failed.offersShieldedPayment)
        assertFalse(ShieldedPurchaseAvailabilityState.Unsupported.offersShieldedPayment)
    }

    @Test
    fun `says why the shielded payment is not offered and when asking again may help`() {
        assertNull(shieldedUnavailableNote(loaded(ShieldedTokenPurchaseAvailability.Available)))

        val noPool = shieldedUnavailableNote(loaded(ShieldedTokenPurchaseAvailability.NoShieldedPool))!!
        assertTrue(noPool.text.contains("no shielded pool"))
        assertFalse("a token without a pool will not gain one by asking again", noPool.offersRetry)
        assertFalse(noPool.isError)

        // An unreadable configuration and a question that could not be asked
        // are not "this token has no pool": both offer a retry.
        for (state in listOf(
            loaded(ShieldedTokenPurchaseAvailability.ConfigurationUnreadable),
            ShieldedPurchaseAvailabilityState.Failed,
        )) {
            val note = shieldedUnavailableNote(state)!!
            assertTrue(note.text.contains("Couldn't read this token's configuration"))
            assertTrue(note.offersRetry)
            assertTrue(note.isError)
        }

        val checking = shieldedUnavailableNote(ShieldedPurchaseAvailabilityState.Loading)!!
        assertFalse(checking.offersRetry)

        val unsupported = shieldedUnavailableNote(ShieldedPurchaseAvailabilityState.Unsupported)!!
        assertTrue(unsupported.text.contains("no shielded support"))
        assertFalse("another build is needed, not another try", unsupported.offersRetry)
    }

    @Test
    fun `enables Buy only with an available token, a wallet and a known non-zero balance`() {
        val available = loaded(ShieldedTokenPurchaseAvailability.Available)
        assertTrue(canSubmitShieldedPurchase(available, hasWallet = true, shieldedBalance = 1))
        assertFalse("no wallet", canSubmitShieldedPurchase(available, hasWallet = false, shieldedBalance = 1))
        assertFalse("balance not known yet", canSubmitShieldedPurchase(available, true, null))
        assertFalse("nothing to spend", canSubmitShieldedPurchase(available, true, 0))
        assertFalse(
            "not offered by Rust",
            canSubmitShieldedPurchase(
                loaded(ShieldedTokenPurchaseAvailability.NoShieldedPool), true, 1_000_000,
            ),
        )
        assertFalse(canSubmitShieldedPurchase(ShieldedPurchaseAvailabilityState.Loading, true, 1_000_000))
    }

    @Test
    fun `reports an accepted purchase as submitted, never as complete`() = runTest {
        val message = shieldedPurchaseOutcomeMessage { }

        assertSame(SHIELDED_PURCHASE_SUBMITTED, message)
        assertTrue(message.contains("submitted"))
        assertTrue(message.contains("a shielded sync confirms it"))
        assertTrue(message.contains("Do not retry"))
        assertFalse(message.contains("complete", ignoreCase = true))
        assertFalse(message.contains("confirmed", ignoreCase = true))
    }

    @Test
    fun `reports a purchase that may have reached a node without inviting a retry`() = runTest {
        for (unconfirmed in listOf(
            DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("result not proved"),
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("no answer"),
        )) {
            val message = shieldedPurchaseOutcomeMessage { throw unconfirmed }
            assertSame(SHIELDED_PURCHASE_MAY_HAVE_GONE_THROUGH, message)
            assertTrue(message.contains("Do not retry"))
        }
    }

    @Test
    fun `passes on an error that means nothing was sent`() = runTest {
        val refused = DashSdkError.PlatformWallet.WalletOperation("insufficient shielded balance")
        try {
            shieldedPurchaseOutcomeMessage { throw refused }
            fail("an error that sent nothing must reach the form's error dialog")
        } catch (e: DashSdkError) {
            assertSame(refused, e)
        }
    }

    @Test
    fun `adds the fee to the agreed total only when both are known`() {
        assertEquals(1_500uL, priceWithFee(1_000uL, 500))
        assertNull(priceWithFee(null, 500))
        assertNull(priceWithFee(1_000uL, null))
        assertNull("an overflowing total is not shown", priceWithFee(ULong.MAX_VALUE, 1))
        assertNull(priceWithFee(1_000uL, -1))
    }

    @Test
    fun `an activity row says tokens were minted only once it is confirmed`() {
        assertEquals("Token Purchase", kindLabel(KIND_TOKEN_PURCHASE))
        assertEquals(10, KIND_TOKEN_PURCHASE)

        assertTrue(tokenPurchaseNote(1).startsWith("The tokens were minted"))
        for (unsettled in listOf(0, 2, 7)) {
            assertFalse(
                "status $unsettled",
                tokenPurchaseNote(unsettled).contains("The tokens were minted"),
            )
        }
        assertTrue(tokenPurchaseNote(0).contains("Not confirmed yet"))
        assertTrue(tokenPurchaseNote(2).contains("no tokens were minted"))

        assertTrue(supportingText(KIND_TOKEN_PURCHASE, 0).startsWith("Pending. Not confirmed yet"))
        assertNotNull(supportingText(KIND_TOKEN_PURCHASE, 1))
        assertEquals("another kind keeps its bare status", "Pending", supportingText(3, 0))
    }
}
