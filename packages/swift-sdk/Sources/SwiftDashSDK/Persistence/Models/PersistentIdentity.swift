import Foundation
import SwiftData

/// SwiftData model for persisting Identity data
@Model
public final class PersistentIdentity {
    // MARK: - Core Properties
    @Attribute(.unique) public var identityId: Data
    public var balance: Int64
    public var revision: Int64
    public var isLocal: Bool
    public var alias: String?
    public var dpnsName: String?
    public var mainDpnsName: String?
    public var identityType: String

    // MARK: - Special Key Storage (stored in keychain)
    public var votingPrivateKeyIdentifier: String?
    public var ownerPrivateKeyIdentifier: String?
    public var payoutPrivateKeyIdentifier: String?

    // MARK: - Public Keys
    @Relationship(deleteRule: .cascade) public var publicKeys: [PersistentPublicKey]

    // MARK: - Timestamps
    public var createdAt: Date
    public var lastUpdated: Date
    public var lastSyncedAt: Date?

    // MARK: - Network
    public var network: String

    // MARK: - Wallet Association
    public var walletId: Data?

    // MARK: - Relationships
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.ownerIdentity) public var documents: [PersistentDocument]
    @Relationship(deleteRule: .nullify) public var tokenBalances: [PersistentTokenBalance]

    // MARK: - Initialization
    public init(
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
        network: String = "testnet",
        walletId: Data? = nil
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
        self.walletId = walletId
        self.publicKeys = []
        self.documents = []
        self.tokenBalances = []
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.lastSyncedAt = nil
    }

    // MARK: - Computed Properties
    public var identityIdString: String {
        identityId.toHexString()
    }

    public var identityIdBase58: String {
        identityId.toBase58String()
    }

    public var formattedBalance: String {
        let dashAmount = Double(balance) / 100_000_000_000
        return String(format: "%.8f DASH", dashAmount)
    }

    public var identityTypeEnum: IdentityType {
        IdentityType(rawValue: identityType) ?? .user
    }

    // MARK: - Methods
    public func updateBalance(_ newBalance: Int64) {
        self.balance = newBalance
        self.lastUpdated = Date()
    }

    public func updateRevision(_ newRevision: Int64) {
        self.revision = newRevision
        self.lastUpdated = Date()
    }

    public func markAsSynced() {
        self.lastSyncedAt = Date()
    }

    public func updateDPNSName(_ name: String?) {
        self.dpnsName = name
        self.lastUpdated = Date()
    }

    public func addPublicKey(_ key: PersistentPublicKey) {
        publicKeys.append(key)
        lastUpdated = Date()
    }

    public func removePublicKey(withId keyId: Int32) {
        publicKeys.removeAll { $0.keyId == keyId }
        lastUpdated = Date()
    }
}


// MARK: - Queries

extension PersistentIdentity {
    public static func predicate(identityId: Data) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.identityId == identityId
        }
    }

    public static var localIdentitiesPredicate: Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.isLocal == true
        }
    }

    public static func predicate(type: IdentityType) -> Predicate<PersistentIdentity> {
        let typeString = type.rawValue
        return #Predicate<PersistentIdentity> { identity in
            identity.identityType == typeString
        }
    }

    public static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.lastSyncedAt == nil || identity.lastSyncedAt! < date
        }
    }

    public static func predicate(network: String) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.network == network
        }
    }

    public static func localIdentitiesPredicate(network: String) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.isLocal == true && identity.network == network
        }
    }
}

// MARK: - Conversion Methods

extension PersistentIdentity {
    /// Create a PersistentIdentity from an IdentityModel
    public static func from(_ identity: IdentityModel, network: AppNetwork) -> PersistentIdentity {
        let persistent = PersistentIdentity(
            identityId: identity.id,
            balance: Int64(identity.balance),
            revision: 0,
            isLocal: identity.isLocal,
            alias: identity.alias,
            dpnsName: identity.dpnsName,
            mainDpnsName: identity.mainDpnsName,
            identityType: identity.type,
            network: network.rawValue,
            walletId: identity.walletId
        )

        // Add public keys
        for publicKey in identity.publicKeys {
            if let persistentKey = PersistentPublicKey.from(publicKey, identityId: identity.idString) {
                persistent.addPublicKey(persistentKey)
            }
        }

        return persistent
    }

    /// Convert to an IdentityModel
    /// Note: This method does not load private keys from keychain. Use separate async methods to load keys if needed.
    public func toIdentityModel() -> IdentityModel {
        // Convert public keys
        let publicKeyModels = publicKeys.compactMap { $0.toIdentityPublicKey() }

        return IdentityModel(
            id: identityId,
            balance: UInt64(balance),
            isLocal: isLocal,
            alias: alias,
            type: identityTypeEnum,
            privateKeys: [],  // Keys are loaded separately via KeychainManager
            votingPrivateKey: nil,
            ownerPrivateKey: nil,
            payoutPrivateKey: nil,
            dpnsName: dpnsName,
            mainDpnsName: mainDpnsName,
            publicKeys: publicKeyModels,
            walletId: walletId,
            network: network
        )
    }
}
