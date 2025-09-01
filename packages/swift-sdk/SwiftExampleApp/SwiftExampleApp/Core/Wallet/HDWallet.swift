import Foundation
import SwiftData

// MARK: - HD Wallet

@Model
public final class HDWallet: HDWalletModels {
    @Attribute(.unique) public var id: UUID
    public var label: String
    public var network: String
    public var createdAt: Date
    public var lastSyncedHeight: Int
    public var isWatchOnly: Bool
    
    // FFI Wallet ID (32 bytes) - links to the rust-dashcore wallet
    public var walletId: Data?
    
    // Serialized wallet bytes from FFI - used to restore wallet on app restart
    public var serializedWalletBytes: Data?
    
    // Encrypted seed (only for non-watch-only wallets)
    public var encryptedSeed: Data?
    
    // Accounts
    @Relationship(deleteRule: .cascade) public var accounts: [HDAccount] = []
    
    // Current account index
    public var currentAccountIndex: Int
    
    // Sync progress (0.0 to 1.0)
    public var syncProgress: Double
    
    // Networks bitfield - tracks which networks this wallet is available on
    // Uses FFINetworks values: DASH(mainnet)=1, TESTNET=2, DEVNET=8
    public var networks: UInt32
    
    public init(label: String, network: Network, isWatchOnly: Bool = false) {
        self.id = UUID()
        self.label = label
        self.network = network.rawValue
        self.createdAt = Date()
        self.lastSyncedHeight = 0
        self.isWatchOnly = isWatchOnly
        self.currentAccountIndex = 0
        self.syncProgress = 0.0
        
        // Initialize networks bitfield based on the initial network
        switch network {
        case .mainnet:
            self.networks = 1  // DASH
        case .testnet:
            self.networks = 2  // TESTNET
        case .devnet:
            self.networks = 8  // DEVNET
        }
    }
    
    public var dashNetwork: Network {
        return Network(rawValue: network) ?? .testnet
    }
    
    // Total balance across all accounts
    public var totalBalance: UInt64 {
        return accounts.reduce(0) { $0 + $1.totalBalance }
    }
    
    // Confirmed balance across all accounts
    public var confirmedBalance: UInt64 {
        return accounts.reduce(0) { $0 + $1.confirmedBalance }
    }
    
    // Unconfirmed balance across all accounts
    public var unconfirmedBalance: UInt64 {
        return accounts.reduce(0) { $0 + $1.unconfirmedBalance }
    }
    
    // All transactions across all accounts
    public var transactions: [HDTransaction] {
        return accounts.flatMap { account in
            account.addresses.flatMap { $0.transactions }
        }
    }
    
    // All addresses across all accounts
    public var addresses: [HDAddress] {
        return accounts.flatMap { $0.addresses }
    }
    
    // All UTXOs across all accounts
    public var utxos: [HDUTXO] {
        return addresses.flatMap { $0.utxos }
    }
    
    public func createAccount(at index: UInt32? = nil) -> HDAccount {
        let accountIndex = index ?? UInt32(accounts.count)
        let account = HDAccount(
            accountNumber: accountIndex,
            label: "Account \(accountIndex)",
            wallet: self
        )
        accounts.append(account)
        return account
    }
}

// MARK: - HD Account

@Model
public final class HDAccount: HDWalletModels {
    @Attribute(.unique) public var id: UUID
    public var accountNumber: UInt32
    public var label: String
    
    // Extended public key for this account (watch-only capability)
    public var extendedPublicKey: String?
    
    // Derivation paths
    @Relationship(deleteRule: .cascade) public var externalAddresses: [HDAddress] = []
    @Relationship(deleteRule: .cascade) public var internalAddresses: [HDAddress] = []
    @Relationship(deleteRule: .cascade) public var coinJoinAddresses: [HDAddress] = []
    @Relationship(deleteRule: .cascade) public var identityFundingAddresses: [HDAddress] = []
    
    // Indexes
    public var externalAddressIndex: UInt32
    public var internalAddressIndex: UInt32
    public var coinJoinExternalIndex: UInt32
    public var coinJoinInternalIndex: UInt32
    public var identityFundingIndex: UInt32
    
    // Balance tracking
    public var confirmedBalance: UInt64
    public var unconfirmedBalance: UInt64
    
    // Parent wallet
    @Relationship(inverse: \HDWallet.accounts) public var wallet: HDWallet?
    
    public init(accountNumber: UInt32, label: String, wallet: HDWallet) {
        self.id = UUID()
        self.accountNumber = accountNumber
        self.label = label
        self.wallet = wallet
        self.externalAddressIndex = 0
        self.internalAddressIndex = 0
        self.coinJoinExternalIndex = 0
        self.coinJoinInternalIndex = 0
        self.identityFundingIndex = 0
        self.confirmedBalance = 0
        self.unconfirmedBalance = 0
    }
    
    public var totalBalance: UInt64 {
        return confirmedBalance + unconfirmedBalance
    }
    
    // All addresses combined
    public var addresses: [HDAddress] {
        return externalAddresses + internalAddresses + coinJoinAddresses + identityFundingAddresses
    }
}

// MARK: - HD Address

@Model
public final class HDAddress: HDWalletModels {
    @Attribute(.unique) public var id: UUID
    @Attribute(.unique) public var address: String
    public var index: UInt32
    public var derivationPath: String
    public var isUsed: Bool
    public var balance: UInt64
    public var lastSeenTime: Date?
    
    // Address type
    public var addressType: String  // "external", "internal", "coinjoin", "identity"
    
    // Parent account
    @Relationship public var account: HDAccount?
    
    // Associated transactions
    @Relationship(deleteRule: .nullify) public var transactions: [HDTransaction] = []
    
    // UTXOs
    @Relationship(deleteRule: .cascade) public var utxos: [HDUTXO] = []
    
    public init(address: String, index: UInt32, derivationPath: String, addressType: AddressType, account: HDAccount) {
        self.id = UUID()
        self.address = address
        self.index = index
        self.derivationPath = derivationPath
        self.addressType = addressType.rawValue
        self.isUsed = false
        self.balance = 0
        self.account = account
    }
    
    public var type: AddressType {
        return AddressType(rawValue: addressType) ?? .external
    }
}

public enum AddressType: String {
    case external = "external"
    case `internal` = "internal"
    case coinJoin = "coinjoin"
    case identity = "identity"
}

// MARK: - HD UTXO

@Model
public final class HDUTXO: HDWalletModels {
    @Attribute(.unique) public var id: UUID
    public var txHash: String
    public var outputIndex: UInt32
    public var amount: UInt64
    public var scriptPubKey: Data
    public var blockHeight: Int?
    public var isSpent: Bool
    public var isCoinbase: Bool
    
    // Parent address
    @Relationship(inverse: \HDAddress.utxos) public var address: HDAddress?
    
    // Spending transaction (if spent)
    public var spendingTxHash: String?
    public var spendingInputIndex: UInt32?
    
    public init(txHash: String, outputIndex: UInt32, amount: UInt64, scriptPubKey: Data, address: HDAddress) {
        self.id = UUID()
        self.txHash = txHash
        self.outputIndex = outputIndex
        self.amount = amount
        self.scriptPubKey = scriptPubKey
        self.address = address
        self.isSpent = false
        self.isCoinbase = false
    }
    
    // Computed property to check if UTXO is confirmed
    public var isConfirmed: Bool {
        return blockHeight != nil
    }
    
    // Alias for txHash
    public var txid: String {
        return txHash
    }
}

// MARK: - Watched Address (for import)

@Model
public final class HDWatchedAddress: HDWalletModels {
    @Attribute(.unique) public var id: UUID
    @Attribute(.unique) public var address: String
    public var label: String?
    public var balance: UInt64
    public var lastSeenTime: Date?
    
    // Parent wallet
    @Relationship public var wallet: HDWallet?
    
    // Associated transactions
    @Relationship(deleteRule: .nullify) public var transactions: [HDTransaction] = []
    
    public init(address: String, label: String? = nil, wallet: HDWallet) {
        self.id = UUID()
        self.address = address
        self.label = label
        self.balance = 0
        self.wallet = wallet
    }
}

// MARK: - Protocol for common functionality

public protocol HDWalletModels: AnyObject {
    var id: UUID { get set }
}