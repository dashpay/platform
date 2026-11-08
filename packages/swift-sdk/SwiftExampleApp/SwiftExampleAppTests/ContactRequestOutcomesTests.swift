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

    func testShouldKeepAPendingPrivateRequestPending() {
        let outcome = ContactRequestSubmissionOutcome.of(
            PlatformWalletError.privateContactRequestPending("not yet read back from the store")
        )
        XCTAssertEqual(outcome, .mayHaveGoneThrough)
    }

    func testShouldShowAWalletPendingReplyOverAnEarlierError() {
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(pending: true, error: "Accept failed: boom"),
            ContactRequestSubmissionOutcome.mayHaveGoneThroughMessage
        )
        XCTAssertEqual(
            ContactRequestSubmissionOutcome.rowMessage(pending: false, error: "Accept failed: boom"),
            "Accept failed: boom"
        )
        XCTAssertNil(ContactRequestSubmissionOutcome.rowMessage(pending: false, error: nil))
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
