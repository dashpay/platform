import XCTest
@testable import SwiftDashSDK

final class ShieldedTipAmountTests: XCTestCase {
    func testRejectsPartialLocaleAndMalformedAmounts() {
        for text in ["1,5", "1abc", "1.5abc", "1,000.5", "1e2", "+1", "-1", " 1", "1 ", "", ".", "1.", ".5", "１２", "0", "0.000000000001"] {
            XCTAssertNil(ShieldedTipAmount(text), text)
        }
    }

    func testExactCreditsAndCanonicalConfirmation() throws {
        for (input, credits, display) in [("1.5", UInt64(150_000_000_000), "1.5"),
                                          ("001.500000000000", 150_000_000_000, "1.5"),
                                          ("0.00000000001", 1, "0.00000000001"),
                                          ("184467440.73709551615", UInt64.max, "184467440.73709551615")] {
            let amount = try XCTUnwrap(ShieldedTipAmount(input))
            XCTAssertEqual(amount.credits, credits)
            XCTAssertEqual(amount.dashString, display)
        }
        XCTAssertNil(ShieldedTipAmount("184467440.73709551616"))
        XCTAssertNil(ShieldedTipAmount("184467441"))
        XCTAssertNil(ShieldedTipAmount("18446744073709551616"))
    }
}
