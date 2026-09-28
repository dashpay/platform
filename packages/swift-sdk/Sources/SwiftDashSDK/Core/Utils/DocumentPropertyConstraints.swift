import DashSDKFFI
import Foundation

/// A rule of a document type's `propertyConstraints` (meta-schema v3, protocol
/// version 14): a named condition every created or replaced document's
/// properties must meet. Consensus checks every rule, in name order, and
/// refuses a document breaking one with `DocumentPropertyConstraintViolatedError`
/// (code 10422); a refused state transition is still paid for. A transfer or a
/// purchase is judged against the rules reading `$ownerId` or the transfer's
/// time or heights too, and a price update against the rules reading the
/// update's (see `readsOwner` and `readsSystem`).
///
/// Rust parses the rules and reports them
/// (`dash_sdk_data_contract_get_property_constraints`); this type only carries
/// what it reports. The fields mirror wasm-dpp2's `DocumentPropertyConstraint`
/// key for key.
public struct DocumentPropertyConstraint: Equatable, Sendable {
    /// The rule's name, its key in `propertyConstraints`.
    public let name: String

    /// The rule exactly as the document type's schema declares it, as compact
    /// JSON text with sorted keys (every operator object has a single key, so
    /// sorting changes nothing a reader would notice). Among its operators:
    /// sizes (`{ "length": path }`, `{ "byteLength": path }`,
    /// `{ "count": path }`), system times and heights as bare operands
    /// (`"$createdAt"`), `{ "contains": [arrayPath, value] }`,
    /// `{ "startsWith": [a, b] }`, `{ "endsWith": [a, b] }`,
    /// `{ "notIn": [operand, [values]] }`, `{ "min": [a, b, ...] }`,
    /// `{ "max": [a, b, ...] }`, `{ "abs": a }`, `{ "ifThen": [if, then] }`
    /// and `{ "ifThenElse": [if, then, else] }`.
    public let ruleJSON: String

    /// Every property the rule reads, in declared order, a property read twice
    /// listed twice. The list describes the rule, not one document: it covers
    /// every branch of an `ifThen` or `ifThenElse`, whichever one a document
    /// takes. `$ownerId` is no property and is not listed: see `readsOwner`;
    /// nor are the system times and heights: see `readsSystem`.
    public let reads: [PropertyConstraintRead]

    /// Whether the rule compares the document's owner, `$ownerId`, in any
    /// branch, as in `reads`: then a transfer or a purchase, which changes
    /// the owner, is judged against it too.
    public let readsOwner: Bool

    /// The system times and heights the rule reads, by name, in declared
    /// order, one read twice listed twice, every branch included as in
    /// `reads`: `$createdAt`, `$updatedAt` and `$transferredAt` (block times,
    /// in milliseconds), each also with `BlockHeight` or `CoreBlockHeight`
    /// appended (the Platform and Core block heights), the names of wasm-dpp2's
    /// `PropertyConstraintSystemProperty`. Consensus judges a price update
    /// against the rules reading `$updatedAt…`, and a transfer or a purchase
    /// against the rules reading `$transferredAt…` (or `$ownerId`).
    ///
    /// Empty for a rule reading none, and for every rule reported by a library
    /// built before the field existed.
    public let readsSystem: [String]

    public init(
        name: String,
        ruleJSON: String,
        reads: [PropertyConstraintRead],
        readsOwner: Bool,
        readsSystem: [String] = []
    ) {
        self.name = name
        self.ruleJSON = ruleJSON
        self.reads = reads
        self.readsOwner = readsOwner
        self.readsSystem = readsSystem
    }

    /// `ruleJSON` indented for display, or `ruleJSON` itself should it not
    /// parse back.
    public var prettyRuleJSON: String {
        guard let data = ruleJSON.data(using: .utf8),
              let rule = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed]),
              let pretty = try? JSONSerialization.data(
                withJSONObject: rule,
                options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes, .fragmentsAllowed]),
              let text = String(data: pretty, encoding: .utf8)
        else {
            return ruleJSON
        }
        return text
    }

    /// Decode the JSON array `dash_sdk_data_contract_get_property_constraints`
    /// returns, keeping its order (name order).
    public static func list(fromJSON json: String) throws -> [DocumentPropertyConstraint] {
        guard let entries = try PropertyConstraintJSON.object(from: json) as? [Any] else {
            throw SDKError.serializationError("propertyConstraints rules are not a JSON array")
        }
        return try entries.map { entry in
            guard let rule = entry as? [String: Any],
                  let name = rule["name"] as? String,
                  let declaration = rule["rule"],
                  let reads = rule["reads"] as? [Any],
                  let readsOwner = DocumentTypedArray.jsonBool(rule["readsOwner"]),
                  let readsSystem = systemReads(rule["readsSystem"])
            else {
                throw SDKError.serializationError("Malformed propertyConstraints rule: \(entry)")
            }
            return DocumentPropertyConstraint(
                name: name,
                ruleJSON: try PropertyConstraintJSON.compactText(declaration),
                reads: try reads.map(PropertyConstraintRead.init(jsonEntry:)),
                readsOwner: readsOwner,
                readsSystem: readsSystem
            )
        }
    }

    /// A rule's `readsSystem` names, `[]` when the key is missing (a library
    /// built before it), or `nil` for anything but an array of strings.
    private static func systemReads(_ value: Any?) -> [String]? {
        guard let value else {
            return []
        }
        guard let entries = value as? [Any] else {
            return nil
        }
        var names: [String] = []
        names.reserveCapacity(entries.count)
        for entry in entries {
            guard let name = entry as? String else {
                return nil
            }
            names.append(name)
        }
        return names
    }
}

/// A property a `propertyConstraints` rule reads, and how it reads it.
public struct PropertyConstraintRead: Hashable, Sendable {
    /// How a rule reads a property; the names are wasm-dpp2's
    /// `PropertyConstraintReadKind`.
    public enum Kind: Hashable, Sendable {
        /// By its value, as an integer operand: an integer or boolean property.
        case value
        /// Only whether the document holds it, in `present` or `absent`.
        case presence
        /// By its value, compared with strings: a string property.
        case text
        /// By its value, compared with identifiers: an identifier property.
        case identifier
        /// By its size, in a `length` or `byteLength` operand: a string
        /// property, measured in characters (as `maxLength` counts them) or
        /// in UTF-8 bytes.
        case length
        /// By its size, in a `count` operand: the items of an array property,
        /// or the bytes of a byte array property.
        case count
        /// By its elements, which a `contains` looks among: a typed array
        /// property.
        case elements
        /// A kind this build does not know, by its name.
        case other(String)

        public init(name: String) {
            switch name {
            case "value": self = .value
            case "presence": self = .presence
            case "text": self = .text
            case "identifier": self = .identifier
            case "length": self = .length
            case "count": self = .count
            case "elements": self = .elements
            default: self = .other(name)
            }
        }

        /// The kind's name, as Rust reports it.
        public var name: String {
            switch self {
            case .value: return "value"
            case .presence: return "presence"
            case .text: return "text"
            case .identifier: return "identifier"
            case .length: return "length"
            case .count: return "count"
            case .elements: return "elements"
            case let .other(name): return name
            }
        }
    }

    /// The property's dotted path.
    public let path: String
    public let kind: Kind

    public init(path: String, kind: Kind) {
        self.path = path
        self.kind = kind
    }

    init(jsonEntry entry: Any) throws {
        guard let read = entry as? [String: Any],
              let path = read["path"] as? String,
              let kind = read["kind"] as? String
        else {
            throw SDKError.serializationError("Malformed propertyConstraints read: \(entry)")
        }
        self.init(path: path, kind: Kind(name: kind))
    }
}

/// The first `propertyConstraints` rule a document breaks, as consensus would
/// report it in `DocumentPropertyConstraintViolatedError` (code 10422).
///
/// Rust judges the document (`dash_sdk_data_contract_check_property_constraints`)
/// with the check consensus runs; this type only carries the verdict. The
/// fields mirror wasm-dpp2's `DocumentPropertyConstraintViolation`.
public struct PropertyConstraintViolation: Error, Equatable, Sendable, LocalizedError {
    /// Why the rule is broken; the names are wasm-dpp2's
    /// `PropertyConstraintViolationKind`.
    public enum Kind: Hashable, Sendable {
        /// The rule evaluates without a fault but does not hold.
        case notMet
        /// A value the rule reads or computes does not fit a 128-bit signed
        /// integer.
        case overflow
        /// A `divide` or `modulo` by zero.
        case divisionByZero
        /// A `power` with a negative exponent.
        case negativeExponent
        /// A value the rule reads is not an integer.
        case notAnInteger
        /// A reason this build does not know, by its name.
        case other(String)

        public init(name: String) {
            switch name {
            case "NotMet": self = .notMet
            case "Overflow": self = .overflow
            case "DivisionByZero": self = .divisionByZero
            case "NegativeExponent": self = .negativeExponent
            case "NotAnInteger": self = .notAnInteger
            default: self = .other(name)
            }
        }

        /// The reason's name, as Rust reports it.
        public var name: String {
            switch self {
            case .notMet: return "NotMet"
            case .overflow: return "Overflow"
            case .divisionByZero: return "DivisionByZero"
            case .negativeExponent: return "NegativeExponent"
            case .notAnInteger: return "NotAnInteger"
            case let .other(name): return name
            }
        }
    }

    /// The broken rule's name.
    public let rule: String
    public let violation: Kind
    /// A readable reason, as in the consensus error's message.
    public let message: String

    public init(rule: String, violation: Kind, message: String) {
        self.rule = rule
        self.violation = violation
        self.message = message
    }

    public var errorDescription: String? {
        "The document breaks the propertyConstraints rule \"\(rule)\" (\(violation.name)): \(message)."
    }

    /// Decode the JSON `dash_sdk_data_contract_check_property_constraints`
    /// returns: `nil` for JSON `null`, when the document meets every rule.
    public static func decode(fromJSON json: String) throws -> PropertyConstraintViolation? {
        let object = try PropertyConstraintJSON.object(from: json)
        if object is NSNull {
            return nil
        }
        guard let violation = object as? [String: Any],
              let rule = violation["rule"] as? String,
              let kind = violation["violation"] as? String,
              let message = violation["message"] as? String
        else {
            throw SDKError.serializationError("Malformed propertyConstraints violation: \(json)")
        }
        return PropertyConstraintViolation(rule: rule, violation: Kind(name: kind), message: message)
    }
}

// MARK: - FFI

extension SDK {
    /// The `propertyConstraints` rules of `documentType`, in name order (the
    /// order consensus checks them in). Empty for a type declaring none, and for
    /// every type while this SDK's protocol version is below 14.
    ///
    /// `serializedContract` is the contract's platform serialization, the bytes
    /// kept beside a fetched contract (`PersistentDataContract.binarySerialization`).
    /// Rust reads it at this SDK's protocol version. Bridges
    /// `dash_sdk_data_contract_get_property_constraints`.
    ///
    /// - Throws: `SDKError.notFound` for a document type the contract does not
    ///   declare, `SDKError.serializationError` for bytes that are not a
    ///   contract.
    public func documentPropertyConstraints(
        serializedContract: Data,
        documentType: String
    ) throws -> [DocumentPropertyConstraint] {
        guard let handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        let result = serializedContract.withUnsafeBytes { contract in
            documentType.withCString { documentType in
                dash_sdk_data_contract_get_property_constraints(
                    handle,
                    contract.bindMemory(to: UInt8.self).baseAddress,
                    UInt(contract.count),
                    documentType
                )
            }
        }
        return try DocumentPropertyConstraint.list(fromJSON: PropertyConstraintJSON.text(of: result))
    }

    /// The first `propertyConstraints` rule a document to create would break,
    /// or `nil` when it meets every rule judged (always so while this SDK's
    /// protocol version is below 14).
    ///
    /// `propertiesJSON` is the properties JSON the document would be created
    /// with (the string handed to `ManagedPlatformWallet.createDocument`) and
    /// `ownerId` the 32-byte identity that would own it, which `$ownerId`
    /// reads. Rust builds the document the create path builds and judges it
    /// with the check consensus runs; nothing but the rules is checked.
    /// `serializedContract` is as for `documentPropertyConstraints`. Bridges
    /// `dash_sdk_data_contract_check_property_constraints`.
    ///
    /// The block the create lands in is not known yet, so Rust estimates its
    /// system values: the device clock stands in for the block time the create
    /// records as `$createdAt`, `$updatedAt` and `$transferredAt`, and a rule
    /// reading a block height (a `readsSystem` name ending in `BlockHeight`
    /// or `CoreBlockHeight`) is not judged at all. So `nil` does not promise
    /// consensus accepts the document: a rule reading a block height, or a
    /// time rule the device clock judges differently from the block time, can
    /// still refuse it.
    ///
    /// - Throws: `SDKError.invalidParameter` for an owner id that is not 32
    ///   bytes or properties that are not a JSON object, `SDKError.notFound` for
    ///   a document type the contract does not declare,
    ///   `SDKError.serializationError` for bytes that are not a contract.
    public func checkDocumentPropertyConstraints(
        serializedContract: Data,
        documentType: String,
        propertiesJSON: String,
        ownerId: Identifier
    ) throws -> PropertyConstraintViolation? {
        guard let handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        // The FFI reads exactly 32 bytes behind the pointer
        guard ownerId.count == 32 else {
            throw SDKError.invalidParameter("Owner ID must be 32 bytes, got \(ownerId.count)")
        }
        let result = serializedContract.withUnsafeBytes { contract in
            ownerId.withUnsafeBytes { owner in
                documentType.withCString { documentType in
                    propertiesJSON.withCString { propertiesJSON in
                        dash_sdk_data_contract_check_property_constraints(
                            handle,
                            contract.bindMemory(to: UInt8.self).baseAddress,
                            UInt(contract.count),
                            documentType,
                            propertiesJSON,
                            owner.bindMemory(to: UInt8.self).baseAddress
                        )
                    }
                }
            }
        }
        return try PropertyConstraintViolation.decode(fromJSON: PropertyConstraintJSON.text(of: result))
    }
}

// MARK: - JSON readers

enum PropertyConstraintJSON {
    /// The C string a `DashSDKResult` carries, freeing it, or the error it
    /// carries as an `SDKError` (keeping its code), freeing that.
    static func text(of result: DashSDKResult) throws -> String {
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        guard let data = result.data else {
            throw SDKError.internalError("No data returned")
        }
        let text = String(cString: data.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(data.assumingMemoryBound(to: CChar.self))
        return text
    }

    /// The JSON value `text` holds, `NSNull` for `null`.
    static func object(from text: String) throws -> Any {
        guard let data = text.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
        else {
            throw SDKError.serializationError("Not JSON: \(text)")
        }
        return object
    }

    /// `value` as compact JSON text with sorted keys.
    static func compactText(_ value: Any) throws -> String {
        guard JSONSerialization.isValidJSONObject([value]),
              let data = try? JSONSerialization.data(
                withJSONObject: value,
                options: [.sortedKeys, .withoutEscapingSlashes, .fragmentsAllowed]),
              let text = String(data: data, encoding: .utf8)
        else {
            throw SDKError.serializationError("Not a JSON value: \(value)")
        }
        return text
    }
}
