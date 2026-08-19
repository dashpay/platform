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
        XCTAssertTrue(changed.contains("Nothing was purchased"))
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
