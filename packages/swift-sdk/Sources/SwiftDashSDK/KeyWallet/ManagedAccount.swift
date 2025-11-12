import Foundation
import DashSDKFFI

/// Swift wrapper for a managed account with address pool management
public class ManagedAccount {
    internal let handle: UnsafeMutablePointer<FFIManagedAccount>
    private let manager: WalletManager
    
    internal init(handle: UnsafeMutablePointer<FFIManagedAccount>, manager: WalletManager) {
        self.handle = handle
        self.manager = manager
    }
    
    deinit {
        managed_account_free(handle)
    }
    
    // MARK: - Properties
    
    /// Get the network this account is on
    public var network: KeyWalletNetwork {
        let ffiNetwork = managed_account_get_network(handle)
        return KeyWalletNetwork(ffiNetwork: ffiNetwork)
    }
    
    /// Get the account type
    public var accountType: AccountType? {
        var index: UInt32 = 0
        let ffiType = managed_account_get_account_type(handle, &index)
        return AccountType(ffiType: ffiType)
    }
    
    /// Check if this is a watch-only account
    public var isWatchOnly: Bool {
        return managed_account_get_is_watch_only(handle)
    }
    
    /// Get the account index
    public var index: UInt32 {
        return managed_account_get_index(handle)
    }
    
    /// Get the transaction count
    public var transactionCount: UInt32 {
        return managed_account_get_transaction_count(handle)
    }
    
    /// Get the UTXO count
    public var utxoCount: UInt32 {
        return managed_account_get_utxo_count(handle)
    }

    // MARK: - Transactions

    /// Get all transactions for this account
    /// - Parameter currentHeight: Current blockchain height for calculating confirmations
    /// - Returns: Array of transactions
    public func getTransactions(currentHeight: UInt32 = 0) throws -> [WalletTransaction] {
        var transactionsPtr: UnsafeMutablePointer<FFITransactionRecord>?
        var count: size_t = 0

        let success = managed_account_get_transactions(handle, &transactionsPtr, &count)

        guard success else {
            throw KeyWalletError.invalidState("Failed to get transactions from managed account")
        }

        // Handle empty case
        guard count > 0, let ptr = transactionsPtr else {
            return []
        }

        defer {
            managed_account_free_transactions(transactionsPtr, count)
        }

        // Convert FFI transactions to Swift transactions
        var transactions: [WalletTransaction] = []
        transactions.reserveCapacity(count)

        for i in 0..<count {
            let ffiTx = ptr.advanced(by: i).pointee

            // Calculate confirmations
            let confirmations: Int
            if ffiTx.height > 0 && currentHeight >= ffiTx.height {
                confirmations = Int(currentHeight - ffiTx.height) + 1
            } else {
                confirmations = 0
            }

            // Determine transaction type
            let type: String
            if ffiTx.net_amount > 0 {
                type = "received"
            } else if ffiTx.net_amount < 0 {
                type = "sent"
            } else {
                type = "self"
            }

            // Convert txid to hex string
            let txidHex = withUnsafeBytes(of: ffiTx.txid) { buffer in
                buffer.map { String(format: "%02x", $0) }.joined()
            }

            // Convert block hash if present
            let blockHashHex: String?
            if ffiTx.height > 0 {
                blockHashHex = withUnsafeBytes(of: ffiTx.block_hash) { buffer in
                    buffer.map { String(format: "%02x", $0) }.joined()
                }
            } else {
                blockHashHex = nil
            }

            let transaction = WalletTransaction(
                txid: txidHex,
                netAmount: ffiTx.net_amount,
                height: ffiTx.height > 0 ? ffiTx.height : nil,
                blockHash: blockHashHex,
                timestamp: ffiTx.timestamp,
                fee: ffiTx.fee > 0 ? ffiTx.fee : nil,
                confirmations: confirmations,
                type: type,
                isOurs: ffiTx.is_ours
            )

            transactions.append(transaction)
        }

        return transactions
    }

    // MARK: - Balance
    
    /// Get the balance for this account
    public func getBalance() throws -> Balance {
        var ffiBalance = FFIBalance()
        let success = managed_account_get_balance(handle, &ffiBalance)
        
        guard success else {
            throw KeyWalletError.invalidState("Failed to get balance for managed account")
        }
        
        return Balance(ffiBalance: ffiBalance)
    }
    
    // MARK: - Address Pools
    
    /// Get the external address pool
    public func getExternalAddressPool() -> AddressPool? {
        guard let poolHandle = managed_account_get_external_address_pool(handle) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }
    
    /// Get the internal address pool
    public func getInternalAddressPool() -> AddressPool? {
        guard let poolHandle = managed_account_get_internal_address_pool(handle) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }
    
    /// Get an address pool by type
    /// - Parameter poolType: The type of address pool to get
    /// - Returns: The address pool if it exists
    public func getAddressPool(type poolType: AddressPoolType) -> AddressPool? {
        guard let poolHandle = managed_account_get_address_pool(handle, poolType.ffiValue) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }
}

// MARK: - Wallet Transaction

/// Information about a transaction from a managed account
public struct WalletTransaction: Identifiable {
    /// Transaction ID (hex string)
    public let txid: String
    /// Net amount for the account (positive = received, negative = sent)
    public let netAmount: Int64
    /// Block height if confirmed
    public let height: UInt32?
    /// Block hash if confirmed (hex string)
    public let blockHash: String?
    /// Unix timestamp
    public let timestamp: UInt64
    /// Fee if known
    public let fee: UInt64?
    /// Number of confirmations
    public let confirmations: Int
    /// Transaction type: "received", "sent", or "self"
    public let type: String
    /// Whether this is our transaction
    public let isOurs: Bool

    public init(
        txid: String,
        netAmount: Int64,
        height: UInt32?,
        blockHash: String?,
        timestamp: UInt64,
        fee: UInt64?,
        confirmations: Int,
        type: String,
        isOurs: Bool
    ) {
        self.txid = txid
        self.netAmount = netAmount
        self.height = height
        self.blockHash = blockHash
        self.timestamp = timestamp
        self.fee = fee
        self.confirmations = confirmations
        self.type = type
        self.isOurs = isOurs
    }

    /// Transaction date
    public var date: Date {
        return Date(timeIntervalSince1970: TimeInterval(timestamp))
    }

    /// Is the transaction confirmed
    public var isConfirmed: Bool {
        return confirmations > 0
    }

    /// Formatted amount string
    public var formattedAmount: String {
        let dash = Double(abs(netAmount)) / 100_000_000.0
        let sign = netAmount < 0 ? "-" : "+"
        return "\(sign)\(String(format: "%.8f", dash)) DASH"
    }

    /// Formatted fee string
    public var formattedFee: String? {
        guard let fee = fee else { return nil }
        let dash = Double(fee) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }

    /// Truncated txid for display (first 8 + last 6 chars)
    public var truncatedTxid: String {
        guard txid.count > 14 else { return txid }
        let start = txid.prefix(8)
        let end = txid.suffix(6)
        return "\(start)...\(end)"
    }

    // Identifiable conformance
    public var id: String { txid }
}
