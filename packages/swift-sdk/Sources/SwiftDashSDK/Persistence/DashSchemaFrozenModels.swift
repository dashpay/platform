import Foundation
import SwiftData

// MARK: - Frozen model definitions for already-released schema versions
//
// A `VersionedSchema` identifies a store by the CHECKSUM of the entities it
// declares, not by the identity of the Swift type list. So a registered
// schema version may only ever reference model types whose *shape* is frozen
// at the moment that version shipped. Pointing `DashSchemaVN.models` at a
// live `@Model` type means the next property added to that type silently
// mutates version N's checksum in place: a store written by the previously
// released binary then matches no schema in `DashMigrationPlan.schemas`, and
// `ModelContainer(for:migrationPlan:configurations:)` fails to open it with
// Cocoa error 134504 ("Cannot use staged migration with an unknown model
// version") instead of migrating it.
//
// This file holds the frozen copies. A frozen copy must be a *nested* type,
// because SwiftData derives the entity name from the unqualified type name —
// `DashSchemaV1.PersistentAssetLock` and the top-level `PersistentAssetLock`
// are two distinct Swift types that both describe the entity named
// "PersistentAssetLock", which is exactly what lets a migration stage map one
// onto the other. (`DashModelMigrationTests` asserts that entity naming, so a
// future SwiftData change to the derivation would fail loudly rather than
// silently renaming an entity.)
//
// V1's own checksum has already drifted from what actually shipped as V1
// (see the `DashSchemaV1` doc comment: several models were changed in place
// while V1 was the only registered version, and dev stores at V1 are
// knowingly expected to fail open and be rebuilt). The frozen copy below is
// therefore the shapes at the first release that froze them. The asset-lock
// copy is shared by V1 and V2; the wallet transaction graph stayed unchanged
// through V3 and is shared by all three. This keeps V1 -> V2 exactly "add
// `PersistentTrackedMasternode`" and V2 -> V3 exactly the asset-lock column.

extension DashSchemaV1 {
    /// Wallet metadata frozen before the numeric chain-lock height was added.
    @Model
    final class PersistentWallet {
        #Index<PersistentWallet>([\.networkRaw], [\.walletGroupId])
        #Unique<PersistentWallet>([\.walletId])

        var walletId: Data
        var walletGroupId: Data = Data()
        var networkRaw: UInt32?
        var network: Network? {
            get {
                guard let raw = networkRaw else { return nil }
                return Network(rawValue: raw) ?? .testnet
            }
            set { networkRaw = newValue?.rawValue }
        }
        var name: String?
        var walletDescription: String?
        var birthHeight: UInt32
        var syncedHeight: UInt32
        var lastSynced: UInt64
        var lastAppliedChainLockBytes: Data?
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

    /// Transaction row frozen before global sweep exclusion was added.
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

    /// Transaction output frozen before winner attribution was added.
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

    /// Pending input frozen before swept-input tombstones were added.
    @Model
    final class PersistentPendingInput {
        #Index<PersistentPendingInput>([\.outpoint], [\.walletId])

        var outpoint: Data
        var inputIndex: UInt32
        var spendingTxid: Data
        var spendingTransaction: PersistentTransaction?
        var walletId: Data
        var createdAt: Date

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

    /// `PersistentAssetLock` frozen at the shape it had when schema V2
    /// shipped — i.e. everything the live model has today EXCEPT
    /// `recipientIsExternal`, which is what V3 adds.
    ///
    /// Referenced by both `DashSchemaV1.models` and `DashSchemaV2.models`.
    /// Do not add properties here and do not "fix" its doc comments to
    /// match the live model: every attribute, its optionality, its default
    /// value, the `@Attribute(.unique)` marker and the `#Index` are all
    /// inputs to the V2 checksum, and changing any of them re-breaks the
    /// V2 stores this type exists to keep openable. Doc comments are not
    /// inputs to the checksum, but keeping them minimal here keeps the
    /// live model the single place worth reading.
    ///
    /// See the live ``SwiftDashSDK/PersistentAssetLock`` for what each
    /// column means.
    @Model
    final class PersistentAssetLock {
        #Index<PersistentAssetLock>([\.walletId])

        @Attribute(.unique) var outPointHex: String
        var walletId: Data
        var transactionBytes: Data
        var fundingTypeRaw: Int
        var identityIndexRaw: Int32
        var accountIndexRaw: Int32 = 0
        var amountDuffs: Int64
        var statusRaw: Int
        var proofBytes: Data?
        var recipientPlatformAddressHash: Data?
        var recipientPlatformAddressType: UInt8?
        var createdAt: Date
        var updatedAt: Date

        init(
            outPointHex: String,
            walletId: Data,
            transactionBytes: Data,
            fundingTypeRaw: Int,
            identityIndexRaw: Int32,
            accountIndexRaw: Int32 = 0,
            amountDuffs: Int64,
            statusRaw: Int,
            proofBytes: Data? = nil
        ) {
            self.outPointHex = outPointHex
            self.walletId = walletId
            self.transactionBytes = transactionBytes
            self.fundingTypeRaw = fundingTypeRaw
            self.identityIndexRaw = identityIndexRaw
            self.accountIndexRaw = accountIndexRaw
            self.amountDuffs = amountDuffs
            self.statusRaw = statusRaw
            self.proofBytes = proofBytes
            self.createdAt = Date()
            self.updatedAt = Date()
        }
    }
}
