import Foundation
import SwiftDashSDK

enum IdentityType: String, CaseIterable {
    case user = "User"
    case masternode = "Masternode"
    case evonode = "Evonode"
}

struct IdentityModel: Identifiable, Equatable, Hashable {
    static func == (lhs: IdentityModel, rhs: IdentityModel) -> Bool {
        lhs.id == rhs.id
    }
    
    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
    let id: Data  // Changed from String to Data
    var balance: UInt64
    var isLocal: Bool
    let alias: String?
    let type: IdentityType
    let privateKeys: [String]
    let votingPrivateKey: String?
    let ownerPrivateKey: String?
    let payoutPrivateKey: String?
    var dpnsName: String?
    
    // DPP-related properties
    let dppIdentity: DPPIdentity?
    let publicKeys: [IdentityPublicKey]
    
    // Cache the base58 representation
    private let _base58String: String
    
    /// Get the identity ID as a base58 string (for FFI calls)
    var idString: String {
        _base58String
    }
    
    /// Get the identity ID as a hex string (for display when needed)
    var idHexString: String {
        id.toHexString()
    }
    
    init(id: Data, balance: UInt64 = 0, isLocal: Bool = true, alias: String? = nil, type: IdentityType = .user, privateKeys: [String] = [], votingPrivateKey: String? = nil, ownerPrivateKey: String? = nil, payoutPrivateKey: String? = nil, dpnsName: String? = nil, dppIdentity: DPPIdentity? = nil, publicKeys: [IdentityPublicKey] = []) {
        self.id = id
        self._base58String = id.toBase58String()
        self.balance = balance
        self.isLocal = isLocal
        self.alias = alias
        self.type = type
        self.privateKeys = privateKeys
        self.votingPrivateKey = votingPrivateKey
        self.ownerPrivateKey = ownerPrivateKey
        self.payoutPrivateKey = payoutPrivateKey
        self.dpnsName = dpnsName
        self.dppIdentity = dppIdentity
        self.publicKeys = publicKeys
    }
    
    /// Initialize with hex string ID for convenience
    init?(idString: String, balance: UInt64 = 0, isLocal: Bool = true, alias: String? = nil, type: IdentityType = .user, privateKeys: [String] = [], votingPrivateKey: String? = nil, ownerPrivateKey: String? = nil, payoutPrivateKey: String? = nil, dpnsName: String? = nil, dppIdentity: DPPIdentity? = nil, publicKeys: [IdentityPublicKey] = []) {
        guard let idData = Data(hexString: idString), idData.count == 32 else { return nil }
        self.init(id: idData, balance: balance, isLocal: isLocal, alias: alias, type: type, privateKeys: privateKeys, votingPrivateKey: votingPrivateKey, ownerPrivateKey: ownerPrivateKey, payoutPrivateKey: payoutPrivateKey, dpnsName: dpnsName, dppIdentity: dppIdentity, publicKeys: publicKeys)
    }
    
    init?(from identity: SwiftDashSDK.Identity) {
        guard let idData = Data(hexString: identity.id), idData.count == 32 else { return nil }
        self.id = idData
        self._base58String = idData.toBase58String()
        self.balance = identity.balance
        self.isLocal = false
        self.alias = nil
        self.type = .user
        self.privateKeys = []
        self.votingPrivateKey = nil
        self.ownerPrivateKey = nil
        self.payoutPrivateKey = nil
        self.dpnsName = nil
        self.dppIdentity = nil
        self.publicKeys = []
    }
    
    /// Create from DPP Identity
    init(from dppIdentity: DPPIdentity, alias: String? = nil, type: IdentityType = .user, privateKeys: [String] = [], dpnsName: String? = nil) {
        self.id = dppIdentity.id  // DPPIdentity already uses Data for id
        self._base58String = dppIdentity.id.toBase58String()
        self.balance = dppIdentity.balance
        self.isLocal = false
        self.alias = alias
        self.type = type
        self.privateKeys = privateKeys
        self.dpnsName = dpnsName
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