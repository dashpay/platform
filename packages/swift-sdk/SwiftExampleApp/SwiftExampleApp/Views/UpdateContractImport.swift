import Foundation

/// Fills the Update Contract screen's five fields from pasted or imported
/// contract JSON, and turns the fields into the wallet's update arguments.
///
/// Unlike the register flow's `PasteboardContractCandidate.classify`, the
/// import rewrites nothing and drops nothing it recognises: every supplied
/// section is written to its field as given (re-serialized, keys sorted),
/// whatever it holds, so a malformed section reaches the wallet and is
/// refused there. Nothing here checks what a section says; that is the
/// wallet's and Platform's to decide.
enum UpdateContractImport {
    /// The five editor strings of the Update Contract screen.
    struct Fields: Equatable {
        var documents: String
        var tokens: String
        var groups: String
        var keywords: String
        var description: String

        /// The fields as the screen opens: documents `{}` (no document type
        /// changes), everything else empty (keep the live value).
        static let empty = Fields(documents: "{}", tokens: "", groups: "", keywords: "", description: "")
    }

    enum ImportError: Error, LocalizedError, Equatable {
        /// The text is not JSON.
        case notJSON
        /// The JSON is not an object.
        case notAnObject
        /// An object with none of the sections and not a document schemas
        /// map. Carries its top-level keys, sorted.
        case noSections(topLevelKeys: [String])
        /// The object names one section twice, under both of its keys.
        case sectionGivenTwice(String, String)
        /// A section's value cannot be written to its field: keywords must be
        /// a JSON array and the description a JSON string.
        case unsupportedValue(section: String, expected: String)

        var errorDescription: String? {
            switch self {
            case .notJSON:
                return "The text is not JSON."
            case .notAnObject:
                return "The JSON is not an object."
            case .noSections(let keys):
                if keys.isEmpty {
                    return "The JSON object is empty."
                }
                return "No contract sections found (documents, documentSchemas, tokens, tokenSchemas, groups, keywords, description). Top-level keys: \(keys.joined(separator: ", "))."
            case .sectionGivenTwice(let first, let second):
                return "The JSON has both `\(first)` and `\(second)`; keep one."
            case .unsupportedValue(let section, let expected):
                return "`\(section)` must be \(expected)."
            }
        }
    }

    /// What an import read: the five fields, and whether the source carries
    /// shared definitions the fields leave out. Kept outside `Fields`, whose
    /// equality the screen uses to clear a review on edit.
    struct Imported {
        let fields: Fields
        /// The object has a `schemaDefs` or `$defs` member that is a
        /// non-empty JSON object. The update does not send definitions, so
        /// Platform keeps the live ones.
        let carriesSharedDefinitions: Bool
    }

    /// The fields `text` fills. A section the text does not carry gets the
    /// value of `Fields.empty`.
    ///
    /// The text must be a JSON object. Read are `documents` or
    /// `documentSchemas`, `tokens` or `tokenSchemas`, `groups`, `keywords` and
    /// `description`; every other key (`id`, `ownerId`, `version`, `config`,
    /// `$formatVersion`, `schemaDefs`, ...) is ignored, so an exported
    /// contract imports as is. An object with none of those keys whose values
    /// all look like document schemas is read as a bare document schemas map.
    static func fields(from text: String) throws -> Fields {
        try read(from: text).fields
    }

    /// `fields(from:)`, plus whether `text` carries shared definitions
    /// (`schemaDefs` or `$defs`) that the fields do not hold.
    static func read(from text: String) throws -> Imported {
        guard let data = text.data(using: .utf8),
              let parsed = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
        else {
            throw ImportError.notJSON
        }
        guard let object = parsed as? [String: Any] else {
            throw ImportError.notAnObject
        }
        let carriesSharedDefinitions = ["schemaDefs", "$defs"].contains { key in
            guard let definitions = object[key] as? [String: Any] else { return false }
            return !definitions.isEmpty
        }

        let documents = try section(object, "documents", "documentSchemas")
        let tokens = try section(object, "tokens", "tokenSchemas")
        let groups = object["groups"]
        let keywords = object["keywords"]
        let description = object["description"]

        var fields = Fields.empty
        guard documents != nil || tokens != nil || groups != nil || keywords != nil || description != nil else {
            guard !object.isEmpty, object.values.allSatisfy(looksLikeDocumentSchema) else {
                throw ImportError.noSections(topLevelKeys: object.keys.sorted())
            }
            fields.documents = try json(object)
            return Imported(fields: fields, carriesSharedDefinitions: carriesSharedDefinitions)
        }

        if let documents {
            fields.documents = try json(documents)
        }
        if let tokens {
            fields.tokens = try json(tokens)
        }
        if let groups {
            fields.groups = try json(groups)
        }
        if let keywords {
            guard keywords is [Any] else {
                throw ImportError.unsupportedValue(section: "keywords", expected: "a JSON array")
            }
            fields.keywords = try json(keywords)
        }
        // An exported contract without a description writes
        // `"description": null` (the serialization format's description is
        // an optional it does not skip), which means no description: the
        // field stays empty and the live one is kept.
        if let description, !(description is NSNull) {
            guard let descriptionText = description as? String else {
                throw ImportError.unsupportedValue(section: "description", expected: "a JSON string")
            }
            fields.description = descriptionText
        }
        return Imported(fields: fields, carriesSharedDefinitions: carriesSharedDefinitions)
    }

    /// The value of a section that has two names, or nil when neither is
    /// present. A present key counts even when its value is `null`.
    private static func section(_ object: [String: Any], _ name: String, _ alias: String) throws -> Any? {
        switch (object[name], object[alias]) {
        case (.some, .some):
            throw ImportError.sectionGivenTwice(name, alias)
        case (.some(let value), nil), (nil, .some(let value)):
            return value
        case (nil, nil):
            return nil
        }
    }

    /// Whether `value` looks like a document type schema: an object with
    /// `properties`, or with `type` equal to `object`.
    private static func looksLikeDocumentSchema(_ value: Any) -> Bool {
        guard let schema = value as? [String: Any] else { return false }
        return schema["properties"] != nil || schema["type"] as? String == "object"
    }

    /// `value` as JSON text with sorted keys.
    private static func json(_ value: Any) throws -> String {
        let data = try JSONSerialization.data(
            withJSONObject: value,
            options: [.sortedKeys, .fragmentsAllowed, .withoutEscapingSlashes]
        )
        guard let text = String(data: data, encoding: .utf8) else {
            throw ImportError.notJSON
        }
        return text
    }

    /// The five fields in the form the wallet calls take. Converts text
    /// formats only; what the contents mean is the wallet's and Platform's
    /// to decide.
    struct WalletArguments {
        let documents: String
        let tokens: String?
        let groups: String?
        let keywords: String?
        let description: String?

        init(_ fields: Fields) throws {
            let documents = fields.documents.trimmingCharacters(in: .whitespacesAndNewlines)
            self.documents = documents.isEmpty ? "{}" : documents
            let tokens = fields.tokens.trimmingCharacters(in: .whitespacesAndNewlines)
            self.tokens = tokens.isEmpty ? nil : tokens
            self.groups = try PasteboardContractCandidate.positionKeyedGroupsJSON(from: fields.groups)
            self.keywords = try Self.keywordsJSON(from: fields.keywords)
            // Verbatim: only an empty field keeps the live description.
            self.description = fields.description.isEmpty ? nil : fields.description
        }

        /// The keywords field as the wallet's JSON array, or nil to keep the
        /// live keywords. A JSON array (text starting with `[` once leading
        /// whitespace is skipped) is passed through as typed; anything else
        /// is the comma separated form.
        static func keywordsJSON(from field: String) throws -> String? {
            if field.drop(while: { $0.isWhitespace }).first == "[" {
                return field
            }
            let keywordList = field
                .split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespaces) }
                .filter { !$0.isEmpty }
            if keywordList.isEmpty {
                return nil
            }
            let data = try JSONSerialization.data(withJSONObject: keywordList)
            return String(data: data, encoding: .utf8)
        }
    }
}
