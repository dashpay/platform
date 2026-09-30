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
