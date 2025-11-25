import Foundation
import SwiftData

/// SwiftData model for persisting data contracts
@Model
public final class PersistentDataContract {
    @Attribute(.unique) public var id: Data
    public var name: String
    public var serializedContract: Data
    public var createdAt: Date
    public var lastAccessedAt: Date

    // Binary serialization (CBOR format)
    public var binarySerialization: Data?

    // Version info
    public var version: Int?
    public var ownerId: Data?

    // Keywords and description
    @Relationship(deleteRule: .cascade, inverse: \PersistentKeyword.dataContract)
    public var keywordRelations: [PersistentKeyword]
    public var contractDescription: String?

    // Schema and document types storage
    public var schemaData: Data
    public var documentTypesData: Data

    // Groups
    public var groupsData: Data?

    // Network
    public var network: String

    // Timestamps
    public var lastUpdated: Date
    public var lastSyncedAt: Date?

    // Contract configuration
    public var canBeDeleted: Bool
    public var readonly: Bool
    public var keepsHistory: Bool
    public var schemaDefs: Int?

    // Document defaults
    public var documentsKeepHistoryContractDefault: Bool
    public var documentsMutableContractDefault: Bool
    public var documentsCanBeDeletedContractDefault: Bool

    // Relationships with cascade delete
    @Relationship(deleteRule: .cascade, inverse: \PersistentToken.dataContract)
    public var tokens: [PersistentToken]?

    @Relationship(deleteRule: .cascade, inverse: \PersistentDocumentType.dataContract)
    public var documentTypes: [PersistentDocumentType]?

    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.dataContract)
    public var documents: [PersistentDocument]

    // Token support tracking
    public var hasTokens: Bool
    public var tokensData: Data?

    // Computed properties
    public var idBase58: String {
        id.toBase58String()
    }

    public var ownerIdBase58: String? {
        ownerId?.toBase58String()
    }

    public var parsedContract: [String: Any]? {
        try? JSONSerialization.jsonObject(with: serializedContract, options: []) as? [String: Any]
    }

    public var binarySerializationHex: String? {
        binarySerialization?.toHexString()
    }

    public var keywords: [String] {
        keywordRelations.map { $0.keyword }
    }

    public var schema: [String: Any] {
        get {
            guard let json = try? JSONSerialization.jsonObject(with: schemaData),
                  let dict = json as? [String: Any] else {
                return [:]
            }
            return dict
        }
        set {
            schemaData = (try? JSONSerialization.data(withJSONObject: newValue)) ?? Data()
            lastUpdated = Date()
        }
    }

    public var documentTypesList: [String] {
        get {
            guard let json = try? JSONSerialization.jsonObject(with: documentTypesData),
                  let array = json as? [String] else {
                return []
            }
            return array
        }
        set {
            documentTypesData = (try? JSONSerialization.data(withJSONObject: newValue)) ?? Data()
            lastUpdated = Date()
        }
    }

    public var tokenConfigurations: [String: Any]? {
        get {
            guard let data = tokensData,
                  let json = try? JSONSerialization.jsonObject(with: data),
                  let dict = json as? [String: Any] else {
                return nil
            }
            return dict
        }
        set {
            if let newValue = newValue {
                tokensData = try? JSONSerialization.data(withJSONObject: newValue)
                hasTokens = true
            } else {
                tokensData = nil
                hasTokens = false
            }
            lastUpdated = Date()
        }
    }

    public var groups: [String: Any]? {
        get {
            guard let data = groupsData,
                  let json = try? JSONSerialization.jsonObject(with: data),
                  let dict = json as? [String: Any] else {
                return nil
            }
            return dict
        }
        set {
            if let newValue = newValue {
                groupsData = try? JSONSerialization.data(withJSONObject: newValue)
            } else {
                groupsData = nil
            }
            lastUpdated = Date()
        }
    }

    public init(
        id: Data,
        name: String,
        serializedContract: Data,
        version: Int? = 1,
        ownerId: Data? = nil,
        schema: [String: Any] = [:],
        documentTypesList: [String] = [],
        keywords: [String] = [],
        description: String? = nil,
        hasTokens: Bool = false,
        network: String = "testnet"
    ) {
        self.id = id
        self.name = name
        self.serializedContract = serializedContract
        self.createdAt = Date()
        self.lastAccessedAt = Date()
        self.version = version
        self.ownerId = ownerId

        // Schema and document types
        self.schemaData = (try? JSONSerialization.data(withJSONObject: schema)) ?? Data()
        self.documentTypesData = (try? JSONSerialization.data(withJSONObject: documentTypesList)) ?? Data()

        // Keywords
        self.keywordRelations = keywords.map { PersistentKeyword(keyword: $0, contractId: id.toBase58String()) }
        self.contractDescription = description

        // Tokens
        self.hasTokens = hasTokens
        self.tokensData = nil

        // Groups
        self.groupsData = nil

        // Documents
        self.documents = []

        // Network and timestamps
        self.network = network
        self.lastUpdated = Date()
        self.lastSyncedAt = nil

        // Default values for contract configuration
        self.canBeDeleted = false
        self.readonly = false
        self.keepsHistory = false
        self.documentsKeepHistoryContractDefault = false
        self.documentsMutableContractDefault = true
        self.documentsCanBeDeletedContractDefault = true
    }

    public func updateLastAccessed() {
        self.lastAccessedAt = Date()
    }

    public func updateVersion(_ newVersion: Int) {
        self.version = newVersion
        self.lastUpdated = Date()
    }

    public func markAsSynced() {
        self.lastSyncedAt = Date()
    }

    public func addDocument(_ document: PersistentDocument) {
        documents.append(document)
        lastUpdated = Date()
    }

    public func removeDocument(withId documentId: String) {
        if let docIdData = Data.identifier(fromBase58: documentId) {
            documents.removeAll { $0.id == docIdData }
        }
        lastUpdated = Date()
    }
}

// MARK: - Queries
extension PersistentDataContract {
    public static func predicate(contractId: String) -> Predicate<PersistentDataContract> {
        guard let idData = Data.identifier(fromBase58: contractId) else {
            return #Predicate<PersistentDataContract> { _ in false }
        }
        return #Predicate<PersistentDataContract> { contract in
            contract.id == idData
        }
    }

    public static func predicate(ownerId: Data) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.ownerId == ownerId
        }
    }

    public static func predicate(name: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.name.localizedStandardContains(name)
        }
    }

    public static var contractsWithTokensPredicate: Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.hasTokens == true
        }
    }

    public static func predicate(keyword: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.keywordRelations.contains { $0.keyword == keyword }
        }
    }

    public static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.lastSyncedAt == nil || contract.lastSyncedAt! < date
        }
    }

    public static func predicate(network: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.network == network
        }
    }

    public static func contractsWithTokensPredicate(network: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.hasTokens == true && contract.network == network
        }
    }
}
