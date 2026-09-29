import Foundation
import XCTest

@testable import SwiftDashSDK

/// The blocking Platform queries run their FFI call on the SDK's query queue,
/// not on the caller's actor. Runs against the FFI mock with no vectors, so no
/// request ever reaches the network: a query that gets as far as DAPI fails
/// with a missing-expectation error.
final class PlatformQueryOffMainTests: XCTestCase {

  override class func setUp() {
    super.setUp()
    SDK.initialize()
  }

  @MainActor
  func testBlockingQueryRunsOffTheMainThread() async throws {
    let sdk = try SDK(mockVectorsDirectory: nil)

    let ranOnMainThread = try await sdk.performBlockingQuery { _ in
      Thread.isMainThread
    }

    XCTAssertFalse(ranOnMainThread)
  }

  /// While a query is parked in its FFI call, the caller's actor stays free:
  /// the main actor itself releases the parked query. If the query held the
  /// main actor, the release could not run and the wait would time out.
  @MainActor
  func testParkedQueryDoesNotHoldTheMainActor() async throws {
    let sdk = try SDK(mockVectorsDirectory: nil)
    let gate = DispatchSemaphore(value: 0)

    async let waitResult = sdk.performBlockingQuery { _ in
      gate.wait(timeout: .now() + 10) == .success
    }

    // Yield so the query can start, then release it from the main actor.
    try await Task.sleep(nanoseconds: 50_000_000)
    gate.signal()

    let released = try await waitResult
    XCTAssertTrue(released, "the parked query was not released by the main actor")
  }

  /// The off-main entry points marshal their arguments, make the FFI call and
  /// surface its error from a main-actor caller. The mock has no recorded
  /// responses, so each one must fail rather than return data.
  @MainActor
  func testOffMainQueriesSurfaceFFIErrors() async throws {
    let sdk = try SDK(mockVectorsDirectory: nil)

    do {
      _ = try await sdk.documentList(
        dataContractId: DPNSVotePoll.contractId,
        documentType: DPNSVotePoll.documentTypeName,
        whereClause: #"[["normalizedParentDomainName","==","dash"]]"#,
        limit: 1)
      XCTFail("documentList should fail without recorded responses")
    } catch {}

    do {
      _ = try await sdk.dpnsCheckAvailability(name: "alice")
      XCTFail("dpnsCheckAvailability should fail without recorded responses")
    } catch {}

    do {
      _ = try await sdk.dpnsContestVoteStateOffMain(normalizedLabel: "a11ce")
      XCTFail("dpnsContestVoteStateOffMain should fail without recorded responses")
    } catch {}

    do {
      _ = try await sdk.dpnsContestIsOpenOffMain(normalizedLabel: "a11ce")
      XCTFail("dpnsContestIsOpenOffMain should fail without recorded responses")
    } catch {}
  }
}
