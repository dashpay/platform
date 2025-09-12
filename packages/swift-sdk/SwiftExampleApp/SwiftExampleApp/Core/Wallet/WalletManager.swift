import Foundation
import SwiftData
import Combine
import SwiftDashSDK
import DashSDKFFI

// MARK: - Wallet Manager

/// WalletManager is a wrapper around the SDK's WalletManager
/// It delegates all wallet operations to the SDK layer while maintaining
/// SwiftUI compatibility through ObservableObject and SwiftData persistence
@MainActor
class WalletManager: ObservableObject {
    @Published public private(set) var wallets: [HDWallet] = []
    @Published public private(set) var currentWallet: HDWallet?
    @Published public private(set) var isLoading = false
    @Published public private(set) var error: WalletError?
    
    // SDK wallet manager - this is the real wallet manager from the SDK
    private let sdkWalletManager: SwiftDashSDK.WalletManager
    private let modelContainer: ModelContainer
    private let storage = WalletStorage()
    
    // Services (initialize in WalletService when SPV is available)
    var transactionService: TransactionService?
    
    /// Initialize with an SDK wallet manager
    /// - Parameters:
    ///   - sdkWalletManager: The SDK wallet manager from SwiftDashSDK
    ///   - modelContainer: SwiftData model container for persistence
    init(sdkWalletManager: SwiftDashSDK.WalletManager, modelContainer: ModelContainer? = nil) throws {
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
        
        // Note: TransactionService is created in WalletService once SPV/UTXO context exists
        print("=== WalletManager.init SUCCESS ===")
        
        Task {
            await loadWallets()
        }
    }
    
    // MARK: - Wallet Management
    
    func createWallet(label: String, network: Network, mnemonic: String? = nil, pin: String, networks: [Network]? = nil) async throws -> HDWallet {
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
                // Do not log mnemonic to console
            } catch {
                print("Failed to generate mnemonic: \(error)")
                throw WalletError.seedGenerationFailed
            }
        }
        
        // Add wallet through SDK (with bitfield networks) and capture serialized bytes for persistence
        let walletId: Data
        let serializedBytes: Data
        do {
            let selectedNetworks = networks ?? [network]
            let keyWalletNetworks = selectedNetworks.map { $0.toKeyWalletNetwork() }

            // Add wallet using SDK's WalletManager with combined network bitfield and serialize
            let result = try sdkWalletManager.addWalletAndSerialize(
                mnemonic: finalMnemonic,
                passphrase: nil,
                networks: keyWalletNetworks,
                birthHeight: 0,
                accountOptions: .default,
                downgradeToPublicKeyWallet: false,
                allowExternalSigning: false
            )
            walletId = result.walletId
            serializedBytes = result.serializedWallet
            
            print("Wallet added with ID: \(walletId.hexString)")
        } catch {
            print("Failed to add wallet: \(error)")
            throw WalletError.walletError("Failed to add wallet: \(error.localizedDescription)")
        }
        
        // Create HDWallet model for SwiftUI
        let wallet = HDWallet(label: label, network: network, isImported: false)
        wallet.walletId = walletId
        
        // Persist serialized wallet bytes for restoration on next launch
        wallet.serializedWalletBytes = serializedBytes
        
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
        _ = wallet.createAccount(at: 0)
        
        // Sync complete wallet state from Rust managed info
        try await syncWalletFromManagedInfo(for: wallet)
        
        // If multiple networks were specified, set the bitfield accordingly
        if let networks = networks {
            var bitfield: UInt32 = 0
            for n in networks {
                switch n {
                case .mainnet: bitfield |= 1
                case .testnet: bitfield |= 2
                case .devnet: bitfield |= 8
                }
            }
            wallet.networks = bitfield
        }

        // Save to database
        try modelContainer.mainContext.save()
        
        await loadWallets()
        currentWallet = wallet
        
        return wallet
    }
    
    func importWallet(label: String, network: Network, mnemonic: String, pin: String) async throws -> HDWallet {
        let wallet = try await createWallet(label: label, network: network, mnemonic: mnemonic, pin: pin)
        wallet.isImported = true
        try modelContainer.mainContext.save()
        return wallet
    }
    
    /// Restore a wallet from serialized bytes via SDK
    public func restoreWalletFromBytes(_ walletBytes: Data) throws -> Data {
        try sdkWalletManager.importWallet(from: walletBytes)
    }
    
    /// Sync wallet data using SwiftDashSDK wrappers (no direct FFI in app)
    private func syncWalletFromManagedInfo(for wallet: HDWallet) async throws {
        guard let walletId = wallet.walletId else { throw WalletError.walletError("Wallet ID not available") }
        let network = wallet.dashNetwork.toKeyWalletNetwork()
        let collection = try sdkWalletManager.getManagedAccountCollection(walletId: walletId, network: network)
        
        for account in wallet.accounts {
            if let managed = collection.getBIP44Account(at: account.accountNumber) {
                if let bal = try? managed.getBalance() {
                    account.confirmedBalance = bal.confirmed
                    account.unconfirmedBalance = bal.unconfirmed
                }
                if let pool = managed.getExternalAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 20) {
                    account.externalAddresses.removeAll()
                    for info in infos {
                        let hd = HDAddress(address: info.address, index: info.index, derivationPath: info.path, addressType: .external, account: account)
                        hd.isUsed = info.used
                        modelContainer.mainContext.insert(hd)
                        account.externalAddresses.append(hd)
                    }
                    account.externalAddressIndex = UInt32(infos.count)
                }
                if let pool = managed.getInternalAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 10) {
                    account.internalAddresses.removeAll()
                    for info in infos {
                        let hd = HDAddress(address: info.address, index: info.index, derivationPath: info.path, addressType: .internal, account: account)
                        hd.isUsed = info.used
                        modelContainer.mainContext.insert(hd)
                        account.internalAddresses.append(hd)
                    }
                    account.internalAddressIndex = UInt32(infos.count)
                }
            }
        }
    }
    
    // Removed: replaced by syncAccountAddresses(using SDK)
    
    public func unlockWallet(with pin: String) async throws -> Data {
        return try storage.retrieveSeed(pin: pin)
    }
    
    public func decryptSeed(_ encryptedSeed: Data?) -> Data? {
        // This method is used internally by other services
        // In a real implementation, this would decrypt using the current PIN
        // For now, return nil to indicate manual unlock is needed
        return nil
    }
    
    /// Get wallet IDs via SDK wrapper
    func getWalletIds() throws -> [Data] { try sdkWalletManager.getWalletIds() }
    
    /// Get wallet balance via SDK wrapper
    func getWalletBalance(walletId: Data) throws -> (confirmed: UInt64, unconfirmed: UInt64) { try sdkWalletManager.getWalletBalance(walletId: walletId) }
    
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
    
    func createWatchOnlyWallet(label: String, network: Network, extendedPublicKey: String) async throws -> HDWallet {
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
    
    /// Get detailed account information including xpub and addresses
    /// - Parameters:
    ///   - wallet: The wallet containing the account
    ///   - accountInfo: The account info to get details for
    /// - Returns: Detailed account information
    func getAccountDetails(for wallet: HDWallet, accountInfo: AccountInfo) async throws -> AccountDetailInfo {
        guard let walletId = wallet.walletId else { throw WalletError.walletError("Wallet ID not available") }
        let network = wallet.dashNetwork.toKeyWalletNetwork()
        let collection = try sdkWalletManager.getManagedAccountCollection(walletId: walletId, network: network)

        // Resolve managed account from category and optional index
        var managed: ManagedAccount?
        switch accountInfo.category {
        case .bip44:
            if let idx = accountInfo.index { managed = collection.getBIP44Account(at: idx) }
        case .bip32:
            if let idx = accountInfo.index { managed = collection.getBIP32Account(at: idx) }
        case .coinjoin:
            if let idx = accountInfo.index { managed = collection.getCoinJoinAccount(at: idx) }
        case .identityRegistration:
            managed = collection.getIdentityRegistrationAccount()
        case .identityInvitation:
            managed = collection.getIdentityInvitationAccount()
        case .identityTopupNotBound:
            managed = collection.getIdentityTopUpNotBoundAccount()
        case .identityTopup:
            if let idx = accountInfo.index { managed = collection.getIdentityTopUpAccount(registrationIndex: idx) }
        case .providerVotingKeys:
            managed = collection.getProviderVotingKeysAccount()
        case .providerOwnerKeys:
            managed = collection.getProviderOwnerKeysAccount()
        case .providerOperatorKeys:
            managed = collection.getProviderOperatorKeysAccount()
        case .providerPlatformKeys:
            managed = collection.getProviderPlatformKeysAccount()
        }

        let derivationPath = derivationPath(for: accountInfo.category, index: accountInfo.index, network: wallet.dashNetwork)
        var externalDetails: [AddressDetail] = []
        var internalDetails: [AddressDetail] = []
        var ffiType = FFIAccountType(rawValue: 0)
        if let m = managed {
            ffiType = FFIAccountType(rawValue: m.accountType?.rawValue ?? 0)
            if let pool = m.getExternalAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 100) {
                externalDetails = infos.map { info in
                    AddressDetail(address: info.address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
            if let pool = m.getInternalAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 100) {
                internalDetails = infos.map { info in
                    AddressDetail(address: info.address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
            // Single pool fallback
            if externalDetails.isEmpty && internalDetails.isEmpty, let pool = m.getAddressPool(type: .single), let infos = try? pool.getAddresses(from: 0, to: 100) {
                externalDetails = infos.map { info in
                    AddressDetail(address: info.address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
        }

        let used = externalDetails.filter { $0.isUsed }.count + internalDetails.filter { $0.isUsed }.count
        let unused = externalDetails.filter { !$0.isUsed }.count + internalDetails.filter { !$0.isUsed }.count
        return AccountDetailInfo(
            account: accountInfo,
            accountType: ffiType,
            xpub: nil,
            derivationPath: derivationPath,
            gapLimit: 20,
            usedAddresses: used,
            unusedAddresses: unused,
            externalAddresses: externalDetails,
            internalAddresses: internalDetails
        )
    }
    
    /// Derive a private key as WIF from seed using a specific path (deferred to SDK)
    public func derivePrivateKeyAsWIF(for wallet: HDWallet, accountInfo: AccountInfo, addressIndex: UInt32) async throws -> String {
        guard let walletId = wallet.walletId else { throw WalletError.walletError("Wallet ID not available") }
        let net = wallet.dashNetwork
        // Obtain a non-owning Wallet wrapper from manager
        guard let sdkWallet = try sdkWalletManager.getWallet(id: walletId, network: net.toKeyWalletNetwork()) else {
            throw WalletError.walletError("Wallet not found in manager")
        }
        
        // Map category to AccountType and master path root
        let coinType = (net == .testnet) ? "1'" : "5'"
        let mapping: (AccountType, UInt32, String)? = {
            switch accountInfo.category {
            case .providerVotingKeys:
                return (.providerVotingKeys, 0, "m/9'/\(coinType)/3'/1'")
            case .providerOwnerKeys:
                return (.providerOwnerKeys, 0, "m/9'/\(coinType)/3'/2'")
            case .providerOperatorKeys:
                return (.providerOperatorKeys, 0, "m/9'/\(coinType)/3'/3'")
            case .providerPlatformKeys:
                return (.providerPlatformKeys, 0, "m/9'/\(coinType)/3'/4'")
            case .bip44:
                let idx = accountInfo.index ?? 0
                return (.standardBIP44, idx, "m/44'/\(coinType)/\(idx)'")
            case .bip32:
                let idx = accountInfo.index ?? 0
                return (.standardBIP32, idx, "m/\(idx)'")
            case .coinjoin:
                let idx = (accountInfo.index ?? 1000) - 1000
                return (.coinJoin, UInt32(idx), "m/9'/\(coinType)/4'/\(idx)'")
            case .identityRegistration, .identityInvitation, .identityTopupNotBound, .identityTopup:
                return nil
            }
        }()
        
        guard let (type, accountIndex, masterPath) = mapping else {
            throw WalletError.notImplemented("Derivation not supported for this account type")
        }
        
        // Get account and derive
        let account = try sdkWallet.getAccount(type: type, index: accountIndex)
        let wif = try account.derivePrivateKeyWIF(wallet: sdkWallet, masterPath: masterPath, index: addressIndex)
        return wif
    }
    
    // Index-based derivation was removed. We now map paths by AccountCategory
    // via derivationPath(for:index:network:) below to avoid conflating type with index.

    private func derivationPath(for category: AccountCategory, index: UInt32?, network: Network) -> String {
        let coinType = network == .testnet ? "1'" : "5'"
        switch category {
        case .bip44:
            return "m/44'/\(coinType)/\(index ?? 0)'"
        case .bip32:
            return "m/\((index ?? 0))'"
        case .coinjoin:
            // Account-level path for coinjoin: m/9'/coinType/4'/account'
            return "m/9'/\(coinType)/4'/\(index ?? 0)'"
        case .identityRegistration:
            return "m/9'/\(coinType)/5'/1'/x"
        case .identityInvitation:
            return "m/9'/\(coinType)/5'/3'/x"
        case .identityTopupNotBound:
            return "m/9'/\(coinType)/5'/2'/x"
        case .identityTopup:
            return "m/9'/\(coinType)/5'/2'/\(index ?? 0)'/x"
        case .providerVotingKeys:
            return "m/9'/\(coinType)/3'/1'/x"
        case .providerOwnerKeys:
            return "m/9'/\(coinType)/3'/2'/x"
        case .providerOperatorKeys:
            return "m/9'/\(coinType)/3'/3'/x"
        case .providerPlatformKeys:
            return "m/9'/\(coinType)/3'/4'/x"
        }
    }
    
    
    // Removed old FFI-based helper; using SwiftDashSDK wrappers instead
    
    /// Get all accounts for a wallet from the FFI wallet manager
    /// - Parameters:
    ///   - wallet: The wallet model
    ///   - network: Optional network override; defaults to wallet.dashNetwork
    /// - Returns: Account information including balances and address counts
    func getAccounts(for wallet: HDWallet, network: Network? = nil) async throws -> [AccountInfo] {
        guard let walletId = wallet.walletId else { throw WalletError.walletError("Wallet ID not available") }
        let effectiveNetwork = (network ?? wallet.dashNetwork).toKeyWalletNetwork()
        let collection: ManagedAccountCollection
        do {
            collection = try sdkWalletManager.getManagedAccountCollection(walletId: walletId, network: effectiveNetwork)
        } catch let err as KeyWalletError {
            // If the managed wallet info isn't found (e.g., after fresh start), try restoring from serialized bytes
            if case .notFound = err, let bytes = wallet.serializedWalletBytes {
                do {
                    let restoredId = try sdkWalletManager.importWallet(from: bytes)
                    if wallet.walletId != restoredId { wallet.walletId = restoredId }
                    // Retry once after import
                    collection = try sdkWalletManager.getManagedAccountCollection(walletId: wallet.walletId!, network: effectiveNetwork)
                } catch {
                    throw err
                }
            } else {
                throw err
            }
        }
        var list: [AccountInfo] = []

        func counts(_ m: ManagedAccount) -> (Int, Int) {
            var ext = 0, intc = 0
            if let p = m.getExternalAddressPool(), let infos = try? p.getAddresses(from: 0, to: 1000) { ext = infos.count }
            if let p = m.getInternalAddressPool(), let infos = try? p.getAddresses(from: 0, to: 1000) { intc = infos.count }
            return (ext, intc)
        }

        // BIP44
        for idx in collection.getBIP44Indices() {
            if let m = collection.getBIP44Account(at: idx) {
                let b = try? m.getBalance()
                let c = counts(m)
                list.append(AccountInfo(category: .bip44, index: idx, label: "Account \(idx)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (c.0, c.1), nextReceiveAddress: nil))
            }
        }
        // BIP32 (5000+)
        for raw in collection.getBIP32Indices() {
            if let m = collection.getBIP32Account(at: raw) {
                let b = try? m.getBalance()
                let c = counts(m)
                list.append(AccountInfo(category: .bip32, index: raw, label: "BIP32 \(raw)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (c.0, c.1), nextReceiveAddress: nil))
            }
        }
        // CoinJoin (1000+)
        for raw in collection.getCoinJoinIndices() {
            if let m = collection.getCoinJoinAccount(at: raw) {
                let b = try? m.getBalance()
                var total = 0
                if let p = m.getAddressPool(type: .single), let infos = try? p.getAddresses(from: 0, to: 1000) { total = infos.count }
                list.append(AccountInfo(category: .coinjoin, index: raw, label: "CoinJoin \(raw)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (total, 0), nextReceiveAddress: nil))
            }
        }
        // Identity accounts
        if let m = collection.getIdentityRegistrationAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .identityRegistration, label: "Identity Registration", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }
        if let m = collection.getIdentityInvitationAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .identityInvitation, label: "Identity Invitation", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }
        if let m = collection.getIdentityTopUpNotBoundAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .identityTopupNotBound, label: "Identity Topup (Not Bound)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }
        for raw in collection.getIdentityTopUpIndices() {
            if let m = collection.getIdentityTopUpAccount(registrationIndex: raw) {
                let b = try? m.getBalance()
                list.append(AccountInfo(category: .identityTopup, index: raw, label: "Identity Topup \(raw)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
            }
        }
        // Provider
        if let m = collection.getProviderVotingKeysAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .providerVotingKeys, label: "Provider Voting Keys", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }
        if let m = collection.getProviderOwnerKeysAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .providerOwnerKeys, label: "Provider Owner Keys", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }
        if let m = collection.getProviderOperatorKeysAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .providerOperatorKeys, label: "Provider Operator Keys (BLS)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }
        if let m = collection.getProviderPlatformKeysAccount() {
            let b = try? m.getBalance()
            list.append(AccountInfo(category: .providerPlatformKeys, label: "Provider Platform Keys (EdDSA)", balance: (b?.confirmed ?? 0, b?.unconfirmed ?? 0), addressCount: (0, 0), nextReceiveAddress: nil))
        }

        // Sort BIP44 by index first, then other types below
        list.sort { (a, b) in
            switch (a.category, b.category) {
            case (.bip44, .bip44): return (a.index ?? 0) < (b.index ?? 0)
            default: return a.label < b.label
            }
        }
        return list
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
    
    func generateAddresses(for account: HDAccount, count: Int, type: AddressType) async throws {
        // Refresh address lists from SDK-managed pools (SDK maintains state)
        guard let wallet = account.wallet else { throw WalletError.walletError("No wallet for account") }
        try await syncWalletFromManagedInfo(for: wallet)
    }
    
    private func generateWatchOnlyAddresses(for account: HDAccount, count: Int, type: AddressType) async throws {
        // For watch-only wallets, we need to derive addresses from extended public key
        // This would require implementing public key derivation
        // For now, throw an error as this requires additional cryptographic operations
        throw WalletError.notImplemented("Watch-only address generation")
    }
    
    func getUnusedAddress(for account: HDAccount, type: AddressType = .external) async throws -> HDAddress {
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
    
    func updateBalance(for account: HDAccount) async {
        guard let wallet = account.wallet,
              let walletId = wallet.walletId else {
            return
        }
        
        // Get balance via SDK wrappers
        do {
            let collection = try sdkWalletManager.getManagedAccountCollection(walletId: walletId, network: wallet.dashNetwork.toKeyWalletNetwork())
            if let managed = collection.getBIP44Account(at: account.accountNumber) {
                if let bal = try? managed.getBalance() {
                    account.confirmedBalance = bal.confirmed
                    account.unconfirmedBalance = bal.unconfirmed
                    try? modelContainer.mainContext.save()
                }
            }
        } catch {
            print("Failed to update balance: \(error)")
        }
    }
    
    // MARK: - Public Utility Methods
    
    func reloadWallets() async {
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
