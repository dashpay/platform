import XCTest
@testable import SwiftDashSDK

final class ProtocolCompatibilityTests: XCTestCase {
    func testShouldKeepVerifiedNetworkVersionSeparateFromEffectiveVersion() throws {
        let snapshot = try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
            {"generation":7,"status":"checked","verifiedProtocolVersion":99,
             "effectiveProtocolVersion":13,"maximumSupportedProtocolVersion":14,
             "unavailableReason":null,"newerProtocolAvailable":true}
            """.utf8))
        XCTAssertEqual(snapshot.status, .checked)
        XCTAssertEqual(snapshot.verifiedProtocolVersion, 99)
        XCTAssertEqual(snapshot.effectiveProtocolVersion, 13)
        XCTAssertEqual(snapshot.generation, 7)
        XCTAssertTrue(snapshot.newerProtocolAvailable)
    }

    func testShouldDecodeUnavailableAndCheckingWithoutInventingVerifiedVersion() throws {
        for (status, reason) in [("unavailable", "\"timedOut\""), ("checking", "null")] {
            let snapshot = try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
                {"generation":8,"status":"\(status)","verifiedProtocolVersion":null,
                 "effectiveProtocolVersion":13,"maximumSupportedProtocolVersion":14,
                 "unavailableReason":\(reason),"newerProtocolAvailable":false}
                """.utf8))
            XCTAssertNil(snapshot.verifiedProtocolVersion)
            XCTAssertEqual(snapshot.status.rawValue, status)
            XCTAssertFalse(snapshot.newerProtocolAvailable)
            if status == "unavailable" {
                XCTAssertEqual(snapshot.unavailableReason, .timedOut)
            }
        }
    }

    func testShouldDecodeRustNoticeWithoutRecomputingIt() throws {
        for status in ["unavailable", "checking", "checked"] {
            for notice in [false, true] {
                let snapshot = try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
                    {"generation":9,"status":"\(status)","verifiedProtocolVersion":99,
                     "effectiveProtocolVersion":13,"maximumSupportedProtocolVersion":14,
                     "newerProtocolAvailable":\(notice)}
                    """.utf8))
                XCTAssertEqual(snapshot.newerProtocolAvailable, notice)
            }
        }
    }

    func testShouldRejectMissingRustNotice() {
        XCTAssertThrowsError(try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
            {"generation":9,"status":"checked","verifiedProtocolVersion":99,
             "effectiveProtocolVersion":14,"maximumSupportedProtocolVersion":14}
            """.utf8)))
    }

    func testShouldRejectUnknownCheckStatus() {
        XCTAssertThrowsError(try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
            {"generation":9,"status":"unexpected","verifiedProtocolVersion":14,
             "effectiveProtocolVersion":14,"maximumSupportedProtocolVersion":14,
             "newerProtocolAvailable":false}
            """.utf8)))
    }

    func testShouldRoundTripGenerationWithoutLosingUInt64Precision() throws {
        let snapshot = ProtocolCompatibility(
            generation: UInt64.max,
            status: .checked,
            verifiedProtocolVersion: 14,
            effectiveProtocolVersion: 13,
            maximumSupportedProtocolVersion: 14,
            newerProtocolAvailable: false
        )
        XCTAssertEqual(
            try JSONDecoder().decode(ProtocolCompatibility.self, from: JSONEncoder().encode(snapshot)),
            snapshot
        )
    }
}
