import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class DpnsMarketplacePresentationTests: XCTestCase {
    func testPriceShowsCreditsAndDash() {
        let formatted = DpnsMarketplaceUI.price(100_000_000_000)
        XCTAssertTrue(formatted.contains("100"))
        XCTAssertTrue(formatted.contains("credits"))
        XCTAssertTrue(formatted.contains("1 DASH"))
    }

    func testStatusesRetainCounterparty() {
        let counterparty = Data(repeating: 0x44, count: 32)
        XCTAssertEqual(DpnsMarketplaceUI.status(.owned), "Owned")
        XCTAssertTrue(
            DpnsMarketplaceUI.status(.sold(to: counterparty)).hasPrefix("Sold to ")
        )
        XCTAssertTrue(
            DpnsMarketplaceUI.status(.transferred(to: counterparty))
                .hasPrefix("Transferred to ")
        )
    }

    func testTypedTradeErrorsProduceActionableMessages() {
        let changed = DpnsMarketplaceUI.error(
            PlatformWalletError.priceChanged(documentId: "doc", expected: 10, actual: 12)
        )
        XCTAssertFalse(changed.contains("Nothing was purchased"))
        XCTAssertTrue(changed.contains("10 credits"))
        XCTAssertTrue(changed.contains("12 credits"))

        let funds = DpnsMarketplaceUI.error(
            PlatformWalletError.insufficientIdentityCredits(
                identityId: "buyer",
                required: 101,
                available: 100
            )
        )
        XCTAssertTrue(funds.contains("including the fee reserve"))

        let contest = DpnsMarketplaceUI.error(
            PlatformWalletError.contestedNameNotTradable(label: "alice", endsAtMs: 0)
        )
        XCTAssertTrue(contest.contains("active contest"))

        let signing = DpnsMarketplaceUI.error(
            PlatformWalletError.signingKeyUnavailable("key 3 is watch-only")
        )
        XCTAssertTrue(signing.contains("signing key is unavailable"))
        XCTAssertTrue(signing.contains("Unlock or repair"))
    }

    /// A failed trade leads with what its payment class allows, never with
    /// a claim the case's own text would make: a changed price that may
    /// have been sent waits for sync, and only a retryable NotSent offers a
    /// retry.
    func testTradeFailureFollowsThePaymentClass() throws {
        let maybe = try XCTUnwrap(DpnsMarketplaceUI.tradeFailure(
            PlatformWalletError.priceChanged(documentId: "doc", expected: 10, actual: 12, payment: .maybeSent)
        ))
        XCTAssertEqual(maybe.action, .wait)
        XCTAssertTrue(maybe.message.hasPrefix(PaymentFailure.Recovery.reconcile.suggestion))
        XCTAssertTrue(maybe.message.contains("12 credits"))
        XCTAssertFalse(maybe.message.contains("Nothing was purchased"))

        let retry = try XCTUnwrap(DpnsMarketplaceUI.tradeFailure(PlatformWalletError.walletOperation(
            "refused", payment: PlatformPaymentClass(delivery: .notSent, isRetryable: true)
        )))
        XCTAssertEqual(retry.action, .retry)
        XCTAssertTrue(retry.message.hasPrefix(PaymentFailure.Recovery.retry.suggestion))

        // An SDK error without a wallet class reads as unconfirmed, no retry.
        let unclassified = try XCTUnwrap(DpnsMarketplaceUI.tradeFailure(SDKError.timeout("timeout")))
        XCTAssertEqual(unclassified.action, .wait)
        XCTAssertTrue(unclassified.message.hasPrefix(PaymentFailure.Recovery.unconfirmed.suggestion))
        XCTAssertFalse(unclassified.message.lowercased().contains("try again"))

        // A wallet error Rust gave no class is an unconfirmed trade failure
        // that holds the screen; only an error that is not an SDK error (the
        // sheet's own check) shows as its own message.
        let classless = try XCTUnwrap(DpnsMarketplaceUI.tradeFailure(PlatformWalletError.notForSale("gone")))
        XCTAssertEqual(classless.recovery, .unconfirmed)
        XCTAssertTrue(classless.message.contains("not listed for sale"))
        XCTAssertNil(DpnsMarketplaceUI.tradeFailure(CocoaError(.validationNumberTooSmall)))
    }

    /// The screen keeps a trade failure that holds its slot, so every trade
    /// (list, delist, transfer, purchase) is refused after a maybe-sent,
    /// lock-held or unclassified failure and allowed after a not-sent one;
    /// an error that is not an SDK error never clears the hold.
    func testTradeFailureHoldsEveryTradeUntilNothingWasSent() {
        let held = AssetLockBuiltOutPoint(rawBytes: Data(repeating: 7, count: 32) + Data([1, 0, 0, 0]))!
        let table: [(Error, Bool)] = [
            (PlatformWalletError.walletOperation("x", payment: .maybeSent), false),
            (PlatformWalletError.walletOperation("x", payment: PlatformPaymentClass(delivery: .lockHeld(held), isRetryable: false)), false),
            (PlatformWalletError.walletOperation("x"), false),
            (SDKError.timeout("timeout"), false),
            (PlatformWalletError.walletOperation("x", payment: PlatformPaymentClass(delivery: .notSent, isRetryable: true)), true),
            (PlatformWalletError.notForSale("gone", payment: PlatformPaymentClass(delivery: .notSent, isRetryable: false)), true),
        ]
        for (error, permits) in table {
            let kept = DpnsMarketplaceUI.failureAfterTrade(nil, error: error)
            XCTAssertEqual(PaymentFailure.permitsFreshPayment(after: kept), permits, "\(error)")
        }
        let hold = DpnsMarketplaceUI.failureAfterTrade(nil, error: PlatformWalletError.walletOperation("x", payment: .maybeSent))
        XCTAssertEqual(DpnsMarketplaceUI.failureAfterTrade(hold, error: CocoaError(.validationNumberTooSmall)), hold)
    }

    func testHistoryRecognizesTransferToSelfAsDelist() {
        let owner = Data(repeating: 0x22, count: 32)
        let event = DpnsNameHistoryEvent.transferred(
            from: owner,
            to: owner,
            atMs: 1_000,
            blockHeight: 42
        )
        XCTAssertEqual(DpnsMarketplaceUI.historyTitle(event), "Delisted")
        XCTAssertTrue(DpnsMarketplaceUI.historyDetail(event).contains("removed the listing"))
    }
}
