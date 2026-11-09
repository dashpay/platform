import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins how the DashPay views read a failed contact request submission and a
/// private-store refresh: an unconfirmed private send or reply stays pending
/// (no fresh submission is offered), and a network that does not serve the
/// private store yet is quiet while real failures are shown.
final class ContactRequestOutcomesTests: XCTestCase {

    func testShouldKeepAnUnconfirmedPrivateSubmissionPending() {
        let outcome = ContactRequestSubmissionOutcome.of(
            PlatformWalletError.shieldedSpendUnconfirmed("result could not be confirmed")
        )
        XCTAssertEqual(outcome, .mayHaveGoneThrough)
    }

    /// A pending private request (code 71), which a public send or accept
    /// can meet too, sent nothing: it is shown as pending, with a private
    /// sync to settle it, rather than as a submission that may have gone
    /// through.
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
