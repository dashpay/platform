package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.tokens.Dashpay
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Port of `ContactRequestOutcomesTests.swift`, code by code: an unconfirmed
 * private send or reply (18) stays pending with no fresh submission offered,
 * a held pair (71) is pending until the per-contact read says free,
 * unrestored payments (72) show "setting up" or why the restoration failed,
 * an unsupported store (73) and a pinned SDK (75) are configuration messages,
 * an unfinished private store read (74) is a retry with no pending row, and a
 * network that does not serve the private store yet is quiet while real
 * failures are shown. A public request or accept that may have gone through
 * (20), or one the wallet refuses while an earlier one awaits its outcome
 * (76), shows "being checked" with no submission offered until the wallet's
 * read of the pair says settled.
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
     * Code 20: a state transition the SDK could not confirm after it may have
     * been accepted, which a private send or accept can now return through
     * the spend contract, and a public one through the blanket conversion. It
     * may have executed: nothing is offered again, on a contact request or a
     * shielded send alike.
     */
    @Test
    fun shouldKeepAnUnconfirmedStateTransitionPending() {
        val wallet = DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("result could not be confirmed")
        val sdk = DashSdkError.StateTransitionResultUnconfirmed("result could not be confirmed")
        assertEquals(ContactRequestSubmissionOutcome.MayHaveGoneThrough, ContactRequestSubmissionOutcome.of(wallet))
        assertEquals(ContactRequestSubmissionOutcome.MayHaveGoneThrough, ContactRequestSubmissionOutcome.of(sdk))
        assertTrue(shieldedOperationMayHaveGoneThrough(wallet))
        assertTrue(
            shieldedOperationMayHaveGoneThrough(
                DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("not proved"),
            ),
        )
        assertFalse(
            shieldedOperationMayHaveGoneThrough(
                DashSdkError.PlatformWallet.ShieldedBroadcastFailed("never sent"),
            ),
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
        var snapshot = PrivateRepliesPendingSnapshot().recorded(privatelyHeld("01"), contacts)
        assertTrue(snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertTrue(snapshot.allowsSubmission("02"))

        snapshot = snapshot.recorded(null, contacts)
        assertTrue(snapshot.isUnavailable)
        assertTrue("the last snapshot stays", snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertFalse(snapshot.allowsSubmission("02"))

        snapshot = snapshot.recorded(privatelyHeld(), contacts)
        assertFalse(snapshot.isUnavailable)
        assertTrue(snapshot.allowsSubmission("01"))
    }

    /**
     * A send refused as pending (code 71) marks the contact pending at once;
     * the next read of the pair, which reads what the refusal read, decides:
     * held keeps it pending, free re-enables it. The readiness alone never
     * re-enables it.
     */
    @Test
    fun shouldReEnableARefusedPairOnlyWhenThePerContactReadSaysFree() {
        var snapshot = PrivateRepliesPendingSnapshot()
            .recorded(privatelyHeld(), listOf("01"))
            .withRefusal("01")
        assertTrue(snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals(
            ContactRequestSubmissionOutcome.PRIVATE_REQUEST_PENDING_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(pending = snapshot.isPending("01"), error = null),
        )

        // A readiness that says the store was read and nothing is held for
        // the wallet does not free one pair.
        snapshot = snapshot.withReadiness(readiness(restored = true, storeRead = true))
        assertFalse(snapshot.allowsSubmission("01"))

        // The pair's read says held: still pending.
        snapshot = snapshot.recorded(privatelyHeld("01"), listOf("01"))
        assertTrue(snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))

        // A payment that will never land frees the pair once the wallet
        // proves so; the next read says free and the controls come back.
        snapshot = snapshot.recorded(privatelyHeld(), listOf("01"))
        assertFalse(snapshot.isPending("01"))
        assertTrue(snapshot.allowsSubmission("01"))
    }

    /** A read that does not cover a refused contact leaves its refusal. */
    @Test
    fun shouldKeepARefusalAReadDidNotCover() {
        val snapshot = PrivateRepliesPendingSnapshot()
            .withRefusal("01")
            .recorded(privatelyHeld(), listOf("02"))
        assertTrue(snapshot.isPending("01"))
        assertTrue(snapshot.allowsSubmission("02"))
    }

    /**
     * Code 74: the private store read every contact request runs first did
     * not finish. Nothing was sent and nothing is pending: its own outcome,
     * with a retry message, and no pending row.
     */
    @Test
    fun shouldOfferARetryForAnUnfinishedPrivateStoreRead() {
        assertEquals(
            ContactRequestSubmissionOutcome.PrivateStoreNotRead,
            ContactRequestSubmissionOutcome.of(
                DashSdkError.PlatformWallet.PrivateContactStoreNotRead("not read to its end"),
            ),
        )
        val snapshot = PrivateRepliesPendingSnapshot().recorded(privatelyHeld(), listOf("01"))
        assertFalse("the screen records no refusal for 74", snapshot.isPending("01"))
        assertTrue(snapshot.allowsSubmission("01"))
        assertTrue(
            ContactRequestSubmissionOutcome.PRIVATE_STORE_NOT_READ_MESSAGE !=
                ContactRequestSubmissionOutcome.PRIVATE_REQUEST_PENDING_MESSAGE,
        )
    }

    /** Code 75 is the developer's setting, shown as such, on a send and on a refresh alike. */
    @Test
    fun shouldShowAPinnedSdkAsADeveloperMessage() {
        val error = DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion("pinned at 14")
        assertEquals(
            ContactRequestSubmissionOutcome.Failed(ContactRequestSubmissionOutcome.SDK_PINNED_MESSAGE),
            ContactRequestSubmissionOutcome.of(error),
        )
        assertEquals(ContactRequestSubmissionOutcome.SDK_PINNED_MESSAGE, PrivateContactRefreshNotice.notice(error))
    }

    /**
     * The store opened, but the wallet's journal restoration failed for a
     * transient reason. A DashPay refresh asks the wallet to retry even
     * though no store-open failure is recorded, reads the readiness after the
     * attempt, and once the retry restores the payments the gate lifts: no
     * bind, no restart and no submitted request.
     */
    @Test
    fun shouldRetryARestorationThatFailedAfterTheStoreOpenedOnARefresh() = runTest {
        var restorationFails = true
        var configures = 0
        val read: suspend () -> PrivateContactReadinessRead? = {
            if (restorationFails) {
                PrivateContactReadinessRead(
                    readiness(restored = false, storeRead = true, restoreFailed = true),
                    "the journal could not be read",
                )
            } else {
                PrivateContactReadinessRead(readiness(restored = true, storeRead = true), null)
            }
        }
        var snapshot = PrivateRepliesPendingSnapshot().recorded(privatelyHeld(), listOf("01"))

        // The first refresh: the store opened, the restoration failed, and the
        // retry it asks for fails too.
        var refreshed = PrivatePaymentsRecovery.refresh(snapshot.readiness, null, read) {
            configures++
            null
        }
        snapshot = snapshot.withReadiness(refreshed.read?.readiness, refreshed.read?.restoreFailure)
        assertEquals("not restored asks the wallet to retry with no store failure recorded", 1, configures)
        assertNull(refreshed.storeFailure)
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals("the journal could not be read", snapshot.restoreFailure)
        assertNotNull(PrivatePaymentsRecovery.recheckDelayMs(snapshot.readiness, 0))

        // The failure was transient: the next refresh asks first, the retry
        // restores, and the readiness shown is the one read after it.
        refreshed = PrivatePaymentsRecovery.refresh(snapshot.readiness, null, read) {
            configures++
            restorationFails = false
            null
        }
        snapshot = snapshot.withReadiness(refreshed.read?.readiness, refreshed.read?.restoreFailure)
        assertEquals(2, configures)
        assertFalse(snapshot.isRestoringPrivatePayments)
        assertNull(snapshot.restoreFailure)
        assertTrue("the gate lifts on the refresh alone", snapshot.allowsSubmission("01"))
        assertNull(PrivatePaymentsRecovery.recheckDelayMs(snapshot.readiness, 1))
    }

    /**
     * A refresh asks for the retry before it reads the readiness it shows:
     * a screen that last showed "not restored" asks first and shows only the
     * read made after it, so recovery without an isBound change re-enables
     * the existing screen.
     */
    @Test
    fun shouldShowTheReadinessReadAfterTheRetry() = runTest {
        val events = mutableListOf<String>()
        var restored = false
        val refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness = readiness(restored = false, storeRead = true),
            storeFailure = null,
            readReadiness = {
                events += "read"
                PrivateContactReadinessRead(readiness(restored = restored, storeRead = true), null)
            },
            configureAgain = {
                events += "configure"
                restored = true
                null
            },
        )
        assertEquals(listOf("configure", "read"), events)
        assertEquals(true, refreshed.read?.readiness?.privatePaymentsRestored)
        val shown = PrivateRepliesPendingSnapshot()
            .withReadiness(readiness(restored = false, storeRead = true))
            .recorded(privatelyHeld(), listOf("01"))
            .withReadiness(refreshed.read?.readiness, refreshed.read?.restoreFailure)
        assertTrue(shown.allowsSubmission("01"))
    }

    /** Restored payments and an open store need no retry: one read. */
    @Test
    fun shouldNotAskTheWalletToRetryWhenThePaymentsAreRestored() = runTest {
        var configures = 0
        var reads = 0
        val refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness = readiness(restored = true, storeRead = true),
            storeFailure = null,
            readReadiness = {
                reads++
                PrivateContactReadinessRead(readiness(restored = true, storeRead = true), null)
            },
            configureAgain = {
                configures++
                null
            },
        )
        assertEquals(0, configures)
        assertEquals(1, reads)
        assertEquals(true, refreshed.read?.readiness?.privatePaymentsRestored)
    }

    /**
     * Opening the store succeeds on the refresh after a failed open: the
     * failure clears, whatever the readiness says.
     */
    @Test
    fun shouldOpenTheStoreAgainWhileAStoreFailureIsRecorded() = runTest {
        var configures = 0
        val refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness = null,
            storeFailure = "disk full",
            readReadiness = { null },
            configureAgain = {
                configures++
                null
            },
        )
        assertEquals(1, configures)
        assertNull(refreshed.storeFailure)
    }

    /**
     * A restoration the wallet deferred behind a scan that holds the store for
     * minutes: the screen keeps refreshing past the first minute (every 3 s
     * for 20 refreshes, then every 15 s, with no last one), and the refresh
     * after the deferred restoration completes re-enables submission and
     * schedules no more.
     */
    @Test
    fun shouldKeepRefreshingWhileARestorationStaysDeferred() = runTest {
        var deferredDone = false
        val read: suspend () -> PrivateContactReadinessRead? = {
            PrivateContactReadinessRead(readiness(restored = deferredDone, storeRead = true), null)
        }
        var snapshot = PrivateRepliesPendingSnapshot().recorded(privatelyHeld(), listOf("01"))
        val pauses = mutableListOf<Long>()
        var refreshes = 0
        // Well past the first minute: 200 automatic refreshes, about 46 minutes.
        for (attempt in 0 until 200) {
            val refreshed = PrivatePaymentsRecovery.refresh(snapshot.readiness, null, read) { null }
            snapshot = snapshot.withReadiness(refreshed.read?.readiness, refreshed.read?.restoreFailure)
            refreshes++
            if (attempt == 150) deferredDone = true
            if (!PrivatePaymentsRecovery.waitForRecheck(snapshot.readiness, attempt) { pauses += it }) break
        }
        assertEquals("it kept refreshing until the restoration completed", 152, refreshes)
        assertTrue(snapshot.allowsSubmission("01"))
        assertEquals("no pause once the payments read restored", 151, pauses.size)
        assertEquals(List(20) { PrivatePaymentsRecovery.FAST_RECHECK_INTERVAL_MS }, pauses.take(20))
        assertEquals(setOf(PrivatePaymentsRecovery.SLOW_RECHECK_INTERVAL_MS), pauses.drop(20).toSet())
    }

    /**
     * The timer stops when the payments read restored, before any read
     * succeeded, and when the screen leaves the composition (its wait is
     * cancelled), however many refreshes came before.
     */
    @Test
    fun shouldStopRefreshingWhenRestoredOrWhenTheScreenGoesAway() = runTest {
        var slept = false
        assertFalse(PrivatePaymentsRecovery.waitForRecheck(readiness(restored = true, storeRead = true), 0) { slept = true })
        assertFalse("nothing is due once restored", slept)
        assertFalse(PrivatePaymentsRecovery.waitForRecheck(null, 0) { slept = true })
        assertFalse(slept)
        var refreshedAfter = false
        try {
            if (PrivatePaymentsRecovery.waitForRecheck(readiness(restored = false, storeRead = true), 500) {
                    throw CancellationException("the screen left the composition")
                }
            ) {
                refreshedAfter = true
            }
        } catch (e: CancellationException) {
            // The screen's effect ends with its cancelled wait.
        }
        assertFalse("a screen that went away refreshes no more", refreshedAfter)
    }

    private fun readiness(
        restored: Boolean,
        storeRead: Boolean,
        unresolved: Boolean = false,
        restoreFailed: Boolean = false,
    ) = Dashpay.PrivateContactReadiness(
        privatePaymentsRestored = restored,
        privateStoreRead = storeRead,
        unresolvedPrivatePayment = unresolved,
        privatePaymentsRestoreFailed = restoreFailed,
    )

    /** A store that cannot protect private payments (code 73) is a configuration error, shown as such. */
    @Test
    fun shouldShowUnsupportedPrivateRecoveryAsAStorageError() {
        assertEquals(
            ContactRequestSubmissionOutcome.Failed(ContactRequestSubmissionOutcome.RECOVERY_UNSUPPORTED_MESSAGE),
            ContactRequestSubmissionOutcome.of(
                DashSdkError.PlatformWallet.PrivateContactRecoveryUnsupported("no atomic scan state"),
            ),
        )
    }

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
            ContactRequestSubmissionOutcome.SETTING_UP_PRIVATE_PAYMENTS_MESSAGE,
            ContactRequestSubmissionOutcome.rowMessage(pending = true, restoring = true, error = null),
        )
    }

    /**
     * Code 72 after a failed restoration shows why it failed, read with the
     * readiness, until a readiness says restored; a shielded store that did
     * not open is shown over both.
     */
    @Test
    fun shouldShowWhyARestorationFailedUntilTheReadinessSaysRestored() {
        var snapshot = PrivateRepliesPendingSnapshot()
            .recorded(privatelyHeld(), listOf("01"))
            .withReadiness(
                readiness(restored = false, storeRead = false, restoreFailed = true),
                "the journal does not read",
            )
        assertTrue(snapshot.isRestoringPrivatePayments)
        assertEquals("the journal does not read", snapshot.restoreFailure)
        assertEquals(
            ContactRequestSubmissionOutcome.restoreFailedMessage("the journal does not read"),
            ContactRequestSubmissionOutcome.rowMessage(
                pending = false,
                restoring = snapshot.isRestoringPrivatePayments,
                restoreFailure = snapshot.restoreFailure,
                error = null,
            ),
        )
        assertEquals(
            ContactRequestSubmissionOutcome.shieldedStoreFailedMessage("disk full"),
            ContactRequestSubmissionOutcome.rowMessage(
                pending = false,
                restoring = true,
                restoreFailure = snapshot.restoreFailure,
                shieldedStoreFailure = "disk full",
                error = null,
            ),
        )

        // A readiness that no longer flags a failure drops the reason.
        snapshot = snapshot.withReadiness(readiness(restored = false, storeRead = false), "stale")
        assertNull(snapshot.restoreFailure)

        snapshot = snapshot.withReadiness(readiness(restored = true, storeRead = false))
        assertFalse(snapshot.isRestoringPrivatePayments)
        assertTrue(snapshot.allowsSubmission("01"))
    }

    /**
     * A code 72 hold survives a successful pending read that finds nothing,
     * and a readiness read after a private sync that still says not
     * restored; only a readiness read that says restored lifts it.
     */
    @Test
    fun shouldHoldOnCode72UntilTheReadinessSaysRestored() {
        var snapshot = PrivateRepliesPendingSnapshot().withRestoringRefusal().recorded(privatelyHeld(), listOf("01"))
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
            .recorded(privatelyHeld(), listOf("01"))
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
        val snapshot = PrivateRepliesPendingSnapshot().recorded(privatelyHeld(), listOf("01"))
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

    /**
     * Code 76: an earlier public request to the pair awaits its outcome, so
     * the wallet signed nothing. Not a failure to retry: the contact shows
     * "being checked" and no submission is offered.
     */
    @Test
    fun shouldShowARefusedUnconfirmedPublicRequestAsBeingChecked() {
        val outcome = ContactRequestSubmissionOutcome.of(
            DashSdkError.PlatformWallet.PublicContactRequestUnconfirmed("an earlier request awaits its outcome"),
        )
        assertEquals(ContactRequestSubmissionOutcome.PublicRequestUnconfirmed, outcome)
        val snapshot = PrivateRepliesPendingSnapshot().recorded(privatelyHeld(), listOf("01"))
            .withOutcome(outcome, "01")
        assertTrue(snapshot.isAwaitingOutcome("01"))
        assertFalse("not a private request pending", snapshot.isPending("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals(
            ContactRequestSubmissionOutcome.PUBLIC_REQUEST_UNCONFIRMED_MESSAGE,
            rowMessage(snapshot, "01"),
        )
    }

    /**
     * Code 20 on a public accept, followed by an empty private-pending read
     * and a public sync that did not settle it: an unsuccessful one (the read
     * after it fails) or an incomplete one (the wallet's public hold still
     * stands). The row stays held through both; a second accept is not
     * offered, and one made anyway is refused (76) with no error to retry;
     * once the sync settles the hold the read frees the pair and the accept
     * comes back.
     */
    @Test
    fun shouldKeepAPublicAcceptHeldAfterCode20ThroughAnEmptyPrivateReadAndAnUnsettledSync() = runTest {
        val wallet = FakeContactRequestHolds()
        var snapshot = PrivateRepliesPendingSnapshot().recorded(wallet.read(), listOf("01"))
        assertTrue("the accept is offered", snapshot.allowsSubmission("01"))

        // The accept broadcasts and its result cannot be confirmed: the
        // wallet holds the pair, the signed transition kept.
        wallet.publicUnconfirmed = setOf("01")
        val accepted = ContactRequestSubmissionOutcome.of(
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("result could not be confirmed"),
        )
        assertEquals(ContactRequestSubmissionOutcome.MayHaveGoneThrough, accepted)
        snapshot = snapshot.withOutcome(accepted, "01")
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals(ContactRequestSubmissionOutcome.PUBLIC_REQUEST_UNCONFIRMED_MESSAGE, rowMessage(snapshot, "01"))

        // An unsuccessful sync: the read after it fails, and the row stays
        // held as the last read left it.
        wallet.failure = DashSdkError.PlatformWallet.WalletOperation("no node answered")
        snapshot = snapshot.recorded(wallet.read(), listOf("01"))
        assertTrue(snapshot.isUnavailable)
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals(ContactRequestSubmissionOutcome.PUBLIC_REQUEST_UNCONFIRMED_MESSAGE, rowMessage(snapshot, "01"))

        // An incomplete sync: the private-pending read is empty, the public
        // hold stands, and the row stays held.
        wallet.failure = null
        val reload = wallet.read()
        assertEquals("the private-pending read is empty", emptySet<String>(), reload?.privatelyHeld)
        snapshot = snapshot.recorded(reload, listOf("01"))
        assertTrue(snapshot.isAwaitingOutcome("01"))
        assertFalse("a second accept is not offered", snapshot.allowsSubmission("01"))
        assertEquals(ContactRequestSubmissionOutcome.PUBLIC_REQUEST_UNCONFIRMED_MESSAGE, rowMessage(snapshot, "01"))

        // A second accept made anyway is refused before anything is signed.
        val retried = ContactRequestSubmissionOutcome.of(
            DashSdkError.PlatformWallet.PublicContactRequestUnconfirmed("an earlier request awaits its outcome"),
        )
        snapshot = snapshot.withOutcome(retried, "01").recorded(wallet.read(), listOf("01"))
        assertFalse(snapshot.allowsSubmission("01"))
        assertEquals(
            "a refusal while held is not an error to retry",
            ContactRequestSubmissionOutcome.PUBLIC_REQUEST_UNCONFIRMED_MESSAGE,
            rowMessage(snapshot, "01"),
        )

        // The sync settles the hold (a proved-used nonce frees the pair; a
        // request found is recorded as sent and the row leaves the list): the
        // next read says free and the accept comes back.
        wallet.publicUnconfirmed = emptySet()
        snapshot = snapshot.recorded(wallet.read(), listOf("01"))
        assertFalse(snapshot.isAwaitingOutcome("01"))
        assertTrue(snapshot.allowsSubmission("01"))
        assertNull(rowMessage(snapshot, "01"))
    }

    /** An unconfirmed submission marks only its own contact, and a read that does not cover it leaves the mark. */
    @Test
    fun shouldKeepAnUnconfirmedSubmissionAReadDidNotCover() {
        val snapshot = PrivateRepliesPendingSnapshot()
            .withOutcome(ContactRequestSubmissionOutcome.MayHaveGoneThrough, "01")
            .recorded(privatelyHeld(), listOf("02"))
        assertTrue(snapshot.isAwaitingOutcome("01"))
        assertTrue(snapshot.allowsSubmission("02"))
    }

    /**
     * The screens' read covers both holds: the private one and the public
     * one. A failed read is null, never "nothing held"; a cancelled one is
     * not swallowed.
     */
    @Test
    fun shouldReadBothHoldsAndTellAFailedReadFromNothingHeld() = runTest {
        val wallet = FakeContactRequestHolds()
        wallet.privatelyHeld = setOf("01")
        wallet.publicUnconfirmed = setOf("02")
        assertEquals(ContactRequestHolds(privatelyHeld = setOf("01"), publicUnconfirmed = setOf("02")), wallet.read())

        wallet.failure = DashSdkError.PlatformWallet.WalletOperation("read failed")
        assertNull(wallet.read())

        wallet.failure = CancellationException("left the screen")
        var cancelled = false
        try {
            wallet.read()
        } catch (e: CancellationException) {
            cancelled = true
        }
        assertTrue(cancelled)
    }
}

/** A read of both holds that found [contacts] held for a private request and none awaiting a public request's outcome. */
private fun privatelyHeld(vararg contacts: String): ContactRequestHolds =
    ContactRequestHolds(privatelyHeld = contacts.toSet())

/** The message the screens show for [contact], from [snapshot]. */
private fun rowMessage(snapshot: PrivateRepliesPendingSnapshot, contact: String): String? =
    ContactRequestSubmissionOutcome.rowMessage(
        pending = snapshot.isPending(contact),
        awaitingOutcome = snapshot.isAwaitingOutcome(contact),
        restoring = snapshot.isRestoringPrivatePayments,
        restoreFailure = snapshot.restoreFailure,
        unavailable = snapshot.isUnavailable,
        error = null,
    )

/** The wallet's two per-contact holds, read through the screens' helper ([contactRequestHolds]) as the wallet answers them. */
private class FakeContactRequestHolds {
    var privatelyHeld: Set<String> = emptySet()
    var publicUnconfirmed: Set<String> = emptySet()

    /** Thrown by every read when set. */
    var failure: Exception? = null

    suspend fun read(): ContactRequestHolds? = contactRequestHolds(
        readPrivatelyHeld = {
            failure?.let { throw it }
            privatelyHeld
        },
        readPublicUnconfirmed = {
            failure?.let { throw it }
            publicUnconfirmed
        },
    )
}
