import Foundation
import SwiftData
import Combine

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

// DESIGN NOTE: This class feels like something that should be in the example app, 
// we, as sdk developers, provide the tools and ffi wrappers, but how to
// use them depends on the sdk user, for example, by implementing the SPV event 
// handlers, the user can decide what to do with the events, but if we implement them in the sdk
// we are taking that decision for them, and maybe not all users want the same thing
@MainActor
public class WalletService: ObservableObject {
    // Published properties
    @Published public private(set) var syncProgress: SPVSyncProgress = SPVSyncProgress.default()
    @Published public var masternodesEnabled = true
    @Published public var lastSyncError: Error?
    @Published var network: AppNetwork
    
    // Internal properties
    private var modelContainer: ModelContainer
    
    // SPV Client and Wallet wrappers
    private var spvClient: SPVClient
    public private(set) var walletManager: CoreWalletManager

    public init(modelContainer: ModelContainer, network: AppNetwork) {
        self.modelContainer = modelContainer
        self.network = network
        
        LoggingPreferences.configure()
        
        let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").appendingPathComponent(network.rawValue).path
        
        // For simplicity, lets unwrap the error. This can only fail due to
        // IO errors when working with the internal storage system, I don't 
        // see how we can recover from that right now easily
        let spvClient = try! SPVClient(
            network: network.sdkNetwork,
            dataDir: dataDir,
            startHeight: 0,
        )
        
        self.spvClient = spvClient
        
        // Create the SDK wallet manager by reusing the SPV client's shared manager
        // TODO: Investigate this error
        self.walletManager = try! CoreWalletManager(spvClient: spvClient, modelContainer: modelContainer)
        
        spvClient.setProgressUpdateEventHandler(SPVProgressUpdateEventHandlerImpl(walletService: self))
        spvClient.setSyncEventsHandler(SPVSyncEventsHandlerImpl(walletService: self))
        spvClient.setNetworkEventsHandler(SPVNetworkEventsHandlerImpl(walletService: self))
        spvClient.setWalletEventsHandler(SPVWalletEventsHandlerImpl(walletService: self))
    }
    
    deinit {
        spvClient.stopSync()
        spvClient.destroy()
    }
    
    private func initializeNewSPVClient() {
      SDKLogger.log("Initializing SPV Client for \(self.self.network.rawValue)...", minimumLevel: .medium)
      
      let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first?.appendingPathComponent("SPV").appendingPathComponent(self.network.rawValue).path
      
      // This ensures no memory leaks when creating a new client
      // and unlocks the storage in case we are about to use the same (we probably are)
      self.spvClient.destroy()
      
      // For simplicity, lets unwrap the error. This can only fail due to
      // IO errors when working with the internal storage system, I don't 
      // see how we can recover from that right now easily
      self.spvClient = try! SPVClient(
          network: self.self.network.sdkNetwork,
          dataDir: dataDir,
          startHeight: 0,
      )
      
      self.spvClient.setProgressUpdateEventHandler(SPVProgressUpdateEventHandlerImpl(walletService: self))
      self.spvClient.setSyncEventsHandler(SPVSyncEventsHandlerImpl(walletService: self))
      self.spvClient.setNetworkEventsHandler(SPVNetworkEventsHandlerImpl(walletService: self))
      self.spvClient.setWalletEventsHandler(SPVWalletEventsHandlerImpl(walletService: self))
      
      try! self.spvClient.setMasternodeSyncEnabled(self.masternodesEnabled)
      
      SDKLogger.log("✅ SPV Client initialized successfully for \(self.network.rawValue) (deferred start)", minimumLevel: .medium)
      
      // Create the SDK wallet manager by reusing the SPV client's shared manager
      // TODO: Investigate this error
      self.walletManager = try! CoreWalletManager(spvClient: self.spvClient, modelContainer: self.modelContainer)
      
      SDKLogger.log("✅ WalletManager wrapper initialized successfully", minimumLevel: .medium)
    }
    
    // MARK: - Trusted Mode / Masternode Sync
    public func setMasternodesEnabled(_ enabled: Bool) {
        masternodesEnabled = enabled
        
        // Try to apply immediately if the client exists
        do { try spvClient.setMasternodeSyncEnabled(enabled) } catch { /* ignore */ }
    }
    public func disableMasternodeSync() {
        setMasternodesEnabled(false)
    }
    public func enableMasternodeSync() {
        setMasternodesEnabled(true)
    }
    
    // MARK: - Sync Management
    
    public func startSync() async {
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
      spvClient.stopSync()
      
      self.initializeNewSPVClient()
    }

    public func clearSpvStorage() {
        if syncProgress.state.isRunning() {
            print("[SPV][Clear] Sync task is running, cannot clear storage")
            return
        }

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
        guard network != self.network else { return }
        self.network = network
        
        print("=== WalletService.switchNetwork START ===")
        print("Switching from \(self.network.rawValue) to \(network.rawValue)")

        self.stopSync() 
        
        self.initializeNewSPVClient()
        
        print("=== WalletService.switchNetwork END ===")
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
    
        func onComplete(_ headerTip: UInt32) {}
        func onBlockHeadersStored(_ tipHeight: UInt32) {}
        func onBlockHeadersSyncCompleted(_ tipHeight: UInt32) {}
        func onFilterHeadersStored(_ startHeight: UInt32, _ endHeight: UInt32, _ tipHeight: UInt32) {}
        func onFilterHeadersSyncCompleted(_ tipHeight: UInt32) {}
        func onFilterStored(_ startHeight: UInt32, _ endHeight: UInt32) {}
        func onFilterSyncCompleted(_ tipHeight: UInt32) {}
        func onBlocksNeeded(_ height: UInt32, _ hash: Data, _ count: UInt32) {}
        func onBlocksProcessed(_ height: UInt32, _ hash: Data, _ newAddressCount: UInt32) {}
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
        ) {}
    
        func onBalanceUpdated(
            _ walletId: String,
            _ spendable: UInt64,
            _ unconfirmed: UInt64,
            _ immature: UInt64,
            _ locked: UInt64
        ) {}
    }
}

// MARK: - SPVEventHandler

// Extension for Data to hex string
extension Data {
    public var hexString: String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
