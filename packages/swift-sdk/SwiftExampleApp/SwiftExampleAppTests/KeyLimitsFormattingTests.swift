//
//  KeyLimitsFormattingTests.swift
//  SwiftExampleAppTests
//
//  How a key's budget, expiry and bounds read on the key screens.
//

import SwiftDashSDK
import XCTest
@testable import SwiftExampleApp

final class KeyLimitsFormattingTests: XCTestCase {
    func testCreditsRenderAsDashWithoutTrailingZeros() {
        XCTAssertEqual(KeyLimitsFormatting.dash(100_000_000_000), "1 DASH")
        XCTAssertEqual(KeyLimitsFormatting.dash(1_000_000_000), "0.01 DASH")
        XCTAssertEqual(KeyLimitsFormatting.dash(123_456_789_012), "1.23456789012 DASH")
        XCTAssertEqual(KeyLimitsFormatting.dash(1), "0.00000000001 DASH")
        XCTAssertEqual(KeyLimitsFormatting.dash(0), "0 DASH")
    }

    func testDashTextParsesToExactCredits() {
        let locale = Locale(identifier: "en_US")
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "1", locale: locale), 100_000_000_000)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "0.01", locale: locale), 1_000_000_000)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "1.23456789012", locale: locale), 123_456_789_012)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "0.00000000001", locale: locale), 1)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: ".5", locale: locale), 50_000_000_000)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "2.", locale: locale), 200_000_000_000)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: " 0.1 ", locale: locale), 10_000_000_000)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "0", locale: locale), 0)
        // 0.1 + 0.2 in binary floating point is not 0.3; the parser never goes through it.
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "0.3", locale: locale), 30_000_000_000)
    }

    func testDashTextIsTheInverseOfTheRendering() {
        let locale = Locale(identifier: "en_US")
        for credits: UInt64 in [0, 1, 999, 1_000_000_000, 123_456_789_012, 100_000_000_000, UInt64.max] {
            XCTAssertEqual(
                KeyLimitsFormatting.credits(fromDash: KeyLimitsFormatting.dashNumber(credits), locale: locale),
                credits
            )
        }
    }

    func testDashTextAcceptsTheLocaleDecimalSeparator() {
        let german = Locale(identifier: "de_DE")
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "0,5", locale: german), 50_000_000_000)
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "0.5", locale: german), 50_000_000_000)
        XCTAssertNil(KeyLimitsFormatting.credits(fromDash: "0,5", locale: Locale(identifier: "en_US")))
    }

    func testDashTextThatIsNotAnAmountParsesToNil() {
        let locale = Locale(identifier: "en_US")
        for text in ["", " ", ".", "abc", "1.2.3", "-1", "+1", "1e3", "1 000", "0x10", "1.5 DASH", "١"] {
            XCTAssertNil(KeyLimitsFormatting.credits(fromDash: text, locale: locale), "\(text) must not parse")
        }
    }

    func testDashTextFinerThanACreditParsesToNil() {
        let locale = Locale(identifier: "en_US")
        XCTAssertNil(KeyLimitsFormatting.credits(fromDash: "0.000000000001", locale: locale))
        XCTAssertNil(KeyLimitsFormatting.credits(fromDash: "1.000000000000", locale: locale))
    }

    func testDashTextPastTheLargestCreditAmountParsesToNil() {
        let locale = Locale(identifier: "en_US")
        // UInt64.max credits is 184467440.73709551615 DASH.
        XCTAssertEqual(KeyLimitsFormatting.credits(fromDash: "184467440.73709551615", locale: locale), UInt64.max)
        XCTAssertNil(KeyLimitsFormatting.credits(fromDash: "184467440.73709551616", locale: locale))
        XCTAssertNil(KeyLimitsFormatting.credits(fromDash: "184467441", locale: locale))
        XCTAssertNil(KeyLimitsFormatting.credits(fromDash: "99999999999999999999999", locale: locale))
    }

    func testPickedDateConvertsToWholeMilliseconds() {
        XCTAssertEqual(KeyLimitsFormatting.millis(from: Date(timeIntervalSince1970: 1_700_000_000.5)), 1_700_000_000_500)
        XCTAssertEqual(KeyLimitsFormatting.millis(from: Date(timeIntervalSince1970: 0)), 0)
        XCTAssertNil(KeyLimitsFormatting.millis(from: Date(timeIntervalSince1970: -1)))
    }

    func testBudgetShowsRemainingOfTotalWhenKnown() {
        XCTAssertEqual(KeyLimitsFormatting.budget(total: 1_000_000_000, remaining: 400_000_000), "0.004 of 0.01 DASH left")
        XCTAssertEqual(KeyLimitsFormatting.budget(total: 1_000_000_000, remaining: nil), "0.01 DASH")
    }

    func testExpiryInstantCountsAsExpired() {
        let expiresAt: TimestampMillis = 1_700_000_000_000
        XCTAssertFalse(KeyLimitsFormatting.isExpired(expiresAt: expiresAt, now: Date(timeIntervalSince1970: 1_699_999_999.999)))
        XCTAssertTrue(KeyLimitsFormatting.isExpired(expiresAt: expiresAt, now: Date(timeIntervalSince1970: 1_700_000_000)))
    }

    func testRelativeExpiryNamesBothDirections() {
        let expiresAt: TimestampMillis = 1_700_000_000_000
        let locale = Locale(identifier: "en_US")
        XCTAssertEqual(
            KeyLimitsFormatting.expiryRelative(expiresAt, now: Date(timeIntervalSince1970: 1_699_996_400), locale: locale),
            "Expires in 1 hour"
        )
        XCTAssertEqual(
            KeyLimitsFormatting.expiryRelative(expiresAt, now: Date(timeIntervalSince1970: 1_700_172_800), locale: locale),
            "Expired 2 days ago"
        )
    }

    func testBoundsNameTheContractAndDocumentType() {
        let id = Data(repeating: 0x44, count: 32)
        let short = KeyLimitsFormatting.shortId(id)
        XCTAssertEqual(short.count, 13)
        XCTAssertEqual(KeyLimitsFormatting.bounds(.singleContract(id: id)), "Contract \(short)")
        XCTAssertEqual(
            KeyLimitsFormatting.bounds(.singleContractDocumentType(id: id, documentTypeName: "profile")),
            "Contract \(short), type profile"
        )
    }
}
