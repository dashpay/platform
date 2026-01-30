import Foundation
import DashSDKFFI

// MARK: - Logging

public enum SPVLogLevel: String, Sendable {
    case off
    case error
    case warn
    case info
    case debug
    case trace
    case paranoid
}

// MARK: - C Callback Functions
// Use top-level C-compatible functions to avoid actor-isolation init issues

private func spvProgressCallbackC(
    progressPtr: UnsafePointer<FFIDetailedSyncProgress>?,
    userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let spvSyncProgress = ffiSyncProgressPtrIntoSpvSyncProgress(progressPtr)
    
    spvEventHandler.spvClient(didUpdateSyncProgress: spvSyncProgress)
}

private func spvCompletionCallbackC(
    success: Bool,
    errorMsg: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let errorString: String? = errorMsg.map { String(cString: $0) }
 
    spvEventHandler.spvClient(didCompleteSync: success, error: errorString)
}

private typealias Byte32 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

private func onBlockCallbackC(
    _ height: UInt32,
    _ hashPtr: UnsafePointer<Byte32>?,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let hash = byte32PtrIntoData(hashPtr)

    let block = SPVBlockEvent(
        height: height,
        hash: hash,
        timestamp: Date()
    )
    
    spvEventHandler.spvClient(didReceiveBlock: block)
}

private func onTransactionCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ confirmed: Bool,
    _ amount: Int64,
    _ addressesPtr: UnsafePointer<CChar>?,
    _ blockHeight: UInt32,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let txid = byte32PtrIntoData(txidPtr)
    
    let addresses = addressesPtrIntoString(addressesPtr)
    
    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: confirmed,
        amount: amount,
        addresses: addresses,
        blockHeight: blockHeight
    )
    
    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func onCompactFilterMatchedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ scripts: UnsafePointer<CChar>?,
    _ wallet: UnsafePointer<CChar>?,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    spvEventHandler.spvClient(didUpdateBlocksHit: 1)
}

private func onMempoolTransactionAddedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ amount: Int64,
    _ addressPtr: UnsafePointer<CChar>?,
    _ isInstantSend: Bool,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    let txid = byte32PtrIntoData(txidPtr)

    let addresses = addressesPtrIntoString(addressPtr)

    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: false,
        amount: amount,
        addresses: addresses,
        blockHeight: 0
    )
    
    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func onMempoolTransactionConfirmedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ blockHeight: UInt32,
    _ blockHashPtr: UnsafePointer<Byte32>?,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    let txid = byte32PtrIntoData(txidPtr)

    // Amount and addresses are not provided here; emit a confirmation-only update
    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: true,
        amount: 0,
        addresses: [],
        blockHeight: blockHeight
    )
    
    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func onMempoolTransactionRemovedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ reason: UInt8,
    _ userData: UnsafeMutableRawPointer?
) { 
    // Intentionally no-op; could surface to UI in future if needed
}

private func onWalletTransactionCallbackC(
    _ walletId: UnsafePointer<CChar>?,
    _ accountIndex: UInt32,
    _ txidPtr: UnsafePointer<Byte32>?,
    _ confirmed: Bool,
    _ amount: Int64,
    _ addressesPtr: UnsafePointer<CChar>?,
    _ blockHeight: UInt32,
    _ isOurs: Bool,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    let txid = byte32PtrIntoData(txidPtr)
    
    let addresses = addressesPtrIntoString(addressesPtr)

    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: confirmed,
        amount: amount,
        addresses: addresses,
        blockHeight: blockHeight
    )

    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func byte32PtrIntoData(_ ptr: UnsafePointer<Byte32>?) -> Data {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Byte32 pointer is nil!")
        return Data()
    }
    
    return Data(bytes: ptr, count: 32)
}

private func addressesPtrIntoString(_ ptr: UnsafePointer<CChar>?) -> [String] {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Addresses pointer is nil!")
        return [""]
    }
    
    let str = String(cString: ptr)
    return str.components(separatedBy: ",")
}

private func rawPtrIntoSpvEventHandler(_ ptr: UnsafeMutableRawPointer?) -> any SPVEventHandler {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "SPVEventHandler pointer is nil!")
        return DummySPVEventHandler()
    }

    return Unmanaged<AnyObject>.fromOpaque(ptr).takeUnretainedValue() as! any SPVEventHandler
}

public func ffiSyncProgressPtrIntoSpvSyncProgress(_ ptr: UnsafePointer<FFIDetailedSyncProgress>?) -> SPVSyncProgress {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Progress pointer is nil!")
        return SPVSyncProgress.default()
    }
    
    let ffiProgress = ptr.pointee
    let overview = ffiProgress.overview

    return SPVSyncProgress(
        stage: SPVSyncStage(ffiStage: ffiProgress.stage),
        currentHeight: overview.header_height,
        targetHeight: ffiProgress.total_height,
        filterHeaderHeight: overview.filter_header_height,
        filterHeight: overview.last_synced_filter_height,
        syncStartedAt: TimeInterval(ffiProgress.sync_start_timestamp),
        rate: ffiProgress.headers_per_second,
        estimatedTimeRemaining: ffiProgress.estimated_seconds_remaining > 0
            ? TimeInterval(ffiProgress.estimated_seconds_remaining)
            : nil,
        peerCount: overview.peer_count
    )
}

// MARK: - SPV Client event handler

public protocol SPVEventHandler: AnyObject {
    func spvClient(didUpdateSyncProgress progress: SPVSyncProgress)
    func spvClient(didReceiveBlock block: SPVBlockEvent)
    func spvClient(didReceiveTransaction transaction: SPVTransactionEvent)
    func spvClient(didCompleteSync success: Bool, error: String?)
    func spvClient(didChangeConnectionStatus connected: Bool, peers: Int)
    func spvClient(didUpdateBlocksHit count: Int)
}

private class DummySPVEventHandler: SPVEventHandler {
    func spvClient(didUpdateSyncProgress progress: SPVSyncProgress) {}
    func spvClient(didReceiveBlock block: SPVBlockEvent) {}
    func spvClient(didReceiveTransaction transaction: SPVTransactionEvent) {}
    func spvClient(didCompleteSync success: Bool, error: String?) {}
    func spvClient(didChangeConnectionStatus connected: Bool, peers: Int) {}
    func spvClient(didUpdateBlocksHit count: Int) {}
}

// MARK: - SPV Sync Progress

public struct SPVSyncProgress: Sendable {
    public let stage: SPVSyncStage
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let filterHeaderHeight: UInt32
    public let filterHeight: UInt32
    public let syncStartedAt: TimeInterval
    public let rate: Double // blocks per second
    public let estimatedTimeRemaining: TimeInterval?
    public let peerCount: UInt32

    public static func `default`() -> SPVSyncProgress {
        SPVSyncProgress(
            stage: .idle,
            currentHeight: 0,
            targetHeight: 0,
            filterHeaderHeight: 0,
            filterHeight: 0,
            syncStartedAt: 0,
            rate: 0,
            estimatedTimeRemaining: nil,
            peerCount: 0
        )
    }

    public static func from(_ overview: FFISyncProgress) -> SPVSyncProgress {
        SPVSyncProgress(
            stage: .idle,
            currentHeight: overview.header_height,
            targetHeight: 0,
            filterHeaderHeight: overview.filter_header_height,
            filterHeight: overview.last_synced_filter_height,
            syncStartedAt: 0, // TODO: This field exists in the Rust struct but the FFI does not provide it yet
            rate: 0,
            estimatedTimeRemaining: nil,
            peerCount: overview.peer_count
        )
    }
}

public enum SPVSyncStage: String, Sendable {
    case idle = "Idle"
    case connecting = "Connecting"
    case queryingHeight = "Querying Height"
    case downloading = "Downloading"
    case validating = "Validating"
    case storing = "Storing"
    case downloadingFilterHeaders = "Downloading Filter Headers"
    case downloadingFilters = "Downloading Filters"
    case downloadingBlocks = "Downloading Blocks"
    case complete = "Complete"
    case failed = "Failed"
    case unknown = "Unknown"
}

extension SPVSyncStage {
    init(ffiStage: FFISyncStage) {
        switch ffiStage.rawValue {
            case 0: self = .connecting
            case 1: self = .queryingHeight
            case 2: self = .downloading
            case 3: self = .validating
            case 4: self = .storing
            case 5: self = .downloadingFilterHeaders
            case 6: self = .downloadingFilters
            case 7: self = .downloadingBlocks
            case 8: self = .complete
            case 9: self = .failed
            default: self = .unknown
        }
    }
}

// MARK: - SPV Event Types

public struct SPVBlockEvent {
    public let height: UInt32
    public let hash: Data
    public let timestamp: Date
}

public struct SPVTransactionEvent {
    public let txid: Data
    public let confirmed: Bool
    public let amount: Int64
    public let addresses: [String]
    public let blockHeight: UInt32?
}

// MARK: - SPV Client

@MainActor
public class SPVClient<T: SPVEventHandler> {
    public var isConnected = false
    public var isSyncing = false
    var blocksHit: Int = 0

    // SPVEventHandler for callbacks
    private let spvEventHandler: T

    // FFI handles
    private let client: UnsafeMutablePointer<FFIDashSpvClient>
    private let config: UnsafeMutablePointer<FFIClientConfig>
    private var hasBeenFreed = false

    // Public accessor for client handle (needed for filter match queries)
    public var clientHandle: UnsafeMutablePointer<FFIDashSpvClient>? {
        return client
    }

    // Sync tracking
    
    fileprivate let swiftLoggingEnabled: Bool = {
        if let env = ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"], env.lowercased() == "1" || env.lowercased() == "true" {
            return true
        }
        return false
    }()

    // Removed: Temporary poller for filter header progress (now event-driven via FFI)

    public init(network: Network = DashSDKNetwork(rawValue: 1), dataDir: String?, startHeight: UInt32, spvEventHandler: T) throws {
        if swiftLoggingEnabled {
            let level = (ProcessInfo.processInfo.environment["SPV_LOG"] ?? "off")
            print("[SPV][Log] Initialized SPV logging level=\(level)")
        }

        // Create configuration based on network raw value
        let configPtr: UnsafeMutablePointer<FFIClientConfig>? = {
            switch network.rawValue {
            case 0:
                return dash_spv_ffi_config_mainnet()
            case 1:
                return dash_spv_ffi_config_testnet()
            case 3:
                // Map devnet to custom FFINetwork value 3
                return dash_spv_ffi_config_new(FFINetwork(rawValue: 3))
            default:
                return dash_spv_ffi_config_testnet()
            }
        }()

        guard let configPtr = configPtr else {
            throw SPVError.configurationFailed
        }

        // If requested, prefer local core peers (defaults to 127.0.0.1 with network default port)
        let useLocalCore = UserDefaults.standard.bool(forKey: "useLocalhostCore")
        // Only restrict to configured peers when using local core, if not, allow DNS discovery
        let restrictToConfiguredPeers = useLocalCore
        if useLocalCore {
            let peers = SPVClient.readLocalCorePeers()
            if swiftLoggingEnabled {
                print("[SPV][Config] Use Local Core enabled; peers=\(peers.joined(separator: ", "))")
            }
            // Add peers via FFI (supports "ip:port" or bare IP for network-default port)
            for addr in peers {
                addr.withCString { cstr in
                    let rc = dash_spv_ffi_config_add_peer(configPtr, cstr)
                    if rc != 0 {
                        print("[SPV][Config] add_peer failed for \(addr): \(SPVClient.getLastDashFFIError())")
                    }
                }
            }
        }

        // Apply restrict-to-configured-peers if requested
        if restrictToConfiguredPeers {
            if swiftLoggingEnabled { print("[SPV][Config] Enabling restrict-to-configured-peers mode") }
            _ = dash_spv_ffi_config_set_restrict_to_configured_peers(configPtr, true)
        }

        // Set data directory if provided
        if let dataDir = dataDir {
            let result = dash_spv_ffi_config_set_data_dir(configPtr, dataDir)
            if result != 0 {
                throw SPVError.configurationFailed
            }
        }

        // Enable mempool tracking and ensure detailed events are available
        dash_spv_ffi_config_set_mempool_tracking(configPtr, true)
        dash_spv_ffi_config_set_mempool_strategy(configPtr, FFIMempoolStrategy(rawValue: 0)) // FetchAll
        _ = dash_spv_ffi_config_set_fetch_mempool_transactions(configPtr, true)
        _ = dash_spv_ffi_config_set_persist_mempool(configPtr, true)

        // Set user agent to include SwiftDashSDK version from the framework bundle
        do {
            let bundle = Bundle(for: SPVClient.self)
            let version = (bundle.infoDictionary?["CFBundleShortVersionString"] as? String)
                ?? (bundle.infoDictionary?["CFBundleVersion"] as? String)
                ?? "dev"
            let ua = "SwiftDashSDK/\(version)"
            // Always print what we're about to set for easier debugging
            print("Setting user agent to \(ua)")
            let rc = dash_spv_ffi_config_set_user_agent(configPtr, ua)
            if rc != 0 {
                print("[SPV][Config] Failed to set user agent (rc=\(rc)): \(SPVClient.getLastDashFFIError())")
                throw SPVError.configurationFailed
            }
            if swiftLoggingEnabled { print("[SPV][Config] User-Agent=\(ua)") }
        }

        _ = dash_spv_ffi_config_set_start_from_height(configPtr, startHeight)

        // Create client
        let client = dash_spv_ffi_client_new(configPtr)
        guard let client = client else {
            print("[SPV][Init] Failed to create client: \(SPVClient.getLastDashFFIError())")
            throw SPVError.initializationFailed
        }

        let result = dash_spv_ffi_client_start(client)
        if result != 0 {
            throw SPVError.startFailed(SPVClient.getLastDashFFIError())
        }

        self.isConnected = true
        self.client = client
        self.config = configPtr
        
        // Set up event callbacks with stable spvEventHandler
        self.spvEventHandler = spvEventHandler
        var callbacks = FFIEventCallbacks()

        callbacks.user_data = Unmanaged.passUnretained(self.spvEventHandler).toOpaque()
        
        callbacks.on_block = onBlockCallbackC
        callbacks.on_transaction = onTransactionCallbackC
        callbacks.on_compact_filter_matched = onCompactFilterMatchedCallbackC
        callbacks.on_mempool_transaction_added = onMempoolTransactionAddedCallbackC
        callbacks.on_mempool_transaction_confirmed = onMempoolTransactionConfirmedCallbackC
        callbacks.on_mempool_transaction_removed = onMempoolTransactionRemovedCallbackC
        callbacks.on_wallet_transaction = onWalletTransactionCallbackC

        dash_spv_ffi_client_set_event_callbacks(client, callbacks)
        
        // Call the event handler to notify about the initial sync progress
        self.spvEventHandler.spvClient(didUpdateSyncProgress: self.getSyncProgress())
    }

    deinit {
        if !hasBeenFreed {
            print("[SPV][deinit] WARNING: SPVClient was not freed before deinit, call SPVClient::destroy")
        }
    }

    private static func readLocalCorePeers() -> [String] {
        // If no override is set, default to 127.0.0.1 and let FFI pick port by network
        let raw = UserDefaults.standard.string(forKey: "corePeerAddresses")?.trimmingCharacters(in: .whitespacesAndNewlines)
        let list = (raw?.isEmpty == false ? raw! : "127.0.0.1")
        return list
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }

    public func getSyncProgress() -> SPVSyncProgress {
        // IMPROVEMENT
        // The return struct lacks information provided by FFIDetailedSyncProgress,
        // Unification of both structs in the FFI would be a better aproach
        guard let ptr = dash_spv_ffi_client_get_sync_progress(client) else {
          print("[SPV][GetSyncProgress] Failed to get sync progress (Should only fail if client is nil, but client is not nil)")
          return SPVSyncProgress.default()
        }
        defer { dash_spv_ffi_sync_progress_destroy(ptr) }
        let p = ptr.pointee
        
        return SPVSyncProgress.from(p)
    }
    
    public static func getLastDashFFIError() -> String {
        guard let errorMsg = dash_spv_ffi_get_last_error() else { return "No error" }
        return String(cString: errorMsg)
    }

    /// Enable/disable masternode sync. If the client is running, apply the update immediately.
    public func setMasternodeSyncEnabled(_ enabled: Bool) throws {
        var rc = dash_spv_ffi_config_set_masternode_sync_enabled(config, enabled)
        if rc != 0 { throw SPVError.configurationFailed }

        rc = dash_spv_ffi_client_update_config(client, config)
        if rc != 0 { throw SPVError.configurationFailed }
    }

    /// Update the starting checkpoint height (sync-from base) at runtime.
    /// Applies to the next sync start and persists in the client's config.
    public func setStartFromHeight(_ height: UInt32) throws {
        var rc = dash_spv_ffi_config_set_start_from_height(config, height)
        if rc != 0 { throw SPVError.configurationFailed }

        rc = dash_spv_ffi_client_update_config(client, config)
        if rc != 0 { throw SPVError.configurationFailed }
    }

    /// Clear all persisted SPV storage (headers, filters, metadata, sync state).
    public func clearStorage() throws {
        let rc = dash_spv_ffi_client_clear_storage(client)
        if rc != 0 {
            throw SPVError.storageOperationFailed(SPVClient.getLastDashFFIError())
        }

        // IMPROVEMENT
        // Manually calling the event doesn't look like the right approach,
        // if FFISPVClient could send us an event callback automatically...
        self.spvEventHandler.spvClient(didUpdateSyncProgress: SPVSyncProgress.default())
    }

    // MARK: - Wallet Transaction Queries

    /// Get the total count of transactions in the wallet's history.
    /// This count persists across app restarts.
    ///
    /// - Returns: Total number of transactions, or 0 if wallet is empty or not initialized
    /// NOTE: FFI function dash_spv_ffi_client_get_transaction_count not available in current build
    public func getTransactionCount() -> UInt64 {
        // NOTE: dash_spv_ffi_client_get_transaction_count is not available in current FFI
        // When available, use: return UInt64(dash_spv_ffi_client_get_transaction_count(client))
        return 0
    }

    /// Get the count of unique blocks that contain wallet transactions.
    /// Only counts confirmed transactions (those with a block height).
    /// This is the persistent "blocks hit" metric that survives app restarts.
    ///
    /// - Returns: Number of blocks with wallet transactions, or 0 if wallet is empty or not initialized
    /// NOTE: FFI function dash_spv_ffi_client_get_blocks_with_transactions_count not available in current build
    public func getBlocksWithTransactionsCount() -> UInt64 {
        // NOTE: dash_spv_ffi_client_get_blocks_with_transactions_count is not available in current FFI
        // When available, use: return UInt64(dash_spv_ffi_client_get_blocks_with_transactions_count(client))
        return 0
    }

    public func destroy() {
        dash_spv_ffi_client_destroy(client)
        dash_spv_ffi_config_destroy(config)

        self.hasBeenFreed = true
    }

    // MARK: - Synchronization

    public func startSync() async throws {
        guard !isSyncing else {
            throw SPVError.alreadySyncing
        }

        self.isSyncing = true
        blocksHit = 0
        
        let spvEventHandlerPtr = Unmanaged.passUnretained(spvEventHandler).toOpaque()
        
        let result = dash_spv_ffi_client_sync_to_tip_with_progress(
            self.client,
            spvProgressCallbackC,
            spvCompletionCallbackC,
            spvEventHandlerPtr
        )

        if result != 0 {
            throw SPVError.syncFailed(SPVClient.getLastDashFFIError())
        }
    }

    public func stopSync() {
        let cancelResult = dash_spv_ffi_client_cancel_sync(client)
        if cancelResult != 0 {
            let message = SPVClient.getLastDashFFIError()
            if swiftLoggingEnabled {
                print("[SPV][Cancel] cancel_sync failed: \(message)")
            }
        }
        isSyncing = false
    }

    // MARK: - Wallet Manager Access

    /// Produce a Swift wallet manager that shares the SPV client's underlying wallet state.
    /// Callers are responsible for retaining the returned instance for as long as needed.
    public func getWalletManager() throws -> WalletManager {
        // This ffi call is expected to never fail
        let ffiWalletManager = dash_spv_ffi_client_get_wallet_manager(self.client)!
        
        return try WalletManager(handle: ffiWalletManager)
    }
}

// MARK: - Supporting Types

public enum SPVError: LocalizedError {
    case notInitialized
    case alreadyInitialized
    case configurationFailed
    case initializationFailed
    case startFailed(String)
    case alreadySyncing
    case syncFailed(String)
    case storageOperationFailed(String)

    public var errorDescription: String? {
        switch self {
        case .notInitialized:
            return "SPV client is not initialized"
        case .alreadyInitialized:
            return "SPV client is already initialized"
        case .configurationFailed:
            return "Failed to configure SPV client"
        case .initializationFailed:
            return "Failed to initialize SPV client"
        case .startFailed(let reason):
            return "Failed to start SPV client: \(reason)"
        case .alreadySyncing:
            return "SPV client is already syncing"
        case .syncFailed(let reason):
            return "Sync failed: \(reason)"
        case .storageOperationFailed(let reason):
            return reason
        }
    }
}
