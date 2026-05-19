import Foundation

// MARK: - State Transition Models based on DPP

/// Base protocol for all state transitions
public protocol StateTransition: Codable, Sendable {
    var type: StateTransitionType { get }
    var signature: BinaryData? { get }
    var signaturePublicKeyId: KeyID? { get }
}

// MARK: - State Transition Type

public enum StateTransitionType: String, Codable, Sendable {
    // Identity transitions
    case identityCreate
    case identityUpdate
    case identityTopUp
    case identityCreditWithdrawal
    case identityCreditTransfer

    // Data Contract transitions
    case dataContractCreate
    case dataContractUpdate

    // Document transitions
    case documentsBatch

    // Token transitions
    case tokenTransfer
    case tokenMint
    case tokenBurn
    case tokenFreeze
    case tokenUnfreeze

    public var name: String {
        switch self {
        case .identityCreate: return "Identity Create"
        case .identityUpdate: return "Identity Update"
        case .identityTopUp: return "Identity Top Up"
        case .identityCreditWithdrawal: return "Identity Credit Withdrawal"
        case .identityCreditTransfer: return "Identity Credit Transfer"
        case .dataContractCreate: return "Data Contract Create"
        case .dataContractUpdate: return "Data Contract Update"
        case .documentsBatch: return "Documents Batch"
        case .tokenTransfer: return "Token Transfer"
        case .tokenMint: return "Token Mint"
        case .tokenBurn: return "Token Burn"
        case .tokenFreeze: return "Token Freeze"
        case .tokenUnfreeze: return "Token Unfreeze"
        }
    }
}

// MARK: - Identity State Transitions

public struct IdentityCreateTransition: StateTransition {
    public var type: StateTransitionType { .identityCreate }
    public let identityId: Identifier
    public let publicKeys: [IdentityPublicKey]
    public let balance: Credits
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(identityId: Identifier, publicKeys: [IdentityPublicKey], balance: Credits, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.identityId = identityId
        self.publicKeys = publicKeys
        self.balance = balance
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct IdentityUpdateTransition: StateTransition {
    public var type: StateTransitionType { .identityUpdate }
    public let identityId: Identifier
    public let revision: Revision
    public let addPublicKeys: [IdentityPublicKey]?
    public let disablePublicKeys: [KeyID]?
    public let publicKeysDisabledAt: TimestampMillis?
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(identityId: Identifier, revision: Revision, addPublicKeys: [IdentityPublicKey]? = nil, disablePublicKeys: [KeyID]? = nil, publicKeysDisabledAt: TimestampMillis? = nil, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.identityId = identityId
        self.revision = revision
        self.addPublicKeys = addPublicKeys
        self.disablePublicKeys = disablePublicKeys
        self.publicKeysDisabledAt = publicKeysDisabledAt
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct IdentityTopUpTransition: StateTransition {
    public var type: StateTransitionType { .identityTopUp }
    public let identityId: Identifier
    public let amount: Credits
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(identityId: Identifier, amount: Credits, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.identityId = identityId
        self.amount = amount
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct IdentityCreditWithdrawalTransition: StateTransition {
    public var type: StateTransitionType { .identityCreditWithdrawal }
    public let identityId: Identifier
    public let amount: Credits
    public let coreFeePerByte: UInt32
    public let pooling: Pooling
    public let outputScript: BinaryData
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(identityId: Identifier, amount: Credits, coreFeePerByte: UInt32, pooling: Pooling, outputScript: BinaryData, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.identityId = identityId
        self.amount = amount
        self.coreFeePerByte = coreFeePerByte
        self.pooling = pooling
        self.outputScript = outputScript
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct IdentityCreditTransferTransition: StateTransition {
    public var type: StateTransitionType { .identityCreditTransfer }
    public let identityId: Identifier
    public let recipientId: Identifier
    public let amount: Credits
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(identityId: Identifier, recipientId: Identifier, amount: Credits, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.identityId = identityId
        self.recipientId = recipientId
        self.amount = amount
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

// MARK: - Data Contract State Transitions

public struct DataContractCreateTransition: StateTransition {
    public var type: StateTransitionType { .dataContractCreate }
    public let dataContract: DPPDataContract
    public let entropy: Bytes32
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(dataContract: DPPDataContract, entropy: Bytes32, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.dataContract = dataContract
        self.entropy = entropy
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct DataContractUpdateTransition: StateTransition {
    public var type: StateTransitionType { .dataContractUpdate }
    public let dataContract: DPPDataContract
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(dataContract: DPPDataContract, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.dataContract = dataContract
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

// MARK: - Document State Transitions

public struct DocumentsBatchTransition: StateTransition {
    public var type: StateTransitionType { .documentsBatch }
    public let ownerId: Identifier
    public let contractId: Identifier
    public let documentTransitions: [DocumentTransition]
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(ownerId: Identifier, contractId: Identifier, documentTransitions: [DocumentTransition], signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.ownerId = ownerId
        self.contractId = contractId
        self.documentTransitions = documentTransitions
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public enum DocumentTransition: Codable, Sendable {
    case create(DocumentCreateTransition)
    case replace(DocumentReplaceTransition)
    case delete(DocumentDeleteTransition)
    case transfer(DocumentTransferTransition)
    case purchase(DocumentPurchaseTransition)
    case updatePrice(DocumentUpdatePriceTransition)
}

public struct DocumentCreateTransition: Codable, Sendable {
    public let id: Identifier
    public let dataContractId: Identifier
    public let ownerId: Identifier
    public let documentType: String
    public let data: [String: PlatformValue]
    public let entropy: Bytes32

    public init(id: Identifier, dataContractId: Identifier, ownerId: Identifier, documentType: String, data: [String: PlatformValue], entropy: Bytes32) {
        self.id = id
        self.dataContractId = dataContractId
        self.ownerId = ownerId
        self.documentType = documentType
        self.data = data
        self.entropy = entropy
    }
}

public struct DocumentReplaceTransition: Codable, Sendable {
    public let id: Identifier
    public let dataContractId: Identifier
    public let ownerId: Identifier
    public let documentType: String
    public let revision: Revision
    public let data: [String: PlatformValue]

    public init(id: Identifier, dataContractId: Identifier, ownerId: Identifier, documentType: String, revision: Revision, data: [String: PlatformValue]) {
        self.id = id
        self.dataContractId = dataContractId
        self.ownerId = ownerId
        self.documentType = documentType
        self.revision = revision
        self.data = data
    }
}

public struct DocumentDeleteTransition: Codable, Sendable {
    public let id: Identifier
    public let dataContractId: Identifier
    public let ownerId: Identifier
    public let documentType: String

    public init(id: Identifier, dataContractId: Identifier, ownerId: Identifier, documentType: String) {
        self.id = id
        self.dataContractId = dataContractId
        self.ownerId = ownerId
        self.documentType = documentType
    }
}

public struct DocumentTransferTransition: Codable, Sendable {
    public let id: Identifier
    public let dataContractId: Identifier
    public let ownerId: Identifier
    public let recipientOwnerId: Identifier
    public let documentType: String
    public let revision: Revision

    public init(id: Identifier, dataContractId: Identifier, ownerId: Identifier, recipientOwnerId: Identifier, documentType: String, revision: Revision) {
        self.id = id
        self.dataContractId = dataContractId
        self.ownerId = ownerId
        self.recipientOwnerId = recipientOwnerId
        self.documentType = documentType
        self.revision = revision
    }
}

public struct DocumentPurchaseTransition: Codable, Sendable {
    public let id: Identifier
    public let dataContractId: Identifier
    public let ownerId: Identifier
    public let documentType: String
    public let price: Credits

    public init(id: Identifier, dataContractId: Identifier, ownerId: Identifier, documentType: String, price: Credits) {
        self.id = id
        self.dataContractId = dataContractId
        self.ownerId = ownerId
        self.documentType = documentType
        self.price = price
    }
}

public struct DocumentUpdatePriceTransition: Codable, Sendable {
    public let id: Identifier
    public let dataContractId: Identifier
    public let ownerId: Identifier
    public let documentType: String
    public let price: Credits

    public init(id: Identifier, dataContractId: Identifier, ownerId: Identifier, documentType: String, price: Credits) {
        self.id = id
        self.dataContractId = dataContractId
        self.ownerId = ownerId
        self.documentType = documentType
        self.price = price
    }
}

// MARK: - Token State Transitions

public struct TokenTransferTransition: StateTransition {
    public var type: StateTransitionType { .tokenTransfer }
    public let tokenId: Identifier
    public let senderId: Identifier
    public let recipientId: Identifier
    public let amount: UInt64
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(tokenId: Identifier, senderId: Identifier, recipientId: Identifier, amount: UInt64, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.tokenId = tokenId
        self.senderId = senderId
        self.recipientId = recipientId
        self.amount = amount
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct TokenMintTransition: StateTransition {
    public var type: StateTransitionType { .tokenMint }
    public let tokenId: Identifier
    public let ownerId: Identifier
    public let recipientId: Identifier?
    public let amount: UInt64
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(tokenId: Identifier, ownerId: Identifier, recipientId: Identifier? = nil, amount: UInt64, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.tokenId = tokenId
        self.ownerId = ownerId
        self.recipientId = recipientId
        self.amount = amount
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct TokenBurnTransition: StateTransition {
    public var type: StateTransitionType { .tokenBurn }
    public let tokenId: Identifier
    public let ownerId: Identifier
    public let amount: UInt64
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(tokenId: Identifier, ownerId: Identifier, amount: UInt64, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.tokenId = tokenId
        self.ownerId = ownerId
        self.amount = amount
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct TokenFreezeTransition: StateTransition {
    public var type: StateTransitionType { .tokenFreeze }
    public let tokenId: Identifier
    public let ownerId: Identifier
    public let frozenOwnerId: Identifier
    public let amount: UInt64
    public let reason: String?
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(tokenId: Identifier, ownerId: Identifier, frozenOwnerId: Identifier, amount: UInt64, reason: String? = nil, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.tokenId = tokenId
        self.ownerId = ownerId
        self.frozenOwnerId = frozenOwnerId
        self.amount = amount
        self.reason = reason
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

public struct TokenUnfreezeTransition: StateTransition {
    public var type: StateTransitionType { .tokenUnfreeze }
    public let tokenId: Identifier
    public let ownerId: Identifier
    public let unfrozenOwnerId: Identifier
    public let amount: UInt64
    public let signature: BinaryData?
    public let signaturePublicKeyId: KeyID?

    public init(tokenId: Identifier, ownerId: Identifier, unfrozenOwnerId: Identifier, amount: UInt64, signature: BinaryData? = nil, signaturePublicKeyId: KeyID? = nil) {
        self.tokenId = tokenId
        self.ownerId = ownerId
        self.unfrozenOwnerId = unfrozenOwnerId
        self.amount = amount
        self.signature = signature
        self.signaturePublicKeyId = signaturePublicKeyId
    }
}

// MARK: - Supporting Types

public enum Pooling: UInt8, Codable, Sendable {
    case never = 0
    case ifAvailable = 1
    case always = 2
}

// MARK: - State Transition Result

public struct StateTransitionResult: Codable, Sendable {
    public let fee: Credits
    public let stateTransitionHash: Identifier
    public let blockHeight: BlockHeight
    public let blockTime: TimestampMillis
    public let error: StateTransitionError?

    public init(fee: Credits, stateTransitionHash: Identifier, blockHeight: BlockHeight, blockTime: TimestampMillis, error: StateTransitionError? = nil) {
        self.fee = fee
        self.stateTransitionHash = stateTransitionHash
        self.blockHeight = blockHeight
        self.blockTime = blockTime
        self.error = error
    }
}

public struct StateTransitionError: Codable, Error, Sendable {
    public let code: UInt32
    public let message: String
    public let data: [String: PlatformValue]?

    public init(code: UInt32, message: String, data: [String: PlatformValue]? = nil) {
        self.code = code
        self.message = message
        self.data = data
    }
}

// MARK: - Broadcast State Transition

public struct BroadcastStateTransitionRequest: Sendable {
    public let stateTransition: any StateTransition
    public let skipValidation: Bool
    public let dryRun: Bool

    public init(stateTransition: any StateTransition, skipValidation: Bool = false, dryRun: Bool = false) {
        self.stateTransition = stateTransition
        self.skipValidation = skipValidation
        self.dryRun = dryRun
    }
}

// MARK: - Wait for State Transition Result

public struct WaitForStateTransitionResultRequest: Sendable {
    public let stateTransitionHash: Identifier
    public let prove: Bool
    public let timeout: TimeInterval

    public init(stateTransitionHash: Identifier, prove: Bool = false, timeout: TimeInterval = 30) {
        self.stateTransitionHash = stateTransitionHash
        self.prove = prove
        self.timeout = timeout
    }
}
