import XCTest
@testable import SwiftExampleApp

/// `KeyLimitsRetryGate`: after a failed key limits update, the sheet offers
/// the same change again only when Platform refused it and the wallet shows
/// the reviewed limits.
final class KeyLimitsRetryGateTests: XCTestCase {
    private let reviewed = KeyLimitsRetryGate.Limits(totalBudget: 1_000, expiresAt: 5_000)

    func testShouldOfferAgainWhenTheWalletShowsTheReviewedLimits() {
        XCTAssertTrue(KeyLimitsRetryGate.mayOfferAgain(refusedByPlatform: true, reviewed: reviewed, current: reviewed))
    }

    func testShouldNotOfferAgainWhenAProvedRaiseMovedTheBudget() {
        // 1,000 + 500 was proved and moved the wallet's copy, then its row
        // failed to store: the same +500 must not be offered at 2,000.
        let current = KeyLimitsRetryGate.Limits(totalBudget: 1_500, expiresAt: 5_000)
        XCTAssertFalse(KeyLimitsRetryGate.mayOfferAgain(refusedByPlatform: true, reviewed: reviewed, current: current))
    }

    func testShouldNotOfferAgainWhenTheExpiryMoved() {
        let current = KeyLimitsRetryGate.Limits(totalBudget: 1_000, expiresAt: 9_000)
        XCTAssertFalse(KeyLimitsRetryGate.mayOfferAgain(refusedByPlatform: true, reviewed: reviewed, current: current))
    }

    func testShouldNotOfferAgainWhenTheWalletRefusedTheCheck() {
        // A refused check (for example a signer lookup that failed for a
        // moment) proves nothing about the failed update.
        XCTAssertFalse(KeyLimitsRetryGate.mayOfferAgain(refusedByPlatform: true, reviewed: reviewed, current: nil))
    }

    func testShouldNotOfferAgainAfterAFailureThatWasNotPlatformsRefusal() {
        // A failure after the proof (the wallet removed, or the proved key
        // not stored) can leave the wallet showing the old limits although
        // the raise landed.
        XCTAssertFalse(KeyLimitsRetryGate.mayOfferAgain(refusedByPlatform: false, reviewed: reviewed, current: reviewed))
    }
}
