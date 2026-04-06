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

