import XCTest
@testable import SwiftExampleApp

/// Tests for `UInt64(jsonValue:)`, the reader the hand-rolled key builders
/// use for every protocol `u64` they pull out of DPP's JSON.
///
/// DPP's `#[json_safe_fields]` writes a `u64` as a JSON number while it fits
/// in a JavaScript safe integer (2^53 - 1 = 9007199254740991) and as a
/// decimal string above that. The two shapes are the whole reason this
/// helper exists: a cast that took only numbers turned a key with
/// `"totalBudget": "9007199254740992"` into a key with no budget, so the
/// wallet would offer it for work Platform meters.
final class JSONSafeIntegerTests: XCTestCase {

    /// The common shape: a value under the ceiling arrives as a JSON number.
    func testReadsAPlainNumber() {
        XCTAssertEqual(UInt64(jsonValue: 0), 0)
        XCTAssertEqual(UInt64(jsonValue: 100_000), 100_000)
        XCTAssertEqual(UInt64(jsonValue: 1_800_000_000_000), 1_800_000_000_000)
        XCTAssertEqual(UInt64(jsonValue: 9_007_199_254_740_991), 9_007_199_254_740_991)
    }

    /// The shape this helper was added for: one past the safe-integer
    /// ceiling, which DPP writes as a string. This is the exact value that
    /// used to read back as nil.
    func testReadsAValueAboveTheSafeIntegerCeilingFromAString() {
        XCTAssertEqual(UInt64(jsonValue: "9007199254740992"), 9_007_199_254_740_992)
        XCTAssertEqual(UInt64(jsonValue: "18446744073709551615"), UInt64.max)
    }

    /// A small value that still arrived as a string reads the same way, so
    /// the helper does not depend on which side of the ceiling a value fell.
    func testReadsASmallNumericString() {
        XCTAssertEqual(UInt64(jsonValue: "0"), 0)
        XCTAssertEqual(UInt64(jsonValue: "42"), 42)
    }

    /// A negative or fractional number is not a `u64`, and must not be
    /// rounded or wrapped into one.
    func testRejectsNegativeAndNonIntegralNumbers() {
        XCTAssertNil(UInt64(jsonValue: -1))
        XCTAssertNil(UInt64(jsonValue: -9_007_199_254_740_992))
        XCTAssertNil(UInt64(jsonValue: 1.5))
        XCTAssertNil(UInt64(jsonValue: -0.5))
        XCTAssertNil(UInt64(jsonValue: "-1"))
        XCTAssertNil(UInt64(jsonValue: "1.5"))
    }

    /// Everything else reads as absent rather than as a guess. A JSON boolean
    /// is the trap here: it parses into an `NSNumber` that answers an integer
    /// cast with 1, so without an explicit guard `true` would read as a
    /// budget of one credit.
    func testRejectsAbsentAndUnrelatedValues() {
        XCTAssertNil(UInt64(jsonValue: nil))
        XCTAssertNil(UInt64(jsonValue: NSNull()))
        XCTAssertNil(UInt64(jsonValue: true))
        XCTAssertNil(UInt64(jsonValue: false))
        XCTAssertNil(UInt64(jsonValue: NSNumber(value: true)))
        XCTAssertNil(UInt64(jsonValue: ""))
        XCTAssertNil(UInt64(jsonValue: " 42"))
        XCTAssertNil(UInt64(jsonValue: "0x2a"))
        XCTAssertNil(UInt64(jsonValue: "not a number"))
        // One past `UInt64.max`, as a string: out of range, not truncated.
        XCTAssertNil(UInt64(jsonValue: "18446744073709551616"))
        XCTAssertNil(UInt64(jsonValue: ["9007199254740992"]))
        XCTAssertNil(UInt64(jsonValue: ["totalBudget": 1]))
    }

    /// End to end through `JSONSerialization`, which is how the key builders
    /// actually receive these values: both shapes come out of the same
    /// payload, and the big one survives.
    func testReadsBothShapesOutOfParsedJSON() throws {
        let payload = """
        {
          "$formatVersion": "1",
          "readOnly": false,
          "disabledAt": 1700000000000,
          "totalBudget": "9007199254740992",
          "expiresAt": 1800000000000
        }
        """
        let data = try XCTUnwrap(payload.data(using: .utf8))
        let object = try XCTUnwrap(
            JSONSerialization.jsonObject(with: data) as? [String: Any])

        XCTAssertEqual(UInt64(jsonValue: object["totalBudget"]), 9_007_199_254_740_992)
        XCTAssertEqual(UInt64(jsonValue: object["expiresAt"]), 1_800_000_000_000)
        XCTAssertEqual(UInt64(jsonValue: object["disabledAt"]), 1_700_000_000_000)
        XCTAssertNil(UInt64(jsonValue: object["missing"]))
        XCTAssertNil(UInt64(jsonValue: object["readOnly"]))
    }
}
