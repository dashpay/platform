import Foundation

// MARK: - Key Type

public enum KeyType: UInt8, CaseIterable, Codable, Sendable {
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

    /// FFI representation value
    public var ffiValue: UInt8 {
        return self.rawValue
    }
}

// MARK: - Key Purpose

public enum KeyPurpose: UInt8, CaseIterable, Codable, Sendable {
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

    /// FFI representation value
    public var ffiValue: UInt8 {
        return self.rawValue
    }
}

// MARK: - Security Level

public enum SecurityLevel: UInt8, CaseIterable, Codable, Comparable, Sendable {
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

    /// FFI representation value
    public var ffiValue: UInt8 {
        return self.rawValue
    }
}

// MARK: - Identity Public Key

public struct IdentityPublicKey: Codable, Equatable, Sendable {
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

public enum ContractBounds: Codable, Equatable, Sendable {
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

// MARK: - Identity Models based on DPP

/// Main Identity structure representing a Dash Platform identity
public struct DPPIdentity: Identifiable, Codable, Equatable, Sendable {
    public let id: Identifier
    public let publicKeys: [KeyID: IdentityPublicKey]
    public let balance: Credits
    public let revision: Revision

    /// Get the identity ID as a base58 string
    public var idString: String {
        id.toBase58String()
    }

    /// Get the identity ID as hex
    public var idHex: String {
        id.toHexString()
    }

    /// Get formatted balance in DASH
    public var formattedBalance: String {
        let dashAmount = Double(balance) / 100_000_000_000 // 1 DASH = 100B credits
        return String(format: "%.8f DASH", dashAmount)
    }

    public init(id: Identifier, publicKeys: [KeyID: IdentityPublicKey], balance: Credits, revision: Revision) {
        self.id = id
        self.publicKeys = publicKeys
        self.balance = balance
        self.revision = revision
    }
}

// MARK: - Partial Identity

/// Represents a partially loaded identity (some data may not be fetched)
public struct PartialIdentity: Identifiable, Sendable {
    public let id: Identifier
    public let loadedPublicKeys: [KeyID: IdentityPublicKey]
    public let balance: Credits?
    public let revision: Revision?
    public let notFoundPublicKeys: Set<KeyID>

    /// Get the identity ID as a base58 string
    public var idString: String {
        id.toBase58String()
    }

    public init(id: Identifier, loadedPublicKeys: [KeyID: IdentityPublicKey], balance: Credits?, revision: Revision?, notFoundPublicKeys: Set<KeyID>) {
        self.id = id
        self.loadedPublicKeys = loadedPublicKeys
        self.balance = balance
        self.revision = revision
        self.notFoundPublicKeys = notFoundPublicKeys
    }
}

// MARK: - Identity Factory

extension DPPIdentity {
    /// Create a new identity with initial keys
    public static func create(
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

    /// Create an identity from raw data
    public static func create(
        idData: Data,
        publicKeys: [IdentityPublicKey] = [],
        balance: Credits = 0
    ) -> DPPIdentity {
        let keysDict = Dictionary(uniqueKeysWithValues: publicKeys.map { ($0.id, $0) })
        return DPPIdentity(
            id: idData,
            publicKeys: keysDict,
            balance: balance,
            revision: 0
        )
    }
}
