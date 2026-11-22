import XCTest
@testable import SwiftExampleApp

/// "At least X DASH" never shows more than the known subtotal: it rounds
/// down with exact decimal arithmetic.
final class LowerBoundDashTextTests: XCTestCase {

    func testShouldRoundAPartialSubtotalDown() {
        // 1.00000001999 DASH would round up to 1.00000002 with ordinary rounding.
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(100_000_001_999)), "At least 1.00000001 DASH")
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(1_000)), "At least 0.00000001 DASH")
    }

    /// A positive subtotal below one duff would floor to 0 DASH; it is
    /// shown as its exact credits instead.
    func testShouldShowASubDuffSubtotalInCredits() {
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(1)), "At least 1 credit")
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(999)), "At least 999 credits")
        XCTAssertEqual(LowerBoundDashText.atLeast(duffs: 0, credits: 2), "At least 2 credits")
    }

    func testShouldNotBoundAZeroSubtotal() {
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(0)), "Not known yet")
        XCTAssertEqual(LowerBoundDashText.atLeast(duffs: 0, credits: 0), "Not known yet")
    }

    func testShouldKeepExactAmountsAndTrimTrailingZeros() {
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(150_000_000_000)), "At least 1.5 DASH")
        XCTAssertEqual(LowerBoundDashText.atLeast(credits: Decimal(100_000_000_000)), "At least 1 DASH")
    }

    func testShouldGroupTheIntegerDigits() {
        XCTAssertEqual(
            LowerBoundDashText.dash(Decimal(string: "12345678.123456789")!),
            "12,345,678.12345678")
        XCTAssertEqual(LowerBoundDashText.dash(Decimal(string: "123")!), "123")
    }

    func testShouldAddCoreDuffsAndCreditsExactly() {
        // 1 duff = 1000 credits: 1 duff + 999 credits stays below 0.00000002.
        XCTAssertEqual(LowerBoundDashText.atLeast(duffs: 1, credits: 999), "At least 0.00000001 DASH")
        XCTAssertEqual(
            LowerBoundDashText.atLeast(duffs: UInt64.max, credits: UInt64.max),
            "At least 184,651,908,177.83261166 DASH")
    }
}
