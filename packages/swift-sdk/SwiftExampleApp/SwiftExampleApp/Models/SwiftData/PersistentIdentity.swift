import Foundation
import SwiftData
import SwiftDashSDK

/// SwiftData model for persisting Identity data
@Model
final class PersistentIdentity {
    // MARK: - Core Properties
    @Attribute(.unique) var identityId: Data
    var balance: Int64
    var revision: Int64
    var isLocal: Bool
    var alias: String?
    var dpnsName: String?
    var mainDpnsName: String?
    var identityType: String
    
    // MARK: - Special Key Storage (stored in keychain)
    var votingPrivateKeyIdentifier: String?
    var ownerPrivateKeyIdentifier: String?
    var payoutPrivateKeyIdentifier: String?
    
    // MARK: - Public Keys
    @Relationship(deleteRule: .cascade) var publicKeys: [PersistentPublicKey]
    
    // MARK: - Timestamps
    var createdAt: Date
    var lastUpdated: Date
    var lastSyncedAt: Date?
    
    // MARK: - Network
    var network: String
    
    // MARK: - Relationships
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.ownerIdentity) var documents: [PersistentDocument]
    @Relationship(deleteRule: .nullify) var tokenBalances: [PersistentTokenBalance]
    
    // MARK: - Initialization
    init(
        identityId: Data,
        balance: Int64 = 0,
        revision: Int64 = 0,
        isLocal: Bool = true,
        alias: String? = nil,
        dpnsName: String? = nil,
        mainDpnsName: String? = nil,
        identityType: IdentityType = .user,
        votingPrivateKeyIdentifier: String? = nil,
        ownerPrivateKeyIdentifier: String? = nil,
        payoutPrivateKeyIdentifier: String? = nil,
        network: String = "testnet"
    ) {
        self.identityId = identityId
        self.balance = balance
        self.revision = revision
        self.isLocal = isLocal
        self.alias = alias
        self.dpnsName = dpnsName
        self.mainDpnsName = mainDpnsName
        self.identityType = identityType.rawValue
        self.votingPrivateKeyIdentifier = votingPrivateKeyIdentifier
        self.ownerPrivateKeyIdentifier = ownerPrivateKeyIdentifier
        self.payoutPrivateKeyIdentifier = payoutPrivateKeyIdentifier
        self.network = network
        self.publicKeys = []
        self.documents = []
        self.tokenBalances = []
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.lastSyncedAt = nil
    }
    
    // MARK: - Computed Properties
    var identityIdString: String {
        identityId.toHexString()
    }
    
    var formattedBalance: String {
        let dashAmount = Double(balance) / 100_000_000_000 // 1 DASH = 100B credits
        return String(format: "%.8f DASH", dashAmount)
    }
    
    var identityTypeEnum: IdentityType {
        IdentityType(rawValue: identityType) ?? .user
    }
    
    // MARK: - Methods
    func updateBalance(_ newBalance: Int64) {
        self.balance = newBalance
        self.lastUpdated = Date()
    }
    
    func updateRevision(_ newRevision: Int64) {
        self.revision = newRevision
        self.lastUpdated = Date()
    }
    
    func markAsSynced() {
        self.lastSyncedAt = Date()
    }
    
    func updateDPNSName(_ name: String?) {
        self.dpnsName = name
        self.lastUpdated = Date()
    }
    
    func addPublicKey(_ key: PersistentPublicKey) {
        publicKeys.append(key)
        lastUpdated = Date()
    }
    
    func removePublicKey(withId keyId: Int32) {
        publicKeys.removeAll { $0.keyId == keyId }
        lastUpdated = Date()
    }
}

// MARK: - Conversion Extensions

extension PersistentIdentity {
    /// Convert to app's IdentityModel
    func toIdentityModel() -> IdentityModel {
        let publicKeyModels = publicKeys.compactMap { $0.toIdentityPublicKey() }
        
        // Convert public keys with private keys to Data array by retrieving from keychain
        let privateKeyData = publicKeys
            .filter { $0.hasPrivateKey }
            .sorted(by: { $0.keyId < $1.keyId })
            .compactMap { $0.getPrivateKeyData() }
        
        // Retrieve special keys from keychain
        let votingKey = votingPrivateKeyIdentifier != nil ? 
            KeychainManager.shared.retrieveSpecialKey(identityId: identityId, keyType: .voting) : nil
        let ownerKey = ownerPrivateKeyIdentifier != nil ?
            KeychainManager.shared.retrieveSpecialKey(identityId: identityId, keyType: .owner) : nil
        let payoutKey = payoutPrivateKeyIdentifier != nil ?
            KeychainManager.shared.retrieveSpecialKey(identityId: identityId, keyType: .payout) : nil
        
        return IdentityModel(
            id: identityId,
            balance: UInt64(balance),
            isLocal: isLocal,
            alias: alias,
            type: identityTypeEnum,
            privateKeys: privateKeyData,
            votingPrivateKey: votingKey,
            ownerPrivateKey: ownerKey,
            payoutPrivateKey: payoutKey,
            dpnsName: dpnsName,
            mainDpnsName: mainDpnsName,
            publicKeys: publicKeyModels
        )
    }
    
    /// Create from IdentityModel
    static func from(_ model: IdentityModel, network: String = "testnet") -> PersistentIdentity {
        // Store special keys in keychain first
        var votingKeyId: String? = nil
        var ownerKeyId: String? = nil
        var payoutKeyId: String? = nil
        
        if let votingKey = model.votingPrivateKey {
            votingKeyId = KeychainManager.shared.storeSpecialKey(votingKey, identityId: model.id, keyType: .voting)
        }
        if let ownerKey = model.ownerPrivateKey {
            ownerKeyId = KeychainManager.shared.storeSpecialKey(ownerKey, identityId: model.id, keyType: .owner)
        }
        if let payoutKey = model.payoutPrivateKey {
            payoutKeyId = KeychainManager.shared.storeSpecialKey(payoutKey, identityId: model.id, keyType: .payout)
        }
        
        let persistent = PersistentIdentity(
            identityId: model.id,
            balance: Int64(model.balance),
            revision: 0, // Default revision, will be updated when fetched from network
            isLocal: model.isLocal,
            alias: model.alias,
            dpnsName: model.dpnsName,
            mainDpnsName: model.mainDpnsName,
            identityType: model.type,
            votingPrivateKeyIdentifier: votingKeyId,
            ownerPrivateKeyIdentifier: ownerKeyId,
            payoutPrivateKeyIdentifier: payoutKeyId,
            network: network
        )
        
        // Add public keys
        for publicKey in model.publicKeys {
            if let persistentKey = PersistentPublicKey.from(publicKey, identityId: model.idString) {
                persistent.addPublicKey(persistentKey)
            }
        }
        
        // Handle private keys - match them to their corresponding public keys using cryptographic validation
        for privateKeyData in model.privateKeys {
            // Find which public key this private key corresponds to
            if let matchingPublicKey = KeyValidation.matchPrivateKeyToPublicKeys(
                privateKeyData: privateKeyData,
                publicKeys: model.publicKeys,
                isTestnet: network == "testnet"
            ) {
                // Find the corresponding persistent public key
                if let persistentKey = persistent.publicKeys.first(where: { $0.keyId == matchingPublicKey.id }) {
                    // Store the private key for this specific public key
                    if let keychainId = KeychainManager.shared.storePrivateKey(privateKeyData, identityId: model.id, keyIndex: persistentKey.keyId) {
                        persistentKey.privateKeyKeychainIdentifier = keychainId
                    }
                }
            }
        }
        
        return persistent
    }
    
    /// Create from DPPIdentity
    static func from(_ dppIdentity: DPPIdentity, alias: String? = nil, type: IdentityType = .user, network: String = "testnet") -> PersistentIdentity {
        let persistent = PersistentIdentity(
            identityId: dppIdentity.id,
            balance: Int64(dppIdentity.balance),
            revision: Int64(dppIdentity.revision),
            isLocal: false,
            alias: alias,
            identityType: type,
            network: network
        )
        
        // Add public keys
        for (_, publicKey) in dppIdentity.publicKeys {
            if let persistentKey = PersistentPublicKey.from(publicKey, identityId: dppIdentity.idString) {
                persistent.addPublicKey(persistentKey)
            }
        }
        
        return persistent
    }
}

// MARK: - Queries

extension PersistentIdentity {
    /// Predicate to find identity by ID
    static func predicate(identityId: Data) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.identityId == identityId
        }
    }
    
    /// Predicate to find local identities
    static var localIdentitiesPredicate: Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.isLocal == true
        }
    }
    
    /// Predicate to find identities by type
    static func predicate(type: IdentityType) -> Predicate<PersistentIdentity> {
        let typeString = type.rawValue
        return #Predicate<PersistentIdentity> { identity in
            identity.identityType == typeString
        }
    }
    
    /// Predicate to find identities needing sync
    static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.lastSyncedAt == nil || identity.lastSyncedAt! < date
        }
    }
    
    /// Predicate to find identities by network
    static func predicate(network: String) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.network == network
        }
    }
    
    /// Predicate to find local identities by network
    static func localIdentitiesPredicate(network: String) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.isLocal == true && identity.network == network
        }
    }
}