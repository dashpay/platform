import Foundation
import SwiftData

/// SwiftData model for persisting Data Contract data
@Model
final class PersistentContract {
    // MARK: - Core Properties
    @Attribute(.unique) var contractId: String
    var name: String
    var version: Int32
    var ownerId: String
    
    // MARK: - Schema Storage
    /// JSON encoded schema data
    var schemaData: Data
    
    // MARK: - Document Types
    /// JSON encoded document types
    var documentTypesData: Data
    
    // MARK: - Metadata
    var keywords: [String]
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
        ownerId: String,
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
        self.keywords = keywords
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
        documents.removeAll { $0.documentId == documentId }
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
            keywords: keywords,
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
            keywords: model.keywords,
            description: model.description,
            hasTokens: !model.tokens.isEmpty,
            network: network
        )
        
        // Convert tokens to JSON representation
        if !model.tokens.isEmpty {
            var tokensDict: [String: Any] = [:]
            for token in model.tokens {
                tokensDict[token.id.uuidString] = tokenConfigurationToJSON(token)
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
                    groupsDict[groupId.uuidString] = [
                        "members": group.members.map { $0.base64EncodedString() },
                        "requiredSigners": group.requiredSigners
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
            "id": token.id.uuidString,
            "name": token.name,
            "symbol": token.symbol,
            "decimals": token.decimals,
            "mintable": token.mintable,
            "burnable": token.burnable,
            "cappedSupply": token.cappedSupply,
            "maxSupply": token.maxSupply as Any,
            "transferable": token.transferable,
            "tradeable": token.tradeable,
            "sellable": token.sellable,
            "freezable": token.freezable,
            "pausable": token.pausable,
            "destructible": token.destructible,
            "destructibleByOwner": token.destructibleByOwner,
            "masterCanMint": token.masterCanMint,
            "masterCanBurn": token.masterCanBurn,
            "groupId": token.groupId?.uuidString as Any
        ]
        
        // Add rules if present
        if !token.rules.isEmpty {
            var rulesArray: [[String: Any]] = []
            for rule in token.rules {
                var ruleDict: [String: Any] = [:]
                switch rule.action {
                case .transfer(let details):
                    ruleDict["action"] = "transfer"
                    if let minAmount = details.minAmount {
                        ruleDict["minAmount"] = minAmount
                    }
                    if let maxAmount = details.maxAmount {
                        ruleDict["maxAmount"] = maxAmount
                    }
                case .mint(let details):
                    ruleDict["action"] = "mint"
                    if let maxAmount = details.maxAmount {
                        ruleDict["maxAmount"] = maxAmount
                    }
                case .burn(let details):
                    ruleDict["action"] = "burn"
                    if let minAmount = details.minAmount {
                        ruleDict["minAmount"] = minAmount
                    }
                    if let maxAmount = details.maxAmount {
                        ruleDict["maxAmount"] = maxAmount
                    }
                }
                
                if let allowedSenders = rule.allowedSenders {
                    ruleDict["allowedSenders"] = allowedSenders.map { $0.base64EncodedString() }
                }
                if let allowedRecipients = rule.allowedRecipients {
                    ruleDict["allowedRecipients"] = allowedRecipients.map { $0.base64EncodedString() }
                }
                
                rulesArray.append(ruleDict)
            }
            json["rules"] = rulesArray
        }
        
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
    static func predicate(ownerId: String) -> Predicate<PersistentContract> {
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
            contract.keywords.contains(keyword)
        }
    }
    
    /// Predicate to find contracts needing sync
    static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentContract> {
        #Predicate<PersistentContract> { contract in
            contract.lastSyncedAt == nil || contract.lastSyncedAt! < date
        }
    }
}