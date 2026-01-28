import Foundation
import SwiftData
import Combine

// MARK: - Timeout Helper

struct TimeoutError: Error {}

func withTimeout<T: Sendable>(seconds: TimeInterval, operation: @escaping @Sendable () async throws -> T) async throws -> T {
    try await withThrowingTaskGroup(of: T.self) { group in
        // Add the actual operation
        group.addTask {
            try await operation()
        }

        // Add timeout task
        group.addTask {
            try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
            throw TimeoutError()
        }

        // Return first result (either completion or timeout)
        let result = try await group.next()!
        group.cancelAll()
        return result
    }
}

// MARK: - Logging Preferences

public enum LoggingPreset: String {
    case low
    case medium
    case high

    var priority: Int {
        switch self {
        case .low: return 0
        case .medium: return 1
        case .high: return 2
        }
    }

    func allows(_ threshold: LoggingPreset) -> Bool {
        priority >= threshold.priority
    }
}

enum LoggingPreferences {
    private static let defaultsKey = "SwiftSDKLogLevel"

    @discardableResult
    @MainActor
    static func configure() -> LoggingPreset {
        let preset = loadPreset()
        let enableSwiftVerbose: Bool

        switch preset {
        case .high:
            enableSwiftVerbose = true
        case .medium:
            enableSwiftVerbose = false
        case .low:
            enableSwiftVerbose = false
        }

        setenv("SPV_SWIFT_LOG", enableSwiftVerbose ? "1" : "0", 1)

        return preset
    }

    static var preset: LoggingPreset { loadPreset() }

    static var shouldEmitDefaultLogs: Bool { preset == .high }

    static func allows(_ threshold: LoggingPreset) -> Bool {
        preset.allows(threshold)
    }

    private static func loadPreset() -> LoggingPreset {
        if let stored = UserDefaults.standard.string(forKey: defaultsKey)?.lowercased(),
           let preset = LoggingPreset(rawValue: stored) {
            return preset
        }
        return .low
    }
}

public enum SDKLogger {
    public static func log(_ message: String, minimumLevel level: LoggingPreset = .medium) {
        guard LoggingPreferences.allows(level) else { return }
        Swift.print(message)
    }

    public static func error(_ message: String) {
        Swift.print(message)
    }
}

func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
    let output = items.map { String(describing: $0) }.joined(separator: separator)
    let lowercased = output.lowercased()
    let shouldAlwaysPrint = output.contains("❌") || output.contains("⚠️") || lowercased.contains("error")

    guard LoggingPreferences.shouldEmitDefaultLogs || shouldAlwaysPrint else { return }
    Swift.print(output, terminator: terminator)
}

@MainActor
public class WalletService: ObservableObject {
    // Sendable wrapper to move non-Sendable references across actor boundaries when safe
    private final class SendableBox<T>: @unchecked Sendable { let value: T; init(_ v: T) { self.value = v } }
    public static let shared = WalletService()
    
    // Published properties
    @Published var currentWallet: HDWallet? // Placeholder - use WalletManager instead
    @Published public var balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
    @Published public var isSyncing = false
    @Published public var stage: SPVSyncStage = .idle
    // Absolute heights for header sync display (current/target)
    @Published public var headerCurrentHeight: Int = 0
    @Published public var headerTargetHeight: Int = 0
    @Published public var blocksHit: Int = 0
    @Published public var lastSyncError: Error?

    private var activeSyncStartTimestamp: TimeInterval = 0
    @Published public var transactions: [CoreTransaction] = [] // Use HDTransaction from wallet
    @Published var currentNetwork: AppNetwork = .testnet
    
    // Internal properties
    private var modelContainer: ModelContainer?
    private var syncTask: Task<Void, Never>?
    private var balanceUpdateTask: Task<Void, Never>?
    // Stats polling removed (progress is event-driven)
    private var isClearingStorage = false
    @Published public var isInitializing = false
    
    // Exposed for WalletViewModel - read-only access to the properly initialized WalletManager
    public private(set) var walletManager: CoreWalletManager?
    
    // SPV Client - new wrapper with proper sync support
    private var spvClient: SPVClient?

    // Mock SDK for now - will be replaced with real SDK
    private var sdk: Any?
    // Latest sync stats (for UI)
    @Published public var latestHeaderHeight: Int = 0
    @Published public var latestFilterHeaderHeight: Int = 0
    @Published public var latestFilterHeight: Int = 0
    @Published public var latestMasternodeListHeight: Int = 0 // TODO: fill when FFI exposes
    // Control whether to sync masternode list (default false; enable only in non-trusted mode)
    @Published public var shouldSyncMasternodes: Bool = false

    // Expose SPV client for filter match queries
    public var spvClientHandle: UnsafeMutablePointer<FFIDashSpvClient>? {
        spvClient?.clientHandle
    }

    /// Returns the expected chain tip for the current network based on wall-clock time.
    private func expectedChainTipHeight() -> Int? {
        switch currentNetwork {
        case .testnet:
            var calendar = Calendar(identifier: .gregorian)
            calendar.timeZone = TimeZone(secondsFromGMT: 0) ?? calendar.timeZone
            guard let anchor = calendar.date(from: DateComponents(year: 2025, month: 9, day: 24)) else { return nil }
            let today = Date()
            let days = max(0, calendar.dateComponents([.day], from: anchor, to: today).day ?? 0)
            return 1_332_564 + (576 * days)
        default:
            return nil
        }
    }

    /// Normalizes raw tip heights reported by the SPV client so the UI presents realistic hints.
    /// When the RPC returns absolute heights inflated by the checkpoint baseline, we fold them back
    /// towards the expected tip to avoid showing impossible denominators.
    fileprivate func normalizedChainTip(_ rawTip: Int, baseline: Int) -> Int {
        guard baseline > 0, let expected = expectedChainTipHeight() else { return rawTip }

        if abs(rawTip - expected) <= 100_000 {
            return rawTip
        }

        let candidate = rawTip - baseline
        if candidate > 0, abs(candidate - expected) <= 100_000 {
            return candidate
        }

        return rawTip
    }

    private init() {}
    
    deinit {
        // Avoid capturing self across an async boundary; capture the client locally
        let client = spvClient
        Task { @MainActor in
            client?.stop()
        }
    }
    
    public func configure(modelContainer: ModelContainer, network: AppNetwork = .testnet) {
        LoggingPreferences.configure()
        SDKLogger.log("=== WalletService.configure START ===", minimumLevel: .medium)
        self.modelContainer = modelContainer
        self.currentNetwork = network
        SDKLogger.log("ModelContainer set: \(modelContainer)", minimumLevel: .high)
        SDKLogger.log("Network set: \(network.rawValue)", minimumLevel: .medium)

        // Initialize SPV Client wrapper
        SDKLogger.log("Initializing SPV Client for \(network.rawValue)...", minimumLevel: .medium)
        spvClient = SPVClient(network: network.sdkNetwork)
        spvClient?.delegate = self
        
        // Capture current references on the main actor to avoid cross-actor hops later
        guard let client = spvClient, let mc = self.modelContainer else { return }
        let clientBox = SendableBox(client)
        let net = currentNetwork
        let mnEnabled = shouldSyncMasternodes

        // Mark as initializing
        isInitializing = true

        Task.detached(priority: .userInitiated) {
            let clientLocal = clientBox.value
            do {
                // Initialize the SPV client with proper configuration
                let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").path
                // Determine baseline from stored per-wallet per-network sync-from heights
                let baseline: UInt32 = await MainActor.run {
                    self.computeNetworkBaselineSyncFromHeight()
                }
                SDKLogger.log("[SPV][Baseline] Using baseline startFromHeight=\(baseline) on \(net.rawValue) during initialize()", minimumLevel: .high)

                try await clientLocal.initialize(dataDir: dataDir, masternodesEnabled: mnEnabled, startHeight: baseline)
                SDKLogger.log("✅ SPV Client initialized successfully for \(net.rawValue) (deferred start)", minimumLevel: .medium)

                // Read any persisted sync state from storage (heights, targets) and surface it to the UI
                await MainActor.run {
                    let snapshot = clientLocal.getSyncSnapshot()
                    let tip = clientLocal.getTipHeight()
                    let checkpoint = clientLocal.getLatestCheckpointHeight()
                    let stats = clientLocal.getStats()

                    WalletService.shared.applyInitialSyncState(
                        baseline: Int(baseline),
                        tip: tip,
                        checkpoint: checkpoint,
                        snapshot: snapshot,
                        stats: stats
                    )

                    if WalletService.shared.latestHeaderHeight == 0,
                       let cp = checkpoint ?? tip {
                        WalletService.shared.latestHeaderHeight = Int(cp)
                    }

                    // Update blocks hit from persistent wallet transaction data
                    // This uses the wallet's stored transactions, not ephemeral sync stats
                    if let spvClient = WalletService.shared.spvClient {
                        let persistentBlocksHit = spvClient.getBlocksWithTransactionsCount()
                        WalletService.shared.blocksHit = Int(min(persistentBlocksHit, UInt64(Int.max)))
                    }
                }

                // Create the SDK wallet manager by reusing the SPV client's shared manager
                do {
                    try await MainActor.run {
                        let sdkWalletManager = try clientLocal.makeSharedWalletManager()
                        let wrapper = try CoreWalletManager(sdkWalletManager: sdkWalletManager, modelContainer: mc)
                        WalletService.shared.walletManager = wrapper
                        WalletService.shared.walletManager?.transactionService = TransactionService(
                            walletManager: wrapper,
                            modelContainer: mc,
                            spvClient: clientLocal
                        )
                        SDKLogger.log("✅ WalletManager wrapper initialized successfully", minimumLevel: .medium)
                    }
                } catch {
                    SDKLogger.error("❌ Failed to initialize WalletManager wrapper:\nError: \(error)")
                }

                // Mark initialization as complete
                await MainActor.run {
                    WalletService.shared.isInitializing = false
                    SDKLogger.log("✅ SPV Client initialization complete", minimumLevel: .medium)
                }
            } catch {
                SDKLogger.error("❌ Failed to initialize SPV Client: \(error)")
                await MainActor.run {
                    WalletService.shared.lastSyncError = error
                    WalletService.shared.isInitializing = false
                }
            }
        }
        
        SDKLogger.log("Loading current wallet...", minimumLevel: .medium)
        loadCurrentWallet()
        SDKLogger.log("=== WalletService.configure END ===", minimumLevel: .medium)
    }

    public func setSharedSDK(_ sdk: Any) {
        self.sdk = sdk
        SDKLogger.log("✅ WalletService configured with shared SDK", minimumLevel: .medium)
    }
    
    
    // MARK: - Wallet Management

    public func createWallet(label: String, mnemonic: String? = nil, pin: String = "1234", isImport: Bool = false) async throws -> HDWallet {
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
                mnemonic: mnemonic,
                pin: pin,
                isImport: isImport
            )
            
            print("Wallet created by WalletManager, ID: \(wallet.id)")
            print("Loading wallet...")
            
            // Load the newly created wallet
            await loadWallet(wallet)

            // Set per-network sync-from heights
            // Imported wallets: mainnet=730000, testnet=0, devnet=0
            // New wallets: use current known tip for the selected network (fallback to latestHeaderHeight/checkpoint)
            let isImported = isImport
            if isImported {
                // Imported wallet: use fixed per-network baselines
                wallet.syncBaseHeight = currentNetwork == .mainnet ? 730_000 : 0;
            } else {
                // New wallet: use the latest checkpoint height of that chain
                switch currentNetwork {
                case .mainnet:
                    let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 0)) ?? 0
                    print("[WalletService] New wallet baseline mainnet checkpoint=\(cp)")
                    wallet.syncBaseHeight = Int(cp)
                case .testnet:
                    let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 1)) ?? 0
                    print("[WalletService] New wallet baseline testnet checkpoint=\(cp)")
                    wallet.syncBaseHeight = Int(cp)
                case .regtest:
                    let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 2)) ?? 0
                    print("[WalletService] New wallet baseline regtest checkpoint=\(cp)")
                    wallet.syncBaseHeight = Int(cp)
                case .devnet:
                    let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 3)) ?? 0
                    print("[WalletService] New wallet baseline devnet checkpoint=\(cp)")
                    wallet.syncBaseHeight = Int(cp)
                }
            }

            // Persist sync-from changes
            try modelContainer?.mainContext.save()
            
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
        guard !isClearingStorage else {
            print("[SPV][Start] Skipping startSync while a storage clear is in progress")
            return
        }
        guard let spvClient = spvClient else {
            print("❌ SPV Client not initialized")
            return
        }

        // Load persistent blocks hit count from wallet on startup
        let persistentBlocksHit = spvClient.getBlocksWithTransactionsCount()
        if persistentBlocksHit > 0 {
            blocksHit = Int(min(persistentBlocksHit, UInt64(Int.max)))
            print("[SPV][Wallet] Restored \(blocksHit) blocks with transactions from persistent storage")
        }

        // Compute baseline from all wallets on the active network and apply before starting
        let baseline: UInt32 = computeNetworkBaselineSyncFromHeight()
        do {
            try spvClient.setStartFromHeight(baseline)
            print("[SPV][Baseline] StartFromHeight applied=\(baseline) for \(currentNetwork.rawValue) before startSync()")
            // Also print per-wallet values for debugging
            logPerWalletSyncFromHeights()
        } catch {
            print("[SPV][Config] Failed to set StartFromHeight: \(error)")
        }

        isSyncing = true
        lastSyncError = nil

        // Capture references on MainActor
        let serviceBox = SendableBox(self)
        let clientBox = SendableBox(spvClient)
        syncTask = Task.detached(priority: .userInitiated) {
            let service = serviceBox.value
            let client = clientBox.value
            defer {
                Task { @MainActor in service.syncTask = nil }
            }

            if Task.isCancelled { return }

            do {
                // Ensure the underlying client is started (connected) before syncing
                let connected = await client.isConnected
                if connected == false {
                    if Task.isCancelled { return }
                    do {
                        try await client.start()
                        if Task.isCancelled { return }
                        print("[SPV] Client started (connected) before sync")
                    } catch {
                        await MainActor.run {
                            service.lastSyncError = error
                            service.isSyncing = false
                        }
                        print("❌ Failed to start client: \(error)")
                        return
                    }
                }

                if Task.isCancelled { return }
                try await client.startSync()
            } catch {
                await MainActor.run {
                    service.lastSyncError = error
                    service.isSyncing = false
                }
                print("❌ Sync failed: \(error)")
            }
        }
    }
    
    public func stopSync() {
        guard isSyncing else { return }

        syncTask?.cancel()
        syncTask = nil

        if let client = spvClient {
            let snapshotBefore = client.getSyncSnapshot()
            let statsBefore = client.getStats()
            let tip = client.getTipHeight()

            client.stopSync()

            let baseline = Int(computeNetworkBaselineSyncFromHeight())
            let checkpoint = client.getLatestCheckpointHeight()
            let statsAfter = client.getStats() ?? statsBefore
            applyInitialSyncState(
                baseline: baseline,
                tip: tip,
                checkpoint: checkpoint,
                snapshot: snapshotBefore,
                stats: statsAfter
            )
        }

        isSyncing = false
    }

    /// Clear SPV persistence either fully (headers, filters, state) or just the sync snapshot.
    public func clearSpvStorage() {
        guard !isClearingStorage else {
            print("[SPV][Clear] Clear already in progress, ignoring duplicate request")
            return
        }
        guard let spvClient = spvClient else { return }

        isClearingStorage = true
        stopSync()

        let clientBox = SendableBox(spvClient)
        let serviceBox = SendableBox(self)

        Task.detached(priority: .userInitiated) {
            let client = clientBox.value
            let service = serviceBox.value

            print("[SPV][Clear] Starting storage clear operation...")

            do {
                // Add timeout protection
                try await withTimeout(seconds: 30) {
                    try await client.clearStorage()
                }

                print("[SPV][Clear] Storage cleared successfully")

                await MainActor.run {
                    service.resetAfterClearingStorage()
                }
            } catch is TimeoutError {
                print("❌ [SPV][Clear] Timeout waiting for storage clear - client may be busy")
                await MainActor.run {
                    service.lastSyncError = SPVError.storageOperationFailed("Clear operation timed out. Try stopping sync first.")
                }
            } catch {
                await MainActor.run {
                    service.lastSyncError = error
                }
                print("❌ Failed to clear SPV storage: \(error)")
            }

            await MainActor.run {
                service.isClearingStorage = false
            }
        }
    }

    private func resetAfterClearingStorage() {
        let baseline = Int(computeNetworkBaselineSyncFromHeight())
        applyInitialSyncState(baseline: baseline, tip: nil, checkpoint: nil, snapshot: nil)

        latestHeaderHeight = 0
        latestMasternodeListHeight = 0
        blocksHit = 0
        lastSyncError = nil

        print("[SPV][Clear] Completed full storage reset for \(currentNetwork.rawValue)")
    }
    
    // MARK: - Network Management

    public func switchNetwork(to network: AppNetwork) async {
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

        // Remove wallet from observable state BEFORE SwiftData delete
        // This prevents "Never access a full future backing data" crash
        if let walletManager = walletManager {
            await walletManager.removeWalletFromObservableState(wallet)

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

// MARK: - SPVClientDelegate

extension WalletService: SPVClientDelegate {
    public func spvClient(_ client: SPVClient, didUpdateSyncProgress progress: SPVSyncProgress) {
        let stage = progress.stage
        let headerCurrent = Int(progress.currentHeight)
        let headerTarget = Int(progress.targetHeight)
        let filterHeaderHeight = Int(progress.filterHeaderHeight)
        let filterHeight = Int(progress.filterHeight)
        
        Task { @MainActor in
            WalletService.shared.stage = stage
            
            WalletService.shared.headerCurrentHeight = headerCurrent
            WalletService.shared.headerTargetHeight = headerTarget
            
            WalletService.shared.latestFilterHeaderHeight = filterHeaderHeight
            WalletService.shared.latestFilterHeight = filterHeight
        }
    }
    
    public func spvClient(_ client: SPVClient, didReceiveBlock block: SPVBlockEvent) {
        SDKLogger.log("📦 New block: height=\(block.height)", minimumLevel: .high)

        // Sync wallet state after processing a block (which may contain relevant transactions)
        Task { @MainActor in
            if let wm = walletManager {
                for wallet in wm.wallets {
                    await wm.syncWalletStateFromRust(for: wallet)
                }
            }
            updateBalance()
        }
    }
    
    public func spvClient(_ client: SPVClient, didReceiveTransaction transaction: SPVTransactionEvent) {
        // Sync wallet state from Rust to SwiftData, then update UI
        Task { @MainActor in
            // Sync ALL wallets from Rust to SwiftData (transaction could belong to any wallet)
            if let wm = walletManager {
                for wallet in wm.wallets {
                    await wm.syncWalletStateFromRust(for: wallet)
                }
            }

            // Then update UI from the now-synchronized SwiftData (if viewing a wallet)
            if currentWallet != nil {
                await loadTransactions()
                updateBalance()
            }
        }
    }
    
    public func spvClient(_ client: SPVClient, didUpdateBlocksHit count: Int) {
        blocksHit = count

        // Sync wallet state periodically during sync (every 50 blocks processed)
        if count > 0 && count % 50 == 0 {
            Task { @MainActor [weak self] in
                guard let self else { return }
                // Sync ALL wallets
                if let wm = self.walletManager {
                    for wallet in wm.wallets {
                        await wm.syncWalletStateFromRust(for: wallet)
                    }
                }
                self.updateBalance()
            }
        }

        Task { @MainActor [weak self] in
            guard let self else { return }

            self.latestFilterHeight = Int(client.syncProgress?.filterHeight ?? 0)
        }
    }
    
    public func spvClient(_ client: SPVClient, didCompleteSync success: Bool, error: String?) {
        Task { @MainActor in
            isSyncing = false

            if success {
                SDKLogger.log("✅ Sync completed successfully", minimumLevel: .medium)

                // Final sync from Rust to SwiftData after sync completes
                if let wm = walletManager {
                    for wallet in wm.wallets {
                        await wm.syncWalletStateFromRust(for: wallet)
                    }
                }
                updateBalance()
            } else {
                SDKLogger.error("❌ Sync failed: \(error ?? "Unknown error")")
                lastSyncError = SPVError.syncFailed(error ?? "Unknown error")
            }
        }
    }
    
    public func spvClient(_ client: SPVClient, didChangeConnectionStatus connected: Bool, peers: Int) {
        SDKLogger.log("🌐 Connection status: \(connected ? "Connected" : "Disconnected") - \(peers) peers", minimumLevel: .high)
    }
}

// MARK: - Baseline Computation & Debug Logging
extension WalletService {
    /// Compute the baseline start-from height across all wallets enabled on the given network.
    /// Defaults: mainnet=730_000, testnet=0, devnet=0 when no wallets are present.
    @MainActor
    func computeNetworkBaselineSyncFromHeight() -> UInt32 {
        let defaults: [AppNetwork: Int] = [.mainnet: 730_000, .testnet: 0, .devnet: 0]
        guard let ctx = modelContainer?.mainContext else {
            return UInt32(defaults[currentNetwork] ?? 0)
        }

        let wallets: [HDWallet] = (try? ctx.fetch(FetchDescriptor<HDWallet>())) ?? []
        // Filter to wallets that include this network
        let perWalletHeights: [Int] = wallets.map { w in
            w.syncBaseHeight
        }

        if let minValue = perWalletHeights.min() {
            return UInt32(minValue)
        }
        return UInt32(defaults[currentNetwork] ?? 0)
    }

    /// Combine the persisted sync snapshot (if available) with the logical baseline so the UI reflects
    /// the real stored progress as soon as the app launches.
    @MainActor
    func applyInitialSyncState(
        baseline: Int,
        tip: UInt32?,
        checkpoint: UInt32?,
        snapshot: SPVSyncSnapshot?,
        stats: SPVStats? = nil
    ) {
        let sanitizedBaseline = max(0, baseline)
        let absoluteHeight: (Int) -> Int = { raw in
            if raw == 0 { return sanitizedBaseline }
            if raw >= sanitizedBaseline { return raw }
            return sanitizedBaseline + raw
        }

        let snapshotHeader = snapshot.map { absoluteHeight(Int($0.headerHeight)) }
        let statsHeader = stats.map { absoluteHeight($0.headerHeight) }
        let headerHeight = max(
            sanitizedBaseline,
            max(snapshotHeader ?? sanitizedBaseline, statsHeader ?? sanitizedBaseline)
        )
        headerCurrentHeight = headerHeight
        if snapshot != nil || stats != nil {
            latestHeaderHeight = max(latestHeaderHeight, headerHeight)
        } else {
            latestHeaderHeight = headerHeight
        }

        let filterHeaderHeightRaw = snapshot.map { absoluteHeight(Int($0.filterHeaderHeight)) }
        let filterHeaderHeight = max(sanitizedBaseline, filterHeaderHeightRaw ?? sanitizedBaseline)
        if snapshot != nil || stats != nil {
            latestFilterHeaderHeight = max(latestFilterHeaderHeight, filterHeaderHeight)
        } else {
            latestFilterHeaderHeight = filterHeaderHeight
        }

        let snapshotFilterRaw = snapshot.map { absoluteHeight(Int($0.lastSyncedFilterHeight)) }
        let statsFilter = stats.map { absoluteHeight($0.filterHeight) }
        let filterHeight = max(
            sanitizedBaseline,
            max(snapshotFilterRaw ?? sanitizedBaseline, statsFilter ?? sanitizedBaseline)
        )
        if snapshot != nil || stats != nil {
            latestFilterHeight = max(latestFilterHeight, filterHeight)
        } else {
            latestFilterHeight = filterHeight
        }

        func absoluteTip(from raw: UInt32?) -> UInt32? {
            guard let raw else { return nil }
            let resolved = absoluteHeight(Int(raw))
            return resolved > 0 ? UInt32(clamping: resolved) : nil
        }

        func absoluteTip(from raw: Int?) -> UInt32? {
            guard let raw else { return nil }
            let resolved = absoluteHeight(raw)
            return resolved > 0 ? UInt32(clamping: resolved) : nil
        }

        let tipCandidates: [UInt32] = [
            absoluteTip(from: tip),
            absoluteTip(from: checkpoint),
            absoluteTip(from: snapshot?.headerHeight),
            absoluteTip(from: stats?.headerHeight)
        ].compactMap { $0 }

        let resolvedTip = tipCandidates.max()

        let resolvedTarget: Int = {
            let tipMax = resolvedTip.map { Int($0) } ?? headerHeight
            let base = max(tipMax, headerHeight)
            if let expected = expectedChainTipHeight() {
                return max(base, expected)
            }
            return base
        }()

        SDKLogger.log(
            "[SPV][Snapshot] baseline=\(sanitizedBaseline) header=\(headerHeight) filterHeader=\(filterHeaderHeight) filters=\(filterHeight) " +
            "resolvedTip=\(resolvedTip.map(String.init) ?? "nil") target=\(resolvedTarget)",
            minimumLevel: .high
        )

        let normalizedTarget = normalizedChainTip(resolvedTarget, baseline: sanitizedBaseline)
        if normalizedTarget > headerTargetHeight {
            headerTargetHeight = normalizedTarget
        }
        if headerTargetHeight < headerHeight {
            headerTargetHeight = headerHeight
        }
    }

    /// Apply baseline heights to the UI counters with an optional known tip.
    @MainActor
    private func applyBaselineHeights(baseline: Int, knownTip: UInt32?) {
        headerCurrentHeight = baseline
        latestFilterHeaderHeight = baseline
        latestFilterHeight = baseline

        if let tip = knownTip, tip > 0 {
            headerTargetHeight = normalizedChainTip(Int(tip), baseline: baseline)
        } else if headerTargetHeight < baseline {
            headerTargetHeight = baseline
        }
    }

    /// Print a concise list of per-wallet sync-from heights for debugging purposes.
    @MainActor
    func logPerWalletSyncFromHeights() {
        guard let ctx = modelContainer?.mainContext else { return }
        let wallets: [HDWallet] = (try? ctx.fetch(FetchDescriptor<HDWallet>())) ?? []
        let items: [(String, Int)] = wallets.compactMap { w in
            return (w.id.uuidString.prefix(8).description, max(0, w.syncBaseHeight))
        }
        let summary = items.map { "\($0.0):\($0.1)" }.joined(separator: ", ")
        print("[SPV][Baseline] Per-wallet sync-from heights: [\(summary)]")
    }
}

// Extension for Data to hex string
extension Data {
    public var hexString: String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
