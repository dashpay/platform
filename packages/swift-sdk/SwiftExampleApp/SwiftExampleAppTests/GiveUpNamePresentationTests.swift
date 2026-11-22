import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class GiveUpNamePresentationTests: XCTestCase {
    func testProtocolVersionRefusalReadsAsNotAvailableWithTheSDKMessage() {
        let message = "broadcast_state_transition needs Platform protocol version 15 for deleting "
            + "or tombstoning a DPNS name; the network is proven at protocol version 14"
        XCTAssertEqual(
            GiveUpNameUI.loadFailure(PlatformWalletError.protocolVersionNotActive(message)),
            .notAvailable(message)
        )
        guard case .failed(let submitted) = GiveUpNameUI.submitFailure(
            PlatformWalletError.protocolVersionNotActive(message)
        ) else {
            return XCTFail("Expected a failure message")
        }
        XCTAssertTrue(submitted.hasPrefix("Not available on this network yet."))
    }

    func testPinnedSdkRefusalAsksForAnAppUpdateNotARetry() {
        let message = "broadcast_state_transition needs Platform protocol version 15 for deleting "
            + "or tombstoning a DPNS name; this SDK is pinned to an earlier protocol version"
        XCTAssertEqual(
            GiveUpNameUI.loadFailure(PlatformWalletError.sdkPinnedBelowProtocolVersion(message)),
            .sdkPinned(message)
        )
    }

    func testUnconfirmedGiveUpOffersNothingMore() {
        XCTAssertEqual(
            GiveUpNameUI.submitFailure(
                PlatformWalletError.transactionBroadcastUnconfirmed("no result in time")
            ),
            .unconfirmed("no result in time")
        )
    }

    func testOtherLoadErrorsAreShownForRetry() {
        guard case .failed(let message) = GiveUpNameUI.loadFailure(
            PlatformWalletError.notFound("DPNS name \"alice\" not found")
        ) else {
            return XCTFail("Expected a retryable failure")
        }
        XCTAssertTrue(message.contains("alice"))

        guard case .failed(let signing) = GiveUpNameUI.submitFailure(
            PlatformWalletError.signingKeyUnavailable("key 3 is watch-only")
        ) else {
            return XCTFail("Expected a failure message")
        }
        XCTAssertTrue(signing.contains("signing key is unavailable"))
    }

    func testMainNameMatchesTheNormalizedPickOnly() {
        XCTAssertTrue(GiveUpNameUI.isMainName("Alice", mainDpnsName: "alice"))
        XCTAssertTrue(GiveUpNameUI.isMainName("alice", mainDpnsName: "Alice"))
        XCTAssertFalse(GiveUpNameUI.isMainName("Alice", mainDpnsName: "bob"))
        XCTAssertFalse(GiveUpNameUI.isMainName("Alice", mainDpnsName: nil))
        XCTAssertFalse(GiveUpNameUI.isMainName("Alice", mainDpnsName: ""))
    }

    func testTombstoneWordingStatesItIsPermanent() {
        XCTAssertTrue(GiveUpNameUI.tombstoneConfirmation(name: "alice").hasPrefix("This is permanent."))
        XCTAssertTrue(GiveUpNameUI.tombstoneExplanation.contains("nobody can undo it"))
        XCTAssertTrue(GiveUpNameUI.deleteExplanation.contains("someone else"))
    }
}
