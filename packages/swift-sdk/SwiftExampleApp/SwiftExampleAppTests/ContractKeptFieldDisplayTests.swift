import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins that a removal record's kept values show their text quoted and
/// escaped, so document text cannot read as another field.
final class ContractKeptFieldDisplayTests: XCTestCase {
    func testShouldQuoteAndEscapeStringsAndMemberNames() {
        let value = ContractKeptFieldValue.object([
            "note": .string(#"x", "admin": true"#),
            "tags": .array([.string("a/b"), .int(-3), .null]),
            #"we"ird"#: .bool(false),
        ])

        XCTAssertEqual(
            value.displayText,
            #"{"note": "x\", \"admin\": true", "tags": ["a/b", -3, null], "we\"ird": false}"#
        )
    }

    func testShouldShowScalarsAsTheyAre() {
        XCTAssertEqual(ContractKeptFieldValue.uint(UInt64.max).displayText, "18446744073709551615")
        XCTAssertEqual(ContractKeptFieldValue.double(1.5).displayText, "1.5")
        XCTAssertEqual(ContractKeptFieldValue.string("").displayText, #""""#)
    }
}
