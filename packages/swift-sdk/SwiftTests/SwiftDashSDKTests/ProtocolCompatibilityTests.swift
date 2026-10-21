import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

final class ProtocolCompatibilityTests: XCTestCase {
    func testShouldKeepVerifiedNetworkVersionSeparateFromEffectiveVersion() throws {
        let snapshot = try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
            {"generation":7,"status":"unsupported","verifiedProtocolVersion":99,
             "effectiveProtocolVersion":13,"maximumSupportedProtocolVersion":14,
             "unavailableReason":null}
            """.utf8))
        XCTAssertEqual(snapshot.status, .unsupported)
        XCTAssertEqual(snapshot.verifiedProtocolVersion, 99)
        XCTAssertEqual(snapshot.effectiveProtocolVersion, 13)
        XCTAssertEqual(snapshot.generation, 7)
    }

    func testShouldDecodeUnavailableAndCheckingWithoutInventingVerifiedVersion() throws {
        for (status, reason) in [("unavailable", "\"timedOut\""), ("checking", "null")] {
            let snapshot = try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
                {"generation":8,"status":"\(status)","verifiedProtocolVersion":null,
                 "effectiveProtocolVersion":13,"maximumSupportedProtocolVersion":14,
                 "unavailableReason":\(reason)}
                """.utf8))
            XCTAssertNil(snapshot.verifiedProtocolVersion)
            XCTAssertEqual(snapshot.status.rawValue, status)
            if status == "unavailable" {
                XCTAssertEqual(snapshot.unavailableReason, .timedOut)
            }
        }
    }

    func testShouldRejectUnknownStatusInsteadOfAllowingWrites() {
        XCTAssertThrowsError(try JSONDecoder().decode(ProtocolCompatibility.self, from: Data("""
            {"generation":9,"status":"unexpected","verifiedProtocolVersion":14,
             "effectiveProtocolVersion":14,"maximumSupportedProtocolVersion":14}
            """.utf8)))
    }

    func testShouldRoundTripPermitWithoutLosingUInt64Precision() throws {
        let permit = PlatformWritePermit(
            instanceID: UInt64.max, generation: UInt64.max - 1, protocolVersion: 14
        )
        XCTAssertEqual(
            try JSONDecoder().decode(PlatformWritePermit.self, from: JSONEncoder().encode(permit)),
            permit
        )
    }

    func testShouldPreserveTypedSDKCompatibilityErrorsThroughContext() {
        for code: UInt32 in 11...14 {
            let ffi = DashSDKError(
                code: DashSDKErrorCode(rawValue: code), message: nil,
                consensus_code: 0, consensus_kind: ConsensusErrorKindNone
            )
            let error = SDKError.stateTransitionFailure("diagnostic", ffiError: ffi)
            switch (code, error) {
            case (11, .protocolCompatibilityUnavailable),
                 (12, .protocolCompatibilityChecking),
                 (13, .unsupportedProtocolVersion),
                 (14, .platformPreparationExpired):
                break
            default:
                XCTFail("Compatibility code \(code) was flattened: \(error)")
            }
        }
    }

    func testShouldPreserveUnconfirmedWriteAsDistinctFromCompatibilityRefusal() {
        let error = PlatformWalletError(code: .errorPlatformWriteUnconfirmed, message: "detail")
        guard case .platformWriteUnconfirmed = error else {
            XCTFail("An unknown outcome must not become a safe-to-retry compatibility refusal")
            return
        }
        XCTAssertEqual(PlatformWalletResultCode.errorPlatformWriteUnconfirmed.rawValue, 63)
        XCTAssertEqual(error.failureReason, "detail")
    }

    func testShouldMapWalletCompatibilityErrorsWithoutMessageMatching() {
        for code in [
            PlatformWalletResultCode.errorProtocolCompatibilityUnavailable,
            .errorProtocolCompatibilityChecking,
            .errorUnsupportedProtocolVersion,
            .errorPlatformPreparationExpired
        ] {
            let error = PlatformWalletError(code: code, message: "unrelated detail")
            switch (code, error) {
            case (.errorProtocolCompatibilityUnavailable, .protocolCompatibilityUnavailable),
                 (.errorProtocolCompatibilityChecking, .protocolCompatibilityChecking),
                 (.errorUnsupportedProtocolVersion, .unsupportedProtocolVersion),
                 (.errorPlatformPreparationExpired, .platformPreparationExpired):
                break
            default:
                XCTFail("Wallet compatibility code \(code) was flattened: \(error)")
            }
        }
    }
}
