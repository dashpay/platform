import Foundation

// MARK: - Key Type

public enum KeyType: UInt8, CaseIterable, Codable {
    case ecdsaSecp256k1 = 0
    case bls12_381 = 1
    case ecdsaHash160 = 2
    case bip13ScriptHash = 3
    case eddsa25519Hash160 = 4
    
    public var name: String {
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

public enum KeyPurpose: UInt8, CaseIterable, Codable {
    case authentication = 0
    case encryption = 1
    case decryption = 2
    case transfer = 3
    case system = 4
    case voting = 5
    case owner = 6
    
    public var name: String {
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
    
    public var description: String {
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

public enum SecurityLevel: UInt8, CaseIterable, Codable, Comparable {
    case master = 0
    case critical = 1
    case high = 2
    case medium = 3
    
    public var name: String {
        switch self {
        case .master: return "Master"
        case .critical: return "Critical"
        case .high: return "High"
        case .medium: return "Medium"
        }
    }
    
    public var description: String {
        switch self {
        case .master: return "Highest security level - can perform any action"
        case .critical: return "Critical operations only"
        case .high: return "High security operations"
        case .medium: return "Standard operations"
        }
    }
    
    public static func < (lhs: SecurityLevel, rhs: SecurityLevel) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

// MARK: - Identity Public Key

public struct IdentityPublicKey: Codable, Equatable {
    public let id: KeyID
    public let purpose: KeyPurpose
    public let securityLevel: SecurityLevel
    public let contractBounds: ContractBounds?
    public let keyType: KeyType
    public let readOnly: Bool
    public let data: BinaryData
    public let disabledAt: TimestampMillis?
    
    /// Check if the key is currently disabled
    public var isDisabled: Bool {
        guard let disabledAt = disabledAt else { return false }
        let currentTime = TimestampMillis(Date().timeIntervalSince1970 * 1000)
        return disabledAt <= currentTime
    }
    
    public init(
        id: KeyID,
        purpose: KeyPurpose,
        securityLevel: SecurityLevel,
        contractBounds: ContractBounds? = nil,
        keyType: KeyType,
        readOnly: Bool,
        data: BinaryData,
        disabledAt: TimestampMillis? = nil
    ) {
        self.id = id
        self.purpose = purpose
        self.securityLevel = securityLevel
        self.contractBounds = contractBounds
        self.keyType = keyType
        self.readOnly = readOnly
        self.data = data
        self.disabledAt = disabledAt
    }
}

// MARK: - Contract Bounds

public enum ContractBounds: Codable, Equatable {
    case singleContract(id: Identifier)
    case singleContractDocumentType(id: Identifier, documentTypeName: String)
    
    public var description: String {
        switch self {
        case .singleContract(let id):
            return "Limited to contract: \(id.toBase58())"
        case .singleContractDocumentType(let id, let docType):
            return "Limited to \(docType) in contract: \(id.toBase58())"
        }
    }
    
    public var contractId: Identifier {
        switch self {
        case .singleContract(let id):
            return id
        case .singleContractDocumentType(let id, _):
            return id
        }
    }
}

// MARK: - Type Aliases
// These are used for compatibility with the FFI layer
public typealias KeyID = UInt32
public typealias BinaryData = Data
public typealias TimestampMillis = UInt64
public typealias Identifier = Data