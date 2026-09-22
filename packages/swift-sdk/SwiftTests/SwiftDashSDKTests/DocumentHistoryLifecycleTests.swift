import XCTest
@testable import SwiftDashSDK

/// The lifecycle block is the only authenticated account of what an erase
/// removed, so a malformed number must be rejected rather than truncated into
/// a count or a timestamp the user then acts on.
final class DocumentHistoryLifecycleTests: XCTestCase {

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

    func testRejectsANumberOutsideUInt64() throws {
        XCTAssertNil(try lifecycle(fromJSON: json(remainingRevisions: "1e40")))
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
}
