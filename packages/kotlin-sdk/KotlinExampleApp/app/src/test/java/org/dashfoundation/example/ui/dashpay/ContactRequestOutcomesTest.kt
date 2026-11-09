package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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

    /**
     * A pending private request (code 71), which a public send or accept can
     * meet too, sent nothing: it is shown as pending, with a private sync to
     * settle it, rather than as a submission that may have gone through.
     */
    @Test
    fun shouldKeepAPendingPrivateRequestPending() {
        val error = DashSdkError.PlatformWallet.PrivateContactRequestPending("not yet read back from the store")
        assertEquals(
            ContactRequestSubmissionOutcome.PrivateRequestPending,
            ContactRequestSubmissionOutcome.of(error),
        )
    }

    @Test
    fun shouldShowAWalletPendingReplyOverAnEarlierError() {
        assertEquals(
            ContactRequestSubmissionOutcome.PRIVATE_REQUEST_PENDING_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(pending = true, error = "Accept failed: boom"),
        )
        assertEquals(
            "Accept failed: boom",
            ContactRequestSubmissionOutcome.rowMessage(pending = false, error = "Accept failed: boom"),
        )
        assertEquals(
            PrivateRepliesPendingSnapshot.UNAVAILABLE_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(
                pending = false,
                unavailable = true,
                error = "Accept failed: boom",
            ),
        )
        assertNull(ContactRequestSubmissionOutcome.rowMessage(pending = false, error = null))
    }

    /**
     * A read that found a private request pending, followed by a failed
     * read: the contact stays pending and nothing is offered for any
     * contact, until a read succeeds again.
     */
    @Test
    fun shouldKeepThePendingSnapshotAndOfferNothingAfterAFailedRead() {
        val contacts = listOf("01", "02")
        var snapshot = PrivateRepliesPendingSnapshot().recorded(setOf("01"), contacts)
        assertTrue(snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertTrue(snapshot.allowsSubmission("02"))

        snapshot = snapshot.recorded(null, contacts)
        assertTrue(snapshot.isUnavailable)
        assertTrue("the last snapshot stays", snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertFalse(snapshot.allowsSubmission("02"))

        snapshot = snapshot.recorded(emptySet(), contacts)
        assertFalse(snapshot.isUnavailable)
        assertTrue(snapshot.allowsSubmission("01"))
    }

    /**
     * A send refused as pending (code 71) when the wallet reserved nothing
     * (its store was not read to its end): the following read finds nothing
     * pending, yet the contact stays pending, with the private sync offered,
     * until a private sync completes.
     */
    @Test
    fun shouldKeepARefusedContactPendingAfterAnEmptyReadUntilAPrivateSync() {
        var snapshot = PrivateRepliesPendingSnapshot()
            .withRefusal("01")
            .recorded(emptySet(), listOf("01"))
        assertFalse(snapshot.isUnavailable)
        assertTrue(snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals(
            ContactRequestSubmissionOutcome.PRIVATE_REQUEST_PENDING_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(pending = snapshot.isPending("01"), error = null),
        )

        snapshot = snapshot.afterPrivateSync().recorded(emptySet(), listOf("01"))
        assertFalse(snapshot.isPending("01"))
        assertTrue(snapshot.allowsSubmission("01"))
    }

    /**
     * A successful read that found nothing pending offers every contact it
     * covered, and nothing it did not.
     */
    @Test
    fun shouldOfferEveryContactASuccessfulEmptyReadCovered() {
        val snapshot = PrivateRepliesPendingSnapshot().recorded(emptySet(), listOf("01"))
        assertFalse(snapshot.isUnavailable)
        assertFalse(snapshot.isPending("01"))
        assertTrue(snapshot.allowsSubmission("01"))
        assertFalse(
            "a contact the read did not cover is not offered yet",
            snapshot.allowsSubmission("02"),
        )
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
