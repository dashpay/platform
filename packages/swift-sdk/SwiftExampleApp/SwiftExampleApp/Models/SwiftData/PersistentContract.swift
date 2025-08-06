import Foundation
import SwiftData

/// SwiftData model for persisting Data Contract data
@Model
final class PersistentContract {
    // MARK: - Core Properties
    @Attribute(.unique) var contractId: String
    var name: String
    var version: Int32
    var ownerId: Data
    
    // MARK: - Schema Storage
    /// JSON encoded schema data
    var schemaData: Data
    
    // MARK: - Document Types
    /// JSON encoded document types
    var documentTypesData: Data
    
    // MARK: - Metadata
    @Relationship(deleteRule: .cascade, inverse: \PersistentKeyword.contract)
    var keywordRelations: [PersistentKeyword]
    var contractDescription: String?
    
    // MARK: - Token Support
    var hasTokens: Bool
    /// JSON encoded token configurations
    var tokensData: Data?
    
    // MARK: - Groups
    /// JSON encoded groups data
    var groupsData: Data?
    
    // MARK: - Timestamps
    var createdAt: Date
    var lastUpdated: Date
    var lastSyncedAt: Date?
    
    // MARK: - Network
    var network: String
    
    // MARK: - Relationships
    @Relationship(deleteRule: .cascade) var documents: [PersistentDocument]
    
    // MARK: - Initialization
    init(
        contractId: String,
        name: String,
        version: Int32 = 1,
        ownerId: Data,
        schema: [String: Any] = [:],
        documentTypes: [String] = [],
        keywords: [String] = [],
        description: String? = nil,
        hasTokens: Bool = false,
        network: String = "testnet"
    ) {
        self.contractId = contractId
        self.name = name
        self.version = version
        self.ownerId = ownerId
        self.schemaData = (try? JSONSerialization.data(withJSONObject: schema)) ?? Data()
        self.documentTypesData = (try? JSONSerialization.data(withJSONObject: documentTypes)) ?? Data()
        self.keywordRelations = keywords.map { PersistentKeyword(keyword: $0, contractId: contractId) }
        self.contractDescription = description
        self.hasTokens = hasTokens
        self.tokensData = nil
        self.groupsData = nil
        self.documents = []
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.lastSyncedAt = nil
        self.network = network
    }
    
    // MARK: - Computed Properties
    /// Get the owner ID as a hex string
    var ownerIdString: String {
        ownerId.toHexString()
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
    
    var documentTypes: [String] {
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
    
    var tokens: [String: Any]? {
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
    
    // MARK: - Methods
    func updateVersion(_ newVersion: Int32) {
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

// MARK: - Conversion Extensions

extension PersistentContract {
    /// Convert to app's ContractModel
    func toContractModel() -> ContractModel {
        // Parse token configurations if available
        var tokenConfigs: [TokenConfiguration] = []
        if let tokensDict = tokens {
            // Convert JSON representation back to TokenConfiguration objects
            // This is simplified - in production you'd have proper deserialization
            tokenConfigs = tokensDict.compactMap { (_, value) in
                guard let tokenData = value as? [String: Any] else { return nil }
                // Create TokenConfiguration from data
                return nil // Placeholder - would implement proper conversion
            }
        }
        
        return ContractModel(
            id: contractId,
            name: name,
            version: Int(version),
            ownerId: ownerId,
            documentTypes: documentTypes,
            schema: schema,
            dppDataContract: nil, // Would need to reconstruct from data
            tokens: tokenConfigs,
            keywords: self.keywords,
            description: contractDescription
        )
    }
    
    /// Create from ContractModel
    static func from(_ model: ContractModel, network: String = "testnet") -> PersistentContract {
        let persistent = PersistentContract(
            contractId: model.id,
            name: model.name,
            version: Int32(model.version),
            ownerId: model.ownerId,
            schema: model.schema,
            documentTypes: model.documentTypes,
            keywords: model.keywords ?? [],
            description: model.description,
            hasTokens: !model.tokens.isEmpty,
            network: network
        )
        
        // Convert tokens to JSON representation
        if !model.tokens.isEmpty {
            var tokensDict: [String: Any] = [:]
            for token in model.tokens {
                tokensDict[token.symbol] = tokenConfigurationToJSON(token)
            }
            persistent.tokens = tokensDict
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
        var json: [String: Any] = [
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

// MARK: - Queries

extension PersistentContract {
    /// Predicate to find contract by ID
    static func predicate(contractId: String) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.contractId == contractId
        }
    }
    
    /// Predicate to find contracts by owner
    static func predicate(ownerId: Data) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.ownerId == ownerId
        }
    }
    
    /// Predicate to find contracts by name
    static func predicate(name: String) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.name.localizedStandardContains(name)
        }
    }
    
    /// Predicate to find contracts with tokens
    static var contractsWithTokensPredicate: Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.hasTokens == true
        }
    }
    
    /// Predicate to find contracts by keyword
    static func predicate(keyword: String) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.keywordRelations.contains { $0.keyword == keyword }
        }
    }
    
    /// Predicate to find contracts needing sync
    static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.lastSyncedAt == nil || contract.lastSyncedAt! < date
        }
    }
    
    /// Predicate to find contracts by network
    static func predicate(network: String) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.network == network
        }
    }
    
    /// Predicate to find contracts with tokens by network
    static func contractsWithTokensPredicate(network: String) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.hasTokens == true && contract.network == network
        }
    }
}