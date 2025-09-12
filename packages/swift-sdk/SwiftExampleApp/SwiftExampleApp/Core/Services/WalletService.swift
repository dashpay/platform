import Foundation
import SwiftData
import Combine
@preconcurrency import SwiftDashSDK

@MainActor
public class WalletService: ObservableObject {
    // Sendable wrapper to move non-Sendable references across actor boundaries when safe
    private final class SendableBox<T>: @unchecked Sendable { let value: T; init(_ v: T) { self.value = v } }
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
    // Control whether to sync masternode list (default false; enable only in non-trusted mode)
    @Published var shouldSyncMasternodes: Bool = false
    
    private init() {}
    
    deinit {
        // Avoid capturing self across an async boundary; capture the client locally
        let client = spvClient
        Task { @MainActor in
            client?.stop()
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
        let clientBox = SendableBox(client)
        let net = currentNetwork
        let mnEnabled = shouldSyncMasternodes
        Task.detached(priority: .userInitiated) {
            let clientLocal = clientBox.value
            do {
                // Initialize the SPV client with proper configuration
                let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").path
                // Determine a start height based on checkpoint before the oldest (non-imported) wallet
                var startHeight: UInt32? = nil
                do {
                    // Fetch only the fields we need on the main actor, avoid moving PersistentModels across actors
                    let walletInfos: [(createdAt: Date, isImported: Bool, networks: Int)] = try await MainActor.run {
                        let descriptor = FetchDescriptor<HDWallet>()
                        let wallets = try self.modelContainer?.mainContext.fetch(descriptor) ?? []
                        return wallets.map { ($0.createdAt, $0.isImported, Int($0.networks)) }
                    }
                    // Filter to current network
                    let filtered = walletInfos.filter { w in
                        switch net {
                        case .mainnet: return (w.networks & 1) != 0
                        case .testnet: return (w.networks & 2) != 0
                        case .devnet: return (w.networks & 8) != 0
                        }
                    }
                    // Prefer oldest non-imported wallet
                    let candidate = filtered.filter { !$0.isImported }.sorted { $0.createdAt < $1.createdAt }.first
                    if let cand = candidate {
                        let ts = UInt32(cand.createdAt.timeIntervalSince1970)
                        if let h = await client.getCheckpointHeight(beforeTimestamp: ts) {
                            startHeight = h
                        }
                    } else {
                        // Fallback for imported-only
                        switch net {
                        case .mainnet:
                            startHeight = 730_000
                        case .testnet, .devnet:
                            startHeight = 0
                        }
                    }
                } catch {
                    // If fetch fails, fall back per-network
                    switch net {
                    case .mainnet: startHeight = 730_000
                    case .testnet, .devnet: startHeight = 0
                    }
                }

                try await clientLocal.initialize(dataDir: dataDir, masternodesEnabled: mnEnabled, startHeight: startHeight)

                // Start the SPV client
                try await clientLocal.start()
                print("✅ SPV Client initialized and started successfully for \(net.rawValue)")

                // Seed UI with latest checkpoint height if we don't have a header yet
                let seedHeight = await clientLocal.getLatestCheckpointHeight()
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
                            spvClient: clientLocal
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
    
    func createWallet(label: String, mnemonic: String? = nil, pin: String = "1234", network: Network? = nil, networks: [Network]? = nil) async throws -> HDWallet {
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
                pin: pin,
                networks: networks
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
    }
    
    private func loadCurrentWallet() {
        guard modelContainer != nil else { return }
        
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
    public func setMasternodesEnabled(_ enabled: Bool) {
        shouldSyncMasternodes = enabled
        // Try to apply immediately if the client exists
        do { try spvClient?.setMasternodeSyncEnabled(enabled) } catch { /* ignore */ }
    }
    public func disableMasternodeSync() {
        setMasternodesEnabled(false)
    }
    public func enableMasternodeSync() {
        setMasternodesEnabled(true)
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
        Task.detached(priority: .userInitiated) {
            do {
                try await spvClient.startSync()
            } catch {
                await MainActor.run {
                    WalletService.shared.lastSyncError = error
                    WalletService.shared.isSyncing = false
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
        stopSync()
        
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
        spvStatsTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { _ in
            // Call FFI off the main actor to avoid UI stalls
            Task.detached(priority: .utility) {
                let clientBox = await MainActor.run { WalletService.shared[keyPath: \WalletService.spvClient].map(SendableBox.init) }
                guard let client = clientBox?.value else { return }
                guard let stats = await client.getStats() else { return }
                await MainActor.run {
                    // Only overwrite with positive values; keep seeded values otherwise
                    if stats.headerHeight > 0 {
                        WalletService.shared.latestHeaderHeight = max(WalletService.shared.latestHeaderHeight, stats.headerHeight)
                    }
                    if stats.filterHeight > 0 {
                        WalletService.shared.latestFilterHeight = max(WalletService.shared.latestFilterHeight, stats.filterHeight)
                    }
                    // Keep latestMasternodeListHeight as 0 until available
                }
            }
        }
        if let t = spvStatsTimer { RunLoop.main.add(t, forMode: .common) }
    }
}

// MARK: - SPVClientDelegate

@MainActor
extension WalletService: SPVClientDelegate {
    public func spvClient(_ client: SPVClient, didUpdateSyncProgress progress: SPVSyncProgress) {
        // Copy needed values to Sendable primitives to avoid capturing 'progress'
        let startHeight = progress.startHeight
        let currentHeight = progress.currentHeight
        let targetHeight = progress.targetHeight
        let rate = progress.rate
        let stage = progress.stage
        let mappedStage = WalletService.mapSyncStage(stage)
        let overall = progress.overallProgress

        Task { @MainActor in
            let base = Double(startHeight)
            let numer = max(0.0, Double(currentHeight) - base)
            let denom = max(1.0, Double(targetHeight) - base)
            let headerPct = min(1.0, max(0.0, numer / denom))

            WalletService.shared.syncProgress = headerPct
            WalletService.shared.headerProgress = headerPct

            WalletService.shared.detailedSyncProgress = SyncProgress(
                current: UInt64(currentHeight),
                total: UInt64(targetHeight),
                rate: rate,
                progress: headerPct,
                stage: mappedStage
            )

            if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
                print("📊 Sync progress: \(stage.rawValue) - \(Int(overall * 100))%")
            }
        }

        Task.detached(priority: .utility) {
            let (clientBox, prevTx, prevMn): (SendableBox<SPVClient>?, Double, Double) = await MainActor.run {
                (WalletService.shared[keyPath: \WalletService.spvClient].map(SendableBox.init), WalletService.shared.transactionProgress, WalletService.shared.masternodeProgress)
            }
            let client = clientBox?.value

            let base = Double(startHeight)
            let numer = max(0.0, Double(currentHeight) - base)
            let denom = max(1.0, Double(targetHeight) - base)
            let headerPct = min(1.0, max(0.0, numer / denom))

            let txPctFinal: Double
            if let snap = await client?.getSyncSnapshot(), snap.headerHeight > 0 {
                txPctFinal = min(1.0, max(0.0, Double(snap.lastSyncedFilterHeight) / Double(snap.headerHeight)))
            } else if let stats = await client?.getStats(), stats.headerHeight > 0 {
                txPctFinal = min(1.0, max(0.0, Double(stats.filterHeight) / Double(stats.headerHeight)))
            } else {
                txPctFinal = prevTx
            }

            let mnPctFinal: Double
            if let snap = await client?.getSyncSnapshot() {
                mnPctFinal = snap.masternodesSynced ? 1.0 : 0.0
            } else {
                mnPctFinal = prevMn
            }

            await MainActor.run {
                WalletService.shared.headerProgress = headerPct
                WalletService.shared.transactionProgress = txPctFinal
                WalletService.shared.masternodeProgress = mnPctFinal
            }
        }
    }
    
    public func spvClient(_ client: SPVClient, didReceiveBlock block: SPVBlockEvent) {
        if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
            print("📦 New block: height=\(block.height)")
        }
    }
    
    public func spvClient(_ client: SPVClient, didReceiveTransaction transaction: SPVTransactionEvent) {
        if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
            print("💰 New transaction: \(transaction.txid.hexString) - amount=\(transaction.amount)")
        }
        
        // Update transactions and balance
        Task { @MainActor in
            await loadTransactions()
            updateBalance()
        }
    }
    
    public func spvClient(_ client: SPVClient, didCompleteSync success: Bool, error: String?) {
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
    
    public func spvClient(_ client: SPVClient, didChangeConnectionStatus connected: Bool, peers: Int) {
        if ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"] == "1" {
            print("🌐 Connection status: \(connected ? "Connected" : "Disconnected") - \(peers) peers")
        }
    }
    
    nonisolated private static func mapSyncStage(_ stage: SPVSyncStage) -> SyncStage {
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
