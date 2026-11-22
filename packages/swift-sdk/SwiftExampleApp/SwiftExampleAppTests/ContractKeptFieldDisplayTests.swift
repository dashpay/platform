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

    func testShouldShowBidiControlsAsVisibleEscapes() {
        let value = ContractKeptFieldValue.object([
            "name\u{202E}": .string("abc\u{202E}def\u{2066}g\u{200F}"),
        ])

        XCTAssertEqual(
            value.displayText,
            #"{"name\u{202E}": "abc\u{202E}def\u{2066}g\u{200F}"}"#
        )
        XCTAssertFalse(value.displayText.unicodeScalars.contains { $0.value == 0x202E })
    }

    func testShouldShowTheTaggedKinds() {
        XCTAssertEqual(ContractKeptFieldValue.identifier("abc").displayText, #""abc""#)
        XCTAssertEqual(ContractKeptFieldValue.bytes("AQID").displayText, "bytes(AQID)")
        XCTAssertEqual(ContractKeptFieldValue.uint128("340282366920938463463374607431768211455").displayText,
                       "340282366920938463463374607431768211455")
        XCTAssertEqual(ContractKeptFieldValue.double(-0.0).displayText, "-0.0")
    }

    /// A cited document type name shows its bidi controls as visible escapes.
    func testShouldShowBidiControlsInAPlainNameAsEscapes() {
        XCTAssertEqual(BidiControls.visible("post\u{202E}tsop"), #"post\u{202E}tsop"#)
        XCTAssertEqual(BidiControls.visible("plain"), "plain")
    }

    /// The catalog's JSON gets JSON escapes, so the text shown cannot reorder
    /// itself and, copied, parses to the same values.
    func testShouldEscapeBidiControlsInCatalogJSONAsValidJSON() throws {
        let original = ["text": "abc\u{202E}def\u{2066}"]
        let encoded = String(decoding: try JSONEncoder().encode(original), as: UTF8.self)
        let shown = BidiControls.jsonEscaped(encoded)

        XCTAssertEqual(shown, #"{"text":"abc\u202Edef\u2066"}"#)
        XCTAssertFalse(shown.unicodeScalars.contains { $0.value == 0x202E })
        let parsed = try JSONDecoder().decode([Swift.String: Swift.String].self, from: Data(shown.utf8))
        XCTAssertEqual(parsed, original)
    }
}
