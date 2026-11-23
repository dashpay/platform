import Foundation
import SwiftDashSDK

/// Fills the Update Contract screen's six fields from pasted or imported
/// contract JSON, and turns the fields into the wallet's update arguments.
///
/// Rust splits the source into sections (`DataContractUpdateSource.split`).
/// Unlike the register flow's `PasteboardContractCandidate.classify`, the
/// import rewrites nothing and drops nothing it recognises: every supplied
/// section is written to its field as the source spells it, members in
/// source order, whatever it holds, so a malformed section reaches the
/// wallet and is refused there. The order matters: it is part of a
/// document type's schema, and protocol version 15 refuses a reorder.
/// Nothing here checks what a section says; that is the wallet's and
/// Platform's to decide.
enum UpdateContractImport {
    /// The most bytes the screen reads from one source (a file or the
    /// clipboard) before parsing it. A UI input budget that keeps a huge
    /// file from being read whole into memory, not a Platform limit:
    /// Platform decides how large a contract may be.
    static let maxInputBytes = 4 * 1024 * 1024

    /// Refuses a source of `byteCount` bytes when it is over
    /// `maxInputBytes`.
    static func checkSize(byteCount: Int) throws {
        if byteCount > maxInputBytes {
            throw ImportError.tooLarge
        }
    }

    /// The six editor strings of the Update Contract screen.
    ///
    /// Equality compares the UTF-8 bytes of every field. Swift `String`
    /// equality treats canonically equivalent spellings as equal (`café`
    /// precomposed and decomposed), but the wallet receives the bytes, so an
    /// edit that changes only the normalization form must still clear a
    /// review made with the other bytes.
    struct Fields: Equatable {
        var documents: String
        var tokens: String
        var groups: String
        var keywords: String
        var description: String
        /// The contract configuration JSON. Empty keeps the live one.
        var config: String

        /// The fields as the screen opens: documents `{}` (no document type
        /// changes), everything else empty (keep the live value).
        static let empty = Fields(
            documents: "{}",
            tokens: "",
            groups: "",
            keywords: "",
            description: "",
            config: ""
        )

        static func == (lhs: Fields, rhs: Fields) -> Bool {
            Array(lhs.documents.utf8) == Array(rhs.documents.utf8)
                && Array(lhs.tokens.utf8) == Array(rhs.tokens.utf8)
                && Array(lhs.groups.utf8) == Array(rhs.groups.utf8)
                && Array(lhs.keywords.utf8) == Array(rhs.keywords.utf8)
                && Array(lhs.description.utf8) == Array(rhs.description.utf8)
                && Array(lhs.config.utf8) == Array(rhs.config.utf8)
        }
    }

    enum ImportError: Error, LocalizedError, Equatable {
        /// The source is over `maxInputBytes`.
        case tooLarge

        var errorDescription: String? {
            switch self {
            case .tooLarge:
                return "The input is larger than this screen's import limit of \(UpdateContractImport.maxInputBytes / (1024 * 1024)) MiB."
            }
        }
    }

    /// What an import read: the six fields, and whether the source carries
    /// shared definitions the fields leave out. Kept outside `Fields`, whose
    /// equality the screen uses to clear a review on edit.
    struct Imported {
        let fields: Fields
        /// The source has a `schemaDefs` or `$defs` member that is a
        /// non-empty JSON object. The update does not send definitions, so
        /// Platform keeps the live ones. Decided by Rust.
        let carriesSharedDefinitions: Bool
    }

    /// The fields `text` fills, and whether it carries shared definitions
    /// (`schemaDefs` or `$defs`) that the fields do not hold. A section the
    /// text does not carry gets the value of `Fields.empty`.
    ///
    /// The split is Rust's (`DataContractUpdateSource.split`, over
    /// `split_contract_update_source`): it decides which members are
    /// sections, reads a bare document schemas map, and refuses text it
    /// cannot split. Each JSON section comes back in source order and is
    /// written to its field verbatim.
    ///
    /// - Throws: the wallet's `PlatformWalletError`, whose message is
    ///   Rust's and is shown as is.
    static func read(from text: String) throws -> Imported {
        let source = try DataContractUpdateSource.split(text)
        let empty = Fields.empty
        let fields = Fields(
            documents: source.documents ?? empty.documents,
            tokens: source.tokens ?? empty.tokens,
            groups: source.groups ?? empty.groups,
            keywords: source.keywords ?? empty.keywords,
            description: source.description ?? empty.description,
            config: source.config ?? empty.config
        )
        return Imported(fields: fields, carriesSharedDefinitions: source.carriesSharedDefinitions)
    }

    /// The six fields in the form the wallet calls take. Converts text
    /// formats only; what the contents mean is the wallet's and Platform's
    /// to decide.
    struct WalletArguments {
        let documents: String
        let tokens: String?
        let groups: String?
        let keywords: String?
        let description: String?
        /// The configuration JSON, or nil to keep the live configuration.
        let config: String?

        init(_ fields: Fields) throws {
            let documents = fields.documents.trimmingCharacters(in: .whitespacesAndNewlines)
            self.documents = documents.isEmpty ? "{}" : documents
            let tokens = fields.tokens.trimmingCharacters(in: .whitespacesAndNewlines)
            self.tokens = tokens.isEmpty ? nil : tokens
            self.groups = try PasteboardContractCandidate.positionKeyedGroupsJSON(from: fields.groups)
            self.keywords = try Self.keywordsJSON(from: fields.keywords)
            // Verbatim: only an empty field keeps the live description.
            self.description = fields.description.isEmpty ? nil : fields.description
            let config = fields.config.trimmingCharacters(in: .whitespacesAndNewlines)
            self.config = config.isEmpty ? nil : config
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
