import Foundation
import SwiftData
import Combine
@preconcurrency import SwiftDashSDK

// MARK: - Logging Preferences

enum LoggingPreset: String {
    case low
    case medium
    case high

    fileprivate var priority: Int {
        switch self {
        case .low: return 0
        case .medium: return 1
        case .high: return 2
        }
    }

    fileprivate func allows(_ threshold: LoggingPreset) -> Bool {
        priority >= threshold.priority
    }
}

enum LoggingPreferences {
    private static let defaultsKey = "SwiftSDKLogLevel"

    @discardableResult
    @MainActor
    static func configure() -> LoggingPreset {
        let preset = loadPreset()
        let spvLevel: SPVLogLevel
        let enableSwiftVerbose: Bool

        switch preset {
        case .high:
            spvLevel = .trace
            enableSwiftVerbose = true
        case .medium:
            spvLevel = .info
            enableSwiftVerbose = false
        case .low:
            spvLevel = .off
            enableSwiftVerbose = false
        }

        setenv("SPV_SWIFT_LOG", enableSwiftVerbose ? "1" : "0", 1)
        setenv("SPV_LOG", spvLevel.rawValue, 1)
        SPVClient.initializeLogging(spvLevel)

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

enum SDKLogger {
    static func log(_ message: String, minimumLevel level: LoggingPreset = .medium) {
        guard LoggingPreferences.allows(level) else { return }
        Swift.print(message)
    }

    static func error(_ message: String) {
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
    @Published public var syncProgress: Double?
    @Published public var detailedSyncProgress: Any? // Use SPVClient.SyncProgress
    @Published public var headerProgress: Double = 0
    @Published public var masternodeProgress: Double = 0
    @Published public var transactionProgress: Double = 0
    // Absolute heights for header sync display (current/target)
    @Published public var headerCurrentHeight: Int = 0
    @Published public var headerTargetHeight: Int = 0
    @Published public var blocksHit: Int = 0
    @Published public var lastSyncError: Error?
    @Published public var transactions: [CoreTransaction] = [] // Use HDTransaction from wallet
    @Published var currentNetwork: Network = .testnet
    
    // Internal properties
    private var modelContainer: ModelContainer?
    private var syncTask: Task<Void, Never>?
    private var balanceUpdateTask: Task<Void, Never>?
    // Stats polling removed (progress is event-driven)
    private var isClearingStorage = false
    
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

    // Expose base sync height to UI in a safe way
    public var baseSyncHeightUI: UInt32 { spvClient?.baseSyncHeight ?? 0 }

    private init() {}
    
    deinit {
        // Avoid capturing self across an async boundary; capture the client locally
        let client = spvClient
        Task { @MainActor in
            client?.stop()
        }
    }
    
    func configure(modelContainer: ModelContainer, network: Network = .testnet) {
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
        Task.detached(priority: .userInitiated) {
            let clientLocal = clientBox.value
            do {
                // Initialize the SPV client with proper configuration
                let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").path
                // Determine baseline from stored per-wallet per-network sync-from heights
                let baseline: UInt32 = await MainActor.run {
                    self.computeNetworkBaselineSyncFromHeight(for: net)
                }
                SDKLogger.log("[SPV][Baseline] Using baseline startFromHeight=\(baseline) on \(net.rawValue) during initialize()", minimumLevel: .high)

                try await clientLocal.initialize(dataDir: dataDir, masternodesEnabled: mnEnabled, startHeight: baseline)
                SDKLogger.log("✅ SPV Client initialized successfully for \(net.rawValue) (deferred start)", minimumLevel: .medium)

                // Read any persisted sync state from storage (heights, targets) and surface it to the UI
                await MainActor.run {
                    let snapshot = clientLocal.getSyncSnapshot()
                    let tip = clientLocal.getTipHeight()
                    let checkpoint = clientLocal.getLatestCheckpointHeight()

                    WalletService.shared.applyInitialSyncState(
                        baseline: Int(baseline),
                        tip: tip,
                        checkpoint: checkpoint,
                        snapshot: snapshot
                    )

                    if WalletService.shared.latestHeaderHeight == 0,
                       let cp = checkpoint ?? tip {
                        WalletService.shared.latestHeaderHeight = Int(cp)
                    }
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
                        SDKLogger.log("✅ WalletManager wrapper initialized successfully", minimumLevel: .medium)
                    }
                } catch {
                    SDKLogger.error("❌ Failed to initialize WalletManager wrapper:\nError: \(error)")
                }
            } catch {
                SDKLogger.error("❌ Failed to initialize SPV Client: \(error)")
                await MainActor.run { WalletService.shared.lastSyncError = error }
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
    
    func createWallet(label: String, mnemonic: String? = nil, pin: String = "1234", network: Network? = nil, networks: [Network]? = nil, isImport: Bool = false) async throws -> HDWallet {
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

            // Set per-network sync-from heights
            // Imported wallets: mainnet=730000, testnet=0, devnet=0
            // New wallets: use current known tip for the selected network (fallback to latestHeaderHeight/checkpoint)
            let isImported = isImport
            if isImported {
                // Imported wallet: use fixed per-network baselines
                wallet.syncFromMainnet = 730_000
                wallet.syncFromTestnet = 0
                wallet.syncFromDevnet = 0
            } else {
                // New wallet: per selected network, use the latest checkpoint height of that chain
                let nets = networks ?? [walletNetwork]
                for n in nets {
                    switch n {
                    case .mainnet:
                        let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 0)) ?? 0
                        print("[WalletService] New wallet baseline mainnet checkpoint=\(cp)")
                        wallet.syncFromMainnet = Int(cp)
                    case .testnet:
                        let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 1)) ?? 0
                        print("[WalletService] New wallet baseline testnet checkpoint=\(cp)")
                        wallet.syncFromTestnet = Int(cp)
                    case .devnet:
                        let cp = SPVClient.latestCheckpointHeight(forNetwork: .init(rawValue: 2)) ?? 0
                        print("[WalletService] New wallet baseline devnet checkpoint=\(cp)")
                        wallet.syncFromDevnet = Int(cp)
                    }
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
        
        // Compute baseline from all wallets on the active network and apply before starting
        let baseline: UInt32 = computeNetworkBaselineSyncFromHeight(for: currentNetwork)
        do {
            try spvClient.setStartFromHeight(baseline)
            print("[SPV][Baseline] StartFromHeight applied=\(baseline) for \(currentNetwork.rawValue) before startSync()")
            // Also print per-wallet values for debugging
            logPerWalletSyncFromHeights(for: currentNetwork)
        } catch {
            print("[SPV][Config] Failed to set StartFromHeight: \(error)")
        }

        isSyncing = true
        lastSyncError = nil
        
        let serviceBox = SendableBox(self)
        syncTask = Task.detached(priority: .userInitiated) {
            let service = serviceBox.value
            defer {
                Task { @MainActor in service.syncTask = nil }
            }

            if Task.isCancelled { return }

            do {
                // Ensure the underlying client is started (connected) before syncing
                let connected = await spvClient.isConnected
                if connected == false {
                    if Task.isCancelled { return }
                    do {
                        try await spvClient.start()
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
                try await spvClient.startSync()
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
        syncTask?.cancel()
        syncTask = nil
        spvClient?.cancelSync()
        isSyncing = false
        syncProgress = nil
        detailedSyncProgress = nil
    }

    /// Clear SPV persistence either fully (headers, filters, state) or just the sync snapshot.
    public func clearSpvStorage(fullReset: Bool = true) {
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

            do {
                if fullReset {
                    try await client.clearStorage()
                } else {
                    try await client.clearSyncState()
                }

                await MainActor.run {
                    service.resetAfterClearingStorage(fullReset: fullReset)
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

    private func resetAfterClearingStorage(fullReset: Bool) {
        headerProgress = 0
        masternodeProgress = 0
        transactionProgress = 0

        let baseline = Int(computeNetworkBaselineSyncFromHeight(for: currentNetwork))
        applyInitialSyncState(baseline: baseline, tip: nil, checkpoint: nil, snapshot: nil)

        latestHeaderHeight = 0
        latestMasternodeListHeight = 0
        blocksHit = 0
        syncProgress = nil
        detailedSyncProgress = nil
        lastSyncError = nil

        let modeDescription = fullReset ? "full storage" : "sync-state"
        print("[SPV][Clear] Completed \(modeDescription) reset for \(currentNetwork.rawValue)")
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

// MARK: - SPVClientDelegate

extension WalletService: SPVClientDelegate {
    public func spvClient(_ client: SPVClient, didUpdateSyncProgress progress: SPVSyncProgress) {
        // Copy needed values to Sendable primitives to avoid capturing 'progress'
        let startHeight = progress.startHeight
        let currentHeight = progress.currentHeight
        let targetHeight = progress.targetHeight
        let rate = progress.rate
        let stage = progress.stage
        let overall = progress.overallProgress
        let stageRawValue = stage.rawValue
        let mappedStage = WalletService.mapSyncStage(stage)

        Task { @MainActor in
            let base = Double(startHeight)
            let numer = max(0.0, Double(currentHeight) - base)
            let denom = max(1.0, Double(targetHeight) - base)
            let headerPct = min(1.0, max(0.0, numer / denom))

            WalletService.shared.syncProgress = headerPct
            WalletService.shared.headerProgress = headerPct

            let baseHeight = Int(startHeight)
            let absHeader = max(Int(currentHeight), baseHeight)
            let absTarget = max(Int(targetHeight), baseHeight)
            let absFilterRaw = max(Int(progress.filterHeight), baseHeight)
            var absFilter = min(absFilterRaw, absTarget)

            var filterPct = 0.0
            if mappedStage == .downloading || mappedStage == .complete {
                let filterNumerator = max(0.0, Double(absFilter - baseHeight))
                let filterDenominator = max(1.0, Double(absTarget - baseHeight))
                filterPct = min(1.0, filterNumerator / filterDenominator)
            } else {
                absFilter = baseHeight
            }

            WalletService.shared.headerCurrentHeight = absHeader
            WalletService.shared.headerTargetHeight = absTarget
            WalletService.shared.latestFilterHeight = absFilter
            WalletService.shared.transactionProgress = filterPct

            WalletService.shared.detailedSyncProgress = SyncProgress(
                current: UInt64(absHeader),
                total: UInt64(absTarget),
                rate: rate,
                progress: headerPct,
                stage: mappedStage
            )

            SDKLogger.log("📊 Sync progress: \(stageRawValue) - \(Int(overall * 100))%", minimumLevel: .high)
        }

        // Use event-driven transaction progress from SPVClient (no polling fallback)
    }
    
    public func spvClient(_ client: SPVClient, didReceiveBlock block: SPVBlockEvent) {
        SDKLogger.log("📦 New block: height=\(block.height)", minimumLevel: .high)
    }
    
    public func spvClient(_ client: SPVClient, didReceiveTransaction transaction: SPVTransactionEvent) {
        SDKLogger.log("💰 New transaction: \(transaction.txid.hexString) - amount=\(transaction.amount)", minimumLevel: .high)
        
        // Update transactions and balance
        Task { @MainActor in
            await loadTransactions()
            updateBalance()
        }
    }
    
    public func spvClient(_ client: SPVClient, didUpdateBlocksHit count: Int) {
        blocksHit = count
    }
    
    public func spvClient(_ client: SPVClient, didCompleteSync success: Bool, error: String?) {
        Task { @MainActor in
            isSyncing = false
            
            if success {
                SDKLogger.log("✅ Sync completed successfully", minimumLevel: .medium)
            } else {
                SDKLogger.error("❌ Sync failed: \(error ?? "Unknown error")")
                lastSyncError = SPVError.syncFailed(error ?? "Unknown error")
            }
        }
    }
    
    public func spvClient(_ client: SPVClient, didChangeConnectionStatus connected: Bool, peers: Int) {
        SDKLogger.log("🌐 Connection status: \(connected ? "Connected" : "Disconnected") - \(peers) peers", minimumLevel: .high)
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

// MARK: - Baseline Computation & Debug Logging
extension WalletService {
    /// Compute the baseline start-from height across all wallets enabled on the given network.
    /// Defaults: mainnet=730_000, testnet=0, devnet=0 when no wallets are present.
    @MainActor
    func computeNetworkBaselineSyncFromHeight(for network: Network) -> UInt32 {
        let defaults: [Network: Int] = [.mainnet: 730_000, .testnet: 0, .devnet: 0]
        guard let ctx = modelContainer?.mainContext else {
            return UInt32(defaults[network] ?? 0)
        }

        let wallets: [HDWallet] = (try? ctx.fetch(FetchDescriptor<HDWallet>())) ?? []
        // Filter to wallets that include this network
        let filtered = wallets.filter { w in
            switch network {
            case .mainnet: return (w.networks & 1) != 0
            case .testnet: return (w.networks & 2) != 0
            case .devnet:  return (w.networks & 8) != 0
            }
        }
        let perWalletHeights: [Int] = filtered.map { w in
            switch network {
            case .mainnet: return max(0, w.syncFromMainnet)
            case .testnet: return max(0, w.syncFromTestnet)
            case .devnet:  return max(0, w.syncFromDevnet)
            }
        }

        if let minValue = perWalletHeights.min() {
            return UInt32(minValue)
        }
        return UInt32(defaults[network] ?? 0)
    }

    /// Combine the persisted sync snapshot (if available) with the logical baseline so the UI reflects
    /// the real stored progress as soon as the app launches.
    @MainActor
    func applyInitialSyncState(
        baseline: Int,
        tip: UInt32?,
        checkpoint: UInt32?,
        snapshot: SPVSyncSnapshot?
    ) {
        let sanitizedBaseline = max(0, baseline)
        let tipCandidates: [UInt32] = [tip, checkpoint, snapshot?.headerHeight]
            .compactMap { value in
                guard let value, value > 0 else { return nil }
                return value
            }
        let resolvedTip = tipCandidates.max()

        guard let snapshot else {
            applyBaselineHeights(baseline: sanitizedBaseline, knownTip: resolvedTip)
            return
        }

        let headerHeight = max(Int(snapshot.headerHeight), sanitizedBaseline)
        headerCurrentHeight = headerHeight

        let snapshotFilter = max(snapshot.filterHeaderHeight, snapshot.lastSyncedFilterHeight)
        latestFilterHeight = max(Int(snapshotFilter), sanitizedBaseline)

        if let resolvedTip {
            let resolved = Int(max(resolvedTip, UInt32(headerHeight)))
            headerTargetHeight = resolved
        } else if headerTargetHeight < headerHeight {
            headerTargetHeight = headerHeight
        }

        if headerHeight > 0 {
            latestHeaderHeight = headerHeight
        }

        if let resolvedTip, resolvedTip > UInt32(sanitizedBaseline) {
            let denom = max(1, Int(resolvedTip) - sanitizedBaseline)
            let headerNumerator = max(0, headerHeight - sanitizedBaseline)
            headerProgress = min(1.0, Double(headerNumerator) / Double(denom))

            let filterNumerator = max(0, latestFilterHeight - sanitizedBaseline)
            transactionProgress = min(1.0, Double(filterNumerator) / Double(denom))
        }
    }

    /// Apply baseline heights to the UI counters with an optional known tip.
    @MainActor
    private func applyBaselineHeights(baseline: Int, knownTip: UInt32?) {
        headerCurrentHeight = baseline
        latestFilterHeight = baseline

        if let tip = knownTip, tip > 0 {
            headerTargetHeight = Int(tip)
        } else if headerTargetHeight < baseline {
            headerTargetHeight = baseline
        }
    }

    /// Print a concise list of per-wallet sync-from heights for debugging purposes.
    @MainActor
    func logPerWalletSyncFromHeights(for network: Network) {
        guard let ctx = modelContainer?.mainContext else { return }
        let wallets: [HDWallet] = (try? ctx.fetch(FetchDescriptor<HDWallet>())) ?? []
        let items: [(String, Int)] = wallets.compactMap { w in
            // Show only wallets on this network
            let enabled: Bool
            let h: Int
            switch network {
            case .mainnet: enabled = (w.networks & 1) != 0; h = w.syncFromMainnet
            case .testnet: enabled = (w.networks & 2) != 0; h = w.syncFromTestnet
            case .devnet:  enabled = (w.networks & 8) != 0; h = w.syncFromDevnet
            }
            guard enabled else { return nil }
            return (w.id.uuidString.prefix(8).description, max(0, h))
        }
        let summary = items.map { "\($0.0):\($0.1)" }.joined(separator: ", ")
        print("[SPV][Baseline] Per-wallet sync-from heights for \(network.rawValue): [\(summary)]")
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

public enum SyncStage: Sendable {
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
