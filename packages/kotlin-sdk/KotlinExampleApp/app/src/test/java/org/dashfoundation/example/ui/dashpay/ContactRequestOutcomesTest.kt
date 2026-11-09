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
     * until the wallet's readiness says the store was read.
     */
    @Test
    fun shouldKeepARefusedContactPendingUntilTheReadinessSaysTheStoreWasRead() {
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

        // A pending read that finds nothing does not lift it; the wallet's
        // readiness saying the store was read to its end does.
        snapshot = snapshot.recorded(emptySet(), listOf("01"))
        assertTrue(snapshot.isPending("01"))
        snapshot = snapshot.withReadiness(readiness(restored = true, storeRead = true))
        assertFalse(snapshot.isPending("01"))
        assertTrue(snapshot.allowsSubmission("01"))
    }

    private fun readiness(restored: Boolean, storeRead: Boolean, unresolved: Boolean = false) =
        org.dashfoundation.dashsdk.tokens.Dashpay.PrivateContactReadiness(
            privatePaymentsRestored = restored,
            privateStoreRead = storeRead,
            unresolvedPrivatePayment = unresolved,
        )

    /** Unrestored private payments (code 72) are their own outcome, apart from a pending request (71). */
    @Test
    fun shouldTellUnrestoredPrivatePaymentsFromAPendingRequest() {
        assertEquals(
            ContactRequestSubmissionOutcome.PrivatePaymentsNotRestored,
            ContactRequestSubmissionOutcome.of(
                DashSdkError.PlatformWallet.PrivateContactPaymentsNotRestored("not restored"),
            ),
        )
        assertEquals(
            ContactRequestSubmissionOutcome.RESTORING_PRIVATE_PAYMENTS_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(pending = true, restoring = true, error = null),
        )
    }

    /**
     * A code 72 hold survives a successful pending read that finds nothing,
     * and a readiness read after a private sync that still says not
     * restored; only a readiness read that says restored lifts it.
     */
    @Test
    fun shouldHoldOnCode72UntilTheReadinessSaysRestored() {
        var snapshot = PrivateRepliesPendingSnapshot().withRestoringRefusal().recorded(emptySet(), listOf("01"))
        assertTrue(snapshot.isRestoringPrivatePayments)
        assertFalse(snapshot.allowsSubmission("01"))

        // A private sync read the store to its end; the payments are still not restored.
        snapshot = snapshot.withReadiness(readiness(restored = false, storeRead = true, unresolved = true))
        assertTrue(snapshot.isRestoringPrivatePayments)
        assertFalse(snapshot.allowsSubmission("01"))

        // A failed readiness read keeps what the last one said.
        snapshot = snapshot.withReadiness(null)
        assertTrue(snapshot.isRestoringPrivatePayments)

        snapshot = snapshot.withReadiness(readiness(restored = true, storeRead = true))
        assertFalse(snapshot.isRestoringPrivatePayments)
        assertTrue(snapshot.allowsSubmission("01"))
    }

    /** A readiness that says not restored holds every contact without any refusal. */
    @Test
    fun shouldShowRestoringFromTheReadinessAlone() {
        val snapshot = PrivateRepliesPendingSnapshot()
            .recorded(emptySet(), listOf("01"))
            .withReadiness(readiness(restored = false, storeRead = false))
        assertTrue(snapshot.isRestoringPrivatePayments)
        assertFalse(snapshot.allowsSubmission("01"))
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
