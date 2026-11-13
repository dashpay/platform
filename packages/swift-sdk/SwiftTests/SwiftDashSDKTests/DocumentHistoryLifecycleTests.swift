import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// The lifecycle block is the only authenticated account of what an erase
/// removed, so a malformed number must be rejected rather than truncated into
/// a count or a timestamp the user then acts on.
final class DocumentHistoryLifecycleTests: XCTestCase {

    func testSubmittedEraseWithUnavailableClosingReadRemainsSubmitted() throws {
        var value = DocumentEraseOutcomeFFI()
        value.kind = 2
        value.before.state = 4
        value.before.remaining_revisions = UInt64.max
        let outcome = try DocumentEraseOutcome(ffi: value)
        guard case .submitted(let before, let after) = outcome else {
            return XCTFail("Rust reported a submitted erase")
        }
        XCTAssertEqual(before?.remainingRevisions, UInt64.max)
        XCTAssertNil(after)
    }

    func testUnknownEraseTagsReportUnreadableResult() {
        var value = DocumentEraseOutcomeFFI()
        value.kind = 255
        XCTAssertThrowsError(try DocumentEraseOutcome(ffi: value)) { error in
            XCTAssertEqual(error.localizedDescription, "Erase result unreadable; read history before retrying.")
        }
    }

    func testUnavailablePreflightIsNotAnAbsentObservation() throws {
        var value = DocumentEraseOutcomeFFI()
        value.kind = 1
        value.reason = 4
        let outcome = try DocumentEraseOutcome(ffi: value)
        guard case .notSubmitted(.preflightUnavailable, let before) = outcome else {
            return XCTFail("Rust did not submit the erase")
        }
        XCTAssertNil(before)
    }

    /// Parses through `JSONSerialization` the way the query path does: that
    /// is what bridges JSON booleans and fractional numbers to `NSNumber`,
    /// which a plain dictionary literal would not reproduce.
    private func lifecycle(fromJSON json: String) throws -> DocumentHistoryLifecycle? {
        let object = try JSONSerialization.jsonObject(with: Data(json.utf8))
        let dictionary = try XCTUnwrap(object as? [String: Any])
        return DocumentHistoryLifecycle(json: dictionary)
    }

    private func json(remainingRevisions: String) -> String {
        """
        {
          "state": "ERASING",
          "remaining_revisions": \(remainingRevisions),
          "deleted_at_ms": 1700000000000,
          "erasing_started_at_ms": 1700000001000,
          "erasing_from_time_ms": 1699999999000,
          "erasing_from_revision": 7
        }
        """
    }

    func testParsesAWellFormedLifecycleBlock() throws {
        let parsed = try XCTUnwrap(lifecycle(fromJSON: json(remainingRevisions: "42")))
        XCTAssertEqual(parsed.state, .erasing)
        XCTAssertEqual(parsed.remainingRevisions, 42)
        XCTAssertEqual(parsed.deletedAtMs, 1_700_000_000_000)
        XCTAssertEqual(parsed.erasingStartedAtMs, 1_700_000_001_000)
        XCTAssertEqual(parsed.erasingFromTimeMs, 1_699_999_999_000)
        XCTAssertEqual(parsed.erasingFromRevision, 7)
    }

    /// `true` bridges to an `NSNumber` whose `uint64Value` is 1, which would
    /// otherwise be read as one retained revision.
    func testRejectsABooleanInPlaceOfACount() throws {
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "true")))
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "false")))
    }

    /// A fractional value would be truncated, reporting fewer revisions than
    /// the number actually claims.
    func testRejectsAFractionalCount() throws {
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "1.5")))
    }

    func testRejectsANegativeCount() throws {
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "-1")))
    }

    func testPreservesTheFullUInt64RangeFromRustJSON() throws {
        for value in ["9007199254740993", "9223372036854775808", "18446744073709551615"] {
            let parsed = try XCTUnwrap(lifecycle(fromJSON: json(remainingRevisions: value)))
            XCTAssertEqual(parsed.remainingRevisions, UInt64(value))
        }
    }

    func testRejectsANumberOutsideUInt64() throws {
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "1e40")))
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "18446744073709551616")))
    }

    func testRejectsANumberSentAsAString() throws {
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "\"42\"")))
    }

    /// A partial block is rejected outright, so a missing field is never read
    /// as a zero timestamp or a zero count.
    func testRejectsAPartialBlock() throws {
        XCTAssertNil(try lifecycle(fromJSON: #"{"state": "DELETED", "remaining_revisions": 3}"#))
    }

    func testRejectsAnUnknownState() throws {
        let unknown = json(remainingRevisions: "1")
            .replacingOccurrences(of: "\"ERASING\"", with: "\"PURGED\"")
        XCTAssertNil(try lifecycle(fromJSON: unknown))
    }

    func testUnchangedDeletedHistoryDoesNotClaimEraseCompletion() throws {
        let deletedJSON = json(remainingRevisions: "5")
            .replacingOccurrences(of: "\"ERASING\"", with: "\"DELETED\"")
        let before = try XCTUnwrap(lifecycle(fromJSON: deletedJSON))
        let after = try XCTUnwrap(lifecycle(fromJSON: deletedJSON))
        let text = DocumentHistoryLifecycle.describeEraseProgress(before: before, after: after)
        XCTAssertTrue(text.hasPrefix("Not established"))
        XCTAssertFalse(text.localizedCaseInsensitiveContains("complete"))
        XCTAssertFalse(text.localizedCaseInsensitiveContains("accepted"))
    }

    func testErasingHistoryShowsTheRemainingRevisions() throws {
        let after = try XCTUnwrap(lifecycle(fromJSON: json(remainingRevisions: "50")))
        let text = DocumentHistoryLifecycle.describeEraseProgress(before: nil, after: after)
        XCTAssertTrue(text.hasPrefix("Erasure in progress: 50"))
        XCTAssertFalse(text.localizedCaseInsensitiveContains("complete"))
    }

    func testAbsentHistoryConfirmsEraseCompletion() throws {
        let absentJSON = json(remainingRevisions: "0")
            .replacingOccurrences(of: "\"ERASING\"", with: "\"ABSENT\"")
        let after = try XCTUnwrap(lifecycle(fromJSON: absentJSON))
        let text = DocumentHistoryLifecycle.describeEraseProgress(before: nil, after: after)
        XCTAssertTrue(text.hasPrefix("Erasure complete"))
    }
}
