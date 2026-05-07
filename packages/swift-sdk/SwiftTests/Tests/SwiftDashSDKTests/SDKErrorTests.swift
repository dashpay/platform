import XCTest

@testable import SwiftDashSDK

final class SDKErrorTests: XCTestCase {
  func testProtocolErrorAssociatedMessageIsCleanForPatternMatchingCallers() {
    let message = "Unicode preface: こんにちは 🚀"

    let error = SDKError.protocolError(message)

    if case .protocolError(let raw) = error {
      XCTAssertEqual(raw, message, "Pattern-matched associated value must be the original message")
    } else {
      XCTFail("Expected SDKError.protocolError")
    }

    XCTAssertEqual(error.message, message)
    XCTAssertEqual(error.localizedDescription, "Protocol Error: \(message)")
  }

  func testProtocolErrorPreservesLiteralMarkerLookalikeWithoutDecodingIt() {
    // The historical embedded-payload marker is no longer recognized; if the
    // upstream message text happens to contain it, the SDK must surface it
    // verbatim instead of silently stripping it.
    let message = "Literal marker stays visible: \n[[dashsdk-consensus:v1:not-a-real-payload]]"

    let error = SDKError.protocolError(message)

    if case .protocolError(let raw) = error {
      XCTAssertEqual(raw, message)
    } else {
      XCTFail("Expected SDKError.protocolError")
    }
    XCTAssertEqual(error.message, message)
  }

  func testSDKDetailedErrorWrapsSDKErrorAndPreservesConsensusErrors() {
    let consensusErrors = [
      SDKConsensusError(
        code: 101,
        kind: "Consensus",
        name: "FirstError",
        message: "Primary failure"
      ),
      SDKConsensusError(
        code: 202,
        kind: "DataContract",
        name: "EmojiError",
        message: "Snowman ☃️ and café"
      ),
    ]
    let inner = SDKError.protocolError("Outer protocol error")

    let detailed = SDKDetailedError(sdkError: inner, consensusErrors: consensusErrors)

    XCTAssertEqual(detailed.consensusErrors, consensusErrors)
    if case .protocolError(let raw) = detailed.sdkError {
      XCTAssertEqual(raw, "Outer protocol error")
    } else {
      XCTFail("Expected wrapped SDKError.protocolError")
    }

    let description = detailed.errorDescription ?? ""
    XCTAssertTrue(description.contains("Protocol Error: Outer protocol error"))
    XCTAssertTrue(description.contains("[Consensus] FirstError (101): Primary failure"))
    XCTAssertTrue(description.contains("[DataContract] EmojiError (202): Snowman ☃️ and café"))
  }

  func testSDKDetailedErrorWithoutConsensusDetailsFallsBackToInnerDescription() {
    let detailed = SDKDetailedError(
      sdkError: .internalError("boom"),
      consensusErrors: []
    )

    XCTAssertEqual(detailed.errorDescription, "Internal Error: boom")
  }

  func testConsumeDashSDKErrorReturnsSDKErrorForExistingCatchLogic() {
    let sdkError = SDKError.finalizeConsumedDashSDKError(
      .protocolError("Protocol mismatch"),
      consensusErrors: [
        SDKConsensusError(code: 1, kind: "Consensus", name: "X", message: "y")
      ]
    )

    if case .protocolError(let message) = sdkError {
      XCTAssertEqual(message, "Protocol mismatch")
    } else {
      XCTFail("Expected SDKError.protocolError")
    }
    XCTAssertNil(sdkError.consensusErrors)
  }

  func testSDKErrorConsensusErrorsDoesNotExposeStructuredDetailsFromScalarValue() {
    let sdkError = SDKError.protocolError("Protocol mismatch")

    XCTAssertNil(sdkError.consensusErrors)
  }

  func testFinalizeConsumedDashSDKErrorIgnoresConsensusDetailsForSDKError() {
    let sdkError = SDKError.finalizeConsumedDashSDKError(
      .protocolError("Protocol mismatch"),
      consensusErrors: [
        SDKConsensusError(code: 1, kind: "Consensus", name: "X", message: "y")
      ]
    )
    let consensusErrors = [
      SDKConsensusError(code: 1, kind: "Consensus", name: "X", message: "y")
    ]

    let detailed = SDKDetailedError(sdkError: sdkError, consensusErrors: consensusErrors)

    XCTAssertNil(sdkError.consensusErrors)
    XCTAssertEqual(detailed.consensusErrors, consensusErrors)
  }
}
