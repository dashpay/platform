import Foundation
import XCTest

@testable import SwiftDashSDK

/// Coverage for `DocumentTypedArray.Element.value(fromInput:)`, which turns
/// the text a user entered for one typed array element into the JSON value
/// the platform wallet's schema sanitizer takes.
///
/// The sanitizer narrows JSON numbers to the element's width, decodes base58
/// identifiers and hex byte arrays, but never parses a number or a boolean out
/// of a string. So integers, numbers and booleans must leave as JSON numbers
/// and booleans, identifiers and byte arrays as strings (never `Data`, which
/// `JSONSerialization` cannot encode inside an array), and a string element
/// exactly as typed. The range, length and `enum` checks are a courtesy that
/// spares a transition consensus would refuse.
///
/// `DocumentTypedArray.values(fromInputs:)` / `jsonArray(fromInputs:)` judge a
/// whole list the same way (count, every row, repeats under `uniqueItems`);
/// the example app refuses to encode a list that fails, so an invalid list is
/// never broadcast and paid for.
final class DocumentTypedArrayElementInputTests: XCTestCase {

    private typealias Element = DocumentTypedArray.Element
    private typealias Value = DocumentTypedArray.ElementValue
    private typealias ListError = DocumentTypedArray.InputError

    // MARK: - Integer

    func testIntegerInputBecomesAJSONInteger() {
        let element = Element.integer(minimum: nil, maximum: nil, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "42"), .success(.integer(42)))
        XCTAssertEqual(element.value(fromInput: " -7 "), .success(.integer(-7)))
    }

    func testIntegerInputThatIsNotAWholeNumberIsRefused() {
        let element = Element.integer(minimum: nil, maximum: nil, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "4.2"), .failure(.notAnInteger("4.2")))
        XCTAssertEqual(element.value(fromInput: "abc"), .failure(.notAnInteger("abc")))
        XCTAssertEqual(element.value(fromInput: "   "), .failure(.empty))
        // Beyond Int64 is not a whole number Swift can send
        XCTAssertEqual(
            element.value(fromInput: "99999999999999999999"),
            .failure(.notAnInteger("99999999999999999999")))
    }

    func testIntegerInputOutsideTheDeclaredRangeIsRefused() {
        let element = Element.integer(minimum: 1, maximum: 10, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "1"), .success(.integer(1)))
        XCTAssertEqual(element.value(fromInput: "10"), .success(.integer(10)))
        XCTAssertEqual(
            element.value(fromInput: "0"),
            .failure(.outOfRange(value: "0", minimum: "1", maximum: "10")))
        XCTAssertEqual(
            element.value(fromInput: "11"),
            .failure(.outOfRange(value: "11", minimum: "1", maximum: "10")))
    }

    func testIntegerInputOutsideTheEnumIsRefusedNamingTheAllowedValues() {
        let element = Element.integer(minimum: 1, maximum: 10, allowedValues: [1, 5, 10])

        XCTAssertEqual(element.value(fromInput: "5"), .success(.integer(5)))
        XCTAssertEqual(
            element.value(fromInput: "4"),
            .failure(.notAllowed(value: "4", allowed: ["1", "5", "10"])))
        // Failing both, the enum is what the user is told about
        XCTAssertEqual(
            element.value(fromInput: "11"),
            .failure(.notAllowed(value: "11", allowed: ["1", "5", "10"])))
    }

    // MARK: - Number

    func testNumberInputBecomesAJSONNumber() {
        let element = Element.number(minimum: nil, maximum: nil, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "2.5"), .success(.number(2.5)))
        XCTAssertEqual(element.value(fromInput: "-3"), .success(.number(-3)))
        XCTAssertEqual(element.value(fromInput: "1e3"), .success(.number(1000)))
    }

    /// The decimal pad shows a comma as the decimal separator in some locales.
    func testNumberInputReadsALoneCommaAsTheDecimalSeparator() {
        let element = Element.number(minimum: nil, maximum: nil, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "2,5"), .success(.number(2.5)))
        XCTAssertEqual(element.value(fromInput: "1,000,5"), .failure(.notANumber("1,000,5")))
    }

    /// JSON cannot carry NaN or infinity, and `JSONSerialization` raises on
    /// them rather than throwing.
    func testNumberInputThatIsNotAFiniteNumberIsRefused() {
        let element = Element.number(minimum: nil, maximum: nil, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "nan"), .failure(.notANumber("nan")))
        XCTAssertEqual(element.value(fromInput: "inf"), .failure(.notANumber("inf")))
        XCTAssertEqual(element.value(fromInput: "two"), .failure(.notANumber("two")))
        XCTAssertEqual(element.value(fromInput: ""), .failure(.empty))
    }

    func testNumberInputOutsideTheDeclaredRangeOrEnumIsRefused() {
        let bounded = Element.number(minimum: -1.5, maximum: 2, allowedValues: nil)
        XCTAssertEqual(bounded.value(fromInput: "-1.5"), .success(.number(-1.5)))
        XCTAssertEqual(
            bounded.value(fromInput: "2.25"),
            .failure(.outOfRange(value: "2.25", minimum: "-1.5", maximum: "2")))

        // An integral input matches an integral member, as 1 equals 1.0 in JSON
        let listed = Element.number(minimum: nil, maximum: nil, allowedValues: [1, 2.5])
        XCTAssertEqual(listed.value(fromInput: "1"), .success(.number(1)))
        XCTAssertEqual(
            listed.value(fromInput: "2"),
            .failure(.notAllowed(value: "2", allowed: ["1", "2.5"])))
    }

    // MARK: - Boolean

    func testBooleanInputBecomesAJSONBoolean() {
        let element = Element.boolean(allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "true"), .success(.boolean(true)))
        XCTAssertEqual(element.value(fromInput: "False"), .success(.boolean(false)))
        XCTAssertEqual(element.value(fromInput: "yes"), .failure(.notABoolean("yes")))
        XCTAssertEqual(element.value(fromInput: "1"), .failure(.notABoolean("1")))
    }

    func testBooleanInputOutsideTheEnumIsRefused() {
        let element = Element.boolean(allowedValues: [true])

        XCTAssertEqual(element.value(fromInput: "true"), .success(.boolean(true)))
        XCTAssertEqual(
            element.value(fromInput: "false"),
            .failure(.notAllowed(value: "false", allowed: ["true"])))
    }

    // MARK: - String

    /// The comma-separated editor this replaces split "a, b" into two
    /// elements; one element keeps its commas and spaces.
    func testStringInputIsSentExactlyAsEntered() {
        let element = Element.string(minLength: nil, maxLength: nil, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "a, b"), .success(.string("a, b")))
        XCTAssertEqual(element.value(fromInput: "  padded "), .success(.string("  padded ")))
        XCTAssertEqual(element.value(fromInput: ""), .success(.string("")))
    }

    func testStringInputOutsideTheDeclaredLengthIsRefused() {
        let element = Element.string(minLength: 2, maxLength: 4, allowedValues: nil)

        XCTAssertEqual(element.value(fromInput: "ab"), .success(.string("ab")))
        XCTAssertEqual(
            element.value(fromInput: "a"),
            .failure(.wrongLength(length: 1, minimum: 2, maximum: 4)))
        XCTAssertEqual(
            element.value(fromInput: "abcde"),
            .failure(.wrongLength(length: 5, minimum: 2, maximum: 4)))
        XCTAssertEqual(
            element.value(fromInput: ""),
            .failure(.wrongLength(length: 0, minimum: 2, maximum: 4)))
    }

    /// JSON Schema counts characters as Unicode code points: an emoji built
    /// from two scalars is two characters, not one grapheme.
    func testStringLengthIsCountedInUnicodeScalars() {
        let element = Element.string(minLength: nil, maxLength: 1, allowedValues: nil)
        let flag = "\u{1F1FA}\u{1F1F8}"

        XCTAssertEqual(flag.count, 1)
        XCTAssertEqual(
            element.value(fromInput: flag),
            .failure(.wrongLength(length: 2, minimum: nil, maximum: 1)))
    }

    func testStringInputOutsideTheEnumIsRefused() {
        let element = Element.string(minLength: nil, maxLength: nil, allowedValues: ["happy", "sad"])

        XCTAssertEqual(element.value(fromInput: "sad"), .success(.string("sad")))
        XCTAssertEqual(
            element.value(fromInput: "Sad"),
            .failure(.notAllowed(value: "Sad", allowed: ["happy", "sad"])))
    }

    // MARK: - Identifier

    func testIdentifierInputIsSentAsBase58() {
        let id = Data(repeating: 0x11, count: 32).toBase58String()

        XCTAssertEqual(Element.identifier.value(fromInput: " \(id) "), .success(.string(id)))
    }

    func testIdentifierInputThatIsNotBase58IsRefused() {
        // 0, O, I and l are not in the base58 alphabet
        XCTAssertEqual(
            Element.identifier.value(fromInput: "0OIl"), .failure(.invalidBase58("0OIl")))
        XCTAssertEqual(Element.identifier.value(fromInput: ""), .failure(.empty))
    }

    func testIdentifierInputThatIsNotThirtyTwoBytesIsRefused() {
        let short = Data(repeating: 0x22, count: 20).toBase58String()

        XCTAssertEqual(
            Element.identifier.value(fromInput: short),
            .failure(.wrongByteCount(count: 20, minimum: 32, maximum: 32)))
    }

    // MARK: - Byte array

    func testByteArrayInputIsSentAsLowercaseHex() {
        let element = Element.byteArray(minSize: nil, maxSize: nil)

        XCTAssertEqual(element.value(fromInput: "DEADbeef"), .success(.string("deadbeef")))
        XCTAssertEqual(element.value(fromInput: "0x00ff"), .success(.string("00ff")))
    }

    /// `Data(hexString:)` would drop an odd final digit and accept a `+`
    /// sign; the element check must not.
    func testByteArrayInputThatIsNotHexIsRefused() {
        let element = Element.byteArray(minSize: nil, maxSize: nil)

        XCTAssertEqual(element.value(fromInput: "abc"), .failure(.invalidHex("abc")))
        XCTAssertEqual(element.value(fromInput: "+f"), .failure(.invalidHex("+f")))
        XCTAssertEqual(element.value(fromInput: "zz"), .failure(.invalidHex("zz")))
        // Fullwidth digits are hex digits to `Character`, not to the sanitizer
        XCTAssertEqual(element.value(fromInput: "\u{FF10}\u{FF11}"), .failure(.invalidHex("\u{FF10}\u{FF11}")))
        XCTAssertEqual(element.value(fromInput: "0x"), .failure(.empty))
    }

    func testByteArrayInputOutsideTheDeclaredSizeIsRefused() {
        let element = Element.byteArray(minSize: 2, maxSize: 3)

        XCTAssertEqual(element.value(fromInput: "aabb"), .success(.string("aabb")))
        XCTAssertEqual(
            element.value(fromInput: "aa"),
            .failure(.wrongByteCount(count: 1, minimum: 2, maximum: 3)))
        XCTAssertEqual(
            element.value(fromInput: "aabbccdd"),
            .failure(.wrongByteCount(count: 4, minimum: 2, maximum: 3)))
    }

    // MARK: - Allowed inputs

    /// A picker offers `allowedInputs`, so every entry must read back as the
    /// member it stands for.
    func testEveryAllowedInputReadsBackAsItsMember() {
        let elements: [(Element, [Value])] = [
            (.integer(minimum: nil, maximum: nil, allowedValues: [3, -1]), [.integer(3), .integer(-1)]),
            (.number(minimum: nil, maximum: nil, allowedValues: [2, 0.5, 1e20]),
             [.number(2), .number(0.5), .number(1e20)]),
            (.boolean(allowedValues: [false, true]), [.boolean(false), .boolean(true)]),
            (.string(minLength: nil, maxLength: nil, allowedValues: ["a, b", " x"]),
             [.string("a, b"), .string(" x")])
        ]

        for (element, members) in elements {
            let inputs = element.allowedInputs ?? []
            XCTAssertEqual(inputs.count, members.count, "\(element)")
            XCTAssertEqual(inputs.map { element.value(fromInput: $0) }, members.map { .success($0) })
        }
        XCTAssertEqual(
            Element.number(minimum: nil, maximum: nil, allowedValues: [2, 0.5]).allowedInputs,
            ["2", "0.5"])
    }

    func testElementsWithoutAnEnumOfferNoAllowedInputs() {
        XCTAssertNil(Element.integer(minimum: 1, maximum: 2, allowedValues: nil).allowedInputs)
        XCTAssertNil(Element.boolean(allowedValues: nil).allowedInputs)
        XCTAssertNil(Element.byteArray(minSize: nil, maxSize: nil).allowedInputs)
        XCTAssertNil(Element.identifier.allowedInputs)
    }

    // MARK: - Whole list

    private func list(
        _ element: Element,
        minItems: Int? = nil,
        maxItems: Int = 8,
        uniqueItems: Bool = false
    ) -> DocumentTypedArray {
        DocumentTypedArray(
            path: "scores", element: element,
            minItems: minItems, maxItems: maxItems, uniqueItems: uniqueItems)
    }

    private let integers = Element.integer(minimum: nil, maximum: nil, allowedValues: nil)

    func testListInputBecomesTypedValuesInRowOrder() throws {
        let scores = list(integers, minItems: 1, maxItems: 3)

        XCTAssertEqual(
            scores.values(fromInputs: ["3", " 1", "2"]),
            .success([.integer(3), .integer(1), .integer(2)]))

        let array = try scores.jsonArray(fromInputs: ["3", "1", "2"]).get()
        let data = try JSONSerialization.data(withJSONObject: ["scores": array])
        XCTAssertEqual(String(data: data, encoding: .utf8), #"{"scores":[3,1,2]}"#)
    }

    func testEmptyListPassesWhenNoMinItemsIsDeclared() {
        XCTAssertEqual(list(integers).values(fromInputs: []), .success([]))
        XCTAssertEqual(list(integers, minItems: 0).values(fromInputs: []), .success([]))
    }

    func testListWithFewerRowsThanMinItemsIsRefused() {
        let scores = list(integers, minItems: 2, maxItems: 4)

        XCTAssertEqual(
            scores.values(fromInputs: ["1"]),
            .failure(.tooFewElements(path: "scores", count: 1, minimum: 2)))
        XCTAssertEqual(
            scores.values(fromInputs: []),
            .failure(.tooFewElements(path: "scores", count: 0, minimum: 2)))
    }

    func testListWithMoreRowsThanMaxItemsIsRefused() {
        XCTAssertEqual(
            list(integers, maxItems: 2).values(fromInputs: ["1", "2", "3"]),
            .failure(.tooManyElements(path: "scores", count: 3, maximum: 2)))
    }

    func testListReportsTheFirstBadRowByIndex() {
        XCTAssertEqual(
            list(integers).values(fromInputs: ["1", "x", "y"]),
            .failure(.invalidElement(path: "scores", index: 1, reason: .notAnInteger("x"))))

        // Every element check applies to its row, the range included
        let bounded = list(.integer(minimum: 0, maximum: 10, allowedValues: nil))
        XCTAssertEqual(
            bounded.values(fromInputs: ["4", "11"]),
            .failure(.invalidElement(
                path: "scores", index: 1,
                reason: .outOfRange(value: "11", minimum: "0", maximum: "10"))))
    }

    /// Repeats are judged on the converted values, as consensus judges the
    /// stored ones, not on the text typed.
    func testRepeatedElementsAreRefusedUnderUniqueItems() {
        XCTAssertEqual(
            list(integers, uniqueItems: true).values(fromInputs: ["5", "7", " 5"]),
            .failure(.repeatedElement(path: "scores", index: 2, firstIndex: 0)))

        let numbers = Element.number(minimum: nil, maximum: nil, allowedValues: nil)
        XCTAssertEqual(
            list(numbers, uniqueItems: true).values(fromInputs: ["1", "1.0"]),
            .failure(.repeatedElement(path: "scores", index: 1, firstIndex: 0)))

        let bytes = Element.byteArray(minSize: nil, maxSize: nil)
        XCTAssertEqual(
            list(bytes, uniqueItems: true).values(fromInputs: ["AABB", "0xaabb"]),
            .failure(.repeatedElement(path: "scores", index: 1, firstIndex: 0)))

        let id = Data(repeating: 0x11, count: 32).toBase58String()
        XCTAssertEqual(
            list(.identifier, uniqueItems: true).values(fromInputs: [id, " \(id) "]),
            .failure(.repeatedElement(path: "scores", index: 1, firstIndex: 0)))
    }

    func testRepeatedElementsPassWithoutUniqueItems() {
        XCTAssertEqual(
            list(integers).values(fromInputs: ["5", "5"]),
            .success([.integer(5), .integer(5)]))
    }

    /// The count is judged before the rows, and every row before repeats.
    func testListChecksTheCountThenTheRowsThenRepeats() {
        let scores = list(integers, minItems: 3, maxItems: 4, uniqueItems: true)

        XCTAssertEqual(
            scores.values(fromInputs: ["x", "x"]),
            .failure(.tooFewElements(path: "scores", count: 2, minimum: 3)))
        XCTAssertEqual(
            scores.values(fromInputs: ["1", "1", "x"]),
            .failure(.invalidElement(path: "scores", index: 2, reason: .notAnInteger("x"))))
    }

    func testJSONArrayRefusesWhatValuesRefuses() {
        guard case let .failure(error) = list(integers, minItems: 1).jsonArray(fromInputs: []) else {
            return XCTFail("an empty list below minItems must be refused")
        }
        XCTAssertEqual(error, .tooFewElements(path: "scores", count: 0, minimum: 1))
    }

    func testListErrorDescriptionsNameThePropertyAndTheRow() {
        let cases: [(ListError, String)] = [
            (.tooFewElements(path: "scores", count: 1, minimum: 2),
             "scores: 1 element; the list takes at least 2."),
            (.tooManyElements(path: "scores", count: 3, maximum: 2),
             "scores: 3 elements; the list takes at most 2."),
            (.invalidElement(path: "scores", index: 1, reason: .notAnInteger("x")),
             "scores[1]: \"x\" is not a whole number."),
            (.repeatedElement(path: "team.leads", index: 2, firstIndex: 0),
             "team.leads[2]: repeats team.leads[0], and the elements must be unique.")
        ]

        for (error, message) in cases {
            XCTAssertEqual(error.localizedDescription, message)
        }
    }

    // MARK: - JSON form

    func testElementValuesEncodeAsTypedJSONInsideAnArray() throws {
        let values: [Value] = [.integer(5), .number(2.5), .boolean(true), .string("a, b")]
        let array = values.map(\.jsonValue)

        XCTAssertTrue(JSONSerialization.isValidJSONObject(["list": array]))
        let data = try JSONSerialization.data(withJSONObject: ["list": array], options: [.sortedKeys])
        XCTAssertEqual(String(data: data, encoding: .utf8), #"{"list":[5,2.5,true,"a, b"]}"#)
    }

    func testErrorDescriptionsNameTheProblem() {
        typealias InputError = DocumentTypedArray.ElementInputError
        let cases: [(InputError, String)] = [
            (.empty, "Enter a value"),
            (.notAnInteger("x"), "\"x\" is not a whole number"),
            (.outOfRange(value: "11", minimum: "1", maximum: "10"), "11 is outside the allowed range (1 to 10)"),
            (.outOfRange(value: "-1", minimum: "0", maximum: nil), "(at least 0)"),
            (.notAllowed(value: "4", allowed: ["1", "5"]), "allowed values: 1, 5"),
            (.wrongLength(length: 5, minimum: nil, maximum: 4), "5 characters; the element takes at most 4"),
            (.wrongByteCount(count: 20, minimum: 32, maximum: 32), "20 bytes; the element takes exactly 32")
        ]

        for (error, fragment) in cases {
            let message = error.localizedDescription
            XCTAssertTrue(message.contains(fragment), "\(error): \(message)")
        }
    }
}
