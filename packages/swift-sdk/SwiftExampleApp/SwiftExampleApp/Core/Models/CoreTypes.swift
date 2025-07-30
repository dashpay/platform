import Foundation

// Core SDK Types
// Note: These are now defined in their respective files:
// - DashNetwork is defined in WalletFFIBridge.swift
// - SPVClient is defined in SPVClient.swift
public typealias WalletFFI = Any

// TransactionType is now defined in HDTransaction.swift

// AddressType is now defined in HDWallet.swift

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

// WalletError is now defined in WalletManager.swift

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