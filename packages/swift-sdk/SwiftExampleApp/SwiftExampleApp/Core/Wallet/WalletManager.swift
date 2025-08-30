import Foundation
import SwiftData
import Combine
import DashSDKFFI

// MARK: - Wallet Manager

/// WalletManager is a wrapper around the FFI wallet manager from rust-dashcore
/// It delegates all wallet operations to the FFI layer while maintaining
/// SwiftUI compatibility through ObservableObject
@MainActor
public class WalletManager: ObservableObject {
    @Published public private(set) var wallets: [HDWallet] = []
    @Published public private(set) var currentWallet: HDWallet?
    @Published public private(set) var isLoading = false
    @Published public private(set) var error: WalletError?
    
    // FFI wallet manager handle - this is the real wallet manager from Rust
    private let ffiWalletManager: OpaquePointer
    private let modelContainer: ModelContainer
    private let storage = WalletStorage()
    
    // Services
    public private(set) var utxoManager: UTXOManager!
    public private(set) var transactionService: TransactionService!
    
    /// Initialize with an FFI wallet manager from SPV client
    /// - Parameters:
    ///   - ffiWalletManager: The FFI wallet manager handle from rust-dashcore
    ///   - modelContainer: SwiftData model container for persistence
    public init(ffiWalletManager: OpaquePointer, modelContainer: ModelContainer? = nil) throws {
        print("=== WalletManager.init START ===")
        
        self.ffiWalletManager = ffiWalletManager
        
        if let container = modelContainer {
            print("Using provided ModelContainer")
            self.modelContainer = container
        } else {
            do {
                print("Creating ModelContainer...")
                self.modelContainer = try ModelContainer(for: HDWallet.self, HDAccount.self, HDAddress.self, HDUTXO.self, HDTransaction.self)
                print("✅ ModelContainer created")
            } catch {
                print("❌ Failed to create ModelContainer: \(error)")
                throw error
            }
        }
        
        // Initialize services
        print("Creating UTXOManager...")
        self.utxoManager = UTXOManager(walletManager: self, modelContainer: self.modelContainer)
        
        print("Creating TransactionService...")
        self.transactionService = TransactionService(
            walletManager: self,
            utxoManager: utxoManager,
            modelContainer: self.modelContainer
        )
        
        print("=== WalletManager.init SUCCESS ===")
        
        Task {
            await loadWallets()
        }
    }
    
    // MARK: - Wallet Management
    
    public func createWallet(label: String, network: DashNetwork, mnemonic: String? = nil, pin: String) async throws -> HDWallet {
        print("WalletManager.createWallet called")
        isLoading = true
        defer { isLoading = false }
        
        // Generate or validate mnemonic
        let finalMnemonic: String
        if let mnemonic = mnemonic {
            print("Validating provided mnemonic...")
            guard WalletFFIBridge.shared.validateMnemonic(mnemonic) else {
                print("Mnemonic validation failed")
                throw WalletError.invalidMnemonic
            }
            finalMnemonic = mnemonic
        } else {
            print("Generating new mnemonic...")
            guard let generated = WalletFFIBridge.shared.generateMnemonic() else {
                throw WalletError.seedGenerationFailed
            }
            finalMnemonic = generated
            print("Generated mnemonic: \(finalMnemonic)")
        }
        
        // Add wallet through FFI
        var error = FFIError()
        var walletBytesPtr: UnsafeMutablePointer<UInt8>?
        var walletBytesLen: size_t = 0
        var walletId = [UInt8](repeating: 0, count: 32)
        
        let ffiNetwork = network == .testnet ? FFINetworks(1) : FFINetworks(0)
        var options = FFIWalletAccountCreationOptions()
        options.option_type = FFIAccountCreationOptionType(0) // Default type
        options.bip44_indices = nil
        options.bip44_count = 0
        options.bip32_indices = nil
        options.bip32_count = 0
        options.coinjoin_indices = nil
        options.coinjoin_count = 0
        options.topup_indices = nil
        options.topup_count = 0
        options.special_account_types = nil
        options.special_account_types_count = 0
        
        let success = finalMnemonic.withCString { mnemonicCStr in
            wallet_manager_add_wallet_from_mnemonic_return_serialized_bytes(
                ffiWalletManager,
                mnemonicCStr,
                nil, // No passphrase
                ffiNetwork,
                0, // Birth height
                &options,
                false, // Don't downgrade to public key wallet
                false, // Don't allow external signing
                &walletBytesPtr,
                &walletBytesLen,
                &walletId,
                &error
            )
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = walletBytesPtr {
                wallet_manager_free_wallet_bytes(ptr, walletBytesLen)
            }
        }
        
        guard success else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
            throw WalletError.walletError(errorMessage)
        }
        
        // Create HDWallet model for SwiftUI
        let wallet = HDWallet(label: label, network: network)
        wallet.walletId = Data(walletId)
        
        // Store the serialized wallet bytes for persistence
        if let ptr = walletBytesPtr, walletBytesLen > 0 {
            wallet.serializedWalletBytes = Data(bytes: ptr, count: walletBytesLen)
        }
        
        // Store encrypted seed (if needed for UI purposes)
        if let seed = WalletFFIBridge.shared.mnemonicToSeed(finalMnemonic) {
            let encryptedSeed = try storage.storeSeed(seed, pin: pin)
            wallet.encryptedSeed = encryptedSeed
        }
        
        // Insert wallet into context
        modelContainer.mainContext.insert(wallet)
        
        // Create default account model
        let account = wallet.createAccount(at: 0)
        
        // Sync complete wallet state from Rust managed info
        try await syncWalletFromManagedInfo(for: wallet)
        
        // Save to database
        try modelContainer.mainContext.save()
        
        await loadWallets()
        currentWallet = wallet
        
        return wallet
    }
    
    public func importWallet(label: String, network: DashNetwork, mnemonic: String, pin: String) async throws -> HDWallet {
        return try await createWallet(label: label, network: network, mnemonic: mnemonic, pin: pin)
    }
    
    /// Restore a wallet from serialized bytes
    /// This is used to restore wallets from persistence on app startup
    public func restoreWalletFromBytes(_ walletBytes: Data) throws -> Data {
        var error = FFIError()
        var walletId = [UInt8](repeating: 0, count: 32)
        
        let success = walletBytes.withUnsafeBytes { bytes in
            let ptr = bytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_import_wallet_from_bytes(
                ffiWalletManager,
                ptr,
                walletBytes.count,
                &walletId,
                &error
            )
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
            throw WalletError.walletError("Failed to restore wallet: \(errorMessage)")
        }
        
        return Data(walletId)
    }
    
    /// Sync wallet data from FFI managed wallet info to Swift models
    /// This function retrieves the complete wallet state from Rust and updates our UI models
    private func syncWalletFromManagedInfo(for wallet: HDWallet) async throws {
        guard let walletId = wallet.walletId else {
            throw WalletError.walletError("Wallet ID not available")
        }
        
        var error = FFIError()
        
        // Get the complete managed wallet info from Rust
        let managedInfoPtr = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_managed_wallet_info(
                ffiWalletManager,
                idPtr,
                &error
            )
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if managedInfoPtr != nil {
                managed_wallet_info_free(managedInfoPtr)
            }
        }
        
        guard managedInfoPtr != nil else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Failed to get managed wallet info"
            throw WalletError.walletError(errorMessage)
        }
        
        // Update balance from managed info
        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var locked: UInt64 = 0
        var total: UInt64 = 0
        
        let balanceSuccess = managed_wallet_get_balance(
            managedInfoPtr,
            &confirmed,
            &unconfirmed,
            &locked,
            &total,
            &error
        )
        
        if balanceSuccess {
            // Update wallet-level balance (this will propagate to accounts)
            // For now, we'll update the first account's balance
            if let firstAccount = wallet.accounts.first {
                firstAccount.confirmedBalance = confirmed
                firstAccount.unconfirmedBalance = unconfirmed
            }
        }
        
        // Sync addresses for each account
        if let managedInfo = managedInfoPtr {
            for account in wallet.accounts {
                try await syncAccountAddressesFromManagedInfo(
                    managedInfo: managedInfo,
                    wallet: wallet,
                    account: account
                )
            }
        }
    }
    
    /// Sync addresses for a specific account from managed wallet info
    private func syncAccountAddressesFromManagedInfo(
        managedInfo: OpaquePointer,
        wallet: HDWallet,
        account: HDAccount
    ) async throws {
        let ffiNetwork = wallet.dashNetwork == .testnet ? FFINetworks(1) : FFINetworks(0)
        var error = FFIError()
        
        // Get external addresses from managed info
        var externalAddressesPtr: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
        var externalCount: size_t = 0
        
        let externalSuccess = managed_wallet_get_bip_44_external_address_range(
            managedInfo,
            nil, // We don't need the wallet ptr for reading
            ffiNetwork,
            account.accountNumber,
            0,   // Start index
            20,  // Get up to 20 addresses
            &externalAddressesPtr,
            &externalCount,
            &error
        )
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            // Free the addresses array
            if let ptr = externalAddressesPtr, externalCount > 0 {
                for i in 0..<externalCount {
                    if let addressPtr = ptr[i] {
                        address_free(addressPtr)
                    }
                }
                // Free the array itself
                ptr.deallocate()
            }
        }
        
        if externalSuccess, let addressesPtr = externalAddressesPtr {
            // Clear existing addresses and add the ones from Rust
            account.externalAddresses.removeAll()
            
            for i in 0..<externalCount {
                if let addressPtr = addressesPtr[i] {
                    let address = String(cString: addressPtr)
                    let index = UInt32(i)
                    let path = "m/44'/5'/\(account.accountNumber)'/0/\(index)"
                    
                    let hdAddress = HDAddress(
                        address: address,
                        index: index,
                        derivationPath: path,
                        addressType: .external,
                        account: account
                    )
                    
                    modelContainer.mainContext.insert(hdAddress)
                    account.externalAddresses.append(hdAddress)
                }
            }
            
            account.externalAddressIndex = UInt32(externalCount)
        }
        
        // Get internal addresses from managed info
        var internalAddressesPtr: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
        var internalCount: size_t = 0
        
        let internalSuccess = managed_wallet_get_bip_44_internal_address_range(
            managedInfo,
            nil, // We don't need the wallet ptr for reading
            ffiNetwork,
            account.accountNumber,
            0,   // Start index
            10,  // Get up to 10 change addresses
            &internalAddressesPtr,
            &internalCount,
            &error
        )
        
        defer {
            // Free the internal addresses array
            if let ptr = internalAddressesPtr, internalCount > 0 {
                for i in 0..<internalCount {
                    if let addressPtr = ptr[i] {
                        address_free(addressPtr)
                    }
                }
                ptr.deallocate()
            }
        }
        
        if internalSuccess, let addressesPtr = internalAddressesPtr {
            // Clear existing addresses and add the ones from Rust
            account.internalAddresses.removeAll()
            
            for i in 0..<internalCount {
                if let addressPtr = addressesPtr[i] {
                    let address = String(cString: addressPtr)
                    let index = UInt32(i)
                    let path = "m/44'/5'/\(account.accountNumber)'/1/\(index)"
                    
                    let hdAddress = HDAddress(
                        address: address,
                        index: index,
                        derivationPath: path,
                        addressType: .internal,
                        account: account
                    )
                    
                    modelContainer.mainContext.insert(hdAddress)
                    account.internalAddresses.append(hdAddress)
                }
            }
            
            account.internalAddressIndex = UInt32(internalCount)
        }
    }
    
    public func unlockWallet(with pin: String) async throws -> Data {
        return try storage.retrieveSeed(pin: pin)
    }
    
    public func decryptSeed(_ encryptedSeed: Data?) -> Data? {
        // This method is used internally by other services
        // In a real implementation, this would decrypt using the current PIN
        // For now, return nil to indicate manual unlock is needed
        return nil
    }
    
    /// Get wallet IDs from FFI
    public func getWalletIds() throws -> [Data] {
        var error = FFIError()
        var walletIdsPtr: UnsafeMutablePointer<UInt8>?
        var count: size_t = 0
        
        let success = wallet_manager_get_wallet_ids(ffiWalletManager, &walletIdsPtr, &count, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = walletIdsPtr {
                wallet_manager_free_wallet_ids(ptr, count)
            }
        }
        
        guard success, let ptr = walletIdsPtr else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
            throw WalletError.walletError(errorMessage)
        }
        
        var walletIds: [Data] = []
        for i in 0..<count {
            let offset = i * 32
            let idData = Data(bytes: ptr.advanced(by: offset), count: 32)
            walletIds.append(idData)
        }
        
        return walletIds
    }
    
    /// Get wallet balance from FFI
    public func getWalletBalance(walletId: Data) throws -> (confirmed: UInt64, unconfirmed: UInt64) {
        guard walletId.count == 32 else {
            throw WalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }
        
        var error = FFIError()
        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0
        
        let success = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_wallet_balance(
                ffiWalletManager, idPtr, &confirmed, &unconfirmed, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard success else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
            throw WalletError.walletError(errorMessage)
        }
        
        return (confirmed: confirmed, unconfirmed: unconfirmed)
    }
    
    public func changeWalletPIN(currentPIN: String, newPIN: String) async throws {
        // Retrieve seed with current PIN
        let seed = try storage.retrieveSeed(pin: currentPIN)
        
        // Re-encrypt with new PIN
        _ = try storage.storeSeed(seed, pin: newPIN)
    }
    
    public func enableBiometricProtection(pin: String) async throws {
        // First verify PIN and get seed
        let seed = try storage.retrieveSeed(pin: pin)
        
        // Enable biometric protection
        try storage.enableBiometricProtection(for: seed)
    }
    
    public func unlockWithBiometric() async throws -> Data {
        return try storage.retrieveSeedWithBiometric()
    }
    
    public func createWatchOnlyWallet(label: String, network: DashNetwork, extendedPublicKey: String) async throws -> HDWallet {
        isLoading = true
        defer { isLoading = false }
        
        let wallet = HDWallet(label: label, network: network, isWatchOnly: true)
        
        // Create account with extended public key
        let account = wallet.createAccount(at: 0)
        account.extendedPublicKey = extendedPublicKey
        
        // Generate addresses from extended public key
        try await generateWatchOnlyAddresses(for: account, count: 20, type: .external)
        try await generateWatchOnlyAddresses(for: account, count: 10, type: .internal)
        
        // Save to database
        modelContainer.mainContext.insert(wallet)
        try modelContainer.mainContext.save()
        
        await loadWallets()
        currentWallet = wallet
        
        return wallet
    }
    
    public func deleteWallet(_ wallet: HDWallet) async throws {
        modelContainer.mainContext.delete(wallet)
        try modelContainer.mainContext.save()
        
        if currentWallet?.id == wallet.id {
            currentWallet = wallets.first(where: { $0.id != wallet.id })
        }
        
        await loadWallets()
    }
    
    // MARK: - Account Management
    
    /// Determines if an account type should show balance in the UI
    /// - Parameter accountIndex: The unique account index
    /// - Returns: true if the account should show balance, false otherwise
    public static func shouldShowBalance(for accountIndex: UInt32) -> Bool {
        switch accountIndex {
        case 0...999:        // BIP44 accounts (including main account at 0)
            return true
        case 1000...1999:    // CoinJoin accounts
            return true
        case 5000...5999:    // BIP32 accounts
            return true
        case 9000:           // Identity Registration
            return false
        case 9001:           // Identity Invitation
            return false
        case 9002:           // Identity Topup (Not Bound)
            return false
        case 9100...9199:    // Identity Topup accounts
            return false
        case 10000...10999:  // Provider Voting Keys
            return false
        case 11000:          // Provider Owner Keys
            return false
        case 11001:          // Provider Operator Keys (BLS)
            return false
        case 11002:          // Provider Platform Keys (EdDSA)
            return false
        default:
            return false
        }
    }
    
    /// Get all accounts for a wallet from the FFI wallet manager
    /// Returns account information including balances and address counts
    public func getAccounts(for wallet: HDWallet) async throws -> [AccountInfo] {
        guard let walletId = wallet.walletId else {
            throw WalletError.walletError("Wallet ID not available")
        }
        
        var error = FFIError()
        var accounts: [AccountInfo] = []
        
        // Get network from wallet (respecting app settings)
        let ffiNetwork = wallet.dashNetwork == .testnet ? FFINetworks(1) : FFINetworks(0)
        
        // Get the managed account collection
        let collectionPtr = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return managed_wallet_get_account_collection(
                ffiWalletManager,
                idPtr,
                ffiNetwork,
                &error
            )
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if collectionPtr != nil {
                managed_account_collection_free(collectionPtr)
            }
        }
        
        guard let collection = collectionPtr else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Failed to get account collection"
            throw WalletError.walletError(errorMessage)
        }
        
        // Helper function to get address counts for an account
        func getAddressCounts(account: OpaquePointer) -> (external: Int, internal: Int) {
            var externalCount = 0
            var internalCount = 0
            
            // Get external address pool and count
            let externalPoolPtr = managed_account_get_external_address_pool(account)
            if let externalPool = externalPoolPtr {
                defer { address_pool_free(externalPool) }
                
                // Get addresses in a reasonable range to count them
                var countOut: size_t = 0
                let addressesPtr = address_pool_get_addresses_in_range(
                    externalPool,
                    0,      // start index
                    1000,   // end index (reasonable max)
                    &countOut,
                    &error
                )
                
                if let addresses = addressesPtr {
                    externalCount = Int(countOut)
                    // Free the addresses
                    for i in 0..<countOut {
                        if let addressInfo = addresses[i] {
                            address_info_free(addressInfo)
                        }
                    }
                    // Free the array itself
                    addresses.deallocate()
                }
            }
            
            // Get internal address pool and count
            let internalPoolPtr = managed_account_get_internal_address_pool(account)
            if let internalPool = internalPoolPtr {
                defer { address_pool_free(internalPool) }
                
                // Get addresses in a reasonable range to count them
                var countOut: size_t = 0
                let addressesPtr = address_pool_get_addresses_in_range(
                    internalPool,
                    0,      // start index
                    1000,   // end index (reasonable max)
                    &countOut,
                    &error
                )
                
                if let addresses = addressesPtr {
                    internalCount = Int(countOut)
                    // Free the addresses
                    for i in 0..<countOut {
                        if let addressInfo = addresses[i] {
                            address_info_free(addressInfo)
                        }
                    }
                    // Free the array itself
                    addresses.deallocate()
                }
            }
            
            return (external: externalCount, internal: internalCount)
        }
        
        // Helper function to add account info
        func addAccountInfo(accountPtr: OpaquePointer?, index: UInt32, label: String, uniqueIndex: UInt32) {
            guard let account = accountPtr else { return }
            
            defer {
                managed_account_free(account)
            }
            
            // Get balance
            var balance = FFIBalance()
            let balanceSuccess = managed_account_get_balance(account, &balance)
            
            let confirmed = balanceSuccess ? balance.confirmed : 0
            let unconfirmed = balanceSuccess ? balance.unconfirmed : 0
            
            // Get address counts from address pools
            let addressCounts = getAddressCounts(account: account)
            
            // Get next receive address
            let nextReceiveAddress: String? = nil // We'll need to implement this separately if needed
            
            let accountInfo = AccountInfo(
                index: uniqueIndex,
                label: label,
                balance: (confirmed: confirmed, unconfirmed: unconfirmed),
                addressCount: (external: addressCounts.external, internal: addressCounts.internal),
                nextReceiveAddress: nextReceiveAddress
            )
            accounts.append(accountInfo)
        }
        
        // Get BIP44 accounts
        var bip44Indices: UnsafeMutablePointer<UInt32>?
        var bip44Count: size_t = 0
        
        if managed_account_collection_get_bip44_indices(collection, &bip44Indices, &bip44Count) {
            defer {
                if let indices = bip44Indices {
                    free_u32_array(indices, bip44Count)
                }
            }
            
            if let indices = bip44Indices {
                for i in 0..<bip44Count {
                    let index = indices[i]
                    let account = managed_account_collection_get_bip44_account(collection, index)
                    addAccountInfo(accountPtr: account, index: index, label: "Account \(index)", uniqueIndex: index)
                }
            }
        }
        
        // Get BIP32 accounts
        var bip32Indices: UnsafeMutablePointer<UInt32>?
        var bip32Count: size_t = 0
        
        if managed_account_collection_get_bip32_indices(collection, &bip32Indices, &bip32Count) {
            defer {
                if let indices = bip32Indices {
                    free_u32_array(indices, bip32Count)
                }
            }
            
            if let indices = bip32Indices {
                for i in 0..<bip32Count {
                    let index = indices[i]
                    let account = managed_account_collection_get_bip32_account(collection, index)
                    addAccountInfo(accountPtr: account, index: index, label: "BIP32 Account \(index)", uniqueIndex: 5000 + index)
                }
            }
        }
        
        // Get CoinJoin accounts
        var coinjoinIndices: UnsafeMutablePointer<UInt32>?
        var coinjoinCount: size_t = 0
        
        if managed_account_collection_get_coinjoin_indices(collection, &coinjoinIndices, &coinjoinCount) {
            defer {
                if let indices = coinjoinIndices {
                    free_u32_array(indices, coinjoinCount)
                }
            }
            
            if let indices = coinjoinIndices {
                for i in 0..<coinjoinCount {
                    let index = indices[i]
                    let account = managed_account_collection_get_coinjoin_account(collection, index)
                    addAccountInfo(accountPtr: account, index: index, label: "CoinJoin \(index)", uniqueIndex: 1000 + index)
                }
            }
        }
        
        // Get identity registration account
        if managed_account_collection_has_identity_registration(collection) {
            let account = managed_account_collection_get_identity_registration(collection)
            addAccountInfo(accountPtr: account, index: 0, label: "Identity Registration", uniqueIndex: 9000)
        }
        
        // Get identity invitation account
        if managed_account_collection_has_identity_invitation(collection) {
            let account = managed_account_collection_get_identity_invitation(collection)
            addAccountInfo(accountPtr: account, index: 0, label: "Identity Invitation", uniqueIndex: 9001)
        }
        
        // Get identity topup accounts
        var topupIndices: UnsafeMutablePointer<UInt32>?
        var topupCount: size_t = 0
        
        if managed_account_collection_get_identity_topup_indices(collection, &topupIndices, &topupCount) {
            defer {
                if let indices = topupIndices {
                    free_u32_array(indices, topupCount)
                }
            }
            
            if let indices = topupIndices {
                for i in 0..<topupCount {
                    let index = indices[i]
                    let account = managed_account_collection_get_identity_topup(collection, index)
                    addAccountInfo(accountPtr: account, index: index, label: "Identity Topup \(index)", uniqueIndex: 9100 + index)
                }
            }
        }
        
        // Get identity topup not bound account
        if managed_account_collection_has_identity_topup_not_bound(collection) {
            let account = managed_account_collection_get_identity_topup_not_bound(collection)
            addAccountInfo(accountPtr: account, index: 0, label: "Identity Topup (Not Bound)", uniqueIndex: 9002)
        }
        
        // Get provider voting keys account
        if managed_account_collection_has_provider_voting_keys(collection) {
            let account = managed_account_collection_get_provider_voting_keys(collection)
            addAccountInfo(accountPtr: account, index: 0, label: "Provider Voting Keys", uniqueIndex: 10000)
        }
        
        // Get provider owner keys account
        if managed_account_collection_has_provider_owner_keys(collection) {
            let account = managed_account_collection_get_provider_owner_keys(collection)
            addAccountInfo(accountPtr: account, index: 0, label: "Provider Owner Keys", uniqueIndex: 11000)
        }
        
        // Get provider operator keys account (BLS)
        if managed_account_collection_has_provider_operator_keys(collection) {
            // Note: This returns void* instead of OpaquePointer, need to cast
            if let voidPtr = managed_account_collection_get_provider_operator_keys(collection) {
                let account = OpaquePointer(voidPtr)
                addAccountInfo(accountPtr: account, index: 0, label: "Provider Operator Keys (BLS)", uniqueIndex: 11001)
            }
        }
        
        // Get provider platform keys account (EdDSA)
        if managed_account_collection_has_provider_platform_keys(collection) {
            // Note: This returns void* instead of OpaquePointer, need to cast
            if let voidPtr = managed_account_collection_get_provider_platform_keys(collection) {
                let account = OpaquePointer(voidPtr)
                addAccountInfo(accountPtr: account, index: 0, label: "Provider Platform Keys (EdDSA)", uniqueIndex: 11002)
            }
        }
        
        // Optional: Get and log the account collection summary for debugging
        let summaryPtr = managed_account_collection_summary(collection)
        if let summary = summaryPtr {
            let summaryString = String(cString: summary)
            print("Account Collection Summary:\n\(summaryString)")
            string_free(summary)
        }
        
        // Sort accounts by index
        accounts.sort { $0.index < $1.index }
        
        return accounts
    }
    
    public func createAccount(in wallet: HDWallet) async throws -> HDAccount {
        guard !wallet.isWatchOnly else {
            throw WalletError.watchOnlyWallet
        }
        
        // Note: The FFI wallet manager handles account creation internally
        // We're just creating UI models here to track them
        let accountIndex = UInt32(wallet.accounts.count)
        let account = wallet.createAccount(at: accountIndex)
        
        // Sync complete wallet state from Rust managed info
        try await syncWalletFromManagedInfo(for: wallet)
        
        try modelContainer.mainContext.save()
        
        return account
    }
    
    // MARK: - Address Management
    
    public func generateAddresses(for account: HDAccount, count: Int, type: AddressType) async throws {
        print("WalletManager.generateAddresses called for type: \(type), count: \(count)")
        
        guard let wallet = account.wallet,
              let walletId = wallet.walletId else {
            print("generateAddresses failed: wallet=\(account.wallet != nil)")
            throw WalletError.seedNotAvailable
        }
        
        // Instead of manually generating addresses, we'll request them from the managed wallet
        // and then sync the complete state
        var error = FFIError()
        
        // Get managed wallet info
        let managedInfoPtr = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_managed_wallet_info(
                ffiWalletManager,
                idPtr,
                &error
            )
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if managedInfoPtr != nil {
                managed_wallet_info_free(managedInfoPtr)
            }
        }
        
        guard managedInfoPtr != nil else {
            let errorMessage = error.message != nil ? String(cString: error.message!) : "Failed to get managed wallet info"
            throw WalletError.walletError(errorMessage)
        }
        
        // Generate addresses through the managed wallet
        // This ensures Rust maintains proper state
        let ffiNetwork = wallet.dashNetwork == .testnet ? FFINetworks(1) : FFINetworks(0)
        
        for _ in 0..<count {
            if type == .external {
                // Get next receive address - this will advance the index in Rust
                let addressPtr = managed_wallet_get_next_bip44_receive_address(
                    managedInfoPtr,
                    nil, // We don't need the wallet ptr
                    ffiNetwork,
                    account.accountNumber,
                    &error
                )
                
                if let ptr = addressPtr {
                    address_free(ptr)
                }
            } else if type == .internal {
                // Get next change address - this will advance the index in Rust
                let addressPtr = managed_wallet_get_next_bip44_change_address(
                    managedInfoPtr,
                    nil, // We don't need the wallet ptr
                    ffiNetwork,
                    account.accountNumber,
                    &error
                )
                
                if let ptr = addressPtr {
                    address_free(ptr)
                }
            }
        }
        
        // Now sync the complete state from Rust to update our UI models
        try await syncWalletFromManagedInfo(for: wallet)
    }
    
    private func generateWatchOnlyAddresses(for account: HDAccount, count: Int, type: AddressType) async throws {
        // For watch-only wallets, we need to derive addresses from extended public key
        // This would require implementing public key derivation
        // For now, throw an error as this requires additional cryptographic operations
        throw WalletError.notImplemented("Watch-only address generation")
    }
    
    public func getUnusedAddress(for account: HDAccount, type: AddressType = .external) async throws -> HDAddress {
        let addresses: [HDAddress]
        switch type {
        case .external:
            addresses = account.externalAddresses
        case .internal:
            addresses = account.internalAddresses
        case .coinJoin:
            addresses = account.coinJoinAddresses
        case .identity:
            addresses = account.identityFundingAddresses
        }
        
        // Find first unused address
        if let unusedAddress = addresses.first(where: { !$0.isUsed }) {
            return unusedAddress
        }
        
        // Generate new addresses if all are used
        try await generateAddresses(for: account, count: 10, type: type)
        
        // Return the first newly generated address
        guard let newAddress = addresses.first(where: { !$0.isUsed }) else {
            throw WalletError.addressGenerationFailed
        }
        
        return newAddress
    }
    
    // MARK: - Balance Management
    
    public func updateBalance(for account: HDAccount) async {
        guard let wallet = account.wallet,
              let walletId = wallet.walletId else {
            return
        }
        
        // Get balance from managed wallet info
        do {
            var error = FFIError()
            
            // Get managed wallet info
            let managedInfoPtr = walletId.withUnsafeBytes { idBytes in
                let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
                return wallet_manager_get_managed_wallet_info(
                    ffiWalletManager,
                    idPtr,
                    &error
                )
            }
            
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
                if managedInfoPtr != nil {
                    managed_wallet_info_free(managedInfoPtr)
                }
            }
            
            guard managedInfoPtr != nil else {
                print("Failed to get managed wallet info")
                return
            }
            
            // Get balance from managed info
            var confirmed: UInt64 = 0
            var unconfirmed: UInt64 = 0
            var locked: UInt64 = 0
            var total: UInt64 = 0
            
            let balanceSuccess = managed_wallet_get_balance(
                managedInfoPtr,
                &confirmed,
                &unconfirmed,
                &locked,
                &total,
                &error
            )
            
            if balanceSuccess {
                account.confirmedBalance = confirmed
                account.unconfirmedBalance = unconfirmed
                
                // Note: wallet.confirmedBalance and wallet.unconfirmedBalance are computed properties
                // They automatically calculate from the sum of all accounts, so we don't need to set them
                
                try? modelContainer.mainContext.save()
            } else {
                print("Failed to get balance from managed info")
            }
        } catch {
            print("Failed to update balance: \(error)")
        }
    }
    
    // MARK: - Private Methods
    
    private func loadWallets() async {
        do {
            let descriptor = FetchDescriptor<HDWallet>(sortBy: [SortDescriptor(\.createdAt)])
            wallets = try modelContainer.mainContext.fetch(descriptor)
            
            // Restore each wallet to the FFI wallet manager
            for wallet in wallets {
                if let walletBytes = wallet.serializedWalletBytes {
                    do {
                        // Restore wallet to FFI and update the wallet ID
                        let restoredWalletId = try restoreWalletFromBytes(walletBytes)
                        
                        // Update wallet ID if it changed (shouldn't happen, but good to verify)
                        if wallet.walletId != restoredWalletId {
                            print("Warning: Wallet ID changed during restoration. Old: \(wallet.walletId?.hexString ?? "nil"), New: \(restoredWalletId.hexString)")
                            wallet.walletId = restoredWalletId
                        }
                        
                        print("Successfully restored wallet '\(wallet.label)' to FFI wallet manager")
                    } catch {
                        print("Failed to restore wallet '\(wallet.label)': \(error)")
                        // Continue loading other wallets even if one fails
                    }
                } else {
                    print("Warning: Wallet '\(wallet.label)' has no serialized bytes - cannot restore to FFI")
                }
            }
            
            if currentWallet == nil, let firstWallet = wallets.first {
                currentWallet = firstWallet
            }
            
            // Save any wallet ID updates
            try? modelContainer.mainContext.save()
        } catch {
            self.error = WalletError.databaseError(error.localizedDescription)
        }
    }
}


// MARK: - Keychain Wrapper

private class KeychainWrapper {
    func set(_ data: Data, forKey key: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecValueData as String: data
        ]
        
        SecItemDelete(query as CFDictionary)
        SecItemAdd(query as CFDictionary, nil)
    }
    
    func data(forKey key: String) -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status == errSecSuccess else { return nil }
        return result as? Data
    }
}

// MARK: - Wallet Errors

public enum WalletError: LocalizedError {
    case invalidMnemonic
    case seedGenerationFailed
    case seedNotAvailable
    case watchOnlyWallet
    case addressGenerationFailed
    case invalidDerivationPath
    case databaseError(String)
    case notImplemented(String)
    case walletError(String)
    case invalidInput(String)
    
    public var errorDescription: String? {
        switch self {
        case .invalidMnemonic:
            return "Invalid mnemonic phrase"
        case .seedGenerationFailed:
            return "Failed to generate seed from mnemonic"
        case .seedNotAvailable:
            return "Seed not available for this wallet"
        case .watchOnlyWallet:
            return "Operation not available for watch-only wallet"
        case .addressGenerationFailed:
            return "Failed to generate address"
        case .invalidDerivationPath:
            return "Invalid derivation path"
        case .databaseError(let message):
            return "Database error: \(message)"
        case .notImplemented(let feature):
            return "\(feature) not implemented yet"
        case .walletError(let message):
            return "Wallet error: \(message)"
        case .invalidInput(let message):
            return "Invalid input: \(message)"
        }
    }
}