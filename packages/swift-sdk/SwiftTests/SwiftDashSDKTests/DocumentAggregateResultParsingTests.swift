import Foundation
import XCTest
@testable import SwiftDashSDK

/// Pins how `SDK.documentSum` / `SDK.documentAverage` read the FFI's JSON:
/// sums and averages arrive as decimal strings so a value past `Int64`
/// (a 128-bit sum tree, protocol version 15) is carried whole, and a value
/// that is not a decimal string fails instead of being dropped or narrowed.
/// Whether a result is grouped comes from the FFI's `grouped` flag, never from
/// the keys: in a grouped result the empty key is a real group (documents
/// missing an optional range property), not the total.
final class DocumentAggregateResultParsingTests: XCTestCase {
    /// Decode a payload the way `processJSONResult` does.
    private func decode(_ json: String) throws -> [String: Any] {
        let object = try JSONSerialization.jsonObject(with: Data(json.utf8))
        return try XCTUnwrap(object as? [String: Any])
    }

    private func assertSerializationError(
        _ body: @autoclosure () throws -> Any,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertThrowsError(try body(), file: file, line: line) { error in
            guard case SDKError.serializationError = error else {
                return XCTFail("expected serializationError, got \(error)", file: file, line: line)
            }
        }
    }

    func testWideSumIsCarriedWhole() throws {
        let result = try DocumentSumResult(
            json: decode(#"{"grouped":false,"sums":{"":"18446744073709551614"}}"#)
        )
        XCTAssertEqual(result.total, Int128(UInt64.max) - 1)
        XCTAssertEqual(result.total.map { String($0) }, "18446744073709551614")
        XCTAssertFalse(result.isGrouped)
    }

    func testGroupedSumsKeepSignAndWidth() throws {
        let result = try DocumentSumResult(
            json: decode(#"{"grouped":true,"sums":{"61":"-5","62":"-27670116110564327424"}}"#)
        )
        XCTAssertEqual(result.sums["61"], -5)
        XCTAssertEqual(result.sums["62"], 3 * Int128(Int64.min))
        XCTAssertNil(result.total)
        XCTAssertTrue(result.isGrouped)
    }

    func testSumThatIsNotADecimalStringThrows() throws {
        // A JSON number (the old payload) would be rounded past 2^53, so it is
        // refused rather than read.
        assertSerializationError(
            try DocumentSumResult(json: decode(#"{"grouped":false,"sums":{"":42}}"#))
        )
        assertSerializationError(
            try DocumentSumResult(json: decode(#"{"grouped":false,"sums":{"":"4x2"}}"#))
        )
        assertSerializationError(try DocumentSumResult(json: decode(#"{"grouped":false,"counts":{}}"#)))
    }

    func testShouldReadAnUngroupedSumTotalUnderTheEmptyKey() throws {
        let result = try DocumentSumResult(json: decode(#"{"grouped":false,"sums":{"":"42"}}"#))
        XCTAssertFalse(result.isGrouped)
        XCTAssertEqual(result.total, 42)
    }

    func testShouldReadAGroupedEmptyKeyAsAGroupNotTheTotal() throws {
        let result = try DocumentSumResult(
            json: decode(#"{"grouped":true,"sums":{"":"10","61":"30"}}"#)
        )
        XCTAssertTrue(result.isGrouped)
        XCTAssertNil(result.total)
        XCTAssertEqual(result.sums.count, 2)
        XCTAssertEqual(result.sums[""], 10)
        XCTAssertEqual(result.sums["61"], 30)
    }

    func testShouldRefuseASumWithoutABooleanGroupedFlag() throws {
        // Without the flag an empty key cannot be told apart from the total.
        assertSerializationError(try DocumentSumResult(json: decode(#"{"sums":{"":"42"}}"#)))
        assertSerializationError(
            try DocumentSumResult(json: decode(#"{"grouped":1,"sums":{"":"42"}}"#))
        )
        assertSerializationError(
            try DocumentSumResult(json: decode(#"{"grouped":"true","sums":{"":"42"}}"#))
        )
    }

    func testWideAverageIsTheRustStringVerbatim() throws {
        let result = try DocumentAverageResult(
            json: decode(
                #"{"grouped":false,"averages":{"":{"count":3,"sum":"27670116110564327421","average":"9223372036854775807.0000"}}}"#
            )
        )
        let entry = try XCTUnwrap(result.total)
        XCTAssertEqual(entry.count, 3)
        XCTAssertEqual(entry.sum, 3 * Int128(Int64.max))
        XCTAssertEqual(entry.average, "9223372036854775807.0000")
    }

    func testAverageWithNoDocumentsIsNil() throws {
        let result = try DocumentAverageResult(
            json: decode(#"{"grouped":false,"averages":{"":{"count":0,"sum":"0","average":null}}}"#)
        )
        let entry = try XCTUnwrap(result.total)
        XCTAssertEqual(entry.count, 0)
        XCTAssertEqual(entry.sum, 0)
        XCTAssertNil(entry.average)
    }

    func testMalformedAverageEntryThrows() throws {
        // Old payload: sum as a JSON number and no average.
        assertSerializationError(
            try DocumentAverageResult(
                json: decode(#"{"grouped":false,"averages":{"":{"count":3,"sum":30}}}"#)
            )
        )
        assertSerializationError(
            try DocumentAverageResult(
                json: decode(#"{"grouped":true,"averages":{"61":{"count":3,"sum":"30","average":10}}}"#)
            )
        )
        assertSerializationError(
            try DocumentAverageResult(
                json: decode(#"{"grouped":true,"averages":{"61":{"sum":"30","average":"10.0000"}}}"#)
            )
        )
    }

    func testShouldReadAnUngroupedAverageTotalUnderTheEmptyKey() throws {
        let result = try DocumentAverageResult(
            json: decode(#"{"grouped":false,"averages":{"":{"count":3,"sum":"30","average":"10.0000"}}}"#)
        )
        XCTAssertFalse(result.isGrouped)
        XCTAssertEqual(result.total?.average, "10.0000")
    }

    func testShouldReadAGroupedEmptyKeyAverageAsAGroupNotTheTotal() throws {
        let result = try DocumentAverageResult(
            json: decode(
                #"{"grouped":true,"averages":{"":{"count":2,"sum":"10","average":"5.0000"},"61":{"count":3,"sum":"30","average":"10.0000"}}}"#
            )
        )
        XCTAssertTrue(result.isGrouped)
        XCTAssertNil(result.total)
        XCTAssertEqual(result.averages.count, 2)
        let emptyKeyGroup = try XCTUnwrap(result.averages[""])
        XCTAssertEqual(emptyKeyGroup.count, 2)
        XCTAssertEqual(emptyKeyGroup.average, "5.0000")
        XCTAssertEqual(result.averages["61"]?.average, "10.0000")
    }

    func testShouldRefuseAnAverageWithoutABooleanGroupedFlag() throws {
        assertSerializationError(
            try DocumentAverageResult(
                json: decode(#"{"averages":{"":{"count":3,"sum":"30","average":"10.0000"}}}"#)
            )
        )
        assertSerializationError(
            try DocumentAverageResult(
                json: decode(#"{"grouped":0,"averages":{"":{"count":3,"sum":"30","average":"10.0000"}}}"#)
            )
        )
    }
}
