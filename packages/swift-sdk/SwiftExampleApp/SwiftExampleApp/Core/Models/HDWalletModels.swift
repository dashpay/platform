import Foundation
import SwiftData

@Model
public final class HDWallet {
    @Attribute(.unique) public var id: UUID
    public var label: String
    public var network: String
    public var createdAt: Date
    public var lastSyncedAt: Date?
    public var syncProgress: Double
    public var accountIndex: Int
    
    // Encrypted mnemonic stored securely
    @Transient public var mnemonic: String?
    
    // Relationships
    @Relationship(deleteRule: .cascade) public var addresses: [HDAddress] = []
    @Relationship(deleteRule: .cascade) public var transactions: [HDTransaction] = []
    @Relationship(deleteRule: .cascade) public var utxos: [HDUTXO] = []
    
    // Computed properties
    public var confirmedBalance: UInt64 {
        utxos.filter { $0.isConfirmed }.reduce(0) { $0 + $1.amount }
    }
    
    public var unconfirmedBalance: UInt64 {
        utxos.filter { !$0.isConfirmed }.reduce(0) { $0 + $1.amount }
    }
    
    public var totalBalance: UInt64 {
        confirmedBalance + unconfirmedBalance
    }
    
    public var receiveAddress: String? {
        addresses.first(where: { $0.type == AddressType.external.rawValue && !$0.isUsed })?.address
    }
    
    public init(
        label: String,
        network: String = DashNetwork.testnet,
        accountIndex: Int = 0
    ) {
        self.id = UUID()
        self.label = label
        self.network = network
        self.createdAt = Date()
        self.syncProgress = 0.0
        self.accountIndex = accountIndex
    }
}

@Model
public final class HDAddress {
    @Attribute(.unique) public var id: UUID
    public var address: String
    public var index: UInt32
    public var type: String // AddressType raw value
    public var isUsed: Bool
    public var createdAt: Date
    public var lastSeenAt: Date?
    
    // Relationships
    public var wallet: HDWallet?
    
    public init(
        address: String,
        index: UInt32,
        type: AddressType,
        isUsed: Bool = false
    ) {
        self.id = UUID()
        self.address = address
        self.index = index
        self.type = type.rawValue
        self.isUsed = isUsed
        self.createdAt = Date()
    }
}

@Model
public final class HDTransaction {
    @Attribute(.unique) public var id: UUID
    public var txid: String
    public var amount: Int64 // Can be negative for sent transactions
    public var fee: UInt64
    public var timestamp: Date
    public var blockHeight: Int64?
    public var confirmations: Int
    public var type: String // TransactionType raw value
    public var memo: String?
    public var isInstantSend: Bool
    public var isAssetLock: Bool
    
    // Serialized transaction data
    public var rawData: Data?
    
    // Relationships
    public var wallet: HDWallet?
    
    // Computed properties
    public var isConfirmed: Bool {
        confirmations >= 6
    }
    
    public var isPending: Bool {
        confirmations == 0
    }
    
    public init(
        txid: String,
        amount: Int64,
        fee: UInt64,
        timestamp: Date,
        type: TransactionType,
        isInstantSend: Bool = false,
        isAssetLock: Bool = false
    ) {
        self.id = UUID()
        self.txid = txid
        self.amount = amount
        self.fee = fee
        self.timestamp = timestamp
        self.confirmations = 0
        self.type = type.rawValue
        self.isInstantSend = isInstantSend
        self.isAssetLock = isAssetLock
    }
}

@Model
public final class HDUTXO {
    @Attribute(.unique) public var id: UUID
    public var txid: String
    public var outputIndex: UInt32
    public var amount: UInt64
    public var address: String
    public var scriptPubKey: Data
    public var blockHeight: Int64?
    public var isConfirmed: Bool
    
    // Relationships
    public var wallet: HDWallet?
    
    public init(
        txid: String,
        outputIndex: UInt32,
        amount: UInt64,
        address: String,
        scriptPubKey: Data,
        blockHeight: Int64? = nil
    ) {
        self.id = UUID()
        self.txid = txid
        self.outputIndex = outputIndex
        self.amount = amount
        self.address = address
        self.scriptPubKey = scriptPubKey
        self.blockHeight = blockHeight
        self.isConfirmed = blockHeight != nil
    }
}

@Model
public final class HDWatchedAddress {
    @Attribute(.unique) public var id: UUID
    public var address: String
    public var label: String
    public var network: String
    public var createdAt: Date
    public var lastSeenAt: Date?
    public var balance: UInt64
    public var transactionCount: Int
    public var watchStatus: String // WatchStatus raw value
    
    public init(
        address: String,
        label: String,
        network: String = DashNetwork.testnet
    ) {
        self.id = UUID()
        self.address = address
        self.label = label
        self.network = network
        self.createdAt = Date()
        self.balance = 0
        self.transactionCount = 0
        self.watchStatus = WatchStatus.inactive.rawValue
    }
}