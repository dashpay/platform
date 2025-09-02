import Foundation
import SwiftData
import Combine
import SwiftDashSDK

@MainActor
public class WalletService: ObservableObject {
    public static let shared = WalletService()
    
    // Published properties
    @Published var currentWallet: HDWallet? // Placeholder - use WalletManager instead
    @Published public var balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
    @Published public var isSyncing = false
    @Published public var syncProgress: Double?
    @Published public var detailedSyncProgress: Any? // Use SPVClient.SyncProgress
    @Published public var headerProgress: Double = 0
    @Published public var masternodeProgress: Double = 0
    @Published public var transactionProgress: Double = 0
    @Published public var lastSyncError: Error?
    @Published public var transactions: [CoreTransaction] = [] // Use HDTransaction from wallet
    @Published var currentNetwork: Network = .testnet
    
    // Internal properties
    private var modelContainer: ModelContainer?
    private var syncTask: Task<Void, Never>?
    private var balanceUpdateTask: Task<Void, Never>?
    private var spvStatsTimer: Timer?
    
    // Exposed for WalletViewModel - read-only access to the properly initialized WalletManager
    private(set) var walletManager: WalletManager?
    
    // SPV Client - new wrapper with proper sync support
    private var spvClient: SPVClient?
    
    // Mock SDK for now - will be replaced with real SDK
    private var sdk: Any?
    // Latest sync stats (for UI)
    @Published var latestHeaderHeight: Int = 0
    @Published var latestFilterHeight: Int = 0
    @Published var latestMasternodeListHeight: Int = 0 // TODO: fill when FFI exposes
    
    private init() {}
    
    deinit {
        // SPVClient handles its own cleanup
        Task { @MainActor in
            spvClient?.stop()
        }
    }
    
    func configure(modelContainer: ModelContainer, network: Network = .testnet) {
        print("=== WalletService.configure START ===")
        self.modelContainer = modelContainer
        self.currentNetwork = network
        print("ModelContainer set: \(modelContainer)")
        print("Network set: \(network.rawValue)")
        
        // Initialize SPV Client wrapper
        print("Initializing SPV Client for \(network.rawValue)...")
        spvClient = SPVClient(network: network.sdkNetwork)
        spvClient?.delegate = self
        
        // Capture current references on the main actor to avoid cross-actor hops later
        guard let client = spvClient, let mc = self.modelContainer else { return }
        let net = currentNetwork
        Task.detached(priority: .userInitiated) {
            do {
                // Initialize the SPV client with proper configuration
                let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").path
                try client.initialize(dataDir: dataDir)

                // Start the SPV client
                try client.start()
                print("✅ SPV Client initialized and started successfully for \(net.rawValue)")

                // Seed UI with latest checkpoint height if we don't have a header yet
                let seedHeight = client.getLatestCheckpointHeight()
                await MainActor.run {
                    if WalletService.shared.latestHeaderHeight == 0, let cp = seedHeight {
                        WalletService.shared.latestHeaderHeight = Int(cp)
                    }
                    WalletService.shared.beginSPVStatsPolling()
                }

                // Create SDK wallet manager (unified, not tied to SPV pointer for now)
                do {
                    let sdkWalletManager = try SwiftDashSDK.WalletManager()
                    let wrapper: WalletManager = try await MainActor.run {
                        try WalletManager(sdkWalletManager: sdkWalletManager, modelContainer: mc)
                    }
                    await MainActor.run {
                        WalletService.shared.walletManager = wrapper
                        WalletService.shared.walletManager?.transactionService = TransactionService(
                            walletManager: wrapper,
                            modelContainer: mc,
                            spvClient: client
                        )
                        print("✅ WalletManager wrapper initialized successfully")
                    }
                } catch {
                    print("❌ Failed to initialize WalletManager wrapper:\nError: \(error)")
                }
            } catch {
                print("❌ Failed to initialize SPV Client: \(error)")
                await MainActor.run { WalletService.shared.lastSyncError = error }
            }
        }
        
        print("Loading current wallet...")
        loadCurrentWallet()
        print("=== WalletService.configure END ===")
    }
    
    public func setSharedSDK(_ sdk: Any) {
        self.sdk = sdk
        print("✅ WalletService configured with shared SDK")
    }
    
    
    // MARK: - Wallet Management
    
    func createWallet(label: String, mnemonic: String? = nil, pin: String = "1234", network: Network? = nil) async throws -> HDWallet {
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
            let walletNetwork = network ?? currentNetwork
            let dashNetwork = walletNetwork  // Already a DashNetwork
            let wallet = try await walletManager.createWallet(
                label: label,
                network: dashNetwork,
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

    // MARK: - Trusted Mode / Masternode Sync
    public func disableMasternodeSync() throws {
        try spvClient?.setMasternodeSyncEnabled(false)
    }
    
    // MARK: - Sync Management
    
    public func startSync() async {
        guard !isSyncing else { return }
        guard let spvClient = spvClient else {
            print("❌ SPV Client not initialized")
            return
        }
        
        isSyncing = true
        lastSyncError = nil
        
        // Kick off sync without blocking the main thread
        Task.detached(priority: .userInitiated) { [weak self] in
            do {
                try await spvClient.startSync()
            } catch {
                await MainActor.run {
                    self?.lastSyncError = error
                    self?.isSyncing = false
                }
                print("❌ Sync failed: \(error)")
            }
        }
    }
    
    public func stopSync() {
        spvClient?.cancelSync()
        isSyncing = false
        syncProgress = nil
        detailedSyncProgress = nil
        spvStatsTimer?.invalidate()
        spvStatsTimer = nil
    }
    
    // MARK: - Network Management
    
    func switchNetwork(to network: Network) async {
        guard network != currentNetwork else { return }
        
        print("=== WalletService.switchNetwork START ===")
        print("Switching from \(currentNetwork.rawValue) to \(network.rawValue)")
        
        // Stop any ongoing sync
        await stopSync()
        
        // Clean up current SPV client
        spvClient?.stop()
        spvClient = nil
        
        // Clear current wallet manager
        walletManager = nil
        currentWallet = nil
        transactions = []
        balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
        
        // Reconfigure with new network
        currentNetwork = network
        if let modelContainer = modelContainer {
            configure(modelContainer: modelContainer, network: network)
        }
        
        print("=== WalletService.switchNetwork END ===")
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
    
    // MARK: - Wallet Deletion
    
    public func walletDeleted(_ wallet: HDWallet) async {
        // If this was the current wallet, clear it
        if currentWallet?.id == wallet.id {
            currentWallet = nil
            transactions = []
            balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
        }
        
        // Reload wallets from the wallet manager
        if let walletManager = walletManager {
            await walletManager.reloadWallets()
            
            // Set a new current wallet if available
            if currentWallet == nil, let firstWallet = walletManager.wallets.first {
                await loadWallet(firstWallet)
            }
        }
    }
    
    // MARK: - Helpers
    
    private func generateMnemonic() -> String {
        // Mock mnemonic generation
        let words = ["abandon", "ability", "able", "about", "above", "absent",
                    "absorb", "abstract", "absurd", "abuse", "access", "accident"]
        return words.joined(separator: " ")
    }
}

// MARK: - SPV Stats Polling
extension WalletService {
    private func beginSPVStatsPolling() {
        spvStatsTimer?.invalidate()
        spvStatsTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            guard let self = self else { return }
            // Call FFI off the main actor to avoid UI stalls
            Task.detached(priority: .utility) { [weak self] in
                let client = await self?.spvClient
                guard let client = client else { return }
                guard let stats = client.getStats() else { return }
                await MainActor.run {
                    // Only overwrite with positive values; keep seeded values otherwise
                    if stats.headerHeight > 0 {
                        self?.latestHeaderHeight = max(self?.latestHeaderHeight ?? 0, stats.headerHeight)
                    }
                    if stats.filterHeight > 0 {
                        self?.latestFilterHeight = max(self?.latestFilterHeight ?? 0, stats.filterHeight)
                    }
                    // Keep latestMasternodeListHeight as 0 until available
                }
            }
        }
        if let t = spvStatsTimer { RunLoop.main.add(t, forMode: .common) }
    }
}

// MARK: - SPVClientDelegate

extension WalletService: SPVClientDelegate {
    nonisolated public func spvClient(_ client: SPVClient, didUpdateSyncProgress progress: SPVSyncProgress) {
        Task { @MainActor in
            // Prefer a deterministic percentage from heights, not FFI's percentage
            let headerPct = min(1.0, max(0.0, Double(progress.currentHeight) / Double(max(1, progress.targetHeight))))

            // Update published properties (top overlay + headers row)
            self.syncProgress = headerPct
            self.headerProgress = headerPct

            // Convert to detailed progress for UI (top overlay)
            self.detailedSyncProgress = SyncProgress(
                current: UInt64(progress.currentHeight),
                total: UInt64(progress.targetHeight),
                rate: progress.rate,
                progress: headerPct,
                stage: mapSyncStage(progress.stage)
            )
            
            if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
                print("📊 Sync progress: \(progress.stage.rawValue) - \(Int(progress.overallProgress * 100))%")
            }
        }

        // Update per-section progress using best available data without blocking UI
        Task.detached(priority: .utility) { [weak self] in
            guard let self = self else { return }
            // Capture actor-isolated values we might need
            let (client, prevTx, prevMn): (SPVClient?, Double, Double) = await MainActor.run {
                (self.spvClient, self.transactionProgress, self.masternodeProgress)
            }

            // 1) Headers: use detailed current/total from progress callback
            let headerPct = min(1.0, max(0.0, Double(progress.currentHeight) / Double(max(1, progress.targetHeight))))

            // 2) Filters: prefer snapshot lastSyncedFilterHeight / headerHeight; fallback to stats ratio
            var txPct = prevTx
            if let snap = client?.getSyncSnapshot(), snap.headerHeight > 0 {
                txPct = min(1.0, max(0.0, Double(snap.lastSyncedFilterHeight) / Double(snap.headerHeight)))
            } else if let stats = client?.getStats(), stats.headerHeight > 0 {
                txPct = min(1.0, max(0.0, Double(stats.filterHeight) / Double(stats.headerHeight)))
            }

            // 3) Masternodes: show only synced/unsynced (no misleading ratio)
            var mnPct = prevMn
            if let snap = client?.getSyncSnapshot() {
                mnPct = snap.masternodesSynced ? 1.0 : 0.0
            }

            await MainActor.run {
                self.headerProgress = headerPct
                self.transactionProgress = txPct
                self.masternodeProgress = mnPct
            }
        }
    }
    
    nonisolated public func spvClient(_ client: SPVClient, didReceiveBlock block: SPVBlockEvent) {
        if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
            print("📦 New block: height=\(block.height)")
        }
    }
    
    nonisolated public func spvClient(_ client: SPVClient, didReceiveTransaction transaction: SPVTransactionEvent) {
        if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
            print("💰 New transaction: \(transaction.txid.hexString) - amount=\(transaction.amount)")
        }
        
        // Update transactions and balance
        Task { @MainActor in
            await loadTransactions()
            updateBalance()
        }
    }
    
    nonisolated public func spvClient(_ client: SPVClient, didCompleteSync success: Bool, error: String?) {
        Task { @MainActor in
            isSyncing = false
            
            if success {
                if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
                    print("✅ Sync completed successfully")
                }
            } else {
                if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
                    print("❌ Sync failed: \(error ?? "Unknown error")")
                }
                lastSyncError = SPVError.syncFailed(error ?? "Unknown error")
            }
        }
    }
    
    nonisolated public func spvClient(_ client: SPVClient, didChangeConnectionStatus connected: Bool, peers: Int) {
        if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
            print("🌐 Connection status: \(connected ? "Connected" : "Disconnected") - \(peers) peers")
        }
    }
    
    private func mapSyncStage(_ stage: SPVSyncStage) -> SyncStage {
        switch stage {
        case .idle:
            return .idle
        case .headers:
            return .headers
        case .masternodes:
            return .filters
        case .transactions:
            return .downloading
        case .complete:
            return .complete
        }
    }
}

// SyncProgress is now defined in SPVClient.swift
// But we need to keep the old SyncProgress for compatibility
public struct SyncProgress {
    public let current: UInt64
    public let total: UInt64
    public let rate: Double
    public let progress: Double
    public let stage: SyncStage
}

public enum SyncStage {
    case idle
    case connecting
    case headers
    case filters
    case downloading
    case complete
}

// Extension for Data to hex string
extension Data {
    var hexString: String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
