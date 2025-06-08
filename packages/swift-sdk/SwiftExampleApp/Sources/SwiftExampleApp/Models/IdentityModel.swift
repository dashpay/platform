import Foundation
import SwiftDashSDK

enum IdentityType: String, CaseIterable {
    case user = "User"
    case masternode = "Masternode"
    case evonode = "Evonode"
}

struct IdentityModel: Identifiable, Equatable {
    let id: String
    let balance: UInt64
    let isLocal: Bool
    let alias: String?
    let type: IdentityType
    let privateKeys: [String]
    let votingPrivateKey: String?
    let ownerPrivateKey: String?
    let payoutPrivateKey: String?
    
    // DPP-related properties
    let dppIdentity: DPPIdentity?
    let publicKeys: [IdentityPublicKey]
    
    init(id: String, balance: UInt64 = 0, isLocal: Bool = true, alias: String? = nil, type: IdentityType = .user, privateKeys: [String] = [], votingPrivateKey: String? = nil, ownerPrivateKey: String? = nil, payoutPrivateKey: String? = nil, dppIdentity: DPPIdentity? = nil, publicKeys: [IdentityPublicKey] = []) {
        self.id = id
        self.balance = balance
        self.isLocal = isLocal
        self.alias = alias
        self.type = type
        self.privateKeys = privateKeys
        self.votingPrivateKey = votingPrivateKey
        self.ownerPrivateKey = ownerPrivateKey
        self.payoutPrivateKey = payoutPrivateKey
        self.dppIdentity = dppIdentity
        self.publicKeys = publicKeys
    }
    
    init?(from identity: Identity) {
        guard let idString = identity.idString() else { return nil }
        self.id = idString
        self.balance = identity.balance() ?? 0
        self.isLocal = false
        self.alias = nil
        self.type = .user
        self.privateKeys = []
        self.votingPrivateKey = nil
        self.ownerPrivateKey = nil
        self.payoutPrivateKey = nil
        self.dppIdentity = nil
        self.publicKeys = []
    }
    
    /// Create from DPP Identity
    init(from dppIdentity: DPPIdentity, alias: String? = nil, type: IdentityType = .user, privateKeys: [String] = []) {
        self.id = dppIdentity.idString
        self.balance = dppIdentity.balance
        self.isLocal = false
        self.alias = alias
        self.type = type
        self.privateKeys = privateKeys
        self.dppIdentity = dppIdentity
        self.publicKeys = Array(dppIdentity.publicKeys.values)
        
        // Extract specific keys for masternodes
        if type == .masternode || type == .evonode {
            self.votingPrivateKey = nil // Would be set separately
            self.ownerPrivateKey = nil  // Would be set separately
            self.payoutPrivateKey = nil // Would be set separately
        } else {
            self.votingPrivateKey = nil
            self.ownerPrivateKey = nil
            self.payoutPrivateKey = nil
        }
    }
    
    var formattedBalance: String {
        let dashAmount = Double(balance) / 100_000_000
        return String(format: "%.8f DASH", dashAmount)
    }
}