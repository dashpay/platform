import Foundation
import os
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

  private struct MainHoldError: Error, CustomStringConvertible {
    let message: String
    var description: String { message }
  }

  /// `identityGet` and `dpnsGetUsername` make their FFI call without the
  /// main actor: the main thread is held for the whole of both calls, and
  /// each still returns (with the mock's missing-expectation error) before it
  /// is released. A call that needed the main actor would wait until the hold
  /// timed out.
  func testIdentityReadsDoNotNeedTheMainActor() async throws {
    let sdk = try SDK(mockVectorsDirectory: nil)
    let mainHeld = DispatchSemaphore(value: 0)
    let releaseMain = DispatchSemaphore(value: 0)
    let holdTimedOut = OSAllocatedUnfairLock(initialState: false)

    DispatchQueue.main.async {
      mainHeld.signal()
      if releaseMain.wait(timeout: .now() + 5) == .timedOut {
        holdTimedOut.withLock { $0 = true }
      }
    }
    try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
      DispatchQueue.global().async {
        if mainHeld.wait(timeout: .now() + 5) == .timedOut {
          releaseMain.signal()
          continuation.resume(
            throwing: MainHoldError(message: "the main queue never ran the hold"))
        } else {
          continuation.resume()
        }
      }
    }

    do {
      _ = try await sdk.identityGet(identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk")
      XCTFail("identityGet should fail without recorded responses")
    } catch SDKError.invalidState {
      XCTFail("identityGet must reach the FFI, not fail on an unavailable SDK handle")
    } catch {}

    do {
      _ = try await sdk.dpnsGetUsername(
        identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk", limit: 1)
      XCTFail("dpnsGetUsername should fail without recorded responses")
    } catch SDKError.invalidState {
      XCTFail("dpnsGetUsername must reach the FFI, not fail on an unavailable SDK handle")
    } catch {}

    let returnedWhileMainWasHeld = !holdTimedOut.withLock { $0 }
    releaseMain.signal()
    XCTAssertTrue(
      returnedWhileMainWasHeld, "an identity read waited for the main actor")
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

    do {
      _ = try await sdk.getContractGroupInfo(
        contractGroupId: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8")
      XCTFail("getContractGroupInfo should fail without recorded responses")
    } catch SDKError.notFound(let message) where message == "Contract group not found" {
      XCTFail("a failed fetch must not be reported as a proved absence")
    } catch {}
  }

  /// A malformed contract group id is refused by the FFI as an invalid
  /// parameter before any request is made.
  @MainActor
  func testContractGroupInfoRejectsAMalformedId() async throws {
    let sdk = try SDK(mockVectorsDirectory: nil)

    do {
      _ = try await sdk.getContractGroupInfo(contractGroupId: "not-base58-0OIl")
      XCTFail("getContractGroupInfo should refuse a malformed id")
    } catch SDKError.invalidParameter {
    } catch {
      XCTFail("expected SDKError.invalidParameter, got \(error)")
    }
  }

  @MainActor
  func testDocumentHistoryPreservesNativeErrors() async throws {
    let sdk = try SDK(mockVectorsDirectory: nil)
    do {
      _ = try await sdk.documentGetHistory(
        dataContractId: "not-base58-0OIl",
        documentType: "note",
        documentId: "not-base58-0OIl")
      XCTFail("documentGetHistory should refuse a malformed id")
    } catch SDKError.invalidParameter(let message) {
      XCTAssertTrue(message.contains("Invalid contract ID"))
    } catch {
      XCTFail("expected the native SDKError.invalidParameter, got \(error)")
    }
  }
}
