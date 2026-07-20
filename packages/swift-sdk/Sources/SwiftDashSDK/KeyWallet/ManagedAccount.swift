import Foundation
import DashSDKFFI

/// Swift wrapper for a managed account with address pool management
public class ManagedAccount {
    internal let handle: OpaquePointer
    private let manager: WalletManager

    internal init(handle: OpaquePointer, manager: WalletManager) {
        self.handle = handle
        self.manager = manager
    }

    deinit {
        managed_core_account_free(handle)
    }

    // MARK: - Properties

    /// Get the network this account is on
    public var network: Network {
        let ffiNetwork = managed_core_account_get_network(handle)
        return Network(ffiNetwork: ffiNetwork)
    }

    /// Get the account type
    public var accountType: AccountType? {
        var index: UInt32 = 0
        let ffiType = managed_core_account_get_account_type(handle, &index)
        return AccountType(ffiType: ffiType)
    }

    // `isWatchOnly` was removed in lockstep with upstream dropping the
    // per-core-account flag (it's now a wallet-level property on
    // `WalletType::WatchOnly`). The corresponding C getter
    // `managed_core_account_get_is_watch_only` is gone too. Query
    // watch-only from the parent wallet handle if needed.

    /// Get the account index
    public var index: UInt32 {
        return managed_core_account_get_index(handle)
    }

    /// Get the UTXO count
    public var utxoCount: UInt32 {
        return managed_core_account_get_utxo_count(handle)
    }

    // `transactionCount` and `getTransactions()` accessors were removed
    // in lockstep with upstream gating
    // `managed_core_account_get_transaction_count` /
    // `managed_core_account_get_transactions` behind the
    // `keep-finalized-transactions` Cargo feature (off by default).
    // The production model delivers tx history through the
    // platform-wallet event channel, not from the in-memory
    // per-account map. Consumers that need history should subscribe
    // to wallet events on the Rust side.

    // MARK: - Balance

    /// Get the balance for this account
    public func getBalance() -> Balance {
        var ffiBalance = FFIBalance()
        let success = managed_core_account_get_balance(handle, &ffiBalance)

        guard success else {
            // managed_core_account_get_balance can only fail if handle or ffiBalance are null
            preconditionFailure("managed_core_account_get_balance failed, invalid state")
        }

        return Balance(ffiBalance: ffiBalance)
    }

    // MARK: - Address Pools

    /// Get the external address pool
    public func getExternalAddressPool() -> AddressPool? {
        guard let poolHandle = managed_core_account_get_external_address_pool(handle) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }

    /// Get the internal address pool
    public func getInternalAddressPool() -> AddressPool? {
        guard let poolHandle = managed_core_account_get_internal_address_pool(handle) else {
            return nil
        }
        return AddressPool(handle: poolHandle)
    }

    /// Get an address pool by type
    /// - Parameter poolType: The type of address pool to get
    /// - Returns: The address pool if it exists
    public func getAddressPool(type poolType: AddressPoolType) -> AddressPool? {
        guard let poolHandle = managed_core_account_get_address_pool(handle, poolType.ffiValue) else {
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
    /// Block height, 0 if not confirmed
    public let height: UInt32
    /// Block hash if confirmed (hex string)
    public let blockHash: String?
    /// Unix timestamp
    public let timestamp: UInt64
    /// Fee if known
    public let fee: UInt64?
    /// Whether this is our transaction
    public let isOurs: Bool

    public init(
        txid: String,
        netAmount: Int64,
        height: UInt32,
        blockHash: String?,
        timestamp: UInt64,
        fee: UInt64?,
        isOurs: Bool
    ) {
        self.txid = txid
        self.netAmount = netAmount
        self.height = height
        self.blockHash = blockHash
        self.timestamp = timestamp
        self.fee = fee
        self.isOurs = isOurs
    }

    /// Transaction date
    public var date: Date {
        return Date(timeIntervalSince1970: TimeInterval(timestamp))
    }

    /// Is the transaction confirmed
    public var isConfirmed: Bool {
        return blockHash != nil && height > 0
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
