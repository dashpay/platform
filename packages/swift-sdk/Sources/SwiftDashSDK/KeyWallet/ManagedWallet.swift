import Foundation
import DashSDKFFI

/// Swift wrapper for managed wallet with address pool management and transaction checking
public class ManagedWallet {
    private let handle: UnsafeMutablePointer<FFIManagedWallet>
    private let network: KeyWalletNetwork
    
    /// Create a managed wallet wrapper from a regular wallet
    /// - Parameter wallet: The wallet to manage
    public init(wallet: Wallet) throws {
        self.network = wallet.network
        
        var error = FFIError()
        guard let managedPointer = wallet_create_managed_wallet(wallet.ffiHandle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }
        
        self.handle = managedPointer
    }
    
    deinit {
        ffi_managed_wallet_free(handle)
    }
    
    // MARK: - Address Generation
    
    /// Get the next unused receive address for a BIP44 account
    /// - Parameters:
    ///   - wallet: The wallet for key derivation
    ///   - accountIndex: The account index
    /// - Returns: The next receive address
    public func getNextReceiveAddress(wallet: Wallet, accountIndex: UInt32 = 0) throws -> String {
        var error = FFIError()
        
        guard let infoHandle = getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet info")
        }
        
        let addressPtr = managed_wallet_get_next_bip44_receive_address(
            infoHandle, wallet.ffiHandle, network.ffiValue, accountIndex, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard let ptr = addressPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        let address = String(cString: ptr)
        address_free(ptr)
        
        return address
    }
    
    /// Get the next unused change address for a BIP44 account
    /// - Parameters:
    ///   - wallet: The wallet for key derivation
    ///   - accountIndex: The account index
    /// - Returns: The next change address
    public func getNextChangeAddress(wallet: Wallet, accountIndex: UInt32 = 0) throws -> String {
        var error = FFIError()
        
        guard let infoHandle = getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet info")
        }
        
        let addressPtr = managed_wallet_get_next_bip44_change_address(
            infoHandle, wallet.ffiHandle, network.ffiValue, accountIndex, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard let ptr = addressPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        let address = String(cString: ptr)
        address_free(ptr)
        
        return address
    }
    
    /// Get a range of external (receive) addresses
    /// - Parameters:
    ///   - wallet: The wallet for key derivation
    ///   - accountIndex: The account index
    ///   - startIndex: Starting index (inclusive)
    ///   - endIndex: Ending index (exclusive)
    /// - Returns: Array of addresses
    public func getExternalAddressRange(wallet: Wallet, accountIndex: UInt32 = 0,
                                       startIndex: UInt32, endIndex: UInt32) throws -> [String] {
        guard endIndex > startIndex else {
            throw KeyWalletError.invalidInput("End index must be greater than start index")
        }
        
        var error = FFIError()
        var addressesPtr: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
        var count: size_t = 0
        
        guard let infoHandle = getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet info")
        }
        
        let success = managed_wallet_get_bip_44_external_address_range(
            infoHandle, wallet.ffiHandle, network.ffiValue, accountIndex,
            startIndex, endIndex, &addressesPtr, &count, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = addressesPtr {
                address_array_free(ptr, count)
            }
        }
        
        guard success, let ptr = addressesPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        var addresses: [String] = []
        for i in 0..<count {
            if let addressCStr = ptr[i] {
                addresses.append(String(cString: addressCStr))
            }
        }
        
        return addresses
    }
    
    /// Get a range of internal (change) addresses
    /// - Parameters:
    ///   - wallet: The wallet for key derivation
    ///   - accountIndex: The account index
    ///   - startIndex: Starting index (inclusive)
    ///   - endIndex: Ending index (exclusive)
    /// - Returns: Array of addresses
    public func getInternalAddressRange(wallet: Wallet, accountIndex: UInt32 = 0,
                                       startIndex: UInt32, endIndex: UInt32) throws -> [String] {
        guard endIndex > startIndex else {
            throw KeyWalletError.invalidInput("End index must be greater than start index")
        }
        
        var error = FFIError()
        var addressesPtr: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
        var count: size_t = 0
        
        guard let infoHandle = getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet info")
        }
        
        let success = managed_wallet_get_bip_44_internal_address_range(
            infoHandle, wallet.ffiHandle, network.ffiValue, accountIndex,
            startIndex, endIndex, &addressesPtr, &count, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = addressesPtr {
                address_array_free(ptr, count)
            }
        }
        
        guard success, let ptr = addressesPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        var addresses: [String] = []
        for i in 0..<count {
            if let addressCStr = ptr[i] {
                addresses.append(String(cString: addressCStr))
            }
        }
        
        return addresses
    }
    
    // MARK: - Address Pool Management
    
    /// Get address pool information
    /// - Parameters:
    ///   - accountType: The account type
    ///   - accountIndex: The account index
    ///   - poolType: The address pool type
    /// - Returns: Address pool information
    public func getAddressPoolInfo(accountType: AccountType, accountIndex: UInt32 = 0,
                                  poolType: AddressPoolType) throws -> AddressPoolInfo {
        var error = FFIError()
        var ffiInfo = FFIAddressPoolInfo()
        
        let success = managed_wallet_get_address_pool_info(
            handle, network.ffiValue, accountType.ffiValue, accountIndex,
            poolType.ffiValue, &ffiInfo, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return AddressPoolInfo(ffiInfo: ffiInfo)
    }
    
    /// Set the gap limit for an address pool
    /// - Parameters:
    ///   - accountType: The account type
    ///   - accountIndex: The account index
    ///   - poolType: The address pool type
    ///   - gapLimit: The new gap limit
    public func setGapLimit(accountType: AccountType, accountIndex: UInt32 = 0,
                           poolType: AddressPoolType, gapLimit: UInt32) throws {
        var error = FFIError()
        
        let success = managed_wallet_set_gap_limit(
            handle, network.ffiValue, accountType.ffiValue, accountIndex,
            poolType.ffiValue, gapLimit, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
    }
    
    /// Generate addresses up to a specific index
    /// - Parameters:
    ///   - wallet: The wallet for key derivation
    ///   - accountType: The account type
    ///   - accountIndex: The account index
    ///   - poolType: The address pool type
    ///   - targetIndex: The target index to generate up to
    public func generateAddressesToIndex(wallet: Wallet, accountType: AccountType,
                                        accountIndex: UInt32 = 0,
                                        poolType: AddressPoolType,
                                        targetIndex: UInt32) throws {
        var error = FFIError()
        
        let success = managed_wallet_generate_addresses_to_index(
            handle, wallet.ffiHandle, network.ffiValue, accountType.ffiValue,
            accountIndex, poolType.ffiValue, targetIndex, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
    }
    
    /// Mark an address as used
    /// - Parameter address: The address to mark as used
    public func markAddressUsed(_ address: String) throws {
        var error = FFIError()
        
        let success = address.withCString { addressCStr in
            managed_wallet_mark_address_used(handle, network.ffiValue, addressCStr, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
    }
    
    // MARK: - Transaction Checking
    
    /// Check if a transaction belongs to the wallet
    /// - Parameters:
    ///   - wallet: The wallet to check against
    ///   - transactionData: The transaction bytes
    ///   - context: The transaction context
    ///   - blockHeight: The block height (0 for mempool)
    ///   - blockHash: The block hash (nil for mempool)
    ///   - timestamp: The timestamp
    ///   - updateState: Whether to update wallet state if transaction is relevant
    /// - Returns: Transaction check result
    public func checkTransaction(wallet: Wallet, transactionData: Data,
                                context: TransactionContext = .mempool,
                                blockHeight: UInt32 = 0,
                                blockHash: Data? = nil,
                                timestamp: UInt32 = 0,
                                updateState: Bool = true) throws -> TransactionCheckResult {
        var error = FFIError()
        var result = FFITransactionCheckResult()
        
        let success = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress
            
            if let hash = blockHash {
                return hash.withUnsafeBytes { hashBytes in
                    let hashPtr = hashBytes.bindMemory(to: UInt8.self).baseAddress
                    
                    return managed_wallet_check_transaction(
                        handle, wallet.ffiHandle, network.ffiValue,
                        txPtr, transactionData.count,
                        context.ffiValue, blockHeight, hashPtr,
                        UInt64(timestamp), updateState, &result, &error)
                }
            } else {
                return managed_wallet_check_transaction(
                    handle, wallet.ffiHandle, network.ffiValue,
                    txPtr, transactionData.count,
                    context.ffiValue, blockHeight, nil,
                    UInt64(timestamp), updateState, &result, &error)
            }
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            transaction_check_result_free(&result)
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return TransactionCheckResult(ffiResult: result)
    }
    
    // MARK: - Balance and UTXOs
    
    /// Get the wallet balance from managed wallet info
    public func getBalance() throws -> Balance {
        guard let infoHandle = getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet info")
        }
        
        var error = FFIError()
        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var locked: UInt64 = 0
        var total: UInt64 = 0
        
        let success = managed_wallet_get_balance(
            infoHandle, &confirmed, &unconfirmed, &locked, &total, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        let ffiBalance = FFIBalance(
            confirmed: confirmed,
            unconfirmed: unconfirmed,
            immature: locked,  // Using locked as immature
            total: total
        )
        
        return Balance(ffiBalance: ffiBalance)
    }
    
    /// Get all UTXOs from the managed wallet
    public func getUTXOs() throws -> [UTXO] {
        guard let infoHandle = getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet info")
        }
        
        var error = FFIError()
        var utxosPtr: UnsafeMutablePointer<FFIUTXO>?
        var count: size_t = 0
        
        let success = managed_wallet_get_utxos(
            infoHandle, network.ffiValue, &utxosPtr, &count, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = utxosPtr {
                utxo_array_free(ptr, count)
            }
        }
        
        guard success, let ptr = utxosPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        var utxos: [UTXO] = []
        for i in 0..<count {
            utxos.append(UTXO(ffiUTXO: ptr[i]))
        }
        
        return utxos
    }
    
    // MARK: - Private Helpers
    
    private func getInfoHandle() -> OpaquePointer? {
        // The handle is an FFIManagedWallet*, which contains an FFIManagedWalletInfo* as inner
        // We treat it as opaque in Swift
        return OpaquePointer(handle)
    }
}

