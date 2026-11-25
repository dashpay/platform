import Foundation
import DashSDKFFI

/// Pasted or imported contract JSON split into the sections a data contract
/// update takes. Returned by ``split(_:)``; each section feeds the matching
/// argument of
/// ``ManagedPlatformWallet/previewDataContractUpdate(ownerIdentityId:contractId:documentSchemasJSON:tokenSchemasJSON:groupsJSON:keywordsJSON:description:contractConfigJSON:)``
/// and `updateDataContract`.
///
/// Mirrors the JSON `platform_wallet_split_contract_update_source` writes,
/// which Rust's `split_contract_update_source` fills. Every JSON section is
/// the source's own text in source order: the order of a schema's members is
/// part of the contract (protocol version 15 refuses a reorder), so a host
/// passes a section on verbatim and never re-serializes it.
public struct DataContractUpdateSource: Decodable, Sendable, Equatable {
    /// Document schemas keyed by type name, as JSON text. Nil when the
    /// source supplies none.
    public let documents: String?
    /// Token configurations keyed by position, as JSON text. Nil when the
    /// source supplies none.
    public let tokens: String?
    /// Groups, as JSON text (an object keyed by position, or an array). Nil
    /// when the source supplies none.
    public let groups: String?
    /// Keywords, as JSON array text. Nil when the source supplies none.
    public let keywords: String?
    /// The description text itself, not JSON. Nil when the source supplies
    /// none or a `null` one.
    public let description: String?
    /// The contract configuration, as JSON text. Nil when the source
    /// supplies none or a `null` one.
    public let config: String?
    /// Whether the source has a non-empty `schemaDefs` or `$defs` object. An
    /// update does not send shared definitions: Platform keeps the live ones.
    /// Computed by Rust.
    public let carriesSharedDefinitions: Bool

    private enum CodingKeys: String, CodingKey {
        case documents
        case tokens
        case groups
        case keywords
        case description
        case config
        case carriesSharedDefinitions
    }

    init(
        documents: String?,
        tokens: String?,
        groups: String?,
        keywords: String?,
        description: String?,
        config: String?,
        carriesSharedDefinitions: Bool
    ) {
        self.documents = documents
        self.tokens = tokens
        self.groups = groups
        self.keywords = keywords
        self.description = description
        self.config = config
        self.carriesSharedDefinitions = carriesSharedDefinitions
    }

    /// Every key is required, a section as a string or `null`: the FFI
    /// writes all of them, so a missing one is a shape mismatch, not a
    /// section the source left out.
    public init(from decoder: Decoder) throws {
        // `Swift.String`: the FFI header imports a C enum constant named
        // `String`, which a bare `String?` would resolve to here.
        let container = try decoder.container(keyedBy: CodingKeys.self)
        documents = try container.decode(Swift.String?.self, forKey: .documents)
        tokens = try container.decode(Swift.String?.self, forKey: .tokens)
        groups = try container.decode(Swift.String?.self, forKey: .groups)
        keywords = try container.decode(Swift.String?.self, forKey: .keywords)
        description = try container.decode(Swift.String?.self, forKey: .description)
        config = try container.decode(Swift.String?.self, forKey: .config)
        carriesSharedDefinitions = try container.decode(Bool.self, forKey: .carriesSharedDefinitions)
    }

    /// Decode the JSON `platform_wallet_split_contract_update_source` writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> DataContractUpdateSource {
        do {
            return try JSONDecoder().decode(DataContractUpdateSource.self, from: Data(json.utf8))
        } catch {
            throw PlatformWalletError.deserialization(
                "Failed to decode the contract update sections: \(error)"
            )
        }
    }

    /// Split pasted or imported contract JSON into the sections of a data
    /// contract update.
    ///
    /// Routes through `split_contract_update_source` (via
    /// `platform_wallet_split_contract_update_source`), which decides
    /// everything: which members are sections, a bare document schemas map,
    /// a section given under both of its names, a `null` description or
    /// config, and whether the source carries shared definitions. Needs no
    /// wallet and touches no network.
    ///
    /// - Throws: `PlatformWalletError.invalidParameter` when `text` holds a
    ///   NUL; otherwise the error Rust returns for text it cannot split
    ///   (not a JSON object, no section, a section given twice, keywords
    ///   that are not an array, a description that is not a string), whose
    ///   message is meant to be shown as is.
    public static func split(_ text: String) throws -> DataContractUpdateSource {
        // `withCString` would hand Rust the text cut at the NUL.
        try KeyDerivation.requireNoNUL(text, "Contract JSON")

        // Receives an owned JSON C string on success; freed with
        // `platform_wallet_string_free` on every path.
        var sectionsJsonPtr: UnsafeMutablePointer<CChar>? = nil
        defer { if let p = sectionsJsonPtr { platform_wallet_string_free(p) } }

        // Rust reads the borrowed C string synchronously, inside the
        // `withCString` scope.
        let result = text.withCString { textPtr in
            platform_wallet_split_contract_update_source(textPtr, &sectionsJsonPtr)
        }

        try result.check()
        // On success the Rust side always writes the JSON; a null pointer is
        // an FFI contract violation.
        guard let jsonPtr = sectionsJsonPtr else {
            throw PlatformWalletError.walletOperation(
                "split_contract_update_source returned no sections JSON"
            )
        }
        return try decode(fromJSON: String(cString: jsonPtr))
    }
}
