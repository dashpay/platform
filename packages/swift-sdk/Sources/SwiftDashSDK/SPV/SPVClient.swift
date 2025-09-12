import Foundation
import DashSDKFFI

// MARK: - C Callback Functions
// These must be global functions to be used as C function pointers

private func spvProgressCallback(
    progressPtr: UnsafePointer<FFIDetailedSyncProgress>?,
    userData: UnsafeMutableRawPointer?
) {
    guard let progressPtr = progressPtr,
          let userData = userData else { return }
    
    let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
    context.handleProgressUpdate(progressPtr)
}

private func spvCompletionCallback(
    success: Bool,
    errorMsg: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    guard let userData = userData else { return }
    
    let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
    context.handleSyncCompletion(success: success, errorMsg: errorMsg)
}

// MARK: - SPV Sync Progress

public struct SPVSyncProgress {
    public let stage: SPVSyncStage
    public let headerProgress: Double
    public let masternodeProgress: Double
    public let transactionProgress: Double
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    // Checkpoint height we started from (0 if none)
    public let startHeight: UInt32
    public let rate: Double // blocks per second
    public let estimatedTimeRemaining: TimeInterval?
    
    public var overallProgress: Double {
        // Weight the different stages
        let headerWeight = 0.4
        let masternodeWeight = 0.3
        let transactionWeight = 0.3
        
        return (headerProgress * headerWeight) +
               (masternodeProgress * masternodeWeight) +
               (transactionProgress * transactionWeight)
    }
}

public enum SPVSyncStage: String {
    case idle = "Idle"
    case headers = "Downloading Headers"
    case masternodes = "Syncing Masternode List"
    case transactions = "Processing Transactions"
    case complete = "Complete"
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

// MARK: - SPV Client Delegate

public protocol SPVClientDelegate: AnyObject {
    func spvClient(_ client: SPVClient, didUpdateSyncProgress progress: SPVSyncProgress)
    func spvClient(_ client: SPVClient, didReceiveBlock block: SPVBlockEvent)
    func spvClient(_ client: SPVClient, didReceiveTransaction transaction: SPVTransactionEvent)
    func spvClient(_ client: SPVClient, didCompleteSync success: Bool, error: String?)
    func spvClient(_ client: SPVClient, didChangeConnectionStatus connected: Bool, peers: Int)
}

// MARK: - SPV Client

public class SPVClient: ObservableObject {
    // Published properties for SwiftUI
    @Published public var isConnected = false
    @Published public var isSyncing = false
    @Published public var syncProgress: SPVSyncProgress?
    @Published public var peerCount: Int = 0
    @Published public var lastError: String?
    
    // Delegate for callbacks
    public weak var delegate: SPVClientDelegate?
    
    // FFI handles
    private var client: UnsafeMutablePointer<FFIDashSpvClient>?
    private var config: UnsafeMutablePointer<FFIClientConfig>?
    
    // Callback context
    private var callbackContext: CallbackContext?
    
    // Network
    private let network: Network
    private var masternodeSyncEnabled: Bool = true
    // If true, SPV will only connect to peers explicitly configured via FFI
    public var restrictToConfiguredPeers: Bool = false
    
    // Sync tracking
    // Height we start syncing from (checkpoint); used to render absolute heights
    fileprivate var startFromHeight: UInt32 = 0
    private var syncStartTime: Date?
    private var lastBlockHeight: UInt32 = 0
    internal var syncCancelled = false
    fileprivate var lastProgressUIUpdate: TimeInterval = 0
    fileprivate let progressUICoalesceInterval: TimeInterval = 0.2
    fileprivate let swiftLoggingEnabled: Bool = {
        if let env = ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"], env.lowercased() == "1" || env.lowercased() == "true" {
            return true
        }
        return false
    }()
    
    public init(network: Network = DashSDKNetwork(rawValue: 1)) {
        self.network = network
    }
    
    deinit {
        Task { @MainActor in
            stop()
            destroyClient()
        }
    }
    
    // MARK: - Client Lifecycle
    
    public func initialize(dataDir: String? = nil, masternodesEnabled: Bool? = nil, startHeight: UInt32? = nil) throws {
        guard client == nil else {
            throw SPVError.alreadyInitialized
        }
        
        // Initialize SPV logging (one-time). Default to off unless SPV_LOG is provided.
        enum SPVLogInit {
            static let once: Void = {
                let level = (ProcessInfo.processInfo.environment["SPV_LOG"] ?? "off")
                level.withCString { cstr in
                    dash_spv_ffi_init_logging(cstr)
                }
            }()
        }
        _ = SPVLogInit.once
        if swiftLoggingEnabled {
            let level = (ProcessInfo.processInfo.environment["SPV_LOG"] ?? "off")
            print("[SPV][Log] Initialized SPV logging level=\(level)")
        }

        // Create configuration based on network raw value
        let configPtr: UnsafeMutablePointer<FFIClientConfig>? = {
            switch network {
            case DashSDKNetwork(rawValue: 0):
                return dash_spv_ffi_config_mainnet()
            case DashSDKNetwork(rawValue: 1):
                return dash_spv_ffi_config_testnet()
            case DashSDKNetwork(rawValue: 2):
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
        if useLocalCore {
            let peers = SPVClient.readLocalCorePeers()
            if swiftLoggingEnabled {
                print("[SPV][Config] Use Local Core enabled; peers=\(peers.joined(separator: ", "))")
            }
            // Add peers via FFI (supports "ip:port" or bare IP for network-default port)
            for addr in peers {
                addr.withCString { cstr in
                    let rc = dash_spv_ffi_config_add_peer(configPtr, cstr)
                    if rc != 0, let err = dash_spv_ffi_get_last_error() {
                        let msg = String(cString: err)
                        print("[SPV][Config] add_peer failed for \(addr): \(msg)")
                    }
                }
            }
            // Enforce restrict mode when using local core by default
            restrictToConfiguredPeers = true
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
        
        // Enable mempool tracking
        dash_spv_ffi_config_set_mempool_tracking(configPtr, true)
        dash_spv_ffi_config_set_mempool_strategy(configPtr, FFIMempoolStrategy(rawValue: 1)) // BloomFilter

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
                if let cErr = dash_spv_ffi_get_last_error() {
                    let err = String(cString: cErr)
                    print("[SPV][Config] Failed to set user agent (rc=\(rc)): \(err)")
                } else {
                    print("[SPV][Config] Failed to set user agent (rc=\(rc))")
                }
                throw SPVError.configurationFailed
            }
            if swiftLoggingEnabled { print("[SPV][Config] User-Agent=\(ua)") }
        }

        // Optionally override masternode sync behavior
        if let m = masternodesEnabled {
            self.masternodeSyncEnabled = m
        }
        _ = dash_spv_ffi_config_set_masternode_sync_enabled(configPtr, masternodeSyncEnabled)

        // Optionally set a starting height checkpoint
        if let h = startHeight {
            // Align to the last checkpoint at or below the requested height
            let netFromConfig = dash_spv_ffi_config_get_network(configPtr)
            var cpOutHeight: UInt32 = 0
            var cpOutHash = [UInt8](repeating: 0, count: 32)
            let rc: Int32 = cpOutHash.withUnsafeMutableBufferPointer { buf in
                dash_spv_ffi_checkpoint_before_height(netFromConfig, h, &cpOutHeight, buf.baseAddress)
            }
            let finalHeight: UInt32 = (rc == 0 && cpOutHeight > 0) ? cpOutHeight : h
            _ = dash_spv_ffi_config_set_start_from_height(configPtr, finalHeight)
            // Remember checkpoint for UI normalization
            self.startFromHeight = finalHeight
        }
        
        // Create client
        client = dash_spv_ffi_client_new(configPtr)
        guard client != nil else {
            throw SPVError.initializationFailed
        }
        
        // Store config for cleanup
        config = configPtr
        
        // Set up event callbacks with stable context
        setupEventCallbacks()
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

    /// Enable/disable masternode sync. If the client is running, apply the update immediately.
    public func setMasternodeSyncEnabled(_ enabled: Bool) throws {
        self.masternodeSyncEnabled = enabled
        if let config = self.config {
            let rc = dash_spv_ffi_config_set_masternode_sync_enabled(config, enabled)
            if rc != 0 { throw SPVError.configurationFailed }
        }
        if let client = self.client, let config = self.config {
            let rc2 = dash_spv_ffi_client_update_config(client, config)
            if rc2 != 0 { throw SPVError.configurationFailed }
        }
    }
    
    public func start() throws {
        guard let client = client else {
            throw SPVError.notInitialized
        }
        
        let result = dash_spv_ffi_client_start(client)
        if result != 0 {
            if let errorMsg = dash_spv_ffi_get_last_error() {
                let error = String(cString: errorMsg)
                Task { @MainActor in self.lastError = error }
                throw SPVError.startFailed(error)
            }
            throw SPVError.startFailed("Unknown error")
        }
        
        Task { @MainActor in self.isConnected = true }
    }
    
    public func stop() {
        guard let client = client else { return }
        
        dash_spv_ffi_client_stop(client)
        Task { @MainActor in
            self.isConnected = false
            self.isSyncing = false
            self.syncProgress = nil
        }
    }
    
    private func destroyClient() {
        if let client = client {
            dash_spv_ffi_client_destroy(client)
            self.client = nil
        }
        
        if let config = config {
            dash_spv_ffi_config_destroy(config)
            self.config = nil
        }
        
        callbackContext = nil
    }
    
    // MARK: - Synchronization
    
    public func startSync() async throws {
        guard let client = client else {
            throw SPVError.notInitialized
        }
        
        guard !isSyncing else {
            throw SPVError.alreadySyncing
        }
        
        await MainActor.run {
            self.isSyncing = true
        }
        syncCancelled = false
        syncStartTime = Date()
        
        // Use a stable callback context; create if needed
        let context: CallbackContext
        if let existing = self.callbackContext {
            context = existing
        } else {
            context = CallbackContext(client: self)
            self.callbackContext = context
        }
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()
        
        // Start sync in the background to avoid blocking the main thread
        let workItem = DispatchWorkItem { [weak self] in
            guard let self = self, let client = self.client else { return }
            let result = dash_spv_ffi_client_sync_to_tip_with_progress(
                client,
                spvProgressCallback,
                spvCompletionCallback,
                contextPtr
            )

            if result != 0 {
                let error = self.lastError ?? "Unknown error"
                Task { @MainActor in
                    self.isSyncing = false
                    self.lastError = error
                }
            }
        }
        DispatchQueue.global(qos: .userInitiated).async(execute: workItem)
    }
    
    public func cancelSync() {
        guard let client = client, isSyncing else { return }
        
        syncCancelled = true
        dash_spv_ffi_client_cancel_sync(client)
        isSyncing = false
        syncProgress = nil
    }
    
    // MARK: - Event Callbacks
    
    private func setupEventCallbacks() {
        guard let client = client else { return }
        
        let context = CallbackContext(client: self)
        self.callbackContext = context
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()
        
        var callbacks = FFIEventCallbacks()
        
        callbacks.on_block = { height, hashPtr, userData in
            guard let userData = userData else { return }
            
            let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            
            var hash = Data()
            if let hashPtr = hashPtr {
                hash = Data(bytes: hashPtr, count: 32)
            }
            
            Task { @MainActor in
                context.client?.handleBlockEvent(height: height, hash: hash)
            }
        }
        
        callbacks.on_transaction = { txidPtr, confirmed, amount, addressesPtr, blockHeight, userData in
            guard let userData = userData else { return }
            
            let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            
            var txid = Data()
            if let txidPtr = txidPtr {
                txid = Data(bytes: txidPtr, count: 32)
            }
            
            var addresses: [String] = []
            if let addressesPtr = addressesPtr {
                let addressesStr = String(cString: addressesPtr)
                addresses = addressesStr.components(separatedBy: ",")
            }
            
            Task { @MainActor in
                context.client?.handleTransactionEvent(
                    txid: txid,
                    confirmed: confirmed,
                    amount: amount,
                    addresses: addresses,
                    blockHeight: blockHeight > 0 ? blockHeight : nil
                )
            }
        }
        
        callbacks.user_data = contextPtr
        
        dash_spv_ffi_client_set_event_callbacks(client, callbacks)
    }
    
    // MARK: - Event Handlers
    
    private func handleBlockEvent(height: UInt32, hash: Data) {
        let block = SPVBlockEvent(
            height: height,
            hash: hash,
            timestamp: Date()
        )

        if swiftLoggingEnabled {
            print("[SPV][Block] height=\(height) hash=\(hash.map { String(format: "%02x", $0) }.joined().prefix(16))…")
        }

        delegate?.spvClient(self, didReceiveBlock: block)
        
        // Update sync progress if we're syncing
        if isSyncing, let progress = syncProgress {
            // Update height tracking for rate calculation
            if lastBlockHeight > 0 {
                let blocksDiff = height - lastBlockHeight
                let timeDiff = Date().timeIntervalSince(syncStartTime ?? Date())
                let rate = timeDiff > 0 ? Double(blocksDiff) / timeDiff : 0
                
                let updatedProgress = SPVSyncProgress(
                    stage: progress.stage,
                    headerProgress: progress.headerProgress,
                    masternodeProgress: progress.masternodeProgress,
                    transactionProgress: progress.transactionProgress,
                    currentHeight: height,
                    targetHeight: progress.targetHeight,
                    startHeight: self.startFromHeight,
                    rate: rate,
                    estimatedTimeRemaining: progress.estimatedTimeRemaining
                )
                
                syncProgress = updatedProgress
                delegate?.spvClient(self, didUpdateSyncProgress: updatedProgress)
            }
            
            lastBlockHeight = height
        }
    }
    
    private func handleTransactionEvent(txid: Data, confirmed: Bool, amount: Int64, addresses: [String], blockHeight: UInt32?) {
        let transaction = SPVTransactionEvent(
            txid: txid,
            confirmed: confirmed,
            amount: amount,
            addresses: addresses,
            blockHeight: blockHeight
        )

        // Debug: print tx event summary
        if swiftLoggingEnabled {
            let txidHex = txid.map { String(format: "%02x", $0) }.joined()
            let bh = blockHeight.map(String.init) ?? "nil"
            print("[SPV][Tx] txid=\(txidHex.prefix(16))… confirmed=\(confirmed) amount=\(amount) blockHeight=\(bh)")
        }

        delegate?.spvClient(self, didReceiveTransaction: transaction)
    }
    
    // MARK: - Wallet Manager Access
    
    public func getWalletManager() -> UnsafeMutablePointer<FFIWalletManager>? {
        guard let client = client else { return nil }
        
        let managerPtr = dash_spv_ffi_client_get_wallet_manager(client)
        return managerPtr?.assumingMemoryBound(to: FFIWalletManager.self)
    }
    
    // MARK: - Statistics
    
    public func getStats() -> SPVStats? {
        guard let client = client else { return nil }
        
        let statsPtr = dash_spv_ffi_client_get_stats(client)
        guard let statsPtr = statsPtr else { return nil }
        
        // Convert FFI stats to Swift struct
        let stats = SPVStats(
            connectedPeers: Int(statsPtr.pointee.connected_peers),
            headerHeight: Int(statsPtr.pointee.header_height),
            filterHeight: Int(statsPtr.pointee.filter_height),
            mempoolSize: 0 // mempool_size not available in current FFI
        )
        
        dash_spv_ffi_spv_stats_destroy(statsPtr)
        
        return stats
    }

    // MARK: - Sync Snapshot
    public func getSyncSnapshot() -> SPVSyncSnapshot? {
        guard let client = client else { return nil }
        guard let ptr = dash_spv_ffi_client_get_sync_progress(client) else { return nil }
        defer { dash_spv_ffi_sync_progress_destroy(ptr) }
        let p = ptr.pointee
        return SPVSyncSnapshot(
            headerHeight: p.header_height,
            filterHeaderHeight: p.filter_header_height,
            masternodeHeight: p.masternode_height,
            headersSynced: p.headers_synced,
            filterHeadersSynced: p.filter_headers_synced,
            masternodesSynced: p.masternodes_synced,
            filterSyncAvailable: p.filter_sync_available,
            filtersDownloaded: p.filters_downloaded,
            lastSyncedFilterHeight: p.last_synced_filter_height
        )
    }

    // MARK: - Checkpoints
    // Tries to fetch the latest checkpoint height for this client's network.
    // Requires newer FFI with dash_spv_ffi_checkpoint_latest. Returns nil if unavailable.
    public func getLatestCheckpointHeight() -> UInt32? {
        // Derive FFINetwork matching how we built config
        let ffiNet: FFINetwork
        switch network {
        case DashSDKNetwork(rawValue: 0): // mainnet
            ffiNet = FFINetwork(rawValue: 0)
        case DashSDKNetwork(rawValue: 1): // testnet
            ffiNet = FFINetwork(rawValue: 1)
        case DashSDKNetwork(rawValue: 2): // devnet
            ffiNet = FFINetwork(rawValue: 3)
        default:
            ffiNet = FFINetwork(rawValue: 1)
        }

        var outHeight: UInt32 = 0
        var outHash = [UInt8](repeating: 0, count: 32)
        let rc: Int32 = outHash.withUnsafeMutableBufferPointer { buf in
            dash_spv_ffi_checkpoint_latest(ffiNet, &outHeight, buf.baseAddress)
        }
        guard rc == 0 else { return nil }
        return outHeight
    }

    /// Returns the checkpoint height at or before a given UNIX timestamp (seconds) for this network
    public func getCheckpointHeight(beforeTimestamp timestamp: UInt32) -> UInt32? {
        let ffiNet: FFINetwork
        switch network {
        case DashSDKNetwork(rawValue: 0): ffiNet = FFINetwork(rawValue: 0)
        case DashSDKNetwork(rawValue: 1): ffiNet = FFINetwork(rawValue: 1)
        case DashSDKNetwork(rawValue: 2): ffiNet = FFINetwork(rawValue: 3)
        default: ffiNet = FFINetwork(rawValue: 1)
        }
        var outHeight: UInt32 = 0
        var outHash = [UInt8](repeating: 0, count: 32)
        let rc: Int32 = outHash.withUnsafeMutableBufferPointer { buf in
            dash_spv_ffi_checkpoint_before_timestamp(ffiNet, timestamp, &outHeight, buf.baseAddress)
        }
        guard rc == 0 else { return nil }
        return outHeight
    }
}

// MARK: - Callback Context

private class CallbackContext {
    weak var client: SPVClient?
    
    init(client: SPVClient) {
        self.client = client
    }

    func handleProgressUpdate(_ progressPtr: UnsafePointer<FFIDetailedSyncProgress>) {
        let ffiProgress = progressPtr.pointee

        // Determine sync stage based on percentage
        let stage: SPVSyncStage
        if ffiProgress.percentage < 0.3 {
            stage = .headers
        } else if ffiProgress.percentage < 0.7 {
            stage = .masternodes
        } else if ffiProgress.percentage < 1.0 {
            stage = .transactions
        } else {
            stage = .complete
        }

        // Calculate estimated time remaining
        var estimatedTime: TimeInterval? = nil
        if ffiProgress.estimated_seconds_remaining > 0 {
            estimatedTime = Double(ffiProgress.estimated_seconds_remaining)
        }

        if client?.swiftLoggingEnabled == true {
            let pct = max(0.0, min(ffiProgress.percentage, 1.0)) * 100.0
            let cur = ffiProgress.current_height
            let tot = ffiProgress.total_height
            let rate = ffiProgress.headers_per_second
            let eta = ffiProgress.estimated_seconds_remaining
            print("[SPV][Progress] stage=\(stage.rawValue) pct=\(String(format: "%.2f", pct))% height=\(cur)/\(tot) rate=\(String(format: "%.2f", rate)) hdr/s eta=\(eta)s")
        }

        // Convert FFI progress into display-friendly absolute heights
        let absoluteCurrent: UInt32 = (client?.startFromHeight ?? 0) &+ ffiProgress.current_height
        let progress = SPVSyncProgress(
            stage: stage,
            headerProgress: min(ffiProgress.percentage / 0.3, 1.0),
            masternodeProgress: min(max((ffiProgress.percentage - 0.3) / 0.4, 0), 1.0),
            transactionProgress: min(max((ffiProgress.percentage - 0.7) / 0.3, 0), 1.0),
            currentHeight: absoluteCurrent,
            targetHeight: ffiProgress.total_height,
            startHeight: client?.startFromHeight ?? 0,
            rate: ffiProgress.headers_per_second,
            estimatedTimeRemaining: estimatedTime
        )
        
        let now = Date().timeIntervalSince1970
        if let client = self.client, now - client.lastProgressUIUpdate >= client.progressUICoalesceInterval {
            client.lastProgressUIUpdate = now
            Task { @MainActor in
                guard let clientStrong = self.client else { return }
                clientStrong.syncProgress = progress
                clientStrong.delegate?.spvClient(clientStrong, didUpdateSyncProgress: progress)
            }
        }
    }
    
    func handleSyncCompletion(success: Bool, errorMsg: UnsafePointer<CChar>?) {
        var error: String? = nil
        if let errorMsg = errorMsg {
            error = String(cString: errorMsg)
        }

        if client?.swiftLoggingEnabled == true {
            if success {
                print("[SPV][Complete] Sync finished successfully")
            } else {
                print("[SPV][Complete] Sync failed: \(error ?? "unknown error")")
            }
        }

        Task { @MainActor in
            guard let client = self.client else { return }
            client.isSyncing = false
            client.lastError = error
            
            if success {
                client.syncProgress = SPVSyncProgress(
                    stage: .complete,
                    headerProgress: 1.0,
                    masternodeProgress: 1.0,
                    transactionProgress: 1.0,
                    currentHeight: client.syncProgress?.targetHeight ?? 0,
                    targetHeight: client.syncProgress?.targetHeight ?? 0,
                    startHeight: client.startFromHeight,
                    rate: 0,
                    estimatedTimeRemaining: nil
                )
            } else {
                client.syncProgress = nil
            }
            
            client.delegate?.spvClient(client, didCompleteSync: success, error: error)
        }
    }
}

// MARK: - Supporting Types

public struct SPVStats {
    public let connectedPeers: Int
    public let headerHeight: Int
    public let filterHeight: Int
    public let mempoolSize: Int
}

// A lightweight snapshot of sync progress from FFISyncProgress
public struct SPVSyncSnapshot {
    public let headerHeight: UInt32
    public let filterHeaderHeight: UInt32
    public let masternodeHeight: UInt32
    public let headersSynced: Bool
    public let filterHeadersSynced: Bool
    public let masternodesSynced: Bool
    public let filterSyncAvailable: Bool
    public let filtersDownloaded: UInt32
    public let lastSyncedFilterHeight: UInt32
}

public enum SPVError: LocalizedError {
    case notInitialized
    case alreadyInitialized
    case configurationFailed
    case initializationFailed
    case startFailed(String)
    case alreadySyncing
    case syncFailed(String)
    
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
        }
    }
}
