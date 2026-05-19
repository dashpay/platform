import Foundation

// MARK: - Data Contract Models based on DPP

/// Main Data Contract structure (using V1 as it's the latest)
public struct DPPDataContract: Identifiable, Codable, Equatable, Sendable {
    public let id: Identifier
    public let version: UInt32
    public let ownerId: Identifier
    public let documentTypes: [DocumentName: DocumentType]
    public let config: DataContractConfig
    public let schemaDefs: [DefinitionName: PlatformValue]?
    public let createdAt: TimestampMillis?
    public let updatedAt: TimestampMillis?
    public let createdAtBlockHeight: BlockHeight?
    public let updatedAtBlockHeight: BlockHeight?
    public let createdAtEpoch: EpochIndex?
    public let updatedAtEpoch: EpochIndex?
    public let groups: [GroupContractPosition: Group]
    public let tokens: [TokenContractPosition: DPPTokenConfiguration]
    public let keywords: [String]
    public let contractDescription: String?

    /// Get the contract ID as a base58 string
    public var idString: String {
        id.toBase58String()
    }

    /// Get the owner ID as a base58 string
    public var ownerIdString: String {
        ownerId.toBase58String()
    }

    /// Get created date
    public var createdDate: Date? {
        guard let createdAt = createdAt else { return nil }
        return Date(timeIntervalSince1970: Double(createdAt) / 1000)
    }

    /// Get updated date
    public var updatedDate: Date? {
        guard let updatedAt = updatedAt else { return nil }
        return Date(timeIntervalSince1970: Double(updatedAt) / 1000)
    }

    /// Alias for contractDescription (for backward compatibility)
    public var description: String? {
        contractDescription
    }

    public init(
        id: Identifier,
        version: UInt32,
        ownerId: Identifier,
        documentTypes: [DocumentName: DocumentType],
        config: DataContractConfig,
        schemaDefs: [DefinitionName: PlatformValue]?,
        createdAt: TimestampMillis?,
        updatedAt: TimestampMillis?,
        createdAtBlockHeight: BlockHeight?,
        updatedAtBlockHeight: BlockHeight?,
        createdAtEpoch: EpochIndex?,
        updatedAtEpoch: EpochIndex?,
        groups: [GroupContractPosition: Group],
        tokens: [TokenContractPosition: DPPTokenConfiguration],
        keywords: [String],
        contractDescription: String?
    ) {
        self.id = id
        self.version = version
        self.ownerId = ownerId
        self.documentTypes = documentTypes
        self.config = config
        self.schemaDefs = schemaDefs
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.createdAtBlockHeight = createdAtBlockHeight
        self.updatedAtBlockHeight = updatedAtBlockHeight
        self.createdAtEpoch = createdAtEpoch
        self.updatedAtEpoch = updatedAtEpoch
        self.groups = groups
        self.tokens = tokens
        self.keywords = keywords
        self.contractDescription = contractDescription
    }
}

// MARK: - Document Type

public struct DocumentType: Codable, Equatable, Sendable {
    public let name: String
    public let schema: JsonSchema
    public let indices: [Index]
    public let properties: [String: DocumentProperty]
    public let security: DocumentTypeSecurity
    public let transientFields: [String]
    public let requiresIdentityEncryptionBoundedKey: KeyBounds?
    public let requiresIdentityDecryptionBoundedKey: KeyBounds?
    public let tokenContractPosition: TokenContractPosition?
    public let signatureVerificationConfiguration: SignatureVerificationConfiguration?
    public let transferable: Transferable
    public let tradeMode: TradeMode

    /// Check if documents of this type can be transferred
    public var canBeTransferred: Bool {
        switch transferable {
        case .never: return false
        case .always: return true
        case .withCreatorPermission: return true
        }
    }

    public init(
        name: String,
        schema: JsonSchema,
        indices: [Index],
        properties: [String: DocumentProperty],
        security: DocumentTypeSecurity,
        transientFields: [String],
        requiresIdentityEncryptionBoundedKey: KeyBounds?,
        requiresIdentityDecryptionBoundedKey: KeyBounds?,
        tokenContractPosition: TokenContractPosition?,
        signatureVerificationConfiguration: SignatureVerificationConfiguration?,
        transferable: Transferable,
        tradeMode: TradeMode
    ) {
        self.name = name
        self.schema = schema
        self.indices = indices
        self.properties = properties
        self.security = security
        self.transientFields = transientFields
        self.requiresIdentityEncryptionBoundedKey = requiresIdentityEncryptionBoundedKey
        self.requiresIdentityDecryptionBoundedKey = requiresIdentityDecryptionBoundedKey
        self.tokenContractPosition = tokenContractPosition
        self.signatureVerificationConfiguration = signatureVerificationConfiguration
        self.transferable = transferable
        self.tradeMode = tradeMode
    }
}

// MARK: - Document Property

public struct DocumentProperty: Codable, Equatable, Sendable {
    public let type: PropertyType
    public let propertyDescription: String?
    public let format: String?
    public let pattern: String?
    public let minLength: Int?
    public let maxLength: Int?
    public let minimum: Double?
    public let maximum: Double?
    public let required: Bool
    public let transient: Bool
    public let position: UInt32?

    public init(
        type: PropertyType,
        propertyDescription: String? = nil,
        format: String? = nil,
        pattern: String? = nil,
        minLength: Int? = nil,
        maxLength: Int? = nil,
        minimum: Double? = nil,
        maximum: Double? = nil,
        required: Bool = false,
        transient: Bool = false,
        position: UInt32? = nil
    ) {
        self.type = type
        self.propertyDescription = propertyDescription
        self.format = format
        self.pattern = pattern
        self.minLength = minLength
        self.maxLength = maxLength
        self.minimum = minimum
        self.maximum = maximum
        self.required = required
        self.transient = transient
        self.position = position
    }
}

// MARK: - Property Type

public enum PropertyType: String, Codable, Sendable {
    case string
    case integer
    case number
    case boolean
    case array
    case object
    case bytes
}

// MARK: - Index

public struct Index: Codable, Equatable, Sendable {
    public let name: String
    public let properties: [IndexProperty]
    public let unique: Bool
    public let contestedUniqueIndexInformation: ContestedUniqueIndexInformation?

    public init(name: String, properties: [IndexProperty], unique: Bool, contestedUniqueIndexInformation: ContestedUniqueIndexInformation? = nil) {
        self.name = name
        self.properties = properties
        self.unique = unique
        self.contestedUniqueIndexInformation = contestedUniqueIndexInformation
    }
}

// MARK: - Index Property

public struct IndexProperty: Codable, Equatable, Sendable {
    public let name: String
    public let order: IndexOrder

    public init(name: String, order: IndexOrder) {
        self.name = name
        self.order = order
    }
}

public enum IndexOrder: String, Codable, Sendable {
    case ascending = "asc"
    case descending = "desc"
}

// MARK: - Contested Unique Index Information

public struct ContestedUniqueIndexInformation: Codable, Equatable, Sendable {
    public let contestResolution: ContestResolution
    public let documentAcceptsContest: Bool
    public let contestDescription: String?

    public init(contestResolution: ContestResolution, documentAcceptsContest: Bool, contestDescription: String? = nil) {
        self.contestResolution = contestResolution
        self.documentAcceptsContest = documentAcceptsContest
        self.contestDescription = contestDescription
    }
}

public enum ContestResolution: UInt8, Codable, Sendable {
    case firstComeFirstServe = 0
    case masternodesVote = 1
}

// MARK: - Document Type Security

public struct DocumentTypeSecurity: Codable, Equatable, Sendable {
    public let insertSignable: Bool
    public let updateSignable: Bool
    public let deleteSignable: Bool

    public init(insertSignable: Bool, updateSignable: Bool, deleteSignable: Bool) {
        self.insertSignable = insertSignable
        self.updateSignable = updateSignable
        self.deleteSignable = deleteSignable
    }
}

// MARK: - Key Bounds

public struct KeyBounds: Codable, Equatable, Sendable {
    public let minItems: UInt32
    public let maxItems: UInt32

    public init(minItems: UInt32, maxItems: UInt32) {
        self.minItems = minItems
        self.maxItems = maxItems
    }
}

// MARK: - Signature Verification Configuration

public struct SignatureVerificationConfiguration: Codable, Equatable, Sendable {
    public let enabled: Bool
    public let requiredSignatures: UInt32
    public let publicKeyIds: [KeyID]?

    public init(enabled: Bool, requiredSignatures: UInt32, publicKeyIds: [KeyID]?) {
        self.enabled = enabled
        self.requiredSignatures = requiredSignatures
        self.publicKeyIds = publicKeyIds
    }
}

// MARK: - Transferable

public enum Transferable: UInt8, Codable, Sendable {
    case never = 0
    case always = 1
    case withCreatorPermission = 2
}

// MARK: - Trade Mode

public enum TradeMode: UInt8, Codable, Sendable {
    case directPurchase = 0
    case sellerSetsPrice = 1
}

// MARK: - Data Contract Config

public struct DataContractConfig: Codable, Equatable, Sendable {
    public let canBeDeleted: Bool
    public let readOnly: Bool
    public let keepsHistory: Bool
    public let documentsKeepRevisionLogForPassedTimeMs: TimestampMillis?
    public let documentsMutableContractDefaultStored: Bool

    public init(
        canBeDeleted: Bool,
        readOnly: Bool,
        keepsHistory: Bool,
        documentsKeepRevisionLogForPassedTimeMs: TimestampMillis? = nil,
        documentsMutableContractDefaultStored: Bool = true
    ) {
        self.canBeDeleted = canBeDeleted
        self.readOnly = readOnly
        self.keepsHistory = keepsHistory
        self.documentsKeepRevisionLogForPassedTimeMs = documentsKeepRevisionLogForPassedTimeMs
        self.documentsMutableContractDefaultStored = documentsMutableContractDefaultStored
    }
}

// MARK: - Group

public struct Group: Codable, Equatable, Sendable {
    public let members: [[UInt8]]
    public let requiredPower: UInt32

    public var memberIdentifiers: [Identifier] {
        members.map { Data($0) }
    }

    public init(members: [[UInt8]], requiredPower: UInt32) {
        self.members = members
        self.requiredPower = requiredPower
    }
}

// MARK: - Token Configuration (DPP version)

public struct DPPTokenConfiguration: Codable, Equatable, Sendable {
    public let name: String
    public let symbol: String
    public let tokenDescription: String?
    public let decimals: UInt8
    public let totalSupplyInLowestDenomination: UInt64
    public let mintable: Bool
    public let burnable: Bool
    public let cappedSupply: Bool
    public let transferable: Bool
    public let tradeable: Bool
    public let sellable: Bool
    public let freezable: Bool
    public let pausable: Bool
    public let destructible: Bool
    public let rulesVersion: UInt16
    public let ruleGroups: TokenRuleGroups?

    /// Get total supply formatted with decimals
    public var formattedTotalSupply: String {
        let divisor = pow(10.0, Double(decimals))
        let amount = Double(totalSupplyInLowestDenomination) / divisor
        return String(format: "%.\(decimals)f %@", amount, symbol)
    }

    /// Alias for tokenDescription (for backward compatibility)
    public var description: String? {
        tokenDescription
    }

    public init(
        name: String,
        symbol: String,
        tokenDescription: String? = nil,
        decimals: UInt8,
        totalSupplyInLowestDenomination: UInt64,
        mintable: Bool,
        burnable: Bool,
        cappedSupply: Bool,
        transferable: Bool,
        tradeable: Bool,
        sellable: Bool,
        freezable: Bool,
        pausable: Bool,
        destructible: Bool = false,
        rulesVersion: UInt16 = 1,
        ruleGroups: TokenRuleGroups? = nil
    ) {
        self.name = name
        self.symbol = symbol
        self.tokenDescription = tokenDescription
        self.decimals = decimals
        self.totalSupplyInLowestDenomination = totalSupplyInLowestDenomination
        self.mintable = mintable
        self.burnable = burnable
        self.cappedSupply = cappedSupply
        self.transferable = transferable
        self.tradeable = tradeable
        self.sellable = sellable
        self.freezable = freezable
        self.pausable = pausable
        self.destructible = destructible
        self.rulesVersion = rulesVersion
        self.ruleGroups = ruleGroups
    }
}

// MARK: - Token Rule Groups

public struct TokenRuleGroups: Codable, Equatable, Sendable {
    public let ownerRules: TokenOwnerRules?
    public let everyoneRules: TokenEveryoneRules?

    public init(ownerRules: TokenOwnerRules? = nil, everyoneRules: TokenEveryoneRules? = nil) {
        self.ownerRules = ownerRules
        self.everyoneRules = everyoneRules
    }
}

public struct TokenOwnerRules: Codable, Equatable, Sendable {
    public let canMint: Bool
    public let canBurn: Bool
    public let canPause: Bool
    public let canFreeze: Bool
    public let canDestroy: Bool
    public let maxMintAmount: UInt64?

    public init(canMint: Bool, canBurn: Bool, canPause: Bool, canFreeze: Bool, canDestroy: Bool, maxMintAmount: UInt64? = nil) {
        self.canMint = canMint
        self.canBurn = canBurn
        self.canPause = canPause
        self.canFreeze = canFreeze
        self.canDestroy = canDestroy
        self.maxMintAmount = maxMintAmount
    }
}

public struct TokenEveryoneRules: Codable, Equatable, Sendable {
    public let canTransfer: Bool
    public let canBurn: Bool
    public let maxTransferAmount: UInt64?

    public init(canTransfer: Bool, canBurn: Bool, maxTransferAmount: UInt64? = nil) {
        self.canTransfer = canTransfer
        self.canBurn = canBurn
        self.maxTransferAmount = maxTransferAmount
    }
}

// MARK: - Json Schema

public struct JsonSchema: Codable, Equatable, Sendable {
    public let type: String
    public let properties: [String: JsonSchemaProperty]
    public let required: [String]
    public let additionalProperties: Bool

    public init(type: String, properties: [String: JsonSchemaProperty], required: [String], additionalProperties: Bool = false) {
        self.type = type
        self.properties = properties
        self.required = required
        self.additionalProperties = additionalProperties
    }
}

public indirect enum JsonSchemaPropertyValue: Codable, Equatable, Sendable {
    case property(JsonSchemaProperty)
}

public struct JsonSchemaProperty: Codable, Equatable, Sendable {
    public let type: String
    public let schemaDescription: String?
    public let format: String?
    public let pattern: String?
    public let minLength: Int?
    public let maxLength: Int?
    public let minimum: Double?
    public let maximum: Double?
    public let items: JsonSchemaPropertyValue?

    public init(
        type: String,
        schemaDescription: String? = nil,
        format: String? = nil,
        pattern: String? = nil,
        minLength: Int? = nil,
        maxLength: Int? = nil,
        minimum: Double? = nil,
        maximum: Double? = nil,
        items: JsonSchemaPropertyValue? = nil
    ) {
        self.type = type
        self.schemaDescription = schemaDescription
        self.format = format
        self.pattern = pattern
        self.minLength = minLength
        self.maxLength = maxLength
        self.minimum = minimum
        self.maximum = maximum
        self.items = items
    }
}

// MARK: - Factory Methods

extension DPPDataContract {
    /// Create a simple data contract
    public static func create(
        id: Identifier? = nil,
        ownerId: Identifier,
        documentTypes: [DocumentName: DocumentType] = [:],
        contractDescription: String? = nil
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
            contractDescription: contractDescription
        )
    }
}
