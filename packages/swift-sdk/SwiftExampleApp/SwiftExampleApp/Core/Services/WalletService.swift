import Foundation
import SwiftData
import Combine
import DashSDKFFI

@MainActor
public class WalletService: ObservableObject {
    public static let shared = WalletService()
    
    // Published properties
    @Published public var currentWallet: HDWallet? // Placeholder - use WalletManager instead
    @Published public var balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
    @Published public var isSyncing = false
    @Published public var syncProgress: Double?
    @Published public var detailedSyncProgress: Any? // Use SPVClient.SyncProgress
    @Published public var lastSyncError: Error?
    @Published public var transactions: [CoreTransaction] = [] // Use HDTransaction from wallet
    
    // Internal properties
    private var modelContainer: ModelContainer?
    private var syncTask: Task<Void, Never>?
    private var balanceUpdateTask: Task<Void, Never>?
    
    // Exposed for WalletViewModel - read-only access to the properly initialized WalletManager
    public private(set) var walletManager: WalletManager?
    
    // SPV Client for Core SDK functionality
    private var spvClient: UnsafeMutablePointer<FFIDashSpvClient>?
    
    // Mock SDK for now - will be replaced with real SDK
    private var sdk: Any?
    
    private init() {}
    
    deinit {
        // Clean up SPV client
        if let client = spvClient {
            dash_core_sdk_destroy_client(client)
        }
        // Note: WalletManager handles its own FFI cleanup
    }
    
    public func configure(modelContainer: ModelContainer) {
        print("=== WalletService.configure START ===")
        self.modelContainer = modelContainer
        print("ModelContainer set: \(modelContainer)")
        
        // We'll initialize WalletManager from the SPV client after we create it
        
        // Initialize SPV Client for testnet
        print("Initializing SPV Client for testnet...")
        if let client = dash_core_sdk_create_client_testnet() {
            self.spvClient = client
            print("✅ SPV Client initialized successfully for testnet")
        } else {
            print("❌ Failed to initialize SPV Client")
        }
        
        // Initialize WalletManager from SPV Client
        print("Initializing WalletManager from SPV Client...")
        if let client = self.spvClient {
            // Get the FFI wallet manager pointer from SPV client
            if let managerPtr = dash_spv_ffi_client_get_wallet_manager(client) {
                let ffiWalletManagerPtr = OpaquePointer(managerPtr)
                print("✅ FFI Wallet Manager pointer obtained from SPV Client")
                
                // Create our refactored WalletManager wrapper
                do {
                    self.walletManager = try WalletManager(
                        ffiWalletManager: ffiWalletManagerPtr,
                        modelContainer: modelContainer
                    )
                    print("✅ WalletManager wrapper initialized successfully")
                } catch {
                    print("❌ Failed to initialize WalletManager wrapper:")
                    print("Error: \(error)")
                }
            } else {
                print("❌ Failed to get FFI wallet manager from SPV Client")
            }
        } else {
            print("❌ Cannot get WalletManager - SPV Client not initialized")
        }
        
        print("Loading current wallet...")
        loadCurrentWallet()
        print("=== WalletService.configure END ===")
    }
    
    public func setSharedSDK(_ sdk: Any) {
        self.sdk = sdk
        print("✅ WalletService configured with shared SDK")
    }
    
    /// Get the SPV client handle
    public func getSPVClient() -> UnsafeMutablePointer<FFIDashSpvClient>? {
        return spvClient
    }
    
    // MARK: - Wallet Management
    
    public func createWallet(label: String, mnemonic: String? = nil, pin: String = "1234") async throws -> HDWallet {
        print("=== WalletService.createWallet START ===")
        print("Label: \(label)")
        print("Has mnemonic: \(mnemonic != nil)")
        print("PIN: \(pin)")
        print("ModelContainer available: \(modelContainer != nil)")
        
        guard let walletManager = walletManager else {
            print("ERROR: WalletManager not initialized")
            print("WalletManager is nil")
            throw WalletError.notImplemented("WalletManager not initialized")
        }
        
        do {
            // Create wallet using our refactored WalletManager that wraps FFI
            print("WalletManager available, creating wallet...")
            let wallet = try await walletManager.createWallet(
                label: label,
                network: .testnet,
                mnemonic: mnemonic,
                pin: pin
            )
            
            print("Wallet created by WalletManager, ID: \(wallet.id)")
            print("Loading wallet...")
            
            // Load the newly created wallet
            await loadWallet(wallet)
            
            print("=== WalletService.createWallet SUCCESS ===")
            return wallet
        } catch {
            print("=== WalletService.createWallet FAILED ===")
            print("Error type: \(type(of: error))")
            print("Error: \(error)")
            throw error
        }
    }
    
    public func loadWallet(_ wallet: HDWallet) async {
        currentWallet = wallet
        
        // Load transactions
        await loadTransactions()
        
        // Update balance
        updateBalance()
        
        // Start sync if needed
        if wallet.syncProgress < 1.0 {
            await startSync()
        }
    }
    
    private func loadCurrentWallet() {
        guard let modelContainer = modelContainer else { return }
        
        // The WalletManager will handle loading and restoring wallets from persistence
        // It will restore the serialized wallet bytes to the FFI wallet manager
        // This happens automatically in WalletManager.init() through loadWallets()
        
        // Just sync the current wallet from WalletManager
        if let walletManager = self.walletManager {
            Task {
                // WalletManager's loadWallets() is called in its init
                // We just need to sync the current wallet
                if let wallet = walletManager.currentWallet {
                    self.currentWallet = wallet
                    await loadWallet(wallet)
                } else if let firstWallet = walletManager.wallets.first {
                    self.currentWallet = firstWallet
                    await loadWallet(firstWallet)
                }
            }
        }
    }
    
    // MARK: - Sync Management
    
    public func startSync() async {
        guard !isSyncing else { return }
        
        isSyncing = true
        lastSyncError = nil
        
        syncTask?.cancel()
        syncTask = Task {
            do {
                // Mock sync progress
                for i in 0...100 {
                    if Task.isCancelled { break }
                    
                    let progress = Double(i) / 100.0
                    await MainActor.run {
                        self.syncProgress = progress
                        self.detailedSyncProgress = SyncProgress(
                            current: UInt64(i),
                            total: 100,
                            rate: 1,
                            progress: progress,
                            stage: .downloading
                        )
                    }
                    
                    try await Task.sleep(nanoseconds: 100_000_000) // 0.1 second
                }
                
                // Update wallet sync status
                if let wallet = currentWallet {
                    wallet.syncProgress = 1.0
                    // wallet.lastSyncedAt = Date() // Property not available
                    try? modelContainer?.mainContext.save()
                }
                
            } catch {
                lastSyncError = error
            }
            
            isSyncing = false
            syncProgress = nil
            detailedSyncProgress = nil
        }
    }
    
    public func stopSync() {
        syncTask?.cancel()
        syncTask = nil
        isSyncing = false
        syncProgress = nil
        detailedSyncProgress = nil
    }
    
    // MARK: - Address Management
    
    public func generateAddresses(for account: HDAccount, count: Int, type: AddressType) async throws {
        guard let walletManager = self.walletManager else {
            throw WalletError.notImplemented("WalletManager not available")
        }
        
        try await walletManager.generateAddresses(for: account, count: count, type: type)
        try? modelContainer?.mainContext.save()
    }
    
    // MARK: - Transaction Management
    
    public func sendTransaction(to address: String, amount: UInt64, memo: String? = nil) async throws -> String {
        guard let wallet = currentWallet else {
            throw WalletError.notImplemented("No active wallet")
        }
        
        guard wallet.confirmedBalance >= amount else {
            throw WalletError.notImplemented("Insufficient funds")
        }
        
        // Mock transaction creation
        let txid = UUID().uuidString
        let transaction = HDTransaction(txHash: txid, timestamp: Date())
        transaction.amount = -Int64(amount)
        transaction.fee = 1000
        transaction.type = "sent"
        transaction.wallet = wallet
        
        modelContainer?.mainContext.insert(transaction)
        try? modelContainer?.mainContext.save()
        
        // Update balance
        updateBalance()
        
        return txid
    }
    
    private func loadTransactions() async {
        guard let wallet = currentWallet else { return }
        
        // Convert HDTransaction to CoreTransaction  
        transactions = wallet.transactions.map { hdTx in
            CoreTransaction(
                id: hdTx.txHash,
                amount: hdTx.amount,
                fee: hdTx.fee,
                timestamp: hdTx.timestamp,
                blockHeight: hdTx.blockHeight != nil ? Int64(hdTx.blockHeight!) : nil,
                confirmations: hdTx.confirmations,
                type: hdTx.type,
                memo: nil,
                inputs: [],
                outputs: [],
                isInstantSend: hdTx.isInstantSend,
                isAssetLock: false,
                rawData: hdTx.rawTransaction
            )
        }.sorted { $0.timestamp > $1.timestamp }
    }
    
    // MARK: - Balance Management
    
    private func updateBalance() {
        guard let wallet = currentWallet else {
            balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
            return
        }
        
        balance = Balance(
            confirmed: wallet.confirmedBalance,
            unconfirmed: 0,
            immature: 0
        )
    }
    
    // MARK: - Address Management
    
    public func getNewAddress() async throws -> String {
        guard let wallet = currentWallet else {
            throw WalletError.notImplemented("No active wallet")
        }
        
        // Find next unused address or create new one
        let currentAccount = wallet.accounts.first ?? wallet.createAccount()
        let existingAddresses = currentAccount.externalAddresses
        let nextIndex = UInt32(existingAddresses.count)
        
        // Mock address generation
        let address = "yMockAddress\(nextIndex)"
        
        let hdAddress = HDAddress(
            address: address,
            index: nextIndex,
            derivationPath: "m/44'/5'/0'/0/\(nextIndex)",
            addressType: .external,
            account: currentAccount
        )
        
        modelContainer?.mainContext.insert(hdAddress)
        try? modelContainer?.mainContext.save()
        
        return address
    }
    
    // MARK: - Helpers
    
    private func generateMnemonic() -> String {
        // Mock mnemonic generation
        let words = ["abandon", "ability", "able", "about", "above", "absent",
                    "absorb", "abstract", "absurd", "abuse", "access", "accident"]
        return words.joined(separator: " ")
    }
}

// SyncProgress is now defined in SPVClient.swift