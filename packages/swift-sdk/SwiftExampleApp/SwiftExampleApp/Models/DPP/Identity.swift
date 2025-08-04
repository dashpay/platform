import Foundation
import SwiftDashSDK

// MARK: - Identity Models based on DPP

/// Main Identity structure
public struct DPPIdentity: Identifiable, Codable, Equatable {
    public let id: Identifier
    public let publicKeys: [KeyID: IdentityPublicKey]
    public let balance: Credits
    public let revision: Revision
    
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

// Note: Identity key types (KeyType, KeyPurpose, SecurityLevel, IdentityPublicKey, ContractBounds) 
// are now imported from SwiftDashSDK

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