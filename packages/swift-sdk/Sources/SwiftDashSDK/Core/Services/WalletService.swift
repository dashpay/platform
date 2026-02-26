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
        
        SDK.initializeSPVLogging(level: SDK.LogLevel.info, enableConsole: true, maxFiles: 5)

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
    @Published public private(set) var syncProgress: SPVSyncProgress = SPVSyncProgress.default()
    @Published var currentWallet: HDWallet? // Placeholder - use WalletManager instead
    @Published public var balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
    @Published public var masternodesEnabled = true
    
    // Absolute heights for header sync display (current/target)
    @Published public var blocksHit: Int = 0
    @Published public var lastSyncError: Error?

    private var activeSyncStartTimestamp: TimeInterval = 0
    @Published public var transactions: [CoreTransaction] = [] // Use HDTransaction from wallet
    @Published var currentNetwork: AppNetwork = .testnet
    
    // Internal properties
    private var modelContainer: ModelContainer?
    
    // Exposed for WalletViewModel - read-only access to the properly initialized WalletManager
    public private(set) var walletManager: CoreWalletManager?
    
    // SPV Client - new wrapper with proper sync support
    private var spvClient: SPVClient?

    // Mock SDK for now - will be replaced with real SDK
    private var sdk: Any?

    private init() {}
    
    deinit {
        // Avoid capturing self across an async boundary; capture the client locally
        guard let client = spvClient else { return }
        Task { @MainActor in
            client.stopSync()
            client.destroy()
        }
    }
    
    public func configure(modelContainer: ModelContainer, network: AppNetwork = .testnet) {
        LoggingPreferences.configure()
        SDKLogger.log("=== WalletService.configure START ===", minimumLevel: .medium)
        self.modelContainer = modelContainer
        self.currentNetwork = network
        SDKLogger.log("ModelContainer set: \(modelContainer)", minimumLevel: .high)
        SDKLogger.log("Network set: \(network.rawValue)", minimumLevel: .medium)

        initializeNewSPVClient()
        
        SDKLogger.log("Loading current wallet...", minimumLevel: .medium)
        
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
        
        SDKLogger.log("=== WalletService.configure END ===", minimumLevel: .medium)
    }

    public func setSharedSDK(_ sdk: Any) {
        self.sdk = sdk
        SDKLogger.log("✅ WalletService configured with shared SDK", minimumLevel: .medium)
    }
    
    private func initializeNewSPVClient() {
      SDKLogger.log("Initializing SPV Client for \(self.currentNetwork.rawValue)...", minimumLevel: .medium)
      
      let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").appendingPathComponent(self.currentNetwork.rawValue).path
      // Currently always starting at 0 for simplicity. While this is
      // currently configurable, the SPVClient should decide using the wallet
      // creation time to determine the start height, removing usage complexity
      // and possible missusage errors
      let startHeight: UInt32 = 0
      let net = currentNetwork
      
      SDKLogger.log("[SPV][Baseline] Using baseline startFromHeight=\(startHeight) on \(net.rawValue) during initialize()", minimumLevel: .high)
      
      do {
          // This ensures no memory leaks when creating a new client
          // and unlocks the storage in case we are about to use the same (we are)
          if self.spvClient != nil {
            self.spvClient!.destroy()
          }
          
          spvClient = try SPVClient(
              network: self.currentNetwork.sdkNetwork,
              dataDir: dataDir,
              startHeight: startHeight,
          )
          
          spvClient?.setProgressUpdateEventHandler(SPVProgressUpdateEventHandlerImpl(walletService: self))
          spvClient?.setSyncEventsHandler(SPVSyncEventsHandlerImpl(walletService: self))
          spvClient?.setNetworkEventsHandler(SPVNetworkEventsHandlerImpl(walletService: self))
          spvClient?.setWalletEventsHandler(SPVWalletEventsHandlerImpl(walletService: self))
          
          try spvClient?.setMasternodeSyncEnabled(self.masternodesEnabled)
      } catch {
          SDKLogger.error("Failed to initialize SPV Client: \(error)")
          self.lastSyncError = error
          return
      }
      
      SDKLogger.log("✅ SPV Client initialized successfully for \(net.rawValue) (deferred start)", minimumLevel: .medium)
      
      // Capture current references on the main actor to avoid cross-actor hops later
      guard let client = spvClient, let mc = self.modelContainer else { return }
      
      // Create the SDK wallet manager by reusing the SPV client's shared manager
      do {
          let sdkWalletManager = try client.getWalletManager()
          let wrapper = try CoreWalletManager(sdkWalletManager: sdkWalletManager, modelContainer: mc)
          self.walletManager = wrapper
          self.walletManager?.transactionService = TransactionService(
              walletManager: wrapper,
              modelContainer: mc,
              spvClient: client
          )
          SDKLogger.log("✅ WalletManager wrapper initialized successfully", minimumLevel: .medium)
      } catch {
          SDKLogger.error("❌ Failed to initialize WalletManager wrapper:\nError: \(error)")
      }
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

    // Placeholder for balance update logic (i think events manage
    // this but have to confirm)
    private func updateBalance() {}
    
    // MARK: - Trusted Mode / Masternode Sync
    public func setMasternodesEnabled(_ enabled: Bool) {
        masternodesEnabled = enabled
        
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
        guard let spvClient = spvClient else {
            print("❌ SPV Client not initialized")
            return
        }
        
        lastSyncError = nil

        do {
            try await spvClient.startSync()
        } catch {
            self.lastSyncError = error
            print("❌ Sync failed: \(error)")
        }
    }
    
    public func stopSync() {
      // pausing and resuming is not supported so, the trick is the following, 
      // stop the old client and create a new one in its initial state xd
      guard let client = spvClient else { return }

      client.stopSync()
      
      initializeNewSPVClient()
      
    }

    public func clearSpvStorage() {
        if syncProgress.state.isRunning() {
            print("[SPV][Clear] Sync task is running, cannot clear storage")
            return
        }
        
        guard let spvClient = spvClient else { return }


        print("[SPV][Clear] Starting storage clear operation...")

        do {
            // Fun fact and maybe a TODO, when SPVClient is initialized it is also 
            // connected to the network, that way we can get information from there,
            // eg targetHeight, but clear storage stops that connection so we have to
            // create a new client to reestablish the connection and be able to call
            // startSync. Solving this relyies on the DashSPV maintainers if 
            // it's possible to be solved
            try spvClient.clearStorage()
            self.initializeNewSPVClient()

            print("[SPV][Clear] Storage cleared successfully")
        } catch {
            self.lastSyncError = error
            print("❌ Failed to clear SPV storage: \(error)")
        }
    }
    
    // MARK: - Network Management

    public func switchNetwork(to network: AppNetwork) async {
        guard network != currentNetwork else { return }
        currentNetwork = network
        
        print("=== WalletService.switchNetwork START ===")
        print("Switching from \(currentNetwork.rawValue) to \(network.rawValue)")

        self.stopSync() 
        
        // Clear current wallet manager
        walletManager = nil
        currentWallet = nil
        transactions = []
        balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
        
        // Reconfigure with new network
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
    
    // MARK: - SPV Event Handlers implementations
    
    internal final class SPVProgressUpdateEventHandlerImpl: SPVProgressUpdateEventHandler, Sendable {
        private let walletService: WalletService
    
        init(walletService: WalletService) {
            self.walletService = walletService
        }
    
        func onProgressUpdate(_ progress: SPVSyncProgress) {
            Task { @MainActor in
                walletService.syncProgress = progress
            }
        }
    }
    
    internal final class SPVSyncEventsHandlerImpl: SPVSyncEventsHandler, Sendable {
        private let walletService: WalletService
    
        init(walletService: WalletService) {
            self.walletService = walletService
        }
    
        func onStart(_ manager: SPVSyncManager) {
            SDKLogger.log("Sync started for manager: \(manager)", minimumLevel: .medium)
        }
    
        func onComplete(_ headerTip: UInt32, _ cycle: UInt32) {
            Task { @MainActor in
                SDKLogger.log("Sync completed, header tip: \(headerTip), cycle: \(cycle)", minimumLevel: .medium)
    
                if let wm = walletService.walletManager {
                    for wallet in wm.wallets {
                        await wm.syncWalletStateFromRust(for: wallet)
                    }
                }
            }
        }
    
        func onBlockHeadersStored(_ tipHeight: UInt32) {}
        func onBlockHeadersSyncCompleted(_ tipHeight: UInt32) {}
        func onFilterHeadersStored(_ startHeight: UInt32, _ endHeight: UInt32, _ tipHeight: UInt32) {}
        func onFilterHeadersSyncCompleted(_ tipHeight: UInt32) {}
        func onFilterStored(_ startHeight: UInt32, _ endHeight: UInt32) {}
        func onFilterSyncCompleted(_ tipHeight: UInt32) {}
        func onBlocksNeeded(_ height: UInt32, _ hash: Data, _ count: UInt32) {}
        func onBlocksProcessed(_ height: UInt32, _ hash: Data, _ newAddressCount: UInt32) {
            Task { @MainActor in
                walletService.blocksHit += 1
            }
        }
        func onMasternodeStateUpdated(_ height: UInt32) {}
        func onChainLockReceived(_ height: UInt32, _ hash:  Data, _ signature: Data, _ validated: Bool) {}
        func onInstantLockReceived(_ txid: Data, _ instantLockData: Data, _ validated: Bool) {}
        func onSyncManagerError(_ manager: SPVSyncManager, _ errorMsg: String) {
            SDKLogger.error("Sync manager \(manager) error: \(errorMsg)")
            
            Task { @MainActor in
                walletService.lastSyncError = SPVError.syncFailed(errorMsg)
            }
        }
    }
 
    internal final class SPVNetworkEventsHandlerImpl: SPVNetworkEventsHandler, Sendable {
        private let walletService: WalletService
    
        init(walletService: WalletService) {
            self.walletService = walletService
        }
    
        func onPeerConnected(_ address: String) {
            SDKLogger.log("Peer connected: \(address)", minimumLevel: .high)
        }
    
        func onPeerDisconnected(_ address: String) {
            SDKLogger.log("Peer disconnected: \(address)", minimumLevel: .high)
        }
    
        func onPeersUpdated(_ connectedCount: UInt32, _ bestHeight: UInt32) {
            SDKLogger.log("Peers updated: \(connectedCount) connected, best height: \(bestHeight)", minimumLevel: .medium)
        }
    }

    internal final class SPVWalletEventsHandlerImpl: SPVWalletEventsHandler, Sendable {
        private let walletService: WalletService
    
        init(walletService: WalletService) {
            self.walletService = walletService
        }
    
        func onTransactionReceived(
            _ walletId: String,
            _ accountIndex: UInt32,
            _ txid: Data,
            _ amount: Int64,
            _ addresses: [String]
        ) {
            Task { @MainActor in
                if let wm = walletService.walletManager {
                    for wallet in wm.wallets {
                        await wm.syncWalletStateFromRust(for: wallet)
                    }
                }
    
                if walletService.currentWallet != nil {
                    await walletService.loadTransactions()
                }
            }
        }
    
        func onBalanceUpdated(
            _ walletId: String,
            _ spendable: UInt64,
            _ unconfirmed: UInt64,
            _ immature: UInt64,
            _ locked: UInt64
        ) {
            Task { @MainActor in
              walletService.balance = Balance(
                  confirmed: spendable,
                  unconfirmed: unconfirmed,
                  immature: immature
              )
            }
        }
    }
}

// MARK: - SPVEventHandler

// Extension for Data to hex string
extension Data {
    public var hexString: String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
