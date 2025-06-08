import Foundation

// MARK: - State Transition Models based on DPP

/// Base protocol for all state transitions
protocol StateTransition: Codable {
    var type: StateTransitionType { get }
    var signature: BinaryData? { get }
    var signaturePublicKeyId: KeyID? { get }
}

// MARK: - State Transition Type

enum StateTransitionType: String, Codable {
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
    
    var name: String {
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

struct IdentityCreateTransition: StateTransition {
    let type = StateTransitionType.identityCreate
    let identityId: Identifier
    let publicKeys: [IdentityPublicKey]
    let balance: Credits
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct IdentityUpdateTransition: StateTransition {
    let type = StateTransitionType.identityUpdate
    let identityId: Identifier
    let revision: Revision
    let addPublicKeys: [IdentityPublicKey]?
    let disablePublicKeys: [KeyID]?
    let publicKeysDisabledAt: TimestampMillis?
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct IdentityTopUpTransition: StateTransition {
    let type = StateTransitionType.identityTopUp
    let identityId: Identifier
    let amount: Credits
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct IdentityCreditWithdrawalTransition: StateTransition {
    let type = StateTransitionType.identityCreditWithdrawal
    let identityId: Identifier
    let amount: Credits
    let coreFeePerByte: UInt32
    let pooling: Pooling
    let outputScript: BinaryData
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct IdentityCreditTransferTransition: StateTransition {
    let type = StateTransitionType.identityCreditTransfer
    let identityId: Identifier
    let recipientId: Identifier
    let amount: Credits
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

// MARK: - Data Contract State Transitions

struct DataContractCreateTransition: StateTransition {
    let type = StateTransitionType.dataContractCreate
    let dataContract: DPPDataContract
    let entropy: Bytes32
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct DataContractUpdateTransition: StateTransition {
    let type = StateTransitionType.dataContractUpdate
    let dataContract: DPPDataContract
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

// MARK: - Document State Transitions

struct DocumentsBatchTransition: StateTransition {
    let type = StateTransitionType.documentsBatch
    let ownerId: Identifier
    let contractId: Identifier
    let documentTransitions: [DocumentTransition]
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

enum DocumentTransition: Codable {
    case create(DocumentCreateTransition)
    case replace(DocumentReplaceTransition)
    case delete(DocumentDeleteTransition)
    case transfer(DocumentTransferTransition)
    case purchase(DocumentPurchaseTransition)
    case updatePrice(DocumentUpdatePriceTransition)
}

struct DocumentCreateTransition: Codable {
    let id: Identifier
    let dataContractId: Identifier
    let ownerId: Identifier
    let documentType: String
    let data: [String: PlatformValue]
    let entropy: Bytes32
}

struct DocumentReplaceTransition: Codable {
    let id: Identifier
    let dataContractId: Identifier
    let ownerId: Identifier
    let documentType: String
    let revision: Revision
    let data: [String: PlatformValue]
}

struct DocumentDeleteTransition: Codable {
    let id: Identifier
    let dataContractId: Identifier
    let ownerId: Identifier
    let documentType: String
}

struct DocumentTransferTransition: Codable {
    let id: Identifier
    let dataContractId: Identifier
    let ownerId: Identifier
    let recipientOwnerId: Identifier
    let documentType: String
    let revision: Revision
}

struct DocumentPurchaseTransition: Codable {
    let id: Identifier
    let dataContractId: Identifier
    let ownerId: Identifier
    let documentType: String
    let price: Credits
}

struct DocumentUpdatePriceTransition: Codable {
    let id: Identifier
    let dataContractId: Identifier
    let ownerId: Identifier
    let documentType: String
    let price: Credits
}

// MARK: - Token State Transitions

struct TokenTransferTransition: StateTransition {
    let type = StateTransitionType.tokenTransfer
    let tokenId: Identifier
    let senderId: Identifier
    let recipientId: Identifier
    let amount: UInt64
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct TokenMintTransition: StateTransition {
    let type = StateTransitionType.tokenMint
    let tokenId: Identifier
    let ownerId: Identifier
    let recipientId: Identifier?
    let amount: UInt64
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct TokenBurnTransition: StateTransition {
    let type = StateTransitionType.tokenBurn
    let tokenId: Identifier
    let ownerId: Identifier
    let amount: UInt64
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct TokenFreezeTransition: StateTransition {
    let type = StateTransitionType.tokenFreeze
    let tokenId: Identifier
    let ownerId: Identifier
    let frozenOwnerId: Identifier
    let amount: UInt64
    let reason: String?
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

struct TokenUnfreezeTransition: StateTransition {
    let type = StateTransitionType.tokenUnfreeze
    let tokenId: Identifier
    let ownerId: Identifier
    let unfrozenOwnerId: Identifier
    let amount: UInt64
    let signature: BinaryData?
    let signaturePublicKeyId: KeyID?
}

// MARK: - Supporting Types

enum Pooling: UInt8, Codable {
    case never = 0
    case ifAvailable = 1
    case always = 2
}

// MARK: - State Transition Result

struct StateTransitionResult: Codable {
    let fee: Credits
    let stateTransitionHash: Identifier
    let blockHeight: BlockHeight
    let blockTime: TimestampMillis
    let error: StateTransitionError?
}

struct StateTransitionError: Codable, Error {
    let code: UInt32
    let message: String
    let data: [String: PlatformValue]?
}

// MARK: - Broadcast State Transition

struct BroadcastStateTransitionRequest {
    let stateTransition: StateTransition
    let skipValidation: Bool
    let dryRun: Bool
}

// MARK: - Wait for State Transition Result

struct WaitForStateTransitionResultRequest {
    let stateTransitionHash: Identifier
    let prove: Bool
    let timeout: TimeInterval
}