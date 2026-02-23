import Foundation

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
