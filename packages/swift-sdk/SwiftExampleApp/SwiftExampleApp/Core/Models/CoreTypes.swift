import Foundation

// Core SDK Types
public typealias DashNetwork = String
public typealias SPVClient = Any
public typealias WalletFFI = Any

// Extension for DashNetwork constants
public extension DashNetwork {
    static let mainnet = "mainnet"
    static let testnet = "testnet"
    static let devnet = "devnet"
    static let regtest = "regtest"
}

// Transaction type enum
public enum TransactionType: String, CaseIterable {
    case sent = "sent"
    case received = "received"
    case pending = "pending"
    case assetLock = "assetLock"
    case instantSend = "instantSend"
    
    var displayName: String {
        switch self {
        case .sent: return "Sent"
        case .received: return "Received"
        case .pending: return "Pending"
        case .assetLock: return "Asset Lock"
        case .instantSend: return "InstantSend"
        }
    }
    
    var icon: String {
        switch self {
        case .sent: return "arrow.up.circle"
        case .received: return "arrow.down.circle"
        case .pending: return "clock"
        case .assetLock: return "lock.circle"
        case .instantSend: return "bolt.circle"
        }
    }
}

// Address type enum
public enum AddressType: String, CaseIterable {
    case external = "external"
    case change = "change"
    case masternode = "masternode"
    
    var displayName: String {
        switch self {
        case .external: return "Receiving"
        case .change: return "Change"
        case .masternode: return "Masternode"
        }
    }
}

// Sync state enum
public enum SyncState: String {
    case notStarted = "not_started"
    case syncing = "syncing"
    case synced = "synced"
    case error = "error"
    
    var displayName: String {
        switch self {
        case .notStarted: return "Not Started"
        case .syncing: return "Syncing"
        case .synced: return "Synced"  
        case .error: return "Error"
        }
    }
}

// Watch status for addresses
public enum WatchStatus: String {
    case active = "active"
    case inactive = "inactive"
    case error = "error"
    
    var displayName: String {
        switch self {
        case .active: return "Watching"
        case .inactive: return "Not Watching"
        case .error: return "Error"
        }
    }
}

// InstantLock result
public struct InstantLock {
    public let txid: String
    public let isConfirmed: Bool
    public let signature: Data?
    public let confirmationTime: Date?
    
    public init(txid: String, isConfirmed: Bool, signature: Data? = nil, confirmationTime: Date? = nil) {
        self.txid = txid
        self.isConfirmed = isConfirmed
        self.signature = signature
        self.confirmationTime = confirmationTime
    }
}

// Errors
public enum WalletError: LocalizedError {
    case invalidMnemonic
    case invalidAddress
    case insufficientFunds
    case syncRequired
    case networkError(String)
    case ffiError(String)
    case unknown(String)
    
    public var errorDescription: String? {
        switch self {
        case .invalidMnemonic:
            return "Invalid mnemonic phrase"
        case .invalidAddress:
            return "Invalid address format"
        case .insufficientFunds:
            return "Insufficient funds"
        case .syncRequired:
            return "Wallet sync required"
        case .networkError(let msg):
            return "Network error: \(msg)"
        case .ffiError(let msg):
            return "FFI error: \(msg)"
        case .unknown(let msg):
            return "Unknown error: \(msg)"
        }
    }
}

// AssetLock errors
public enum AssetLockError: LocalizedError {
    case insufficientBalance
    case assetLockGenerationFailed
    case instantLockTimeout
    case broadcastFailed(String)
    
    public var errorDescription: String? {
        switch self {
        case .insufficientBalance:
            return "Insufficient balance to create asset lock"
        case .assetLockGenerationFailed:
            return "Failed to generate asset lock transaction"
        case .instantLockTimeout:
            return "Timed out waiting for InstantLock confirmation"
        case .broadcastFailed(let reason):
            return "Failed to broadcast transaction: \(reason)"
        }
    }
}