import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins how the DashPay views read a failed contact request submission and a
/// private-store refresh, code by code: an unconfirmed private send or reply
/// (18) stays pending with no fresh submission offered, a held pair (71) is
/// pending until the per-contact read says free, unrestored payments (72)
/// show "setting up" or why the restoration failed, an unsupported store (73)
/// and a pinned SDK (75) are configuration messages, an unfinished private
/// store read (74) is a retry with no pending row, and a network that does
/// not serve the private store yet is quiet while real failures are shown.
final class ContactRequestOutcomesTests: XCTestCase {

    /// Code 18: a note spend whose broadcast may have reached a node, a
    /// verdict included, may still land; nothing is offered again.
    func testShouldKeepAnUnconfirmedPrivateSubmissionPending() {
        let outcome = ContactRequestSubmissionOutcome.of(
            PlatformWalletError.shieldedSpendUnconfirmed("result could not be confirmed")
        )
        XCTAssertEqual(outcome, .mayHaveGoneThrough)
    }

    /// Code 20: a state transition the SDK could not confirm after it may
    /// have been accepted, which a private send or accept can now return
    /// through the spend contract, and a public one through the blanket
    /// conversion. It may have executed: nothing is offered again.
    func testShouldKeepAnUnconfirmedStateTransitionPending() {
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.of(
                PlatformWalletError.transactionBroadcastUnconfirmed("result could not be confirmed")),
            .mayHaveGoneThrough)
    }

    /// A shielded send treats code 20 as it treats 18: it may have gone
    /// through, so no retry is offered; an ordinary failure is not.
    func testShouldNotOfferARetryForAShieldedOperationThatMayHaveGoneThrough() {
        XCTAssertTrue(SendViewModel.mayHaveGoneThrough(.shieldedSpendUnconfirmed("not proved")))
        XCTAssertTrue(SendViewModel.mayHaveGoneThrough(.transactionBroadcastUnconfirmed("not confirmed")))
        XCTAssertFalse(SendViewModel.mayHaveGoneThrough(.shieldedBroadcastFailed("never sent")))
    }

    /// A pending private request (code 71), which a public send or accept
    /// can meet too, sent nothing: it is shown as pending rather than as a
    /// submission that may have gone through.
    func testShouldKeepAPendingPrivateRequestPending() {
        let outcome = ContactRequestSubmissionOutcome.of(
            PlatformWalletError.privateContactRequestPending("not yet read back from the store")
        )
        XCTAssertEqual(outcome, .privateRequestPending)
    }

    func testShouldShowAWalletPendingReplyOverAnEarlierError() {
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(pending: true, error: "Accept failed: boom"),
            ContactRequestSubmissionOutcome.privateRequestPendingMessage
        )
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(pending: false, error: "Accept failed: boom"),
            "Accept failed: boom"
        )
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(
                pending: false, unavailable: true, error: "Accept failed: boom"),
            PrivateRepliesPendingSnapshot.unavailableMessage
        )
        XCTAssertNil(ContactRequestSubmissionOutcome.rowMessage(pending: false, error: nil))
    }

    /// A read that found a private request pending, followed by a failed
    /// read: the contact stays pending and nothing is offered for any
    /// contact, until a read succeeds again.
    func testShouldKeepThePendingSnapshotAndOfferNothingAfterAFailedRead() {
        let pendingContact = Data(repeating: 1, count: 32)
        let otherContact = Data(repeating: 2, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([pendingContact], for: [pendingContact, otherContact])
        XCTAssertTrue(snapshot.isPending(pendingContact))
        XCTAssertFalse(snapshot.allowsSubmission(to: pendingContact))
        XCTAssertTrue(snapshot.allowsSubmission(to: otherContact))

        snapshot.record(nil, for: [pendingContact, otherContact])
        XCTAssertTrue(snapshot.isUnavailable)
        XCTAssertTrue(snapshot.isPending(pendingContact), "the last snapshot stays")
        XCTAssertFalse(snapshot.allowsSubmission(to: pendingContact))
        XCTAssertFalse(snapshot.allowsSubmission(to: otherContact))

        snapshot.record([], for: [pendingContact, otherContact])
        XCTAssertFalse(snapshot.isUnavailable)
        XCTAssertTrue(snapshot.allowsSubmission(to: pendingContact))
    }

    /// A send refused as pending (code 71) marks the contact pending at once;
    /// the next read of the pair, which reads what the refusal read, decides:
    /// held keeps it pending, free re-enables it. The readiness alone never
    /// re-enables it.
    func testShouldReEnableARefusedPairOnlyWhenThePerContactReadSaysFree() {
        let contact = Data(repeating: 1, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        snapshot.recordRefusal(of: contact)
        XCTAssertTrue(snapshot.isPending(contact))
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(
                pending: snapshot.isPending(contact), error: nil),
            ContactRequestSubmissionOutcome.privateRequestPendingMessage)

        // A readiness that says the store was read and nothing is held for
        // the wallet does not free one pair.
        snapshot.recordReadiness(PrivateContactReadiness(
            privatePaymentsRestored: true, privateStoreRead: true, unresolvedPrivatePayment: false))
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))

        // The pair's read says held: still pending.
        snapshot.record([contact], for: [contact])
        XCTAssertTrue(snapshot.isPending(contact))
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))

        // A payment that will never land frees the pair once the wallet
        // proves so; the next read says free and the controls come back.
        snapshot.record([], for: [contact])
        XCTAssertFalse(snapshot.isPending(contact))
        XCTAssertTrue(snapshot.allowsSubmission(to: contact))
    }

    /// A read that does not cover a refused contact leaves its refusal.
    func testShouldKeepARefusalAReadDidNotCover() {
        let refused = Data(repeating: 1, count: 32)
        let other = Data(repeating: 2, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.recordRefusal(of: refused)
        snapshot.record([], for: [other])
        XCTAssertTrue(snapshot.isPending(refused))
        XCTAssertTrue(snapshot.allowsSubmission(to: other))
    }

    /// Code 74: the private store read every contact request runs first did
    /// not finish. Nothing was sent and nothing is pending: its own outcome,
    /// with a retry message, and no pending row.
    func testShouldOfferARetryForAnUnfinishedPrivateStoreRead() {
        let contact = Data(repeating: 1, count: 32)
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.of(
                PlatformWalletError.privateContactStoreNotRead("not read to its end")),
            .privateStoreNotRead)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        XCTAssertFalse(snapshot.isPending(contact), "the view records no refusal for 74")
        XCTAssertTrue(snapshot.allowsSubmission(to: contact))
        XCTAssertNotEqual(
            ContactRequestSubmissionOutcome.privateStoreNotReadMessage,
            ContactRequestSubmissionOutcome.privateRequestPendingMessage)
    }

    /// Code 75 is the developer's setting, shown as such, on a send and on a
    /// refresh alike.
    func testShouldShowAPinnedSdkAsADeveloperMessage() {
        let error = PlatformWalletError.sdkPinnedBelowProtocolVersion("pinned at 14")
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.of(error),
            .failed(ContactRequestSubmissionOutcome.sdkPinnedMessage))
        XCTAssertEqual(
            PrivateContactRefreshNotice.notice(for: error),
            ContactRequestSubmissionOutcome.sdkPinnedMessage)
    }

    /// Unrestored private payments (code 72) are their own outcome, apart
    /// from a pending request (71), with their own message.
    func testShouldTellUnrestoredPrivatePaymentsFromAPendingRequest() {
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.of(
                PlatformWalletError.privateContactPaymentsNotRestored("not restored")),
            .privatePaymentsNotRestored)
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(pending: true, restoring: true, error: nil),
            ContactRequestSubmissionOutcome.settingUpPrivatePaymentsMessage)
        XCTAssertNotEqual(
            ContactRequestSubmissionOutcome.settingUpPrivatePaymentsMessage,
            ContactRequestSubmissionOutcome.privateRequestPendingMessage)
    }

    /// Code 72 after a failed restoration shows why it failed, read with the
    /// readiness, until a readiness says restored; a shielded store that did
    /// not open is shown over both.
    func testShouldShowWhyARestorationFailedUntilTheReadinessSaysRestored() {
        let contact = Data(repeating: 1, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        snapshot.recordReadiness(
            PrivateContactReadiness(
                privatePaymentsRestored: false, privateStoreRead: false,
                unresolvedPrivatePayment: false, privatePaymentsRestoreFailed: true),
            restoreFailure: "the journal does not read")
        XCTAssertTrue(snapshot.isRestoringPrivatePayments)
        XCTAssertEqual(snapshot.restoreFailure, "the journal does not read")
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(
                pending: false, restoring: snapshot.isRestoringPrivatePayments,
                restoreFailure: snapshot.restoreFailure, error: nil),
            ContactRequestSubmissionOutcome.restoreFailedMessage("the journal does not read"))
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(
                pending: false, restoring: true, restoreFailure: snapshot.restoreFailure,
                shieldedStoreFailure: "disk full", error: nil),
            ContactRequestSubmissionOutcome.shieldedStoreFailedMessage("disk full"))

        // A readiness that no longer flags a failure drops the reason.
        snapshot.recordReadiness(
            PrivateContactReadiness(
                privatePaymentsRestored: false, privateStoreRead: false,
                unresolvedPrivatePayment: false),
            restoreFailure: "stale")
        XCTAssertNil(snapshot.restoreFailure)

        snapshot.recordReadiness(PrivateContactReadiness(
            privatePaymentsRestored: true, privateStoreRead: false, unresolvedPrivatePayment: false))
        XCTAssertFalse(snapshot.isRestoringPrivatePayments)
        XCTAssertTrue(snapshot.allowsSubmission(to: contact))
    }

    /// A store that cannot protect private payments (code 73) is a
    /// configuration error, shown as such.
    func testShouldShowUnsupportedPrivateRecoveryAsAStorageError() {
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.of(
                PlatformWalletError.privateContactRecoveryUnsupported("no atomic scan state")),
            .failed(ContactRequestSubmissionOutcome.recoveryUnsupportedMessage))
    }

    /// A code 72 hold survives a successful pending read that finds nothing,
    /// and a readiness read after a private sync that still says not
    /// restored; only a readiness read that says restored lifts it.
    func testShouldHoldOnCode72UntilTheReadinessSaysRestored() {
        let contact = Data(repeating: 1, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.recordRestoringRefusal()
        snapshot.record([], for: [contact])
        XCTAssertTrue(snapshot.isRestoringPrivatePayments)
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))

        // A private sync read the store to its end; the payments are still
        // not restored.
        snapshot.recordReadiness(PrivateContactReadiness(
            privatePaymentsRestored: false, privateStoreRead: true, unresolvedPrivatePayment: true))
        XCTAssertTrue(snapshot.isRestoringPrivatePayments)
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))

        // A failed readiness read keeps what the last one said.
        snapshot.recordReadiness(nil)
        XCTAssertTrue(snapshot.isRestoringPrivatePayments)

        snapshot.recordReadiness(PrivateContactReadiness(
            privatePaymentsRestored: true, privateStoreRead: true, unresolvedPrivatePayment: false))
        XCTAssertFalse(snapshot.isRestoringPrivatePayments)
        XCTAssertTrue(snapshot.allowsSubmission(to: contact))
    }

    /// A readiness that says not restored holds every contact without any
    /// refusal, as the wallet would refuse them.
    func testShouldShowRestoringFromTheReadinessAlone() {
        let contact = Data(repeating: 1, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        snapshot.recordReadiness(PrivateContactReadiness(
            privatePaymentsRestored: false, privateStoreRead: false, unresolvedPrivatePayment: false))
        XCTAssertTrue(snapshot.isRestoringPrivatePayments)
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))
    }

    /// A successful read that found nothing pending offers every contact it
    /// covered, and nothing it did not.
    func testShouldOfferEveryContactASuccessfulEmptyReadCovered() {
        let contact = Data(repeating: 1, count: 32)
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        XCTAssertFalse(snapshot.isUnavailable)
        XCTAssertFalse(snapshot.isPending(contact))
        XCTAssertTrue(snapshot.allowsSubmission(to: contact))
        XCTAssertFalse(
            snapshot.allowsSubmission(to: Data(repeating: 2, count: 32)),
            "a contact the read did not cover is not offered yet")
    }

    private func readiness(restored: Bool, failed: Bool = false) -> PrivateContactReadiness {
        PrivateContactReadiness(
            privatePaymentsRestored: restored, privateStoreRead: true,
            unresolvedPrivatePayment: false, privatePaymentsRestoreFailed: failed)
    }

    /// The store opened, but the wallet's journal restoration failed for a
    /// transient reason. A DashPay refresh asks the wallet to retry even
    /// though no store-open failure is recorded, reads the readiness after
    /// the attempt, and once the retry restores the payments the gate lifts:
    /// no bind, no restart and no submitted request.
    @MainActor
    func testShouldRetryARestorationThatFailedAfterTheStoreOpenedOnARefresh() {
        let contact = Data(repeating: 1, count: 32)
        var restorationFails = true
        var configures = 0
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        let read = { () -> (readiness: PrivateContactReadiness?, restoreFailure: String?) in
            restorationFails
                ? (self.readiness(restored: false, failed: true), "the journal could not be read")
                : (self.readiness(restored: true), nil)
        }

        // The first refresh: the store opened, the restoration failed, and
        // the retry it asks for fails too.
        var refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: snapshot.readiness, storeFailure: nil, readReadiness: read,
            configureAgain: { configures += 1; return nil })
        snapshot.recordReadiness(refreshed.readiness, restoreFailure: refreshed.restoreFailure)
        XCTAssertEqual(configures, 1, "not restored asks the wallet to retry with no store failure recorded")
        XCTAssertNil(refreshed.storeFailure)
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))
        XCTAssertEqual(snapshot.restoreFailure, "the journal could not be read")
        XCTAssertTrue(PrivatePaymentsRecovery.rechecks(after: snapshot.readiness, attempt: 0))

        // The failure was transient: the next refresh asks first, the retry
        // restores, and the readiness shown is the one read after it.
        refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: snapshot.readiness, storeFailure: nil, readReadiness: read,
            configureAgain: { configures += 1; restorationFails = false; return nil })
        snapshot.recordReadiness(refreshed.readiness, restoreFailure: refreshed.restoreFailure)
        XCTAssertEqual(configures, 2)
        XCTAssertFalse(snapshot.isRestoringPrivatePayments)
        XCTAssertNil(snapshot.restoreFailure)
        XCTAssertTrue(snapshot.allowsSubmission(to: contact), "the gate lifts on the refresh alone")
        XCTAssertFalse(PrivatePaymentsRecovery.rechecks(after: snapshot.readiness, attempt: 1))
    }

    /// A refresh asks for the retry before it reads the readiness it shows:
    /// a view that last showed "not restored" asks first and shows only the
    /// read made after it.
    @MainActor
    func testShouldShowTheReadinessReadAfterTheRetry() {
        var events: [String] = []
        var restored = false
        let refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: readiness(restored: false), storeFailure: nil,
            readReadiness: {
                events.append("read")
                return (self.readiness(restored: restored), nil)
            },
            configureAgain: {
                events.append("configure")
                restored = true
                return nil
            })
        XCTAssertEqual(events, ["configure", "read"])
        XCTAssertEqual(refreshed.readiness?.privatePaymentsRestored, true)
    }

    /// Restored payments and an open store need no retry: one read.
    @MainActor
    func testShouldNotAskTheWalletToRetryWhenThePaymentsAreRestored() {
        var configures = 0
        var reads = 0
        let refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: readiness(restored: true), storeFailure: nil,
            readReadiness: { reads += 1; return (self.readiness(restored: true), nil) },
            configureAgain: { configures += 1; return nil })
        XCTAssertEqual(configures, 0)
        XCTAssertEqual(reads, 1)
        XCTAssertEqual(refreshed.readiness?.privatePaymentsRestored, true)
    }

    /// A store that did not open is opened again on a refresh, whatever the
    /// readiness says, and its failure clears once it opens.
    @MainActor
    func testShouldOpenTheStoreAgainWhileAStoreFailureIsRecorded() {
        var configures = 0
        let refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: nil, storeFailure: "disk full",
            readReadiness: { (nil, nil) },
            configureAgain: { configures += 1; return nil })
        XCTAssertEqual(configures, 1)
        XCTAssertNil(refreshed.storeFailure)
    }

    /// A restoration the wallet deferred behind a busy store completes later,
    /// changing nothing the view observes: the view refreshes again on its
    /// own, bounded, while the payments stay unrestored, and the refresh after
    /// the deferred restoration lifts the gate.
    @MainActor
    func testShouldRefreshAgainWhileARestorationIsDeferred() {
        let contact = Data(repeating: 1, count: 32)
        var deferredDone = false
        var snapshot = PrivateRepliesPendingSnapshot()
        snapshot.record([], for: [contact])
        let read = { () -> (readiness: PrivateContactReadiness?, restoreFailure: String?) in
            (self.readiness(restored: deferredDone), nil)
        }
        var refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: snapshot.readiness, storeFailure: nil, readReadiness: read,
            configureAgain: { nil })
        snapshot.recordReadiness(refreshed.readiness, restoreFailure: refreshed.restoreFailure)
        XCTAssertFalse(snapshot.allowsSubmission(to: contact))
        XCTAssertTrue(PrivatePaymentsRecovery.rechecks(after: snapshot.readiness, attempt: 0))
        XCTAssertFalse(
            PrivatePaymentsRecovery.rechecks(
                after: snapshot.readiness, attempt: PrivatePaymentsRecovery.recheckLimit),
            "bounded")
        XCTAssertFalse(PrivatePaymentsRecovery.rechecks(after: nil, attempt: 0))

        deferredDone = true
        refreshed = PrivatePaymentsRecovery.refresh(
            lastReadiness: snapshot.readiness, storeFailure: nil, readReadiness: read,
            configureAgain: { nil })
        snapshot.recordReadiness(refreshed.readiness, restoreFailure: refreshed.restoreFailure)
        XCTAssertTrue(snapshot.allowsSubmission(to: contact))
        XCTAssertFalse(PrivatePaymentsRecovery.rechecks(after: snapshot.readiness, attempt: 1))
    }

    func testShouldAcceptACollisionPrivatelyWhenSendPrivatelyIsOn() {
        let accept = CollisionAccept.of(sendPrivately: true)
        XCTAssertEqual(accept, .privateReply)
        XCTAssertEqual(accept.buttonTitle, "Accept privately")
        XCTAssertEqual(
            accept.message,
            "This person already sent you a request. Accept it with a private reply instead? "
                + "The reply is paid from this wallet's shielded balance, and no public request "
                + "names you as its sender."
        )
    }

    func testShouldAcceptACollisionInKindWhenSendPrivatelyIsOff() {
        let accept = CollisionAccept.of(sendPrivately: false)
        XCTAssertEqual(accept, .replyInKind)
        XCTAssertEqual(accept.buttonTitle, "Accept")
        XCTAssertEqual(accept.message, "This person already sent you a request. Accept it instead?")
    }

    func testShouldOfferARetryForAnOrdinaryFailure() {
        let error = PlatformWalletError.shieldedBroadcastFailed("rejected by the relay")
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.of(error),
            .failed(error.localizedDescription)
        )
    }

    func testShouldStayQuietWhenTheNetworkDoesNotServeThePrivateStoreYet() {
        XCTAssertNil(PrivateContactRefreshNotice.notice(
            for: PlatformWalletError.protocolVersionNotActive("needs protocol version 15")
        ))
    }

    func testShouldShowAPrivateRefreshThatFailed() {
        let error = PlatformWalletError.walletOperation("no node answered")
        XCTAssertEqual(
            PrivateContactRefreshNotice.notice(for: error),
            "Private requests could not be read: \(error.localizedDescription)"
        )
    }

    func testShouldShowRequestsThatWereReadButNotSaved() {
        XCTAssertNil(PrivateContactRefreshNotice.notice(unpersistedIdentityCount: 0))
        XCTAssertEqual(
            PrivateContactRefreshNotice.notice(unpersistedIdentityCount: 2),
            "Private requests for 2 identities were read but not saved. Pull to refresh again."
        )
    }
}
