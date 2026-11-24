package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityBalance
import org.dashfoundation.dashsdk.wallet.ShieldedTipRecipient
import org.dashfoundation.dashsdk.wallet.ShieldedTipRecipientChange
import org.dashfoundation.dashsdk.wallet.ShieldedTipSource
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator.Key
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator.Kind
import org.dashfoundation.example.services.shielded.ShieldedIdentityPaymentCoordinator.State
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * How KotlinExampleApp reports a shielded tip to a DashPay username, and
 * that its send cannot be made twice from a recreated screen. The wallet
 * decides every tip rule; these tests pin only how the app words and routes
 * what the wallet returned.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ShieldedTipPresentationTest {

    private val identityId = ByteArray(32) { 1 }
    private val recipient = ShieldedTipRecipient(ByteArray(32) { 2 }, ByteArray(43) { 3 })

    @Test
    fun `should report a returned tip as submitted, never as complete`() {
        assertEquals(ShieldedPaymentOutcome.Submitted, ShieldedTipMessages.outcome(null))
        val text = ShieldedTipMessages.submittedText(150_000_000_000)
        assertTrue(text.contains("submitted"))
        assertTrue(text.contains("1.5 DASH"))
    }

    @Test
    fun `should not offer a resend for a tip that may have gone through`() {
        for (error in listOf(
            DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("not proved"),
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("not confirmed"),
            DashSdkError.StateTransitionResultUnconfirmed("result lost"),
        )) {
            assertEquals(ShieldedPaymentOutcome.MayHaveGoneThrough, ShieldedTipMessages.outcome(error))
        }
        assertTrue(ShieldedTipMessages.mayHaveGoneThroughText(1).contains("Do not send it again"))
    }

    @Test
    fun `should word a tip the wallet refused as pending, with nothing sent`() {
        val outcome = ShieldedTipMessages.outcome(DashSdkError.PlatformWallet.ShieldedTipPending("held"))
        assertEquals(ShieldedPaymentOutcome.Failed(ShieldedTipMessages.PENDING), outcome)
        assertTrue(ShieldedTipMessages.PENDING.startsWith("Nothing was sent"))
    }

    @Test
    fun `should word an identity the wallet does not hold, and keep a removed wallet's own message`() {
        assertEquals(
            ShieldedPaymentOutcome.Failed(ShieldedTipMessages.IDENTITY_NOT_HELD),
            ShieldedTipMessages.outcome(DashSdkError.NotFound("no identity")),
        )
        assertEquals(
            ShieldedPaymentOutcome.Failed("wallet removed"),
            ShieldedTipMessages.outcome(DashSdkError.PlatformWallet.NotFound("wallet removed")),
        )
    }

    @Test
    fun `should show the wallet's own message for any other refusal`() {
        assertEquals(
            ShieldedPaymentOutcome.Failed("recipient changed since the review"),
            ShieldedTipMessages.outcome(
                DashSdkError.PlatformWallet.WalletOperation("recipient changed since the review"),
            ),
        )
        assertNull(ShieldedTipMessages.refusal(DashSdkError.PlatformWallet.WalletOperation("x")))
    }

    @Test
    fun `should rethrow a cancellation instead of reporting an outcome`() {
        try {
            ShieldedTipMessages.outcome(CancellationException("gone"))
            fail("a cancellation must not become an outcome")
        } catch (_: CancellationException) {
        }
    }

    @Test
    fun `should never show an unknown identity balance as zero`() {
        assertEquals("Not read yet", ShieldedTipMessages.identityBalanceText(null))
        val unverified = ShieldedTipMessages.identityBalanceText(ShieldedIdentityBalance.Unverified)
        assertTrue(unverified.contains("prepared"))
        val unavailable = ShieldedTipMessages.identityBalanceText(ShieldedIdentityBalance.Unavailable(7))
        assertTrue(unavailable.contains("account 7"))
        for (text in listOf(unverified, unavailable)) assertFalse(text.contains("0 DASH"))
        assertEquals(
            "0.00000000001 DASH",
            ShieldedTipMessages.identityBalanceText(ShieldedIdentityBalance.Available(7, 1)),
        )
        assertEquals("Not known yet", ShieldedTipMessages.mainBalanceText(null))
        assertEquals("0 DASH", ShieldedTipMessages.mainBalanceText(0))
    }

    @Test
    fun `should word each recipient change the wallet reports`() {
        assertTrue(ShieldedTipMessages.changeText(ShieldedTipRecipientChange.New).contains("First tip"))
        assertTrue(ShieldedTipMessages.changeText(ShieldedTipRecipientChange.Unchanged).contains("Same recipient"))
        val changed = ShieldedTipMessages.changeText(ShieldedTipRecipientChange.Changed(recipient))
        assertTrue(changed.contains("Changed"))
        assertTrue(changed.contains("Previously"))
    }

    @Test
    fun `should pass the picked account to the wallet as it is`() {
        assertEquals(ShieldedTipSource.Identity(identityId), ShieldedTipSourceChoice.IDENTITY.source(identityId))
        assertEquals(
            ShieldedTipSource.Account(SHIELDED_TIP_MAIN_ACCOUNT),
            ShieldedTipSourceChoice.MAIN_ACCOUNT.source(identityId),
        )
    }

    @Test
    fun `should refuse a second tip from a recreated screen while the first is proving`() = runTest {
        val coordinator = ShieldedIdentityPaymentCoordinator(TestScope(UnconfinedTestDispatcher(testScheduler)))
        val key = Key(Kind.SHIELDED_TIP, "aa", "bb")
        val proof = CompletableDeferred<ShieldedPaymentOutcome>()
        var sends = 0

        assertTrue(coordinator.start(key, 5) { sends++; proof.await() })
        // A rotation or tab switch recreates the screen; its tap pays nothing.
        assertFalse(coordinator.start(key, 5) { sends++; ShieldedPaymentOutcome.Submitted })
        assertEquals(1, sends)
        // A shield or top-up of the same identity is its own slot.
        assertTrue(coordinator.start(Key(Kind.TOP_UP_FROM_POOL, "aa", "bb"), 1) { ShieldedPaymentOutcome.Submitted })

        proof.complete(ShieldedTipMessages.outcome(DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("lost")))
        assertEquals(State.Done(5, ShieldedPaymentOutcome.MayHaveGoneThrough), coordinator.states.value[key])
        // An outcome not shown yet still blocks a new tip.
        assertFalse(coordinator.start(key, 5) { sends++; ShieldedPaymentOutcome.Submitted })
        assertEquals(1, sends)

        coordinator.acknowledge(key)
        assertNull(coordinator.states.value[key])
    }
}
