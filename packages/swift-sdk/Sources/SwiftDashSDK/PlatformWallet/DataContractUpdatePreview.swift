import Foundation

/// What a data contract update would change, read against the contract as
/// Platform holds it. Returned by
/// ``ManagedPlatformWallet/previewDataContractUpdate(ownerIdentityId:contractId:documentSchemasJSON:tokenSchemasJSON:groupsJSON:keywordsJSON:description:contractConfigJSON:)``.
///
/// Mirrors the JSON `platform_wallet_preview_data_contract_update` writes,
/// which Rust's `DataContractUpdatePreview` fills. It reports what the update
/// changes, not whether Platform accepts it: the network checks the contract
/// update rules when the update is broadcast.
public struct DataContractUpdatePreview: Decodable, Sendable, Equatable {
    /// The entries of one keyed section of a contract that an update adds or
    /// changes. An update never removes an entry.
    public struct SectionChanges: Decodable, Sendable, Equatable {
        /// Keys the contract does not have yet.
        public let added: [String]
        /// Keys the contract has, with a different definition.
        public let changed: [String]

        public init(added: [String], changed: [String]) {
            self.added = added
            self.changed = changed
        }
    }

    /// The version of the contract Platform holds.
    public let currentVersion: UInt32
    /// The version the update would give it.
    public let newVersion: UInt32
    /// Whether the update would change anything besides the version, which
    /// every update raises. Computed by Rust.
    public let hasChanges: Bool
    /// Document types, by name.
    public let documentTypes: SectionChanges
    /// Tokens, by contract position (decimal strings).
    public let tokens: SectionChanges
    /// Groups, by contract position (decimal strings).
    public let groups: SectionChanges
    /// Whether the keywords would differ.
    public let keywordsChanged: Bool
    /// Whether the description would differ.
    public let descriptionChanged: Bool
    /// Whether the contract configuration would differ.
    public let configChanged: Bool

    /// Decode the JSON `platform_wallet_preview_data_contract_update` writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> DataContractUpdatePreview {
        do {
            return try JSONDecoder().decode(DataContractUpdatePreview.self, from: Data(json.utf8))
        } catch {
            throw PlatformWalletError.deserialization(
                "Failed to decode the data contract update preview: \(error)"
            )
        }
    }
}
