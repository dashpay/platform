import Foundation

// MARK: - Identity Models based on DPP

/// Main Identity structure
struct DPPIdentity: Identifiable, Codable, Equatable {
    let id: Identifier
    let publicKeys: [KeyID: IdentityPublicKey]
    let balance: Credits
    let revision: Revision
    
    /// Get the identity ID as a string
    var idString: String {
        id.toBase58String()
    }
    
    /// Get the identity ID as hex
    var idHex: String {
        id.toHexString()
    }
    
    /// Get formatted balance in DASH
    var formattedBalance: String {
        let dashAmount = Double(balance) / 100_000_000
        return String(format: "%.8f DASH", dashAmount)
    }
}

// MARK: - Identity Public Key

struct IdentityPublicKey: Codable, Equatable {
    let id: KeyID
    let purpose: KeyPurpose
    let securityLevel: SecurityLevel
    let contractBounds: ContractBounds?
    let keyType: KeyType
    let readOnly: Bool
    let data: BinaryData
    let disabledAt: TimestampMillis?
    
    /// Check if the key is currently disabled
    var isDisabled: Bool {
        guard let disabledAt = disabledAt else { return false }
        let currentTime = TimestampMillis(Date().timeIntervalSince1970 * 1000)
        return disabledAt <= currentTime
    }
}

// MARK: - Key Type

enum KeyType: UInt8, CaseIterable, Codable {
    case ecdsaSecp256k1 = 0
    case bls12_381 = 1
    case ecdsaHash160 = 2
    case bip13ScriptHash = 3
    case eddsa25519Hash160 = 4
    
    var name: String {
        switch self {
        case .ecdsaSecp256k1: return "ECDSA secp256k1"
        case .bls12_381: return "BLS12-381"
        case .ecdsaHash160: return "ECDSA Hash160"
        case .bip13ScriptHash: return "BIP13 Script Hash"
        case .eddsa25519Hash160: return "EdDSA 25519 Hash160"
        }
    }
}

// MARK: - Key Purpose

enum KeyPurpose: UInt8, CaseIterable, Codable {
    case authentication = 0
    case encryption = 1
    case decryption = 2
    case transfer = 3
    case system = 4
    case voting = 5
    case owner = 6
    
    var name: String {
        switch self {
        case .authentication: return "Authentication"
        case .encryption: return "Encryption"
        case .decryption: return "Decryption"
        case .transfer: return "Transfer"
        case .system: return "System"
        case .voting: return "Voting"
        case .owner: return "Owner"
        }
    }
    
    var description: String {
        switch self {
        case .authentication: return "Used for platform authentication"
        case .encryption: return "Used to encrypt data"
        case .decryption: return "Used to decrypt data"
        case .transfer: return "Used to transfer credits"
        case .system: return "System level operations"
        case .voting: return "Used for voting (masternodes)"
        case .owner: return "Owner key (masternodes)"
        }
    }
}

// MARK: - Security Level

enum SecurityLevel: UInt8, CaseIterable, Codable, Comparable {
    case master = 0
    case critical = 1
    case high = 2
    case medium = 3
    
    var name: String {
        switch self {
        case .master: return "Master"
        case .critical: return "Critical"
        case .high: return "High"
        case .medium: return "Medium"
        }
    }
    
    var description: String {
        switch self {
        case .master: return "Highest security level - can perform any action"
        case .critical: return "Critical operations only"
        case .high: return "High security operations"
        case .medium: return "Standard operations"
        }
    }
    
    static func < (lhs: SecurityLevel, rhs: SecurityLevel) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

// MARK: - Contract Bounds

enum ContractBounds: Codable, Equatable {
    case singleContract(id: Identifier)
    case singleContractDocumentType(id: Identifier, documentTypeName: String)
    
    var description: String {
        switch self {
        case .singleContract(let id):
            return "Limited to contract: \(id.toBase58String())"
        case .singleContractDocumentType(let id, let docType):
            return "Limited to \(docType) in contract: \(id.toBase58String())"
        }
    }
    
    var contractId: Identifier {
        switch self {
        case .singleContract(let id):
            return id
        case .singleContractDocumentType(let id, _):
            return id
        }
    }
}

// MARK: - Partial Identity

/// Represents a partially loaded identity
struct PartialIdentity: Identifiable {
    let id: Identifier
    let loadedPublicKeys: [KeyID: IdentityPublicKey]
    let balance: Credits?
    let revision: Revision?
    let notFoundPublicKeys: Set<KeyID>
    
    /// Get the identity ID as a string
    var idString: String {
        id.toBase58String()
    }
}

// MARK: - Identity Factory

extension DPPIdentity {
    /// Create a new identity with initial keys
    static func create(
        id: Identifier,
        publicKeys: [IdentityPublicKey] = [],
        balance: Credits = 0
    ) -> DPPIdentity {
        let keysDict = Dictionary(uniqueKeysWithValues: publicKeys.map { ($0.id, $0) })
        return DPPIdentity(
            id: id,
            publicKeys: keysDict,
            balance: balance,
            revision: 0
        )
    }
    
    /// Create an identity from our simplified IdentityModel
    init?(from model: IdentityModel) {
        // model.id is already Data, no conversion needed
        let idData = model.id
        
        self.id = idData
        self.publicKeys = [:]
        self.balance = model.balance
        self.revision = 0
        
        // Note: In a real implementation, we would convert private keys to public keys
    }
}