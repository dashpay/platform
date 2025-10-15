import Foundation
import SwiftData

@Model
final class PersistentDataContract {
    @Attribute(.unique) var id: Data
    var name: String
    var serializedContract: Data
    var createdAt: Date
    var lastAccessedAt: Date
    
    // Binary serialization (CBOR format)
    var binarySerialization: Data?
    
    // Version info
    var version: Int?
    var ownerId: Data?
    
    // Keywords and description
    @Relationship(deleteRule: .cascade, inverse: \PersistentKeyword.dataContract)
    var keywordRelations: [PersistentKeyword]
    var contractDescription: String?
    
    // Schema and document types storage
    var schemaData: Data
    var documentTypesData: Data
    
    // Groups
    var groupsData: Data?
    
    // Network
    var network: String
    
    // Timestamps
    var lastUpdated: Date
    var lastSyncedAt: Date?
    
    // Contract configuration
    var canBeDeleted: Bool
    var readonly: Bool
    var keepsHistory: Bool
    var schemaDefs: Int?
    
    // Document defaults
    var documentsKeepHistoryContractDefault: Bool
    var documentsMutableContractDefault: Bool
    var documentsCanBeDeletedContractDefault: Bool
    
    // Relationships with cascade delete
    @Relationship(deleteRule: .cascade, inverse: \PersistentToken.dataContract)
    var tokens: [PersistentToken]?
    
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocumentType.dataContract)
    var documentTypes: [PersistentDocumentType]?
    
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.dataContract)
    var documents: [PersistentDocument]
    
    // Token support tracking
    var hasTokens: Bool
    var tokensData: Data?
    
    // Computed properties
    var idBase58: String {
        id.toBase58String()
    }
    
    var ownerIdBase58: String? {
        ownerId?.toBase58String()
    }
    
    var parsedContract: [String: Any]? {
        try? JSONSerialization.jsonObject(with: serializedContract, options: []) as? [String: Any]
    }
    
    var binarySerializationHex: String? {
        binarySerialization?.toHexString()
    }
    
    /// Get keywords as string array
    var keywords: [String] {
        keywordRelations.map { $0.keyword }
    }
    
    var schema: [String: Any] {
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
    
    var documentTypesList: [String] {
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
    
    var tokenConfigurations: [String: Any]? {
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
    
    var groups: [String: Any]? {
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
    
    init(
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
    
    func updateLastAccessed() {
        self.lastAccessedAt = Date()
    }
    
    func updateVersion(_ newVersion: Int) {
        self.version = newVersion
        self.lastUpdated = Date()
    }
    
    func markAsSynced() {
        self.lastSyncedAt = Date()
    }
    
    func addDocument(_ document: PersistentDocument) {
        documents.append(document)
        lastUpdated = Date()
    }
    
    func removeDocument(withId documentId: String) {
        if let docIdData = Data.identifier(fromBase58: documentId) {
            documents.removeAll { $0.id == docIdData }
        }
        lastUpdated = Date()
    }
}

// MARK: - Queries
extension PersistentDataContract {
    /// Predicate to find contract by ID (base58 string)
    static func predicate(contractId: String) -> Predicate<PersistentDataContract> {
        guard let idData = Data.identifier(fromBase58: contractId) else {
            return #Predicate<PersistentDataContract> { _ in false }
        }
        return #Predicate<PersistentDataContract> { contract in
            contract.id == idData
        }
    }
    
    /// Predicate to find contracts by owner
    static func predicate(ownerId: Data) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.ownerId == ownerId
        }
    }
    
    /// Predicate to find contracts by name
    static func predicate(name: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.name.localizedStandardContains(name)
        }
    }
    
    /// Predicate to find contracts with tokens
    static var contractsWithTokensPredicate: Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.hasTokens == true
        }
    }
    
    /// Predicate to find contracts by keyword
    static func predicate(keyword: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.keywordRelations.contains { $0.keyword == keyword }
        }
    }
    
    /// Predicate to find contracts needing sync
    static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.lastSyncedAt == nil || contract.lastSyncedAt! < date
        }
    }
    
    /// Predicate to find contracts by network
    static func predicate(network: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.network == network
        }
    }
    
    /// Predicate to find contracts with tokens by network
    static func contractsWithTokensPredicate(network: String) -> Predicate<PersistentDataContract> {
        #Predicate<PersistentDataContract> { contract in
            contract.hasTokens == true && contract.network == network
        }
    }
}

// MARK: - Conversion Extensions

extension PersistentDataContract {
    /// Convert to app's ContractModel
    func toContractModel() -> ContractModel {
        // Parse token configurations if available
        var tokenConfigs: [TokenConfiguration] = []
        if let tokensDict = tokenConfigurations {
            // Convert JSON representation back to TokenConfiguration objects
            // This is simplified - in production you'd have proper deserialization
            tokenConfigs = tokensDict.compactMap { (_, value) in
                guard let _ = value as? [String: Any] else { return nil }
                // Create TokenConfiguration from data
                return nil // Placeholder - would implement proper conversion
            }
        }
        
        return ContractModel(
            id: idBase58,
            name: name,
            version: version ?? 1,
            ownerId: ownerId ?? Data(),
            documentTypes: documentTypesList,
            schema: schema,
            dppDataContract: nil, // Would need to reconstruct from data
            tokens: tokenConfigs,
            keywords: self.keywords,
            description: contractDescription
        )
    }
    
    /// Create from ContractModel
    static func from(_ model: ContractModel, network: String = "testnet") -> PersistentDataContract {
        let idData = Data.identifier(fromBase58: model.id) ?? Data()
        let persistent = PersistentDataContract(
            id: idData,
            name: model.name,
            serializedContract: Data(), // Will be set below
            version: model.version,
            ownerId: model.ownerId,
            schema: model.schema,
            documentTypesList: model.documentTypes,
            keywords: model.keywords,
            description: model.description,
            hasTokens: !model.tokens.isEmpty,
            network: network
        )
        
        // Serialize the contract data
        if let serialized = try? JSONSerialization.data(withJSONObject: model.schema) {
            persistent.serializedContract = serialized
        }
        
        // Convert tokens to JSON representation
        if !model.tokens.isEmpty {
            var tokensDict: [String: Any] = [:]
            for token in model.tokens {
                tokensDict[token.symbol] = tokenConfigurationToJSON(token)
            }
            persistent.tokenConfigurations = tokensDict
        }
        
        // Copy DPP data contract data if available
        if let dppContract = model.dppDataContract {
            // Convert document types from DPP format
            var schemaDict: [String: Any] = [:]
            for (docType, documentType) in dppContract.documentTypes {
                var docSchema: [String: Any] = [:]
                docSchema["type"] = "object"
                docSchema["indices"] = documentType.indices.map { index in
                    return [
                        "name": index.name,
                        "properties": index.properties.map { $0.name },
                        "unique": index.unique
                    ]
                }
                docSchema["properties"] = documentType.properties.mapValues { prop in
                    return ["type": prop.type.rawValue]
                }
                schemaDict[docType] = docSchema
            }
            persistent.schema = schemaDict
            
            // Convert groups if available
            if !dppContract.groups.isEmpty {
                var groupsDict: [String: Any] = [:]
                for (groupId, group) in dppContract.groups {
                    groupsDict[String(groupId)] = [
                        "members": group.members.map { member in 
                            Data(member).base64EncodedString() 
                        },
                        "requiredPower": group.requiredPower
                    ]
                }
                persistent.groups = groupsDict
            }
        }
        
        return persistent
    }
    
    /// Convert TokenConfiguration to JSON representation
    private static func tokenConfigurationToJSON(_ token: TokenConfiguration) -> [String: Any] {
        let json: [String: Any] = [
            "name": token.name,
            "symbol": token.symbol,
            "description": token.description as Any,
            "decimals": token.decimals,
            "totalSupplyInLowestDenomination": token.totalSupplyInLowestDenomination,
            "mintable": token.mintable,
            "burnable": token.burnable,
            "cappedSupply": token.cappedSupply,
            "transferable": token.transferable,
            "tradeable": token.tradeable,
            "sellable": token.sellable,
            "freezable": token.freezable,
            "pausable": token.pausable
        ]
        
        return json
    }
}
