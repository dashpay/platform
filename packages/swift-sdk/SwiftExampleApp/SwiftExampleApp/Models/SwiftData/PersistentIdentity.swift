import Foundation
import SwiftData

/// SwiftData model for persisting Identity data
@Model
final class PersistentIdentity {
    // MARK: - Core Properties
    @Attribute(.unique) var identityId: Data
    var balance: Int64
    var revision: Int64
    var isLocal: Bool
    var alias: String?
    var identityType: String
    
    // MARK: - Key Storage
    var privateKeys: [String]
    var votingPrivateKey: String?
    var ownerPrivateKey: String?
    var payoutPrivateKey: String?
    
    // MARK: - Public Keys
    @Relationship(deleteRule: .cascade) var publicKeys: [PersistentPublicKey]
    
    // MARK: - Timestamps
    var createdAt: Date
    var lastUpdated: Date
    var lastSyncedAt: Date?
    
    // MARK: - Network
    var network: String
    
    // MARK: - Relationships
    @Relationship(deleteRule: .cascade) var documents: [PersistentDocument]
    @Relationship(deleteRule: .nullify) var tokenBalances: [PersistentTokenBalance]
    
    // MARK: - Initialization
    init(
        identityId: Data,
        balance: Int64 = 0,
        revision: Int64 = 0,
        isLocal: Bool = true,
        alias: String? = nil,
        identityType: IdentityType = .user,
        privateKeys: [String] = [],
        votingPrivateKey: String? = nil,
        ownerPrivateKey: String? = nil,
        payoutPrivateKey: String? = nil,
        network: String = "testnet"
    ) {
        self.identityId = identityId
        self.balance = balance
        self.revision = revision
        self.isLocal = isLocal
        self.alias = alias
        self.identityType = identityType.rawValue
        self.privateKeys = privateKeys
        self.votingPrivateKey = votingPrivateKey
        self.ownerPrivateKey = ownerPrivateKey
        self.payoutPrivateKey = payoutPrivateKey
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
        let dashAmount = Double(balance) / 100_000_000
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
        
        return IdentityModel(
            id: identityId,
            balance: UInt64(balance),
            isLocal: isLocal,
            alias: alias,
            type: identityTypeEnum,
            privateKeys: privateKeys,
            votingPrivateKey: votingPrivateKey,
            ownerPrivateKey: ownerPrivateKey,
            payoutPrivateKey: payoutPrivateKey,
            dppIdentity: nil, // Would need to reconstruct from data
            publicKeys: publicKeyModels
        )
    }
    
    /// Create from IdentityModel
    static func from(_ model: IdentityModel, network: String = "testnet") -> PersistentIdentity {
        let persistent = PersistentIdentity(
            identityId: model.id,
            balance: Int64(model.balance),
            revision: Int64(model.dppIdentity?.revision ?? 0),
            isLocal: model.isLocal,
            alias: model.alias,
            identityType: model.type,
            privateKeys: model.privateKeys,
            votingPrivateKey: model.votingPrivateKey,
            ownerPrivateKey: model.ownerPrivateKey,
            payoutPrivateKey: model.payoutPrivateKey,
            network: network
        )
        
        // Add public keys
        for publicKey in model.publicKeys {
            if let persistentKey = PersistentPublicKey.from(publicKey, identityId: model.idString) {
                persistent.addPublicKey(persistentKey)
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