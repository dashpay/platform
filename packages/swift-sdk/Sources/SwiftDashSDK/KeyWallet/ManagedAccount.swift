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
