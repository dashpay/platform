package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Port of `ContactRequestOutcomesTests.swift`: an unconfirmed private reply
 * stays pending (no fresh submission is offered), and a network that does
 * not serve the private store yet is quiet while real failures are shown.
 */
class ContactRequestOutcomesTest {

    @Test
    fun shouldKeepAnUnconfirmedPrivateSubmissionPending() {
        val error = DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("result could not be confirmed")
        assertEquals(
            ContactRequestSubmissionOutcome.MayHaveGoneThrough,
            ContactRequestSubmissionOutcome.of(error),
        )
    }

    @Test
    fun shouldKeepAPendingPrivateRequestPending() {
        val error = DashSdkError.PlatformWallet.PrivateContactRequestPending("not yet read back from the store")
        assertEquals(
            ContactRequestSubmissionOutcome.MayHaveGoneThrough,
            ContactRequestSubmissionOutcome.of(error),
        )
    }

    @Test
    fun shouldShowAWalletPendingReplyOverAnEarlierError() {
        assertEquals(
            ContactRequestSubmissionOutcome.MAY_HAVE_GONE_THROUGH_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(pending = true, error = "Accept failed: boom"),
        )
        assertEquals(
            "Accept failed: boom",
            ContactRequestSubmissionOutcome.rowMessage(pending = false, error = "Accept failed: boom"),
        )
        assertNull(ContactRequestSubmissionOutcome.rowMessage(pending = false, error = null))
    }

    @Test
    fun shouldOfferARetryForAnOrdinaryFailure() {
        val error = DashSdkError.PlatformWallet.ShieldedBroadcastFailed("rejected by the relay")
        assertEquals(
            ContactRequestSubmissionOutcome.Failed("rejected by the relay"),
            ContactRequestSubmissionOutcome.of(error),
        )
    }

    @Test
    fun shouldStayQuietWhenTheNetworkDoesNotServeThePrivateStoreYet() {
        assertNull(
            PrivateContactRefreshNotice.notice(
                DashSdkError.PlatformWallet.ProtocolVersionNotActive("needs protocol version 15"),
            ),
        )
    }

    @Test
    fun shouldShowAPrivateRefreshThatFailed() {
        assertEquals(
            "Private requests could not be read: no node answered",
            PrivateContactRefreshNotice.notice(DashSdkError.PlatformWallet.WalletOperation("no node answered")),
        )
    }

    @Test
    fun shouldShowRequestsThatWereReadButNotSaved() {
        assertNull(PrivateContactRefreshNotice.notice(unpersistedIdentityCount = 0))
        assertEquals(
            "Private requests for 2 identities were read but not saved. Pull to refresh again.",
            PrivateContactRefreshNotice.notice(unpersistedIdentityCount = 2),
        )
    }
}
