import CoreFoundation
import Foundation

/// A typed array property of a document type (meta-schema v3, protocol
/// version 14): a list of scalars, declared as `type: "array"` with an
/// `items` schema naming what every element is and a required `maxItems`.
///
/// ```json
/// "scores": {
///   "type": "array",
///   "items": { "type": "integer", "minimum": 0, "maximum": 100 },
///   "minItems": 1,
///   "maxItems": 8,
///   "uniqueItems": true,
///   "position": 0
/// }
/// ```
///
/// A property that declares `byteArray` itself is a byte array (or an
/// identifier), never a typed array, whatever else it declares.
///
/// The fields mirror wasm-dpp2's `DocumentTypedArrayProperty` key for key.
/// DPP validated the declaration when the contract was registered; this type
/// only reads it back off the schema as authored and does not re-validate it.
/// The element checks of `Element.value(fromInput:)` are a client-side
/// courtesy: consensus is the authority on what a document may hold.
public struct DocumentTypedArray: Equatable, Sendable {
    /// Dotted path of the property within the document type: `"reasons"`, or
    /// `"team.leads"` for one nested in an object property.
    public let path: String

    /// What every element is.
    public let element: Element

    /// The fewest elements a document may hold; `nil` when not declared.
    public let minItems: Int?

    /// The most elements a document may hold. Every typed array declares it.
    public let maxItems: Int

    /// Whether a document repeating an element is refused. `false` when the
    /// schema does not declare `uniqueItems`.
    public let uniqueItems: Bool

    /// A declaration from its parts, as `init?(path:propertySchema:)` reads
    /// them.
    public init(path: String, element: Element, minItems: Int?, maxItems: Int, uniqueItems: Bool) {
        self.path = path
        self.element = element
        self.minItems = minItems
        self.maxItems = maxItems
        self.uniqueItems = uniqueItems
    }

    /// Read one property schema dictionary, as authored in the contract.
    ///
    /// `nil` when the property is not a typed array: not `type: "array"`, a
    /// `byteArray` key on the property, no `items` object, an `items` schema
    /// that is not one scalar DPP admits as an element, or no integer
    /// `maxItems`. DPP refuses a contract declaring the last three, so they
    /// can only reach a client through hand-edited JSON.
    public init?(path: String, propertySchema: [String: Any]) {
        guard propertySchema["type"] as? String == "array",
              propertySchema["byteArray"] == nil,
              let items = propertySchema["items"] as? [String: Any],
              let element = Element(itemsSchema: items),
              let maxItems = Self.jsonInteger(propertySchema["maxItems"])
        else {
            return nil
        }
        self.init(
            path: path,
            element: element,
            minItems: Self.jsonInteger(propertySchema["minItems"]),
            maxItems: maxItems,
            uniqueItems: Self.jsonBool(propertySchema["uniqueItems"]) ?? false
        )
    }

    /// Every typed array a document type declares, sorted by path. Walks into
    /// `object` properties, naming a nested typed array by its dotted path.
    ///
    /// `documentTypeSchema` is the whole document type dictionary as authored
    /// in the contract (`PersistentDocumentType.schema`).
    public static func all(inDocumentTypeSchema documentTypeSchema: [String: Any]?) -> [DocumentTypedArray] {
        var found: [DocumentTypedArray] = []
        collect(properties: documentTypeSchema?["properties"], prefix: nil, into: &found)
        return found.sorted { $0.path < $1.path }
    }

    /// The typed array a document type declares as its top-level property
    /// `name`, or `nil` when that property is absent or not a typed array.
    /// A nested typed array is not found by its dotted path here: use
    /// `all(inDocumentTypeSchema:)`.
    public static func named(
        _ name: String,
        inDocumentTypeSchema documentTypeSchema: [String: Any]?
    ) -> DocumentTypedArray? {
        guard let properties = documentTypeSchema?["properties"] as? [String: Any],
              let propertySchema = properties[name] as? [String: Any]
        else {
            return nil
        }
        return DocumentTypedArray(path: name, propertySchema: propertySchema)
    }

    private static func collect(
        properties: Any?,
        prefix: String?,
        into found: inout [DocumentTypedArray]
    ) {
        guard let properties = properties as? [String: Any] else { return }
        for (name, value) in properties {
            guard let propertySchema = value as? [String: Any] else { continue }
            let path = prefix.map { "\($0).\(name)" } ?? name
            if let typedArray = DocumentTypedArray(path: path, propertySchema: propertySchema) {
                found.append(typedArray)
            } else if propertySchema["type"] as? String == "object" {
                collect(properties: propertySchema["properties"], prefix: path, into: &found)
            }
        }
    }
}

// MARK: - Element

extension DocumentTypedArray {
    /// What every element of a typed array is, read from its `items` schema.
    ///
    /// The cases mirror wasm-dpp2's `DocumentTypedArrayItem`. The bounds are
    /// the schema keywords': `minLength` / `maxLength` count a string
    /// element's characters, `minSize` / `maxSize` (the items' `minItems` /
    /// `maxItems`) a byte array element's bytes, and `minimum` / `maximum` an
    /// integer or number element's range. `allowedValues` is the items'
    /// `enum`, in declared order. Each is `nil` when the schema omits it.
    public enum Element: Equatable, Sendable {
        case integer(minimum: Int?, maximum: Int?, allowedValues: [Int]?)
        case number(minimum: Double?, maximum: Double?, allowedValues: [Double]?)
        case boolean(allowedValues: [Bool]?)
        case string(minLength: Int?, maxLength: Int?, allowedValues: [String]?)
        case byteArray(minSize: Int?, maxSize: Int?)
        /// A 32-byte identifier: a byte array element carrying the identifier
        /// `contentMediaType`.
        case identifier
    }
}

extension DocumentTypedArray.Element {
    /// The `contentMediaType` that makes a byte array element an identifier.
    static let identifierMediaType = "application/x.dash.dpp.identifier"

    /// Parse an `items` schema the way DPP types an element: an integer,
    /// number, boolean or string schema, or a `byteArray: true` array, which
    /// the identifier media type makes an identifier. Anything else (an
    /// object, an array of arrays) is not an element DPP admits.
    ///
    /// An `enum` member or bound that no Swift value of the element type can
    /// hold (an integer beyond `Int`) is left out, and an `enum` left empty
    /// that way reads as `nil`: the client then checks less, and consensus
    /// still checks everything.
    init?(itemsSchema items: [String: Any]) {
        typealias Reader = DocumentTypedArray
        switch items["type"] as? String {
        case "integer":
            self = .integer(
                minimum: Reader.jsonInteger(items["minimum"]),
                maximum: Reader.jsonInteger(items["maximum"]),
                allowedValues: Reader.members(items["enum"], Reader.jsonInteger)
            )
        case "number":
            self = .number(
                minimum: Reader.jsonDouble(items["minimum"]),
                maximum: Reader.jsonDouble(items["maximum"]),
                allowedValues: Reader.members(items["enum"], Reader.jsonDouble)
            )
        case "boolean":
            self = .boolean(allowedValues: Reader.members(items["enum"], Reader.jsonBool))
        case "string":
            self = .string(
                minLength: Reader.jsonInteger(items["minLength"]),
                maxLength: Reader.jsonInteger(items["maxLength"]),
                allowedValues: Reader.members(items["enum"]) { $0 as? String }
            )
        case "array":
            guard Reader.jsonBool(items["byteArray"]) == true else { return nil }
            if items["contentMediaType"] as? String == Self.identifierMediaType {
                self = .identifier
            } else {
                self = .byteArray(
                    minSize: Reader.jsonInteger(items["minItems"]),
                    maxSize: Reader.jsonInteger(items["maxItems"])
                )
            }
        default:
            return nil
        }
    }
}

// MARK: - JSON readers

extension DocumentTypedArray {
    /// A JSON number, but never a JSON boolean: `JSONSerialization` hands
    /// both back as `NSNumber`, and an `NSNumber` boolean casts to `1` / `0`.
    static func jsonNumber(_ value: Any?) -> NSNumber? {
        guard let number = value as? NSNumber,
              CFGetTypeID(number) != CFBooleanGetTypeID()
        else {
            return nil
        }
        return number
    }

    /// A JSON integer that fits `Int`; `nil` for a fraction or a boolean.
    static func jsonInteger(_ value: Any?) -> Int? {
        jsonNumber(value).flatMap { Int(exactly: $0) }
    }

    /// A JSON number read as `Double`; `nil` for a boolean.
    static func jsonDouble(_ value: Any?) -> Double? {
        jsonNumber(value)?.doubleValue
    }

    /// A JSON boolean; `nil` for a number, which an `NSNumber` cast to `Bool`
    /// would otherwise accept.
    static func jsonBool(_ value: Any?) -> Bool? {
        guard let number = value as? NSNumber,
              CFGetTypeID(number) == CFBooleanGetTypeID()
        else {
            return nil
        }
        return number.boolValue
    }

    /// The members of an `enum` list that `read` accepts, or `nil` when there
    /// is no list or none of its members is readable.
    static func members<T>(_ value: Any?, _ read: (Any?) -> T?) -> [T]? {
        guard let list = value as? [Any] else { return nil }
        let members = list.compactMap { read($0) }
        return members.isEmpty ? nil : members
    }
}

// MARK: - Element input

extension DocumentTypedArray {
    /// One element in the JSON form the platform wallet's schema sanitizer
    /// takes (`DocumentType::sanitize_document_properties`): integers, numbers
    /// and booleans as JSON numbers and booleans, which the sanitizer narrows
    /// to the element's width but never parses out of a string; a string
    /// element, an identifier as base58, and a byte array as hex, all as JSON
    /// strings the sanitizer decodes. Never `Data`: `JSONSerialization`
    /// cannot encode `Data` inside an array.
    public enum ElementValue: Hashable, Sendable {
        /// An integer element, sent as a JSON integer.
        case integer(Int)
        /// A number element, sent as a JSON number (always finite).
        case number(Double)
        /// A boolean element, sent as a JSON boolean.
        case boolean(Bool)
        /// A string element, an identifier as base58, or a byte array as
        /// lowercase hex.
        case string(String)

        /// The value as a `JSONSerialization`-encodable object: an `Int`,
        /// `Double`, `Bool` or `String`.
        public var jsonValue: Any {
            switch self {
            case let .integer(value): return value
            case let .number(value): return value
            case let .boolean(value): return value
            case let .string(value): return value
            }
        }

        /// Text that `Element.value(fromInput:)` reads back as this value. A
        /// whole number is written without a fraction (`2`, not `2.0`).
        public var inputText: String {
            switch self {
            case let .integer(value):
                return String(value)
            case let .number(value):
                if value.rounded() == value, abs(value) < 1e15 {
                    return String(Int64(value))
                }
                return String(value)
            case let .boolean(value):
                return value ? "true" : "false"
            case let .string(value):
                return value
            }
        }
    }

    /// Why the text entered for one element is not a value of its element.
    public enum ElementInputError: Error, Equatable, Sendable, LocalizedError {
        /// Nothing entered for an element that is not a string.
        case empty
        /// Not a whole number `Int` can hold.
        case notAnInteger(String)
        /// Not a finite decimal number.
        case notANumber(String)
        /// Neither `true` nor `false`.
        case notABoolean(String)
        /// Below `minimum` or above `maximum`; the bounds are given as input
        /// text, `nil` when not declared.
        case outOfRange(value: String, minimum: String?, maximum: String?)
        /// Not a member of the element's `enum`, given as input text.
        case notAllowed(value: String, allowed: [String])
        /// A string element's length in characters outside
        /// `minLength` / `maxLength`.
        case wrongLength(length: Int, minimum: Int?, maximum: Int?)
        /// Not base58 text.
        case invalidBase58(String)
        /// Not an even number of hex digits.
        case invalidHex(String)
        /// A byte array's size, or an identifier's (which must be exactly 32),
        /// outside the declared bounds.
        case wrongByteCount(count: Int, minimum: Int?, maximum: Int?)

        public var errorDescription: String? {
            switch self {
            case .empty:
                return "Enter a value."
            case let .notAnInteger(text):
                return "\"\(text)\" is not a whole number."
            case let .notANumber(text):
                return "\"\(text)\" is not a number."
            case let .notABoolean(text):
                return "\"\(text)\" is not true or false."
            case let .outOfRange(value, minimum, maximum):
                return "\(value) is outside the allowed range (\(Self.bounds(minimum, maximum)))."
            case let .notAllowed(value, allowed):
                return "\"\(value)\" is not one of the allowed values: \(allowed.joined(separator: ", "))."
            case let .wrongLength(length, minimum, maximum):
                return "\(length) characters; the element takes \(Self.bounds(minimum.map(String.init), maximum.map(String.init)))."
            case .invalidBase58:
                return "Not a valid base58 identifier."
            case .invalidHex:
                return "Not valid hex: use an even number of 0-9 and a-f digits."
            case let .wrongByteCount(count, minimum, maximum):
                return "\(count) bytes; the element takes \(Self.bounds(minimum.map(String.init), maximum.map(String.init)))."
            }
        }

        private static func bounds(_ minimum: String?, _ maximum: String?) -> String {
            switch (minimum, maximum) {
            case let (minimum?, maximum?) where minimum == maximum:
                return "exactly \(minimum)"
            case let (minimum?, maximum?):
                return "\(minimum) to \(maximum)"
            case let (minimum?, nil):
                return "at least \(minimum)"
            case let (nil, maximum?):
                return "at most \(maximum)"
            case (nil, nil):
                return "any"
            }
        }
    }
}

extension DocumentTypedArray {
    /// Why the rows entered for a typed array cannot be sent as its value.
    /// The message names the property by its path, and a row by its
    /// zero-based index in the path syntax (`scores[1]`).
    public enum InputError: Error, Equatable, Sendable, LocalizedError {
        /// Fewer rows than `minItems`.
        case tooFewElements(path: String, count: Int, minimum: Int)
        /// More rows than `maxItems`.
        case tooManyElements(path: String, count: Int, maximum: Int)
        /// The row at `index` is not a value of the element.
        case invalidElement(path: String, index: Int, reason: ElementInputError)
        /// Under `uniqueItems`, the row at `index` converts to the same value
        /// as the row at `firstIndex`.
        case repeatedElement(path: String, index: Int, firstIndex: Int)

        public var errorDescription: String? {
            switch self {
            case let .tooFewElements(path, count, minimum):
                return "\(path): \(Self.elements(count)); the list takes at least \(minimum)."
            case let .tooManyElements(path, count, maximum):
                return "\(path): \(Self.elements(count)); the list takes at most \(maximum)."
            case let .invalidElement(path, index, reason):
                return "\(path)[\(index)]: \(reason.localizedDescription)"
            case let .repeatedElement(path, index, firstIndex):
                return "\(path)[\(index)]: repeats \(path)[\(firstIndex)], and the elements must be unique."
            }
        }

        private static func elements(_ count: Int) -> String {
            count == 1 ? "1 element" : "\(count) elements"
        }
    }

    /// Convert every row entered for this typed array, in order, into the
    /// list to send, or say why the list cannot be sent.
    ///
    /// Checks, in order: the row count against `minItems` / `maxItems`, each
    /// row with `Element.value(fromInput:)` (the first bad row is reported),
    /// and, under `uniqueItems`, that no two rows convert to the same value.
    /// Uniqueness compares the converted values, so `1` and `1.0` in a number
    /// list, or one identifier typed twice with different spacing, repeat.
    /// A caller that leaves an empty optional list out of the document skips
    /// this check for it; an empty list that is sent must pass `minItems`.
    ///
    /// Like the element check, this is a client-side courtesy that spares a
    /// transition consensus would refuse and still charge for.
    public func values(
        fromInputs inputs: [String]
    ) -> Result<[ElementValue], InputError> {
        if let minItems, inputs.count < minItems {
            return .failure(.tooFewElements(path: path, count: inputs.count, minimum: minItems))
        }
        if inputs.count > maxItems {
            return .failure(.tooManyElements(path: path, count: inputs.count, maximum: maxItems))
        }

        var values: [ElementValue] = []
        values.reserveCapacity(inputs.count)
        for (index, input) in inputs.enumerated() {
            switch element.value(fromInput: input) {
            case let .success(value):
                values.append(value)
            case let .failure(reason):
                return .failure(.invalidElement(path: path, index: index, reason: reason))
            }
        }

        if uniqueItems {
            var firstIndexOf: [ElementValue: Int] = [:]
            for (index, value) in values.enumerated() {
                if let firstIndex = firstIndexOf[value] {
                    return .failure(.repeatedElement(path: path, index: index, firstIndex: firstIndex))
                }
                firstIndexOf[value] = index
            }
        }

        return .success(values)
    }

    /// `values(fromInputs:)` as `JSONSerialization`-encodable elements: the
    /// array to put in a document's properties JSON under this path.
    public func jsonArray(fromInputs inputs: [String]) -> Result<[Any], InputError> {
        values(fromInputs: inputs).map { $0.map(\.jsonValue) }
    }
}

extension DocumentTypedArray.Element {
    /// The input text of each value the element's `enum` allows, in declared
    /// order: what a picker offers. `value(fromInput:)` accepts every entry.
    /// `nil` when the element declares no `enum`; byte array and identifier
    /// elements never do.
    public var allowedInputs: [String]? {
        typealias Value = DocumentTypedArray.ElementValue
        switch self {
        case let .integer(_, _, allowedValues):
            return allowedValues?.map { Value.integer($0).inputText }
        case let .number(_, _, allowedValues):
            return allowedValues?.map { Value.number($0).inputText }
        case let .boolean(allowedValues):
            return allowedValues?.map { Value.boolean($0).inputText }
        case let .string(_, _, allowedValues):
            return allowedValues
        case .byteArray, .identifier:
            return nil
        }
    }

    /// Convert the text entered for one element into the value to send, or
    /// say why it is not one.
    ///
    /// - Integer: a whole number within `minimum` / `maximum`.
    /// - Number: a finite decimal within `minimum` / `maximum`. A lone comma
    ///   reads as the decimal separator, which the decimal pad shows in some
    ///   locales.
    /// - Boolean: `true` or `false`.
    /// - String: the text exactly as entered, spaces and commas included,
    ///   with its length counted in Unicode scalars as JSON Schema counts
    ///   characters. An empty string is a value.
    /// - Identifier: base58 text decoding to 32 bytes, sent as base58.
    /// - Byte array: hex digits (an optional `0x` prefix is dropped) within
    ///   the declared size, sent as lowercase hex.
    ///
    /// Surrounding whitespace is ignored for every kind but string. A value
    /// outside the element's `enum` fails. These checks spare the user a
    /// transition consensus would refuse; they decide nothing consensus
    /// does not decide again.
    public func value(
        fromInput text: String
    ) -> Result<DocumentTypedArray.ElementValue, DocumentTypedArray.ElementInputError> {
        typealias Value = DocumentTypedArray.ElementValue
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if case .string = self {
            // A string element keeps its text exactly, and may be empty
        } else if trimmed.isEmpty {
            return .failure(.empty)
        }

        switch self {
        case let .string(minLength, maxLength, allowedValues):
            return Self.stringValue(
                text, minLength: minLength, maxLength: maxLength, allowedValues: allowedValues)

        case let .integer(minimum, maximum, allowedValues):
            guard let value = Int(trimmed) else { return .failure(.notAnInteger(trimmed)) }
            return Self.checked(
                Value.integer(value),
                isAllowed: allowedValues.map { $0.contains(value) },
                allowed: allowedInputs,
                isBelowMinimum: minimum.map { value < $0 } ?? false,
                isAboveMaximum: maximum.map { value > $0 } ?? false,
                minimum: minimum.map { Value.integer($0).inputText },
                maximum: maximum.map { Value.integer($0).inputText }
            )

        case let .number(minimum, maximum, allowedValues):
            let commaCount = trimmed.filter { $0 == "," }.count
            let normalized = commaCount == 1 && !trimmed.contains(".")
                ? trimmed.replacingOccurrences(of: ",", with: ".")
                : trimmed
            guard let value = Double(normalized), value.isFinite else {
                return .failure(.notANumber(trimmed))
            }
            return Self.checked(
                Value.number(value),
                isAllowed: allowedValues.map { $0.contains(value) },
                allowed: allowedInputs,
                isBelowMinimum: minimum.map { value < $0 } ?? false,
                isAboveMaximum: maximum.map { value > $0 } ?? false,
                minimum: minimum.map { Value.number($0).inputText },
                maximum: maximum.map { Value.number($0).inputText }
            )

        case let .boolean(allowedValues):
            let value: Bool
            switch trimmed.lowercased() {
            case "true": value = true
            case "false": value = false
            default: return .failure(.notABoolean(trimmed))
            }
            return Self.checked(
                Value.boolean(value),
                isAllowed: allowedValues.map { $0.contains(value) },
                allowed: allowedInputs,
                isBelowMinimum: false,
                isAboveMaximum: false,
                minimum: nil,
                maximum: nil
            )

        case .identifier:
            guard let bytes = Data.identifier(fromBase58: trimmed) else {
                return .failure(.invalidBase58(trimmed))
            }
            guard bytes.count == 32 else {
                return .failure(.wrongByteCount(count: bytes.count, minimum: 32, maximum: 32))
            }
            return .success(.string(bytes.toBase58String()))

        case let .byteArray(minSize, maxSize):
            let digits = trimmed.hasPrefix("0x") || trimmed.hasPrefix("0X")
                ? String(trimmed.dropFirst(2))
                : trimmed
            guard !digits.isEmpty else { return .failure(.empty) }
            // `isASCII` too: `isHexDigit` also admits the fullwidth digits
            guard digits.count.isMultiple(of: 2),
                  digits.allSatisfy({ $0.isASCII && $0.isHexDigit })
            else {
                return .failure(.invalidHex(trimmed))
            }
            let count = digits.count / 2
            let tooSmall = minSize.map { count < $0 } ?? false
            let tooLarge = maxSize.map { count > $0 } ?? false
            if tooSmall || tooLarge {
                return .failure(.wrongByteCount(count: count, minimum: minSize, maximum: maxSize))
            }
            return .success(.string(digits.lowercased()))
        }
    }

    private static func stringValue(
        _ text: String,
        minLength: Int?,
        maxLength: Int?,
        allowedValues: [String]?
    ) -> Result<DocumentTypedArray.ElementValue, DocumentTypedArray.ElementInputError> {
        if let allowedValues, !allowedValues.contains(text) {
            return .failure(.notAllowed(value: text, allowed: allowedValues))
        }
        let length = text.unicodeScalars.count
        if let minLength, length < minLength {
            return .failure(.wrongLength(length: length, minimum: minLength, maximum: maxLength))
        }
        if let maxLength, length > maxLength {
            return .failure(.wrongLength(length: length, minimum: minLength, maximum: maxLength))
        }
        return .success(.string(text))
    }

    /// The `enum` is checked first: when a value fails both, naming the
    /// allowed values says more than naming the range.
    private static func checked(
        _ value: DocumentTypedArray.ElementValue,
        isAllowed: Bool?,
        allowed: [String]?,
        isBelowMinimum: Bool,
        isAboveMaximum: Bool,
        minimum: String?,
        maximum: String?
    ) -> Result<DocumentTypedArray.ElementValue, DocumentTypedArray.ElementInputError> {
        if isAllowed == false {
            return .failure(.notAllowed(value: value.inputText, allowed: allowed ?? []))
        }
        if isBelowMinimum || isAboveMaximum {
            return .failure(.outOfRange(value: value.inputText, minimum: minimum, maximum: maximum))
        }
        return .success(value)
    }
}
