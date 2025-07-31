import Foundation
import SwiftData
import Combine

// MARK: - Wallet Manager

@MainActor
public class WalletManager: ObservableObject {
    @Published public private(set) var wallets: [HDWallet] = []
    @Published public private(set) var currentWallet: HDWallet?
    @Published public private(set) var isLoading = false
    @Published public private(set) var error: WalletError?
    
    private let modelContainer: ModelContainer
    private let addressManager: AddressManager
    private let keyManager: KeyManager
    private let storage = WalletStorage()
    
    // Services
    public private(set) var utxoManager: UTXOManager!
    public private(set) var transactionService: TransactionService!
    
    public init() throws {
        self.modelContainer = try ModelContainer(for: HDWallet.self, HDAccount.self, HDAddress.self, HDUTXO.self, HDTransaction.self)
        self.addressManager = AddressManager()
        self.keyManager = KeyManager()
        
        // Initialize services
        self.utxoManager = UTXOManager(walletManager: self, modelContainer: modelContainer)
        self.transactionService = TransactionService(
            walletManager: self,
            utxoManager: utxoManager,
            modelContainer: modelContainer
        )
        
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
            guard keyManager.validateMnemonic(mnemonic) else {
                print("Mnemonic validation failed")
                throw WalletError.invalidMnemonic
            }
            finalMnemonic = mnemonic
        } else {
            print("Generating new mnemonic...")
            finalMnemonic = keyManager.generateMnemonic()
            print("Generated mnemonic: \(finalMnemonic)")
        }
        
        // Derive seed from mnemonic
        print("Deriving seed from mnemonic...")
        guard let seed = keyManager.mnemonicToSeed(finalMnemonic) else {
            print("Seed generation failed")
            throw WalletError.seedGenerationFailed
        }
        print("Seed generated successfully, length: \(seed.count)")
        
        // Create wallet
        let wallet = HDWallet(label: label, network: network)
        
        // Encrypt and store seed with PIN
        let encryptedSeed = try storage.storeSeed(seed, pin: pin)
        wallet.encryptedSeed = encryptedSeed
        
        // Insert wallet into context first
        modelContainer.mainContext.insert(wallet)
        
        // Create default account
        let account = wallet.createAccount(at: 0)
        
        // Derive account extended public key
        let accountPath = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: network == .testnet)
        if let accountKey = HDKeyDerivation.deriveKey(seed: seed, path: accountPath, network: network) {
            // Store the derived key info
            if let derivedKey = WalletFFIBridge.shared.deriveKey(
                seed: seed,
                path: accountPath.stringRepresentation,
                network: network
            ) {
                account.extendedPublicKey = derivedKey.publicKey.base64EncodedString()
            }
        }
        
        // Generate initial addresses
        try await generateAddresses(for: account, count: 20, type: .external)
        try await generateAddresses(for: account, count: 10, type: .internal)
        
        // Save to database
        try modelContainer.mainContext.save()
        
        await loadWallets()
        currentWallet = wallet
        
        return wallet
    }
    
    public func importWallet(label: String, network: DashNetwork, mnemonic: String, pin: String) async throws -> HDWallet {
        return try await createWallet(label: label, network: network, mnemonic: mnemonic, pin: pin)
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
    
    public func createAccount(in wallet: HDWallet) async throws -> HDAccount {
        guard !wallet.isWatchOnly else {
            throw WalletError.watchOnlyWallet
        }
        
        let accountIndex = UInt32(wallet.accounts.count)
        let account = wallet.createAccount(at: accountIndex)
        
        // Derive account extended public key
        if let seed = keyManager.decryptSeed(wallet.encryptedSeed ?? Data()) {
            let accountPath = DerivationPath.dashBIP44(
                account: accountIndex,
                change: 0,
                index: 0,
                testnet: wallet.dashNetwork == .testnet
            )
            
            if let accountKey = HDKeyDerivation.deriveKey(seed: seed, path: accountPath, network: wallet.dashNetwork) {
                account.extendedPublicKey = accountKey.publicKey.base64EncodedString()
            }
        }
        
        // Generate initial addresses
        try await generateAddresses(for: account, count: 20, type: .external)
        try await generateAddresses(for: account, count: 10, type: .internal)
        
        try modelContainer.mainContext.save()
        
        return account
    }
    
    // MARK: - Address Management
    
    public func generateAddresses(for account: HDAccount, count: Int, type: AddressType) async throws {
        print("WalletManager.generateAddresses called for type: \(type), count: \(count)")
        
        guard let wallet = account.wallet,
              !wallet.isWatchOnly,
              let seed = keyManager.decryptSeed(wallet.encryptedSeed ?? Data()) else {
            print("generateAddresses failed: wallet=\(account.wallet != nil), isWatchOnly=\(account.wallet?.isWatchOnly ?? false)")
            throw WalletError.seedNotAvailable
        }
        
        let startIndex: UInt32
        switch type {
        case .external:
            startIndex = account.externalAddressIndex
        case .internal:
            startIndex = account.internalAddressIndex
        case .coinJoin:
            startIndex = type == .external ? account.coinJoinExternalIndex : account.coinJoinInternalIndex
        case .identity:
            startIndex = account.identityFundingIndex
        }
        
        for i in 0..<count {
            let index = startIndex + UInt32(i)
            let path: DerivationPath
            
            switch type {
            case .external:
                path = DerivationPath.dashBIP44(
                    account: account.accountNumber,
                    change: 0,
                    index: index,
                    testnet: wallet.dashNetwork == .testnet
                )
            case .internal:
                path = DerivationPath.dashBIP44(
                    account: account.accountNumber,
                    change: 1,
                    index: index,
                    testnet: wallet.dashNetwork == .testnet
                )
            case .coinJoin:
                let change: UInt32 = type == .external ? 0 : 1
                path = DerivationPath.coinJoin(
                    account: account.accountNumber,
                    change: change,
                    index: index,
                    testnet: wallet.dashNetwork == .testnet
                )
            case .identity:
                path = DerivationPath.dip13Identity(
                    account: account.accountNumber,
                    identityIndex: 0,
                    keyType: .topup,
                    keyIndex: index,
                    testnet: wallet.dashNetwork == .testnet
                )
            }
            
            if let derivedKey = WalletFFIBridge.shared.deriveKey(
                seed: seed,
                path: path.stringRepresentation,
                network: wallet.dashNetwork
            ),
            let address = WalletFFIBridge.shared.addressFromPublicKey(
                derivedKey.publicKey,
                network: wallet.dashNetwork
            ) {
                
                print("Creating HDAddress: index=\(index), address=\(address), type=\(type)")
                
                let hdAddress = HDAddress(
                    address: address,
                    index: index,
                    derivationPath: path.stringRepresentation,
                    addressType: type,
                    account: account
                )
                
                print("HDAddress created with id=\(hdAddress.id), all properties set")
                
                // Insert address into context
                modelContainer.mainContext.insert(hdAddress)
                
                switch type {
                case .external:
                    account.externalAddresses.append(hdAddress)
                    account.externalAddressIndex = index + 1
                case .internal:
                    account.internalAddresses.append(hdAddress)
                    account.internalAddressIndex = index + 1
                case .coinJoin:
                    account.coinJoinAddresses.append(hdAddress)
                    if type == .external {
                        account.coinJoinExternalIndex = index + 1
                    } else {
                        account.coinJoinInternalIndex = index + 1
                    }
                case .identity:
                    account.identityFundingAddresses.append(hdAddress)
                    account.identityFundingIndex = index + 1
                }
            }
        }
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
        var confirmedBalance: UInt64 = 0
        var unconfirmedBalance: UInt64 = 0
        
        let allAddresses = account.externalAddresses + account.internalAddresses + 
                          account.coinJoinAddresses + account.identityFundingAddresses
        
        for address in allAddresses {
            let addressBalance = address.utxos.reduce(UInt64(0)) { sum, utxo in
                if !utxo.isSpent {
                    if utxo.blockHeight != nil {
                        return sum + utxo.amount
                    } else {
                        unconfirmedBalance += utxo.amount
                        return sum
                    }
                }
                return sum
            }
            confirmedBalance += addressBalance
        }
        
        account.confirmedBalance = confirmedBalance
        account.unconfirmedBalance = unconfirmedBalance
        
        try? modelContainer.mainContext.save()
    }
    
    // MARK: - Private Methods
    
    private func loadWallets() async {
        do {
            let descriptor = FetchDescriptor<HDWallet>(sortBy: [SortDescriptor(\.createdAt)])
            wallets = try modelContainer.mainContext.fetch(descriptor)
            
            if currentWallet == nil, let firstWallet = wallets.first {
                currentWallet = firstWallet
            }
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
        }
    }
}