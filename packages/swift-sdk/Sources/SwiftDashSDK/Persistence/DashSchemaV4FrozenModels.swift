import Foundation
import SwiftData

// The V4 relationship component, frozen before contractBoundsScope was added.
// SwiftData identifies a store by its entity checksum, so V4 must never use
// the live models again. All 24 related models travel together: relationships
// and inverse key paths must resolve to this same set of nested types.
// Keep stored properties, defaults, indexes and relationships unchanged.
// Initializers are retained for migration fixtures; runtime helpers stay on
// the live models. See DashSchemaFrozenModels.swift for earlier versions.

extension DashSchemaV4 {
    @Model
    final class PersistentAccount {
        #Unique<PersistentAccount>([
            \.wallet,
            \.accountType,
            \.accountIndex,
            \.standardTag,
            \.registrationIndex,
            \.keyClass,
            \.userIdentityId,
            \.friendIdentityId,
        ])
        var accountType: UInt32
        var accountIndex: UInt32
        var accountTypeName: String
        var balanceConfirmed: UInt64
        var balanceUnconfirmed: UInt64
        var externalHighestUsed: Int32
        var internalHighestUsed: Int32
        var standardTag: UInt8
        var registrationIndex: UInt32
        var keyClass: UInt32
        var userIdentityId: Data
        var friendIdentityId: Data
        @Attribute(.unique) var accountExtendedPubKeyBytes: Data?
        var createdAt: Date
        var lastUpdated: Date
        var wallet: PersistentWallet
        @Relationship(deleteRule: .cascade, inverse: \PersistentCoreAddress.account)
        var coreAddresses: [PersistentCoreAddress]
        @Relationship(deleteRule: .cascade, inverse: \PersistentPlatformAddress.account)
        var platformAddresses: [PersistentPlatformAddress]
        var involvedTransactions: [PersistentTransaction] = []
        init(
            wallet: PersistentWallet,
            accountType: UInt32,
            accountIndex: UInt32,
            accountTypeName: String
        ) {
            self.wallet = wallet
            self.accountType = accountType
            self.accountIndex = accountIndex
            self.accountTypeName = accountTypeName
            self.balanceConfirmed = 0
            self.balanceUnconfirmed = 0
            self.externalHighestUsed = -1
            self.internalHighestUsed = -1
            self.standardTag = 0
            self.registrationIndex = 0
            self.keyClass = 0
            self.userIdentityId = Data()
            self.friendIdentityId = Data()
            self.accountExtendedPubKeyBytes = nil
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.coreAddresses = []
            self.platformAddresses = []
            self.involvedTransactions = []
        }
    }

    @Model
    final class PersistentCoreAddress {
        @Attribute(.unique) var address: String
        var publicKey: Data
        var keyType: UInt8 = 0
        var poolTypeTag: UInt8
        var addressIndex: UInt32
        var derivationPath: String
        var isUsed: Bool
        var firstSeenHeight: UInt32
        var lastSeenHeight: UInt32
        var balance: UInt64
        var createdAt: Date
        var lastUpdated: Date
        var account: PersistentAccount?
        @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.coreAddress)
        var txos: [PersistentTxo] = []
        init(
            address: String,
            publicKey: Data = Data(),
            keyType: UInt8 = 0,
            poolTypeTag: UInt8,
            addressIndex: UInt32,
            derivationPath: String,
            isUsed: Bool = false,
            balance: UInt64 = 0
        ) {
            self.address = address
            self.publicKey = publicKey
            self.keyType = keyType
            self.poolTypeTag = poolTypeTag
            self.addressIndex = addressIndex
            self.derivationPath = derivationPath
            self.isUsed = isUsed
            self.firstSeenHeight = 0
            self.lastSeenHeight = 0
            self.balance = balance
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDPNSName {
        #Unique<PersistentDPNSName>([\.networkRaw, \.normalizedParentDomainName, \.normalizedLabel])
        var networkRaw: UInt32
        var label: String
        var normalizedLabel: String
        var parentDomainName: String
        var normalizedParentDomainName: String
        var acquiredAt: UInt64
        var isOwned: Bool = true
        var documentIdBase58: String?
        var priceCredits: Int64?
        var saleStatusRaw: Int16 = 0
        var counterpartyIdBase58: String?
        var documentCreatedAtMs: UInt64?
        var documentUpdatedAtMs: UInt64?
        var documentTransferredAtMs: UInt64?
        var marketplaceUpdatedAt: UInt64 = 0
        var identity: PersistentIdentity
        var createdAt: Date
        var lastUpdated: Date
        init(
            identity: PersistentIdentity,
            label: String,
            parentDomainName: String = "dash",
            acquiredAt: UInt64 = 0,
            isOwned: Bool = true
        ) {
            self.identity = identity
            self.networkRaw = identity.networkRaw
            self.label = label
            self.normalizedLabel = Self.normalize(label)
            self.parentDomainName = parentDomainName
            self.normalizedParentDomainName = Self.normalize(parentDomainName)
            self.acquiredAt = acquiredAt
            self.isOwned = isOwned
            self.documentIdBase58 = nil
            self.priceCredits = nil
            self.saleStatusRaw = 0
            self.counterpartyIdBase58 = nil
            self.documentCreatedAtMs = nil
            self.documentUpdatedAtMs = nil
            self.documentTransferredAtMs = nil
            self.marketplaceUpdatedAt = 0
            self.createdAt = Date()
            self.lastUpdated = Date()
        }

        static func normalize(_ input: String) -> String {
            String(input.map { c -> Character in
                switch c {
                case "o", "O": return "0"
                case "i", "I": return "1"
                case "l", "L": return "1"
                default: return Character(c.lowercased())
                }
            })
        }
    }

    @Model
    final class PersistentDashpayContactProfile {
        #Unique<PersistentDashpayContactProfile>([
            \.networkRaw, \.ownerIdentityId, \.contactIdentityId
        ])
        var networkRaw: UInt32
        var ownerIdentityId: Data
        var contactIdentityId: Data
        var displayName: String?
        var publicMessage: String?
        var bio: String?
        var avatarUrl: String?
        var avatarHash: Data?
        var avatarFingerprint: Data?
        var checkedAtMs: UInt64
        var owner: PersistentIdentity
        var createdAt: Date
        var lastUpdated: Date
        init(
            owner: PersistentIdentity,
            contactIdentityId: Data,
            checkedAtMs: UInt64,
            displayName: String? = nil,
            publicMessage: String? = nil,
            bio: String? = nil,
            avatarUrl: String? = nil,
            avatarHash: Data? = nil,
            avatarFingerprint: Data? = nil
        ) {
            self.owner = owner
            self.networkRaw = owner.networkRaw
            self.ownerIdentityId = owner.identityId
            self.contactIdentityId = contactIdentityId
            self.checkedAtMs = checkedAtMs
            self.displayName = displayName
            self.publicMessage = publicMessage
            self.bio = bio
            self.avatarUrl = avatarUrl
            self.avatarHash = avatarHash
            self.avatarFingerprint = avatarFingerprint
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDashpayContactRequest {
        #Unique<PersistentDashpayContactRequest>([
            \.networkRaw, \.ownerIdentityId, \.contactIdentityId, \.isOutgoing
        ])
        var networkRaw: UInt32
        var ownerIdentityId: Data
        var contactIdentityId: Data
        var isOutgoing: Bool
        var senderKeyIndex: UInt32
        var recipientKeyIndex: UInt32
        var accountReference: UInt32
        var encryptedPublicKey: Data
        var encryptedAccountLabel: Data?
        var autoAcceptProof: Data?
        var coreHeightCreatedAt: UInt32
        var createdAtMillis: UInt64
        var paymentChannelBroken: Bool = false
        var contactAlias: String?
        var contactNote: String?
        var contactHidden: Bool = false
        var contactAccountLabel: String?
        var contactAcceptedAccounts: [UInt32] = []
        var owner: PersistentIdentity
        var createdAt: Date
        var lastUpdated: Date
        init(
            owner: PersistentIdentity,
            contactIdentityId: Data,
            isOutgoing: Bool,
            senderKeyIndex: UInt32,
            recipientKeyIndex: UInt32,
            accountReference: UInt32,
            encryptedPublicKey: Data,
            encryptedAccountLabel: Data? = nil,
            autoAcceptProof: Data? = nil,
            coreHeightCreatedAt: UInt32,
            createdAtMillis: UInt64,
            paymentChannelBroken: Bool = false
        ) {
            self.owner = owner
            self.networkRaw = owner.networkRaw
            self.ownerIdentityId = owner.identityId
            self.contactIdentityId = contactIdentityId
            self.isOutgoing = isOutgoing
            self.senderKeyIndex = senderKeyIndex
            self.recipientKeyIndex = recipientKeyIndex
            self.accountReference = accountReference
            self.encryptedPublicKey = encryptedPublicKey
            self.encryptedAccountLabel = encryptedAccountLabel
            self.autoAcceptProof = autoAcceptProof
            self.coreHeightCreatedAt = coreHeightCreatedAt
            self.createdAtMillis = createdAtMillis
            self.paymentChannelBroken = paymentChannelBroken
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDashpayIgnoredSender {
        #Unique<PersistentDashpayIgnoredSender>([
            \.networkRaw, \.ownerIdentityId, \.ignoredSenderId
        ])
        var networkRaw: UInt32
        var ownerIdentityId: Data
        var ignoredSenderId: Data
        var owner: PersistentIdentity
        var ignoredAt: Date
        init(
            owner: PersistentIdentity,
            ignoredSenderId: Data
        ) {
            self.owner = owner
            self.networkRaw = owner.networkRaw
            self.ownerIdentityId = owner.identityId
            self.ignoredSenderId = ignoredSenderId
            self.ignoredAt = Date()
        }
    }

    @Model
    final class PersistentDashpayPayment {
        #Unique<PersistentDashpayPayment>([
            \.networkRaw, \.ownerIdentityId, \.txid
        ])
        var networkRaw: UInt32
        var ownerIdentityId: Data
        var counterpartyIdentityId: Data
        var amountDuffs: UInt64
        var directionRaw: UInt8
        var statusRaw: UInt8
        var txid: String
        var memo: String?
        var owner: PersistentIdentity
        var createdAt: Date
        var lastUpdated: Date
        init(
            owner: PersistentIdentity,
            counterpartyIdentityId: Data,
            amountDuffs: UInt64,
            direction: DashPayPaymentDirection,
            status: DashPayPaymentStatus,
            txid: String,
            memo: String? = nil
        ) {
            self.owner = owner
            self.networkRaw = owner.networkRaw
            self.ownerIdentityId = owner.identityId
            self.counterpartyIdentityId = counterpartyIdentityId
            self.amountDuffs = amountDuffs
            self.directionRaw = direction.rawValue
            self.statusRaw = status.rawValue
            self.txid = txid
            self.memo = memo
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDashpayProfile {
        #Unique<PersistentDashpayProfile>([\.networkRaw, \.identity])
        var networkRaw: UInt32
        var displayName: String?
        var publicMessage: String?
        var bio: String?
        var avatarUrl: String?
        var avatarHash: Data?
        var avatarFingerprint: Data?
        var identity: PersistentIdentity
        var createdAt: Date
        var lastUpdated: Date
        init(
            identity: PersistentIdentity,
            displayName: String? = nil,
            publicMessage: String? = nil,
            bio: String? = nil,
            avatarUrl: String? = nil,
            avatarHash: Data? = nil,
            avatarFingerprint: Data? = nil
        ) {
            self.identity = identity
            self.networkRaw = identity.networkRaw
            self.displayName = displayName
            self.publicMessage = publicMessage
            self.bio = bio
            self.avatarUrl = avatarUrl
            self.avatarHash = avatarHash
            self.avatarFingerprint = avatarFingerprint
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDataContract {
        #Index<PersistentDataContract>([\.networkRaw])
        @Attribute(.unique) var id: Data
        var name: String
        var serializedContract: Data
        var createdAt: Date
        var lastAccessedAt: Date
        var binarySerialization: Data?
        var version: Int?
        var ownerId: Data?
        @Relationship(deleteRule: .cascade, inverse: \PersistentKeyword.dataContract)
        var keywordRelations: [PersistentKeyword]
        var contractDescription: String?
        var schemaData: Data
        var documentTypesData: Data
        var groupsData: Data?
        var networkRaw: UInt32
        var lastUpdated: Date
        var lastSyncedAt: Date?
        var canBeDeleted: Bool
        var readonly: Bool
        var keepsHistory: Bool
        var schemaDefs: Int?
        var documentsKeepHistoryContractDefault: Bool
        var documentsMutableContractDefault: Bool
        var documentsCanBeDeletedContractDefault: Bool
        @Relationship(deleteRule: .cascade, inverse: \PersistentToken.dataContract)
        var tokens: [PersistentToken]?
        @Relationship(deleteRule: .cascade, inverse: \PersistentDocumentType.dataContract)
        var documentTypes: [PersistentDocumentType]?
        @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.dataContract)
        var documents: [PersistentDocument]
        @Relationship(deleteRule: .nullify, inverse: \PersistentIdentity.ownedDataContracts)
        var ownerIdentity: PersistentIdentity?
        var hasTokens: Bool
        var tokensData: Data?
        init(
            id: Data,
            name: String,
            serializedContract: Data,
            version: Int? = 1,
            ownerId: Data? = nil,
            schema: [String: Any] = [:],
            documentTypesList: [String] = [],
            keywords: [String] = [],
            description: String? = nil,
            hasTokens: Bool = false,
            network: Network
        ) {
            self.id = id
            self.name = name
            self.serializedContract = serializedContract
            self.createdAt = Date()
            self.lastAccessedAt = Date()
            self.version = version
            self.ownerId = ownerId
            self.schemaData = (try? JSONSerialization.data(withJSONObject: schema)) ?? Data()
            self.documentTypesData = (try? JSONSerialization.data(withJSONObject: documentTypesList)) ?? Data()
            self.keywordRelations = keywords.map { PersistentKeyword(keyword: $0, contractId: id.toBase58String()) }
            self.contractDescription = description
            self.hasTokens = hasTokens
            self.tokensData = nil
            self.groupsData = nil
            self.documents = []
            self.ownerIdentity = nil
            self.networkRaw = network.rawValue
            self.lastUpdated = Date()
            self.lastSyncedAt = nil
            self.canBeDeleted = false
            self.readonly = false
            self.keepsHistory = false
            self.documentsKeepHistoryContractDefault = false
            self.documentsMutableContractDefault = true
            self.documentsCanBeDeletedContractDefault = true
        }
    }

    @Model
    final class PersistentDocument {
        #Index<PersistentDocument>([\.networkRaw])
        @Attribute(.unique) var documentId: String
        var documentType: String
        var revision: Int32
        var data: Data
        var contractId: String
        var ownerId: String
        var contractIdData: Data
        var ownerIdData: Data
        var createdAt: Date
        var updatedAt: Date
        var transferredAt: Date?
        var createdAtBlockHeight: Int64?
        var updatedAtBlockHeight: Int64?
        var transferredAtBlockHeight: Int64?
        var createdAtCoreBlockHeight: Int64?
        var updatedAtCoreBlockHeight: Int64?
        var transferredAtCoreBlockHeight: Int64?
        var networkRaw: UInt32
        var isDeleted: Bool = false
        var localCreatedAt: Date
        var localUpdatedAt: Date
        var documentType_relation: PersistentDocumentType?
        var dataContract: PersistentDataContract?
        var ownerIdentity: PersistentIdentity?
        init(
            documentId: String,
            documentType: String,
            revision: Int32,
            data: Data,
            contractId: String,
            ownerId: String,
            network: Network
        ) {
            self.documentId = documentId
            self.documentType = documentType
            self.revision = revision
            self.data = data
            self.contractId = contractId
            self.ownerId = ownerId
            self.contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
            self.ownerIdData = Data.identifier(fromBase58: ownerId) ?? Data()
            self.networkRaw = network.rawValue
            self.createdAt = Date()
            self.updatedAt = Date()
            self.localCreatedAt = Date()
            self.localUpdatedAt = Date()
        }
    }

    @Model
    final class PersistentDocumentType {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var name: String
        var schemaJSON: Data
        var propertiesJSON: Data
        var documentsKeepHistory: Bool
        var documentsMutable: Bool
        var documentsCanBeDeleted: Bool
        var documentsTransferable: Bool
        var indexOnly: Bool = false
        var requiredFieldsJSON: Data?
        var securityLevel: Int
        var tradeMode: Int
        var creationRestrictionMode: Int
        var requiresIdentityEncryptionBoundedKey: Bool
        var requiresIdentityDecryptionBoundedKey: Bool
        var createdAt: Date
        var lastAccessedAt: Date
        var dataContract: PersistentDataContract?
        @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.documentType_relation)
        var documents: [PersistentDocument]?
        @Relationship(deleteRule: .cascade, inverse: \PersistentIndex.documentType)
        var indices: [PersistentIndex]?
        @Relationship(deleteRule: .cascade, inverse: \PersistentProperty.documentType)
        var propertiesList: [PersistentProperty]?
        init(contractId: Data, name: String, schemaJSON: Data, propertiesJSON: Data) {
            var idData = contractId
            idData.append(name.data(using: .utf8) ?? Data())
            self.id = idData
            self.contractId = contractId
            self.name = name
            self.schemaJSON = schemaJSON
            self.propertiesJSON = propertiesJSON
            self.documentsKeepHistory = false
            self.documentsMutable = true
            self.documentsCanBeDeleted = true
            self.documentsTransferable = false
            self.securityLevel = 0
            self.tradeMode = 0
            self.creationRestrictionMode = 0
            self.requiresIdentityEncryptionBoundedKey = false
            self.requiresIdentityDecryptionBoundedKey = false
            self.createdAt = Date()
            self.lastAccessedAt = Date()
        }
    }

    @Model
    final class PersistentIdentity {
        #Index<PersistentIdentity>([\.networkRaw])
        @Attribute(.unique) var identityId: Data
        var balance: Int64
        var revision: Int64
        var isLocal: Bool
        var alias: String?
        var dpnsName: String?
        var mainDpnsName: String?
        var identityType: String
        var votingPrivateKeyIdentifier: String?
        var ownerPrivateKeyIdentifier: String?
        var payoutPrivateKeyIdentifier: String?
        @Relationship(deleteRule: .cascade) var publicKeys: [PersistentPublicKey]
        var createdAt: Date
        var lastUpdated: Date
        var lastSyncedAt: Date?
        var networkRaw: UInt32
        var wallet: PersistentWallet?
        var identityIndex: UInt32 = 0
        @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.ownerIdentity) var documents: [PersistentDocument]
        @Relationship(deleteRule: .nullify) var tokenBalances: [PersistentTokenBalance]
        @Relationship(deleteRule: .cascade, inverse: \PersistentDPNSName.identity)
        var dpnsNames: [PersistentDPNSName] = []
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayProfile.identity)
        var dashpayProfile: PersistentDashpayProfile?
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayContactRequest.owner)
        var contactRequests: [PersistentDashpayContactRequest] = []
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayPayment.owner)
        var dashpayPayments: [PersistentDashpayPayment] = []
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayIgnoredSender.owner)
        var dashpayIgnoredSenders: [PersistentDashpayIgnoredSender] = []
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayContactProfile.owner)
        var contactProfiles: [PersistentDashpayContactProfile] = []
        var ownedDataContracts: [PersistentDataContract]
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
            network: Network,
            identityIndex: UInt32 = 0
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
            self.networkRaw = network.rawValue
            self.identityIndex = identityIndex
            self.publicKeys = []
            self.documents = []
            self.tokenBalances = []
            self.dpnsNames = []
            self.dashpayProfile = nil
            self.contactRequests = []
            self.dashpayPayments = []
            self.dashpayIgnoredSenders = []
            self.contactProfiles = []
            self.ownedDataContracts = []
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.lastSyncedAt = nil
        }
    }

    @Model
    final class PersistentIndex {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var documentTypeName: String
        var name: String
        var unique: Bool
        var nullSearchable: Bool
        var contested: Bool
        var countable: String?
        var rangeCountable: Bool = false
        var summable: String?
        var rangeSummable: Bool = false
        var averageable: String?
        var rangeAverageable: Bool = false
        var rankedCountable: Bool = false
        var rankedSummable: Bool = false
        var rankedAverageable: Bool = false
        var terminal: String?
        var preallocated: Bool = false
        var timeRangeJSON: Data?
        var propertiesJSON: Data
        var contestedDetailsJSON: Data?
        var createdAt: Date
        var documentType: PersistentDocumentType?
        init(contractId: Data, documentTypeName: String, name: String, properties: [String]) {
            var idData = contractId
            idData.append(documentTypeName.data(using: .utf8) ?? Data())
            idData.append(name.data(using: .utf8) ?? Data())
            self.id = idData
            self.contractId = contractId
            self.documentTypeName = documentTypeName
            self.name = name
            self.unique = false
            self.nullSearchable = false
            self.contested = false
            if let jsonData = try? JSONSerialization.data(withJSONObject: properties, options: []) {
                self.propertiesJSON = jsonData
            } else {
                self.propertiesJSON = Data()
            }
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentKeyword {
        @Attribute(.unique) var id: String
        var keyword: String
        var contractId: String
        var dataContract: PersistentDataContract?
        init(keyword: String, contractId: String) {
            self.id = "\(contractId)_\(keyword)"
            self.keyword = keyword
            self.contractId = contractId
        }
    }

    @Model
    final class PersistentPendingInput {
        #Index<PersistentPendingInput>([\.outpoint], [\.walletId], [\.walletId, \.isSweptTombstone])
        var outpoint: Data
        var inputIndex: UInt32
        var spendingTxid: Data
        var spendingTransaction: PersistentTransaction?
        var walletId: Data
        var createdAt: Date
        var isSweptTombstone: Bool = false
        var winnerMinedHeight: UInt32?
        init(
            outpoint: Data,
            inputIndex: UInt32,
            spendingTxid: Data,
            spendingTransaction: PersistentTransaction?,
            walletId: Data
        ) {
            self.outpoint = outpoint
            self.inputIndex = inputIndex
            self.spendingTxid = spendingTxid
            self.spendingTransaction = spendingTransaction
            self.walletId = walletId
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentPlatformAddress {
        #Index<PersistentPlatformAddress>([\.walletId])
        @Attribute(.unique) var address: String
        var addressType: UInt8
        @Attribute(.unique) var addressHash: Data
        var publicKey: Data
        var accountIndex: UInt32
        var addressIndex: UInt32
        var derivationPath: String
        var isUsed: Bool
        var balance: UInt64
        var nonce: UInt32
        var firstSeenHeight: UInt32
        var lastSeenHeight: UInt64
        var walletId: Data
        var createdAt: Date
        var lastUpdated: Date
        var account: PersistentAccount?
        init(
            address: String,
            addressType: UInt8,
            addressHash: Data,
            publicKey: Data = Data(),
            accountIndex: UInt32,
            addressIndex: UInt32,
            derivationPath: String,
            isUsed: Bool = false,
            balance: UInt64 = 0,
            nonce: UInt32 = 0,
            walletId: Data
        ) {
            self.address = address
            self.addressType = addressType
            self.addressHash = addressHash
            self.publicKey = publicKey
            self.accountIndex = accountIndex
            self.addressIndex = addressIndex
            self.derivationPath = derivationPath
            self.isUsed = isUsed
            self.balance = balance
            self.nonce = nonce
            self.firstSeenHeight = 0
            self.lastSeenHeight = 0
            self.walletId = walletId
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentProperty {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var documentTypeName: String
        var name: String
        var type: String
        var format: String?
        var contentMediaType: String?
        var byteArray: Bool
        var minItems: Int?
        var maxItems: Int?
        var pattern: String?
        var minLength: Int?
        var maxLength: Int?
        var minValue: Int?
        var maxValue: Int?
        var fieldDescription: String?
        var transient: Bool
        var isRequired: Bool
        var createdAt: Date
        var documentType: PersistentDocumentType?
        init(contractId: Data, documentTypeName: String, name: String, type: String) {
            var idData = contractId
            idData.append(documentTypeName.data(using: .utf8) ?? Data())
            idData.append(name.data(using: .utf8) ?? Data())
            self.id = idData
            self.contractId = contractId
            self.documentTypeName = documentTypeName
            self.name = name
            self.type = type
            self.byteArray = false
            self.transient = false
            self.isRequired = false
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentPublicKey {
        var keyId: Int32
        var purpose: String
        var securityLevel: String
        var keyType: String
        var readOnly: Bool
        var disabledAt: Int64?
        var publicKeyData: Data
        var contractBoundsData: Data?
        var contractBoundsDocumentTypeName: String?
        var privateKeyKeychainIdentifier: String?
        var walletId: Data?
        var identityDerivationPath: String?
        var identityId: String
        var createdAt: Date
        var lastAccessed: Date?
        @Relationship(inverse: \PersistentIdentity.publicKeys)
        var identity: PersistentIdentity?
        init(
            keyId: Int32,
            purpose: KeyPurpose,
            securityLevel: SecurityLevel,
            keyType: KeyType,
            publicKeyData: Data,
            readOnly: Bool = false,
            disabledAt: Int64? = nil,
            contractBounds: [Data]? = nil,
            contractBoundsDocumentTypeName: String? = nil,
            identityId: String
        ) {
            self.keyId = keyId
            self.purpose = String(purpose.rawValue)
            self.securityLevel = String(securityLevel.rawValue)
            self.keyType = String(keyType.rawValue)
            self.publicKeyData = publicKeyData
            self.readOnly = readOnly
            self.disabledAt = disabledAt
            if let contractBounds = contractBounds {
                self.contractBoundsData = try? JSONSerialization.data(withJSONObject: contractBounds.map { $0.base64EncodedString() })
            } else {
                self.contractBoundsData = nil
            }
            self.contractBoundsDocumentTypeName = contractBoundsDocumentTypeName
            self.identityId = identityId
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentToken {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var position: Int
        var name: String
        var baseSupply: String
        var maxSupply: String?
        var decimals: Int
        var localizations: [String: TokenLocalization]?
        var isPaused: Bool
        var allowTransferToFrozenBalance: Bool
        var keepsTransferHistory: Bool
        var keepsFreezingHistory: Bool
        var keepsMintingHistory: Bool
        var keepsBurningHistory: Bool
        var keepsDirectPricingHistory: Bool
        var keepsDirectPurchaseHistory: Bool
        var conventionsChangeRules: ChangeControlRules?
        var maxSupplyChangeRules: ChangeControlRules?
        var manualMintingRules: ChangeControlRules?
        var manualBurningRules: ChangeControlRules?
        var freezeRules: ChangeControlRules?
        var unfreezeRules: ChangeControlRules?
        var destroyFrozenFundsRules: ChangeControlRules?
        var emergencyActionRules: ChangeControlRules?
        var perpetualDistribution: TokenPerpetualDistribution?
        var preProgrammedDistribution: TokenPreProgrammedDistribution?
        var newTokensDestinationIdentity: Data?
        var mintingAllowChoosingDestination: Bool
        var distributionChangeRules: TokenDistributionChangeRules?
        var tradeMode: TokenTradeMode
        var tradeModeChangeRules: ChangeControlRules?
        var mainControlGroupPosition: Int?
        var mainControlGroupCanBeModified: String?
        var tokenDescription: String?
        var createdAt: Date
        var lastUpdatedAt: Date
        var dataContract: PersistentDataContract?
        @Relationship(deleteRule: .cascade)
        var balances: [PersistentTokenBalance]?
        @Relationship(deleteRule: .cascade)
        var historyEvents: [PersistentTokenHistoryEvent]?
        init(contractId: Data, position: Int, name: String, baseSupply: String, decimals: Int = 8) {
            var idData = contractId
            withUnsafeBytes(of: position.bigEndian) { bytes in
                idData.append(contentsOf: bytes)
            }
            self.id = idData
            self.contractId = contractId
            self.position = position
            self.name = name
            self.baseSupply = baseSupply
            self.decimals = decimals
            self.isPaused = false
            self.allowTransferToFrozenBalance = true
            self.keepsTransferHistory = true
            self.keepsFreezingHistory = true
            self.keepsMintingHistory = true
            self.keepsBurningHistory = true
            self.keepsDirectPricingHistory = true
            self.keepsDirectPurchaseHistory = true
            self.mintingAllowChoosingDestination = true
            self.tradeMode = TokenTradeMode.notTradeable
            self.createdAt = Date()
            self.lastUpdatedAt = Date()
        }
    }

    @Model
    final class PersistentTokenBalance {
        #Index<PersistentTokenBalance>([\.networkRaw])
        var tokenId: String
        var identityId: Data
        var balance: Int64
        var frozen: Bool
        var createdAt: Date
        var lastUpdated: Date
        var lastSyncedAt: Date?
        var tokenName: String?
        var tokenSymbol: String?
        var tokenDecimals: Int32?
        var networkRaw: UInt32
        @Relationship(deleteRule: .nullify) var identity: PersistentIdentity?
        @Relationship(inverse: \PersistentToken.balances) var token: PersistentToken?
        init(
            tokenId: String,
            identityId: Data,
            balance: Int64 = 0,
            frozen: Bool = false,
            tokenName: String? = nil,
            tokenSymbol: String? = nil,
            tokenDecimals: Int32? = nil,
            network: Network
        ) {
            self.tokenId = tokenId
            self.identityId = identityId
            self.balance = balance
            self.frozen = frozen
            self.tokenName = tokenName
            self.tokenSymbol = tokenSymbol
            self.tokenDecimals = tokenDecimals
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.lastSyncedAt = nil
            self.networkRaw = network.rawValue
        }
    }

    @Model
    final class PersistentTokenHistoryEvent {
        @Attribute(.unique) var id: UUID
        var eventType: String
        var transactionId: Data?
        var blockHeight: Int64?
        var coreBlockHeight: Int64?
        var fromIdentity: Data?
        var toIdentity: Data?
        var performedByIdentity: Data
        var amount: String?
        var balanceBefore: String?
        var balanceAfter: String?
        var additionalDataJSON: Data?
        var eventDescription: String?
        var createdAt: Date
        var eventTimestamp: Date
        @Relationship(inverse: \PersistentToken.historyEvents)
        var token: PersistentToken?
        init(
            eventType: TokenEventType,
            performedByIdentity: Data,
            eventTimestamp: Date = Date()
        ) {
            self.id = UUID()
            self.eventType = eventType.rawValue
            self.performedByIdentity = performedByIdentity
            self.eventTimestamp = eventTimestamp
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentTransaction {
        #Index<PersistentTransaction>([\.firstSeen])
        @Attribute(.unique) var txid: Data
        var transactionData: Data
        var context: UInt32
        var blockHeight: UInt32
        var blockHash: Data?
        var blockTimestamp: UInt32
        var blockPosition: UInt32 = 0
        var hasBlockPosition: Bool = false
        var direction: UInt32
        var transactionType: String
        var transactionTypeKind: UInt8 = 0xFF
        var netAmount: Int64
        var fee: UInt64?
        var label: String
        var firstSeen: UInt64
        var providerServiceAddress: String? = nil
        var providerProTxHash: Data? = nil
        var providerCollateralTxid: Data? = nil
        var providerCollateralVout: UInt32 = 0
        var providerOwnerKeyHash: Data? = nil
        var providerVotingKeyHash: Data? = nil
        var createdAt: Date
        var lastUpdated: Date
        @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.transaction)
        var outputs: [PersistentTxo] = []
        @Relationship(inverse: \PersistentTxo.spendingTransaction)
        var inputs: [PersistentTxo] = []
        @Relationship(deleteRule: .cascade, inverse: \PersistentPendingInput.spendingTransaction)
        var pendingInputs: [PersistentPendingInput] = []
        @Relationship(inverse: \PersistentAccount.involvedTransactions)
        var involvedAccounts: [PersistentAccount] = []
        init(
            txid: Data,
            transactionData: Data,
            context: UInt32 = 0,
            blockHeight: UInt32 = 0,
            direction: UInt32 = 0,
            transactionType: String = "Standard",
            netAmount: Int64 = 0,
            firstSeen: UInt64 = 0
        ) {
            self.txid = txid
            self.transactionData = transactionData
            self.context = context
            self.blockHeight = blockHeight
            self.blockTimestamp = 0
            self.direction = direction
            self.transactionType = transactionType
            self.netAmount = netAmount
            self.firstSeen = firstSeen
            self.label = ""
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentTxo {
        #Index<PersistentTxo>([\.walletId])
        @Attribute(.unique) var outpoint: Data
        var vout: UInt32
        var amount: UInt64
        var address: String
        var scriptPubKey: Data
        var height: UInt32
        var isCoinbase: Bool
        var isConfirmed: Bool
        var isInstantLocked: Bool
        var isLocked: Bool
        var isSpent: Bool
        var createdAt: Date
        var lastUpdated: Date
        var walletId: Data = Data()
        var transaction: PersistentTransaction?
        var spendingTransaction: PersistentTransaction?
        var supersededByTxid: Data?
        var spendingInputIndex: UInt32? = nil
        var account: PersistentAccount?
        var coreAddress: PersistentCoreAddress?
        init(
            transaction: PersistentTransaction,
            vout: UInt32,
            amount: UInt64,
            address: String,
            scriptPubKey: Data = Data(),
            height: UInt32 = 0
        ) {
            self.outpoint = Self.makeOutpoint(txid: transaction.txid, vout: vout)
            self.vout = vout
            self.amount = amount
            self.address = address
            self.scriptPubKey = scriptPubKey
            self.height = height
            self.isCoinbase = false
            self.isConfirmed = false
            self.isInstantLocked = false
            self.isLocked = false
            self.isSpent = false
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.transaction = transaction
        }
        static func makeOutpoint(txid: Data, vout: UInt32) -> Data {
            var data = Data(capacity: 36)
            data.append(txid)
            var v = vout.littleEndian
            withUnsafeBytes(of: &v) { data.append(contentsOf: $0) }
            return data
        }
    }

    @Model
    final class PersistentWallet {
        #Index<PersistentWallet>([\.networkRaw], [\.walletGroupId])
        #Unique<PersistentWallet>([\.walletId])
        var walletId: Data
        var walletGroupId: Data = Data()
        var networkRaw: UInt32?
        var name: String?
        var walletDescription: String?
        var birthHeight: UInt32
        var syncedHeight: UInt32
        var lastSynced: UInt64
        var lastAppliedChainLockBytes: Data?
        var lastAppliedChainLockHeight: UInt32?
        var isImported: Bool = false
        var seedBindingVerifiedMarker: String?
        var createdAt: Date
        var lastUpdated: Date
        @Relationship(deleteRule: .cascade, inverse: \PersistentAccount.wallet)
        var accounts: [PersistentAccount]
        @Relationship(deleteRule: .nullify, inverse: \PersistentIdentity.wallet)
        var identities: [PersistentIdentity]
        init(
            walletId: Data,
            walletGroupId: Data = Data(),
            network: Network? = nil,
            name: String? = nil,
            walletDescription: String? = nil,
            birthHeight: UInt32 = 0,
            syncedHeight: UInt32 = 0,
            isImported: Bool = false
        ) {
            self.walletId = walletId
            self.walletGroupId = walletGroupId
            self.networkRaw = network?.rawValue
            self.name = name
            self.walletDescription = walletDescription
            self.birthHeight = birthHeight
            self.syncedHeight = syncedHeight
            self.lastSynced = 0
            self.isImported = isImported
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.accounts = []
            self.identities = []
        }
    }
}
