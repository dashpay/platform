import Foundation
import SwiftData
import Combine
import SwiftDashSDK

// MARK: - Wallet Manager

/// WalletManager is a wrapper around the SDK's WalletManager
/// It delegates all wallet operations to the SDK layer while maintaining
/// SwiftUI compatibility through ObservableObject and SwiftData persistence
@MainActor
public class WalletManager: ObservableObject {
    @Published public private(set) var wallets: [HDWallet] = []
    @Published public private(set) var currentWallet: HDWallet?
    @Published public private(set) var isLoading = false
    @Published public private(set) var error: WalletError?
    
    // SDK wallet manager - this is the real wallet manager from the SDK
    private let sdkWalletManager: SwiftDashSDK.WalletManager
    private let modelContainer: ModelContainer
    private let storage = WalletStorage()
    
    // Services
    public private(set) var utxoManager: UTXOManager!
    public private(set) var transactionService: TransactionService!
    
    /// Initialize with an SDK wallet manager
    /// - Parameters:
    ///   - sdkWalletManager: The SDK wallet manager from SwiftDashSDK
    ///   - modelContainer: SwiftData model container for persistence
    public init(sdkWalletManager: SwiftDashSDK.WalletManager, modelContainer: ModelContainer? = nil) throws {
        print("=== WalletManager.init START ===")
        
        self.sdkWalletManager = sdkWalletManager
        
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
    
    public func createWallet(label: String, network: Network, mnemonic: String? = nil, pin: String) async throws -> HDWallet {
        print("WalletManager.createWallet called")
        isLoading = true
        defer { isLoading = false }
        
        // Generate or validate mnemonic using SDK
        let finalMnemonic: String
        if let mnemonic = mnemonic {
            print("Validating provided mnemonic...")
            guard SwiftDashSDK.Mnemonic.validate(mnemonic) else {
                print("Mnemonic validation failed")
                throw WalletError.invalidMnemonic
            }
            finalMnemonic = mnemonic
        } else {
            print("Generating new mnemonic...")
            do {
                finalMnemonic = try SwiftDashSDK.Mnemonic.generate(wordCount: 12)
                print("Generated mnemonic: \(finalMnemonic)")
            } catch {
                print("Failed to generate mnemonic: \(error)")
                throw WalletError.seedGenerationFailed
            }
        }
        
        // Add wallet through SDK
        let walletId: Data
        do {
            // Convert Network to KeyWalletNetwork
            let keyWalletNetwork = network.toKeyWalletNetwork()
            
            // Add wallet using SDK's WalletManager
            walletId = try sdkWalletManager.addWallet(
                mnemonic: finalMnemonic,
                passphrase: nil,
                network: keyWalletNetwork,
                accountOptions: .default
            )
            
            print("Wallet added with ID: \(walletId.hexString)")
        } catch {
            print("Failed to add wallet: \(error)")
            throw WalletError.walletError("Failed to add wallet: \(error.localizedDescription)")
        }
        
        // Create HDWallet model for SwiftUI
        let wallet = HDWallet(label: label, network: network)
        wallet.walletId = walletId
        
        // Get the wallet from SDK to store serialized bytes
        if let sdkWallet = try? sdkWalletManager.getWallet(id: walletId) {
            // TODO: We need a way to serialize the wallet for persistence
            // For now, just store the wallet ID
            print("Got wallet from SDK")
        }
        
        // Store encrypted seed (if needed for UI purposes)
        do {
            let seed = try SwiftDashSDK.Mnemonic.toSeed(mnemonic: finalMnemonic)
            let encryptedSeed = try storage.storeSeed(seed, pin: pin)
            wallet.encryptedSeed = encryptedSeed
        } catch {
            print("Failed to store seed: \(error)")
            // Continue anyway - wallet is already created
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
    
    public func importWallet(label: String, network: Network, mnemonic: String, pin: String) async throws -> HDWallet {
        return try await createWallet(label: label, network: network, mnemonic: mnemonic, pin: pin)
    }
    
    /// Restore a wallet from serialized bytes
    /// This is used to restore wallets from persistence on app startup
    public func restoreWalletFromBytes(_ walletBytes: Data) throws -> Data {
        // Use SDK's importWallet method
        return try sdkWalletManager.importWallet(from: walletBytes)
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
        let ffiNetwork = wallet.dashNetwork.toKeyWalletNetwork().ffiValue
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
    
    public func createWatchOnlyWallet(label: String, network: Network, extendedPublicKey: String) async throws -> HDWallet {
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
    
    /// Get detailed account information including xpub and addresses
    /// - Parameters:
    ///   - wallet: The wallet containing the account
    ///   - accountInfo: The account info to get details for
    /// - Returns: Detailed account information
    public func getAccountDetails(for wallet: HDWallet, accountInfo: AccountInfo) async throws -> AccountDetailInfo {
        guard let walletId = wallet.walletId else {
            throw WalletError.walletError("Wallet ID not available")
        }
        
        var error = FFIError()
        let ffiNetwork = wallet.dashNetwork.toKeyWalletNetwork().ffiValue
        
        // Get extended public key
        var xpub: String?
        
        // Try to get xpub using wallet_get_account_xpub if available
        // This is a BIP44 account derivation
        if accountInfo.index <= 999 {
            // BIP44 accounts
            // First get the wallet handle from the manager
            let walletHandle = walletId.withUnsafeBytes { idBytes in
                let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
                return wallet_manager_get_wallet(ffiWalletManager, idPtr, &error)
            }
            
            defer {
                // Free the wallet handle after we're done
                if walletHandle != nil {
                    wallet_free_const(walletHandle)
                }
            }
            
            if walletHandle != nil {
                let xpubPtr = wallet_get_account_xpub(walletHandle, ffiNetwork, accountInfo.index, &error)
                if let ptr = xpubPtr {
                    xpub = String(cString: ptr)
                    string_free(ptr)
                }
            }
        }
        
        // Get derivation path based on account type
        let derivationPath = getDerivationPath(for: accountInfo.index, network: wallet.dashNetwork)
        
        // Get managed account collection to get address details
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
        
        // Get the specific managed account
        let accountPtr: OpaquePointer? = getManagedAccount(from: collection, accountInfo: accountInfo)
        
        defer {
            if let account = accountPtr {
                managed_account_free(account)
            }
        }
        
        // Default values
        var gapLimit: UInt32 = 20  // Default gap limit
        var externalAddresses: [AddressDetail] = []
        var internalAddresses: [AddressDetail] = []
        var accountType: FFIAccountType = STANDARD_BIP44  // Default to BIP44
        
        if let account = accountPtr {
            // Get the account type
            var typeError: UInt32 = 0
            accountType = managed_account_get_account_type(account, &typeError)
            
            // Check if this account type has internal/external addresses
            let hasInternalExternal = (accountType == STANDARD_BIP44 || accountType == STANDARD_BIP32)
            
            if hasInternalExternal {
                // BIP44/BIP32 accounts have external and internal pools
                
                // Get address pool info for external addresses
                if let externalPool = managed_account_get_external_address_pool(account) {
                    defer { address_pool_free(externalPool) }
                    
                    // Get external addresses
                    var countOut: size_t = 0
                    let addressesPtr = address_pool_get_addresses_in_range(
                        externalPool,
                        0,      // start index
                        100,    // end index (reasonable limit for display)
                        &countOut,
                        &error
                    )
                    
                    if let addresses = addressesPtr {
                        for i in 0..<countOut {
                            if let addressInfo = addresses[i] {
                                let address = addressInfo.pointee.address != nil ? String(cString: addressInfo.pointee.address!) : ""
                                let index = addressInfo.pointee.index
                                let path = addressInfo.pointee.path != nil ? String(cString: addressInfo.pointee.path!) : ""
                                let isUsed = addressInfo.pointee.used
                                
                                // Extract public key
                                var publicKeyHex = ""
                                if let pubKeyPtr = addressInfo.pointee.public_key {
                                    let pubKeyData = Data(bytes: pubKeyPtr, count: addressInfo.pointee.public_key_len)
                                    publicKeyHex = pubKeyData.toHexString()
                                }
                                
                                externalAddresses.append(AddressDetail(
                                    address: address,
                                    index: index,
                                    path: path,
                                    isUsed: isUsed,
                                    publicKey: publicKeyHex
                                ))
                                
                                address_info_free(addressInfo)
                            }
                        }
                        addresses.deallocate()
                    }
                }
                
                // Get address pool info for internal addresses
                if let internalPool = managed_account_get_internal_address_pool(account) {
                    defer { address_pool_free(internalPool) }
                    
                    // Get internal addresses
                    var countOut: size_t = 0
                    let addressesPtr = address_pool_get_addresses_in_range(
                        internalPool,
                        0,      // start index
                        50,     // end index (reasonable limit for display)
                        &countOut,
                        &error
                    )
                    
                    if let addresses = addressesPtr {
                        for i in 0..<countOut {
                            if let addressInfo = addresses[i] {
                                let address = addressInfo.pointee.address != nil ? String(cString: addressInfo.pointee.address!) : ""
                                let index = addressInfo.pointee.index
                                let path = addressInfo.pointee.path != nil ? String(cString: addressInfo.pointee.path!) : ""
                                let isUsed = addressInfo.pointee.used
                                
                                // Extract public key
                                var publicKeyHex = ""
                                if let pubKeyPtr = addressInfo.pointee.public_key {
                                    let pubKeyData = Data(bytes: pubKeyPtr, count: addressInfo.pointee.public_key_len)
                                    publicKeyHex = pubKeyData.toHexString()
                                }
                                
                                internalAddresses.append(AddressDetail(
                                    address: address,
                                    index: index,
                                    path: path,
                                    isUsed: isUsed,
                                    publicKey: publicKeyHex
                                ))
                                
                                address_info_free(addressInfo)
                            }
                        }
                        addresses.deallocate()
                    }
                }
            } else {
                // Non-BIP44/BIP32 accounts (Identity, CoinJoin, Provider keys) have a single address pool
                // Use FFIAddressPoolType.Single (value 2) for these accounts
                let singlePoolType = FFIAddressPoolType(2) // Single pool type
                if let addressPool = managed_account_get_address_pool(account, singlePoolType) {
                    defer { address_pool_free(addressPool) }
                    
                    // Get addresses from the single pool
                    var countOut: size_t = 0
                    let addressesPtr = address_pool_get_addresses_in_range(
                        addressPool,
                        0,      // start index
                        100,    // end index (reasonable limit for display)
                        &countOut,
                        &error
                    )
                    
                    if let addresses = addressesPtr {
                        for i in 0..<countOut {
                            if let addressInfo = addresses[i] {
                                let address = addressInfo.pointee.address != nil ? String(cString: addressInfo.pointee.address!) : ""
                                let index = addressInfo.pointee.index
                                let path = addressInfo.pointee.path != nil ? String(cString: addressInfo.pointee.path!) : ""
                                let isUsed = addressInfo.pointee.used
                                
                                // Extract public key
                                var publicKeyHex = ""
                                if let pubKeyPtr = addressInfo.pointee.public_key {
                                    let pubKeyData = Data(bytes: pubKeyPtr, count: addressInfo.pointee.public_key_len)
                                    publicKeyHex = pubKeyData.toHexString()
                                }
                                
                                // Put all addresses in external array for non-BIP44/BIP32 accounts
                                externalAddresses.append(AddressDetail(
                                    address: address,
                                    index: index,
                                    path: path,
                                    isUsed: isUsed,
                                    publicKey: publicKeyHex
                                ))
                                
                                address_info_free(addressInfo)
                            }
                        }
                        addresses.deallocate()
                    }
                }
            }
        }
        
        // Calculate used/unused addresses
        let usedAddresses = externalAddresses.filter { $0.isUsed }.count + internalAddresses.filter { $0.isUsed }.count
        let unusedAddresses = externalAddresses.filter { !$0.isUsed }.count + internalAddresses.filter { !$0.isUsed }.count
        
        return AccountDetailInfo(
            account: accountInfo,
            accountType: accountType,
            xpub: xpub,
            derivationPath: derivationPath,
            gapLimit: gapLimit,
            usedAddresses: usedAddresses,
            unusedAddresses: unusedAddresses,
            externalAddresses: externalAddresses,
            internalAddresses: internalAddresses
        )
    }
    
    /// Derive a private key as WIF from seed using a specific path
    public func derivePrivateKeyAsWIF(from seed: Data, path: String, network: Network) async throws -> String {
        // First derive the private key bytes
        let privateKeyData = try await derivePrivateKey(from: seed, path: path, network: network)
        
        // Convert to hex string
        let privateKeyHex = privateKeyData.toHexString()
        
        // Convert to WIF using the FFI function
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                privateKeyHex.withCString { hexCString in
                    let result = dash_sdk_private_key_to_wif(hexCString, network == .testnet)
                    
                    if result.error == nil, let data = result.data {
                        // The data should be a C string for WIF
                        let wif = String(cString: data.assumingMemoryBound(to: CChar.self))
                        // Note: We don't free the string as it's managed by the SDK
                        continuation.resume(returning: wif)
                    } else if let error = result.error {
                        let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Failed to convert to WIF"
                        dash_sdk_error_free(error)
                        continuation.resume(throwing: WalletError.walletError(errorMessage))
                    } else {
                        continuation.resume(throwing: WalletError.walletError("Failed to convert to WIF"))
                    }
                }
            }
        }
    }
    
    /// Derive a private key from seed using a specific path
    public func derivePrivateKey(from seed: Data, path: String, network: Network) async throws -> Data {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var error = FFIError()
                
                // Convert DashNetwork to FFINetwork enum value
                let ffiNetwork = FFINetwork(rawValue: network == .mainnet ? 0 : 1)
                
                let extPrivKey = seed.withUnsafeBytes { seedBytes in
                    path.withCString { pathCString in
                        derivation_derive_private_key_from_seed(
                            seedBytes.bindMemory(to: UInt8.self).baseAddress,
                            seed.count,
                            pathCString,
                            ffiNetwork,
                            &error
                        )
                    }
                }
                
                if error.message != nil {
                    let errorMessage = String(cString: error.message!)
                    error_message_free(error.message)
                    continuation.resume(throwing: WalletError.walletError(errorMessage))
                    return
                }
                
                guard let extPrivKey = extPrivKey else {
                    continuation.resume(throwing: WalletError.walletError("Failed to derive private key"))
                    return
                }
                
                defer { derivation_xpriv_free(extPrivKey) }
                
                // Get the private key bytes
                var privateKeyData = Data(count: 32)
                let result = privateKeyData.withUnsafeMutableBytes { buffer in
                    if let baseAddress = buffer.bindMemory(to: UInt8.self).baseAddress {
                        return dash_key_xprv_private_key(extPrivKey, baseAddress)
                    }
                    return Int32(-1)
                }
                
                if result != 0 {
                    continuation.resume(throwing: WalletError.walletError("Failed to extract private key bytes"))
                    return
                }
                
                continuation.resume(returning: privateKeyData)
            }
        }
    }
    
    /// Get the derivation path for an account based on its index
    private func getDerivationPath(for accountIndex: UInt32, network: Network) -> String {
        let coinType = network == .testnet ? "1'" : "5'"  // Dash coin type
        
        switch accountIndex {
        case 0...999:
            // BIP44 accounts
            return "m/44'/\(coinType)/\(accountIndex)'"
        case 1000...1999:
            // CoinJoin accounts
            let index = accountIndex - 1000
            return "m/9'/\(coinType)/\(index)'"
        case 5000...5999:
            // BIP32 accounts
            let index = accountIndex - 5000
            return "m/\(index)'"
        case 9000:
            // Identity Registration
            return "m/9'/\(coinType)/5'/0"
        case 9001:
            // Identity Invitation
            return "m/9'/\(coinType)/5'/1"
        case 9002:
            // Identity Topup (Not Bound)
            return "m/9'/\(coinType)/5'/2"
        case 9100...9199:
            // Identity Topup accounts
            let index = accountIndex - 9100
            return "m/9'/\(coinType)/5'/3/\(index)'"
        case 10000...10999:
            // Provider Voting Keys
            let index = accountIndex - 10000
            return "m/9'/\(coinType)/6'/\(index)'"
        case 11000:
            // Provider Owner Keys
            return "m/9'/\(coinType)/7'/0"
        case 11001:
            // Provider Operator Keys (BLS)
            return "m/9'/\(coinType)/7'/1"
        case 11002:
            // Provider Platform Keys (EdDSA)
            return "m/9'/\(coinType)/7'/2"
        default:
            return "m/custom/\(accountIndex)'"
        }
    }
    
    
    /// Get managed account from collection based on account info
    private func getManagedAccount(from collection: OpaquePointer, accountInfo: AccountInfo) -> OpaquePointer? {
        switch accountInfo.index {
        case 0...999:
            // BIP44 accounts
            return managed_account_collection_get_bip44_account(collection, accountInfo.index)
        case 1000...1999:
            // CoinJoin accounts
            let index = accountInfo.index - 1000
            return managed_account_collection_get_coinjoin_account(collection, index)
        case 5000...5999:
            // BIP32 accounts
            let index = accountInfo.index - 5000
            return managed_account_collection_get_bip32_account(collection, index)
        case 9000:
            // Identity Registration
            return managed_account_collection_get_identity_registration(collection)
        case 9001:
            // Identity Invitation
            return managed_account_collection_get_identity_invitation(collection)
        case 9002:
            // Identity Topup (Not Bound)
            return managed_account_collection_get_identity_topup_not_bound(collection)
        case 9100...9199:
            // Identity Topup accounts
            let index = accountInfo.index - 9100
            return managed_account_collection_get_identity_topup(collection, index)
        case 10000...10999:
            // Provider Voting Keys
            return managed_account_collection_get_provider_voting_keys(collection)
        case 11000:
            // Provider Owner Keys
            return managed_account_collection_get_provider_owner_keys(collection)
        case 11001:
            // Provider Operator Keys (BLS)
            if let voidPtr = managed_account_collection_get_provider_operator_keys(collection) {
                return OpaquePointer(voidPtr)
            }
            return nil
        case 11002:
            // Provider Platform Keys (EdDSA)
            if let voidPtr = managed_account_collection_get_provider_platform_keys(collection) {
                return OpaquePointer(voidPtr)
            }
            return nil
        default:
            return nil
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
        let ffiNetwork = wallet.dashNetwork.toKeyWalletNetwork().ffiValue
        
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
        let ffiNetwork = wallet.dashNetwork.toKeyWalletNetwork().ffiValue
        
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
    
    // MARK: - Public Utility Methods
    
    public func reloadWallets() async {
        await loadWallets()
    }
    
    // MARK: - Private Methods
    
    private func loadWallets() async {
        do {
            let descriptor = FetchDescriptor<HDWallet>(sortBy: [SortDescriptor(\.createdAt)])
            wallets = try modelContainer.mainContext.fetch(descriptor)
            
            // Restore each wallet to the FFI wallet manager
            for wallet in wallets {
                // Migrate networks field if not set (for existing wallets)
                if wallet.networks == 0 {
                    // Set networks based on the wallet's current network
                    switch wallet.dashNetwork {
                    case .mainnet:
                        wallet.networks = 1 << 0  // DASH_FLAG
                    case .testnet:
                        wallet.networks = 1 << 1  // TESTNET_FLAG
                    case .devnet:
                        wallet.networks = 8  // DEVNET
                    }
                    print("Migrated networks field for wallet '\(wallet.label)' to \(wallet.networks)")
                }
                
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