import DashSDKFFI
import Foundation
import XCTest
@testable import SwiftDashSDK

final class SDKUnconfirmedErrorTests: XCTestCase {
    func testDirectSDKDecoderPreservesUnconfirmedResultAfterMessageIsFreed() {
        XCTAssertEqual(StateTransitionResultUnconfirmed.rawValue, 11)
        for detail in ["wait timed out", "invalid proof", "address nonce mismatch"] {
            let rendered = "Identity top-up may have executed: \(detail)"
            let message = strdup(rendered)
            let decoded = SDKError.fromDashSDKError(DashSDKError(
                code: StateTransitionResultUnconfirmed,
                message: message,
                consensus_code: 0,
                consensus_kind: ConsensusErrorKindNone
            ))
            free(message)

            guard case .stateTransitionResultUnconfirmed(let preserved) = decoded else {
                return XCTFail("Expected unconfirmed result, got \(decoded)")
            }
            XCTAssertEqual(preserved, rendered)
            XCTAssertNil(decoded.consensusError)
            XCTAssertTrue(decoded.localizedDescription.contains("before submitting again"))
            XCTAssertTrue(decoded.localizedDescription.contains(rendered))
        }
    }

    func testUnconfirmedCodeTakesPrecedenceOverInnerConsensusForBothHelpers() {
        let ffi = DashSDKError(
            code: StateTransitionResultUnconfirmed,
            message: nil,
            consensus_code: 40603,
            consensus_kind: ConsensusErrorKindState
        )
        // Rust emits no consensus fields for this code. Even contradictory
        // input must not turn a possibly completed write into a safe refusal.
        let contextual = SDKError.stateTransitionFailure(
            "Top-up result unconfirmed: address nonce mismatch", ffiError: ffi,
            otherwise: SDKError.timeout
        )
        for error in [SDKError.fromDashSDKError(ffi), contextual] {
            guard case .stateTransitionResultUnconfirmed = error else {
                return XCTFail("Expected unconfirmed result, got \(error)")
            }
            XCTAssertNil(error.consensusError)
        }
        guard case .stateTransitionResultUnconfirmed(let message) = contextual else { return }
        XCTAssertEqual(message, "Top-up result unconfirmed: address nonce mismatch")
    }

    func testDefiniteRefusalAndOrdinaryTimeoutKeepExistingCases() {
        let refusal = DashSDKError(
            code: DashSDKErrorCode(rawValue: 5), message: nil,
            consensus_code: 40603, consensus_kind: ConsensusErrorKindState
        )
        for error in [
            SDKError.fromDashSDKError(refusal),
            SDKError.stateTransitionFailure("Refused", ffiError: refusal)
        ] {
            XCTAssertEqual(error.consensusError?.code, 40603)
        }
        let timeout = DashSDKError(
            code: DashSDKErrorCode(rawValue: 8), message: nil,
            consensus_code: 0, consensus_kind: ConsensusErrorKindNone
        )
        guard case .timeout = SDKError.fromDashSDKError(timeout) else {
            return XCTFail("An ordinary timeout must keep its existing case")
        }
        guard case .internalError = SDKError.stateTransitionFailure("Failed", ffiError: timeout) else {
            return XCTFail("An unmarked contextual failure must keep its existing fallback")
        }
    }

    func testUserFacingHelpersPreserveUnconfirmedWarningWithoutSuggestingRetry() {
        for detail in ["timeout", "networkError connection lost", "invalidParameter nonce mismatch"] {
            let error = SDKError.stateTransitionResultUnconfirmed(detail)
            let displayed = ErrorCategorizer.toUserFacingError(error)
            XCTAssertEqual(ErrorFormatter.userFriendlyMessage(from: error), error.localizedDescription)
            XCTAssertEqual(displayed.message, error.localizedDescription)
            XCTAssertTrue(displayed.message.contains(detail))
            XCTAssertTrue(displayed.message.contains("before submitting again"))
            XCTAssertFalse(displayed.isRetryable)
            XCTAssertNil(displayed.recoverySuggestion)
        }
        XCTAssertTrue(ErrorCategorizer.toUserFacingError(SDKError.timeout("timeout")).isRetryable)
    }
}
