import Foundation

// MARK: - Data Contract Models based on DPP

/// Main Data Contract structure (using V1 as it's the latest)
struct DPPDataContract: Identifiable, Codable, Equatable {
    let id: Identifier
    let version: UInt32
    let ownerId: Identifier
    let documentTypes: [DocumentName: DocumentType]
    let config: DataContractConfig
    let schemaDefs: [DefinitionName: PlatformValue]?
    let createdAt: TimestampMillis?
    let updatedAt: TimestampMillis?
    let createdAtBlockHeight: BlockHeight?
    let updatedAtBlockHeight: BlockHeight?
    let createdAtEpoch: EpochIndex?
    let updatedAtEpoch: EpochIndex?
    let groups: [GroupContractPosition: Group]
    let tokens: [TokenContractPosition: TokenConfiguration]
    let keywords: [String]
    let description: String?
    
    /// Get the contract ID as a string
    var idString: String {
        id.toBase58String()
    }
    
    /// Get the owner ID as a string
    var ownerIdString: String {
        ownerId.toBase58String()
    }
    
    /// Get created date
    var createdDate: Date? {
        guard let createdAt = createdAt else { return nil }
        return Date(timeIntervalSince1970: Double(createdAt) / 1000)
    }
    
    /// Get updated date
    var updatedDate: Date? {
        guard let updatedAt = updatedAt else { return nil }
        return Date(timeIntervalSince1970: Double(updatedAt) / 1000)
    }
}

// MARK: - Document Type

struct DocumentType: Codable, Equatable {
    let name: String
    let schema: JsonSchema
    let indices: [Index]
    let properties: [String: DocumentProperty]
    let security: DocumentTypeSecurity
    let transientFields: [String]
    let requiresIdentityEncryptionBoundedKey: KeyBounds?
    let requiresIdentityDecryptionBoundedKey: KeyBounds?
    let tokenContractPosition: TokenContractPosition?
    let signatureVerificationConfiguration: SignatureVerificationConfiguration?
    let transferable: Transferable
    let tradeMode: TradeMode
    
    /// Check if documents of this type can be transferred
    var canBeTransferred: Bool {
        switch transferable {
        case .never: return false
        case .always: return true
        case .withCreatorPermission: return true
        }
    }
}

// MARK: - Document Property

struct DocumentProperty: Codable, Equatable {
    let type: PropertyType
    let description: String?
    let format: String?
    let pattern: String?
    let minLength: Int?
    let maxLength: Int?
    let minimum: Double?
    let maximum: Double?
    let required: Bool
    let transient: Bool
    let position: UInt32?
}

// MARK: - Property Type

enum PropertyType: String, Codable {
    case string
    case integer
    case number
    case boolean
    case array
    case object
    case bytes
}

// MARK: - Index

struct Index: Codable, Equatable {
    let name: String
    let properties: [IndexProperty]
    let unique: Bool
    let contestedUniqueIndexInformation: ContestedUniqueIndexInformation?
}

// MARK: - Index Property

struct IndexProperty: Codable, Equatable {
    let name: String
    let order: IndexOrder
}

enum IndexOrder: String, Codable {
    case ascending = "asc"
    case descending = "desc"
}

// MARK: - Contested Unique Index Information

struct ContestedUniqueIndexInformation: Codable, Equatable {
    let contestResolution: ContestResolution
    let documentAcceptsContest: Bool
    let description: String?
}

enum ContestResolution: UInt8, Codable {
    case firstComeFirstServe = 0
    case masternodesVote = 1
}

// MARK: - Document Type Security

struct DocumentTypeSecurity: Codable, Equatable {
    let insertSignable: Bool
    let updateSignable: Bool
    let deleteSignable: Bool
}

// MARK: - Key Bounds

struct KeyBounds: Codable, Equatable {
    let minItems: UInt32
    let maxItems: UInt32
}

// MARK: - Signature Verification Configuration

struct SignatureVerificationConfiguration: Codable, Equatable {
    let enabled: Bool
    let requiredSignatures: UInt32
    let publicKeyIds: [KeyID]?
}

// MARK: - Transferable

enum Transferable: UInt8, Codable {
    case never = 0
    case always = 1
    case withCreatorPermission = 2
}

// MARK: - Trade Mode

enum TradeMode: UInt8, Codable {
    case directPurchase = 0
    case sellerSetsPrice = 1
}

// MARK: - Data Contract Config

struct DataContractConfig: Codable, Equatable {
    let canBeDeleted: Bool
    let readOnly: Bool
    let keepsHistory: Bool
    let documentsKeepRevisionLogForPassedTimeMs: TimestampMillis?
    let documentsMutableContractDefaultStored: Bool
}

// MARK: - Group

struct Group: Codable, Equatable {
    let members: [[UInt8]] // Array of identity IDs (each 32 bytes)
    let requiredPower: UInt32
    
    var memberIdentifiers: [Identifier] {
        members.map { Data($0) }
    }
}

// MARK: - Token Configuration

struct TokenConfiguration: Codable, Equatable {
    let name: String
    let symbol: String
    let description: String?
    let decimals: UInt8
    let totalSupplyInLowestDenomination: UInt64
    let mintable: Bool
    let burnable: Bool
    let cappedSupply: Bool
    let transferable: Bool
    let tradeable: Bool
    let sellable: Bool
    let freezable: Bool
    let pausable: Bool
    let destructible: Bool
    let rulesVersion: UInt16
    let ruleGroups: TokenRuleGroups?
    
    /// Get total supply formatted with decimals
    var formattedTotalSupply: String {
        let divisor = pow(10.0, Double(decimals))
        let amount = Double(totalSupplyInLowestDenomination) / divisor
        return String(format: "%.\(decimals)f %@", amount, symbol)
    }
}

// MARK: - Token Rule Groups

struct TokenRuleGroups: Codable, Equatable {
    let ownerRules: TokenOwnerRules?
    let everyoneRules: TokenEveryoneRules?
}

struct TokenOwnerRules: Codable, Equatable {
    let canMint: Bool
    let canBurn: Bool
    let canPause: Bool
    let canFreeze: Bool
    let canDestroy: Bool
    let maxMintAmount: UInt64?
}

struct TokenEveryoneRules: Codable, Equatable {
    let canTransfer: Bool
    let canBurn: Bool
    let maxTransferAmount: UInt64?
}

// MARK: - Json Schema

struct JsonSchema: Codable, Equatable {
    let type: String
    let properties: [String: JsonSchemaProperty]
    let required: [String]
    let additionalProperties: Bool
}

indirect enum JsonSchemaPropertyValue: Codable, Equatable {
    case property(JsonSchemaProperty)
}

struct JsonSchemaProperty: Codable, Equatable {
    let type: String
    let description: String?
    let format: String?
    let pattern: String?
    let minLength: Int?
    let maxLength: Int?
    let minimum: Double?
    let maximum: Double?
    let items: JsonSchemaPropertyValue?
}

// MARK: - Factory Methods

extension DPPDataContract {
    /// Create a simple data contract
    static func create(
        id: Identifier? = nil,
        ownerId: Identifier,
        documentTypes: [DocumentName: DocumentType] = [:],
        description: String? = nil
    ) -> DPPDataContract {
        let contractId = id ?? Data(UUID().uuidString.utf8).prefix(32).paddedToLength(32)
        
        return DPPDataContract(
            id: contractId,
            version: 0,
            ownerId: ownerId,
            documentTypes: documentTypes,
            config: DataContractConfig(
                canBeDeleted: false,
                readOnly: false,
                keepsHistory: true,
                documentsKeepRevisionLogForPassedTimeMs: nil,
                documentsMutableContractDefaultStored: true
            ),
            schemaDefs: nil,
            createdAt: TimestampMillis(Date().timeIntervalSince1970 * 1000),
            updatedAt: nil,
            createdAtBlockHeight: nil,
            updatedAtBlockHeight: nil,
            createdAtEpoch: nil,
            updatedAtEpoch: nil,
            groups: [:],
            tokens: [:],
            keywords: [],
            description: description
        )
    }
}