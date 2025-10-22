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

extension SPVClient {
    /// Initialize SPV/Rust-side logging. Call once early in app startup.
    /// If not called, `initialize(...)` will default to reading `SPV_LOG` env var.
    @MainActor
    public static func initializeLogging(_ level: SPVLogLevel) {
        level.rawValue.withCString { cstr in
            _ = dash_spv_ffi_init_logging(cstr)
        }
        LogInitState.manualInitialized = true
    }
}

// MARK: - C Callback Functions
// Use top-level C-compatible functions to avoid actor-isolation init issues

private func spvProgressCallback(
    progressPtr: UnsafePointer<FFIDetailedSyncProgress>?,
    userData: UnsafeMutableRawPointer?
) {
    guard let progressPtr = progressPtr,
          let userData = userData else { return }
    let snapshot = progressPtr.pointee
    let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
    DispatchQueue.main.async {
        context.handleProgressUpdate(snapshot)
    }
}

private func spvCompletionCallback(
    success: Bool,
    errorMsg: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    guard let userData = userData else { return }
    let errorString: String? = errorMsg.map { String(cString: $0) }
    let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
    DispatchQueue.main.async {
        context.handleSyncCompletion(success: success, error: errorString)
    }
}

// Global C-compatible event callbacks that use userData context
private func onBlockCallbackC(
    _ height: UInt32,
    _ hashPtr: UnsafePointer<UInt8>?,
    _ userData: UnsafeMutableRawPointer?
) {
    guard let userData = userData else { return }
    let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
    var hash = Data()
    if let hashPtr = hashPtr {
        hash = Data(bytes: hashPtr, count: 32)
    }
    let clientRef = context.client
    Task { @MainActor [weak clientRef] in
        clientRef?.handleBlockEvent(height: height, hash: hash)
    }
}

private func onTransactionCallbackC(
    _ txidPtr: UnsafePointer<UInt8>?,
    _ confirmed: Bool,
    _ amount: Int64,
    _ addressesPtr: UnsafePointer<CChar>?,
    _ blockHeight: UInt32,
    _ userData: UnsafeMutableRawPointer?
) {
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
    let clientRef = context.client
    Task { @MainActor [weak clientRef] in
        clientRef?.handleTransactionEvent(
            txid: txid,
            confirmed: confirmed,
            amount: amount,
            addresses: addresses,
            blockHeight: blockHeight > 0 ? blockHeight : nil
        )
    }
}

// MARK: - SPV Sync Progress

public struct SPVSyncProgress {
    public let stage: SPVSyncStage
    public let headerProgress: Double
    /// Represents filter header progress until a dedicated masternode stage is exposed.
    public let masternodeProgress: Double
    /// Represents compact filter download progress ("Filters" stage).
    public let transactionProgress: Double
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    /// Absolute blockchain height reached for filter headers.
    public let filterHeaderHeight: UInt32
    /// Absolute blockchain height reached for compact filters.
    public let filterHeight: UInt32
    /// UNIX timestamp (seconds) when the current sync run started. 0 if unavailable.
    public let syncStartedAt: TimeInterval
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

public enum SPVSyncStage: String, Sendable {
    case idle = "Idle"
    case headers = "Downloading Headers"
    case masternodes = "Syncing Masternode List"
    case transactions = "Processing Transactions"
    case complete = "Complete"
}
extension SPVSyncStage {
    init(ffiStage: FFISyncStage) {
        switch ffiStage.rawValue {
        case 5: // Complete
            self = .complete
        case 6: // Failed
            self = .headers
        default:
            self = .headers
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

// MARK: - SPV Client Delegate

@MainActor
public protocol SPVClientDelegate: AnyObject {
    func spvClient(_ client: SPVClient, didUpdateSyncProgress progress: SPVSyncProgress)
    func spvClient(_ client: SPVClient, didReceiveBlock block: SPVBlockEvent)
    func spvClient(_ client: SPVClient, didReceiveTransaction transaction: SPVTransactionEvent)
    func spvClient(_ client: SPVClient, didCompleteSync success: Bool, error: String?)
    func spvClient(_ client: SPVClient, didChangeConnectionStatus connected: Bool, peers: Int)
    func spvClient(_ client: SPVClient, didUpdateBlocksHit count: Int)
}

// MARK: - SPV Client

@MainActor
public class SPVClient: ObservableObject {
    // Published properties for SwiftUI
    @Published public var isConnected = false
    @Published public var isSyncing = false
    @Published public var syncProgress: SPVSyncProgress?
    @Published public var peerCount: Int = 0
    @Published public var lastError: String?
    @Published public var blocksHit: Int = 0
    
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
    fileprivate var currentSyncStartTimestamp: Int64 = 0
    fileprivate var lastProgressUIUpdate: TimeInterval = 0
    fileprivate let progressUICoalesceInterval: TimeInterval = 0.2
    fileprivate let swiftLoggingEnabled: Bool = {
        if let env = ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"], env.lowercased() == "1" || env.lowercased() == "true" {
            return true
        }
        return false
    }()
    
    // Removed: Temporary poller for filter header progress (now event-driven via FFI)
    
    public init(network: Network = DashSDKNetwork(rawValue: 1)) {
        self.network = network
    }

    // Expose a read-only view of the sync base (checkpoint) height for UI/consumers.
    // This is the absolute blockchain height we consider as the base when syncing from a checkpoint.
    public var baseSyncHeight: UInt32 { startFromHeight }
    
    deinit {
        // Minimal teardown; prefer explicit stop() by callers.
    }
    
    // MARK: - Client Lifecycle
    
    @MainActor
    public func initialize(dataDir: String? = nil, masternodesEnabled: Bool? = nil, startHeight: UInt32? = nil) throws {
        guard client == nil else {
            throw SPVError.alreadyInitialized
        }
        
        // Initialize SPV logging (one-time) unless already initialized manually.
        if !LogInitState.manualInitialized {
            let level = (ProcessInfo.processInfo.environment["SPV_LOG"] ?? "off")
            _ = level.withCString { cstr in
                dash_spv_ffi_init_logging(cstr)
            }
        }
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

    /// Update the starting checkpoint height (sync-from base) at runtime.
    /// Applies to the next sync start and persists in the client's config.
    public func setStartFromHeight(_ height: UInt32) throws {
        self.startFromHeight = height
        if let config = self.config {
            let rc = dash_spv_ffi_config_set_start_from_height(config, height)
            if rc != 0 { throw SPVError.configurationFailed }
        }
        if let client = self.client, let config = self.config {
            let rc2 = dash_spv_ffi_client_update_config(client, config)
            if rc2 != 0 { throw SPVError.configurationFailed }
        }
    }
    
    public func start() throws {
        guard self.client != nil else {
            throw SPVError.notInitialized
        }
        
        let result = dash_spv_ffi_client_start(client)
        if result != 0 {
            if let errorMsg = dash_spv_ffi_get_last_error() {
                let error = String(cString: errorMsg)
                self.lastError = error
                throw SPVError.startFailed(error)
            }
            throw SPVError.startFailed("Unknown error")
        }
        
        self.isConnected = true
    }
    
    public func stop() {
        stopSync(preserveProgress: false)
    }

    /// Clear all persisted SPV storage (headers, filters, metadata, sync state).
    public func clearStorage() throws {
        guard let client = client else { throw SPVError.notInitialized }

        let rc = dash_spv_ffi_client_clear_storage(client)
        if rc != 0 {
            if let errorMsg = dash_spv_ffi_get_last_error() {
                let message = String(cString: errorMsg)
                throw SPVError.storageOperationFailed(message)
            } else {
                throw SPVError.storageOperationFailed("Failed to clear SPV storage (code \(rc))")
            }
        }

        self.isConnected = false
        self.isSyncing = false
        self.syncProgress = nil
        self.lastError = nil
    }

    /// Clear only the persisted sync-state snapshot while keeping headers/filters.
    public func clearSyncState() throws {
        guard let client = client else { throw SPVError.notInitialized }

        let rc = dash_spv_ffi_client_clear_sync_state(client)
        if rc != 0 {
            if let errorMsg = dash_spv_ffi_get_last_error() {
                let message = String(cString: errorMsg)
                throw SPVError.storageOperationFailed(message)
            } else {
                throw SPVError.storageOperationFailed("Failed to clear sync state (code \(rc))")
            }
        }

        self.syncProgress = nil
        self.lastError = nil
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
        guard self.client != nil else {
            throw SPVError.notInitialized
        }
        
        guard !isSyncing else {
            throw SPVError.alreadySyncing
        }
        
        self.isSyncing = true
        syncCancelled = false
        syncStartTime = Date()
        blocksHit = 0
        // Reset UI progress to known baseline (0%) before events arrive
        self.syncProgress = SPVSyncProgress(
            stage: .headers,
            headerProgress: 0.0,
            masternodeProgress: 0.0,
            transactionProgress: 0.0,
            currentHeight: self.startFromHeight,
            targetHeight: 0,
            filterHeaderHeight: self.startFromHeight,
            filterHeight: self.startFromHeight,
            syncStartedAt: 0,
            startHeight: self.startFromHeight,
            rate: 0.0,
            estimatedTimeRemaining: nil
        )
        
        // Use a stable callback context; create if needed
        let context: CallbackContext
        if let existing = self.callbackContext {
            context = existing
        } else {
            context = CallbackContext(client: self)
            self.callbackContext = context
        }
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()

        guard let clientPtr = self.client else {
            throw SPVError.notInitialized
        }

        // Start sync in the background to avoid blocking the main thread
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let result = dash_spv_ffi_client_sync_to_tip_with_progress(
                clientPtr,
                spvProgressCallback,
                spvCompletionCallback,
                contextPtr
            )

            guard result != 0 else { return }

            let errorMessage: String = {
                if let raw = dash_spv_ffi_get_last_error() {
                    return String(cString: raw)
                }
                return "Unknown error"
            }()

            Task { @MainActor [weak self] in
                guard let self else { return }
                self.isSyncing = false
                self.lastError = errorMessage
            }
        }
        // Filter progress now updates via FFI event callback; no polling needed
    }
    
    public func cancelSync() {
        guard let client = client, isSyncing else { return }

        syncCancelled = true

        let cancelResult = dash_spv_ffi_client_cancel_sync(client)
        if cancelResult != 0, let err = dash_spv_ffi_get_last_error() {
            let message = String(cString: err)
            if swiftLoggingEnabled {
                print("[SPV][Cancel] cancel_sync failed: \(message)")
            }
            lastError = message
        }
        isSyncing = false
    }

    public func stopSync(preserveProgress: Bool = true) {
        guard let client = client else { return }

        let stopResult = dash_spv_ffi_client_stop(client)
        if stopResult != 0, let err = dash_spv_ffi_get_last_error() {
            let message = String(cString: err)
            if swiftLoggingEnabled {
                print("[SPV][Stop] stop failed: \(message)")
            }
            lastError = message
        } else {
            isConnected = false
        }

        isSyncing = false

        if !preserveProgress {
            syncProgress = nil
        }
    }
    
    // MARK: - Event Callbacks
    
    private func setupEventCallbacks() {
        guard let client = client else { return }
        
        let context = CallbackContext(client: self)
        self.callbackContext = context
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()
        
        var callbacks = FFIEventCallbacks()

        callbacks.on_block = onBlockCallbackC
        callbacks.on_transaction = onTransactionCallbackC

        callbacks.on_compact_filter_matched = { _blockHashPtr, _scripts, _wallet, userData in
            guard let userData = userData else { return }
            let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            let clientRef = context.client
            Task { @MainActor [weak clientRef] in
                guard let client = clientRef else { return }
                client.blocksHit &+= 1
                client.delegate?.spvClient(client, didUpdateBlocksHit: client.blocksHit)
            }
        }

        callbacks.user_data = contextPtr
        
        dash_spv_ffi_client_set_event_callbacks(client, callbacks)
    }

    // MARK: - Filter progress event handler
    // MARK: - Event Handlers
    
    fileprivate func handleBlockEvent(height: UInt32, hash: Data) {
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
            var updatedRate: Double = progress.rate
            if lastBlockHeight > 0 {
                // Use signed math and clamp to avoid underflow on reorgs or height resets
                let blocksDiffSigned = Int64(height) - Int64(lastBlockHeight)
                let blocksDiff = blocksDiffSigned > 0 ? blocksDiffSigned : 0

                let timeDiff = Date().timeIntervalSince(syncStartTime ?? Date())
                updatedRate = timeDiff > 0 ? Double(blocksDiff) / timeDiff : 0
            }

            let baseHeight = startFromHeight

            let snapshotFilterHeightRaw = self.getSyncSnapshot()?.lastSyncedFilterHeight ?? baseHeight
            let statsFilterHeightRaw = self.getStats()?.filterHeight ?? Int(baseHeight)
            let statsFilterHeight = statsFilterHeightRaw < Int(baseHeight) ? Int(baseHeight) : statsFilterHeightRaw
            let snapshotFilterHeight = max(Int(baseHeight), Int(snapshotFilterHeightRaw))

            let bestObservedFilterHeight = max(Int(progress.filterHeight), max(snapshotFilterHeight, statsFilterHeight))
            let clampedFilterHeight = min(bestObservedFilterHeight, Int(UInt32.max))
            let newFilterHeight = UInt32(clampedFilterHeight)

            let candidateTarget = max(progress.targetHeight, max(progress.filterHeaderHeight, newFilterHeight))
            let denominator = max(1.0, Double(candidateTarget) - Double(baseHeight))
            let filterNumerator = max(0.0, Double(newFilterHeight) - Double(baseHeight))
            let computedTransactionProgress = min(1.0, filterNumerator / denominator)

            let filterHeadersDone = progress.filterHeaderHeight >= progress.targetHeight || progress.masternodeProgress >= 0.999
            let stageAllowsFilters = progress.stage == .transactions || progress.stage == .complete
            let filtersStageReady = stageAllowsFilters || filterHeadersDone
            let nextFilterHeight = filtersStageReady ? newFilterHeight : progress.filterHeight
            let nextTransactionProgress = filtersStageReady
                ? max(progress.transactionProgress, computedTransactionProgress)
                : progress.transactionProgress

            let nextStage: SPVSyncStage
            if progress.stage == .complete {
                nextStage = .complete
            } else if filtersStageReady && nextTransactionProgress > progress.transactionProgress {
                nextStage = .transactions
            } else {
                nextStage = progress.stage
            }

            let updatedProgress = SPVSyncProgress(
                stage: nextStage,
                headerProgress: progress.headerProgress,
                masternodeProgress: progress.masternodeProgress,
                transactionProgress: nextTransactionProgress,
                currentHeight: height,
                targetHeight: candidateTarget,
                filterHeaderHeight: progress.filterHeaderHeight,
                filterHeight: nextFilterHeight,
                syncStartedAt: progress.syncStartedAt,
                startHeight: baseHeight,
                rate: updatedRate,
                estimatedTimeRemaining: progress.estimatedTimeRemaining
            )

            syncProgress = updatedProgress
            delegate?.spvClient(self, didUpdateSyncProgress: updatedProgress)

            // Always record the latest observed height (even across reorgs)
            lastBlockHeight = height
        }
    }
    
    fileprivate func handleTransactionEvent(txid: Data, confirmed: Bool, amount: Int64, addresses: [String], blockHeight: UInt32?) {
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
        
        return dash_spv_ffi_client_get_wallet_manager(client)
    }

    /// Produce a Swift wallet manager that shares the SPV client's underlying wallet state.
    /// Callers are responsible for retaining the returned instance for as long as needed.
    public func makeSharedWalletManager() throws -> WalletManager {
        guard let client = client else { throw SPVError.notInitialized }
        return try WalletManager(fromSPVClient: client)
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
            filtersDownloaded: UInt64(statsPtr.pointee.filters_downloaded),
            filterHeadersDownloaded: UInt64(statsPtr.pointee.filter_headers_downloaded),
            blocksProcessed: UInt64(statsPtr.pointee.blocks_processed),
            mempoolSize: 0 // mempool_size not available in current FFI
        )
        
        dash_spv_ffi_spv_stats_destroy(statsPtr)
        
        return stats
    }

    // MARK: - Tip Info
    /// Returns the current chain tip height known to the client (absolute), or nil if unavailable.
    public func getTipHeight() -> UInt32? {
        guard let client = client else { return nil }
        var out: UInt32 = 0
        let rc = dash_spv_ffi_client_get_tip_height(client, &out)
        if rc == 0 { return out }
        return nil
    }

    /// Returns the current chain tip hash (32 bytes) known to the client, or nil if unavailable.
    public func getTipHash() -> Data? {
        guard let client = client else { return nil }
        var buf = [UInt8](repeating: 0, count: 32)
        let rc = buf.withUnsafeMutableBufferPointer { bp -> Int32 in
            guard let base = bp.baseAddress else { return -1 }
            return dash_spv_ffi_client_get_tip_hash(client, base)
        }
        if rc == 0 { return Data(buf) }
        return nil
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
        switch network.rawValue {
        case 0: ffiNet = FFINetwork(rawValue: 0)
        case 1: ffiNet = FFINetwork(rawValue: 1)
        case 3: ffiNet = FFINetwork(rawValue: 3)
        default: ffiNet = FFINetwork(rawValue: 1)
        }

        var outHeight: UInt32 = 0
        var outHash = [UInt8](repeating: 0, count: 32)
        let rc: Int32 = outHash.withUnsafeMutableBufferPointer { buf in
            dash_spv_ffi_checkpoint_latest(ffiNet, &outHeight, buf.baseAddress)
        }
        guard rc == 0 else { return nil }
        return outHeight
    }

    /// Static helper: get latest checkpoint height for an arbitrary network
    /// without depending on the client's configured network.
    public static func latestCheckpointHeight(forNetwork net: DashSDKNetwork) -> UInt32? {
        let ffiNet: FFINetwork
        switch net.rawValue {
        case 0: ffiNet = FFINetwork(rawValue: 0)
        case 1: ffiNet = FFINetwork(rawValue: 1)
        case 3: ffiNet = FFINetwork(rawValue: 3)
        default: ffiNet = FFINetwork(rawValue: 1)
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
        switch network.rawValue {
            case 0: ffiNet = FFINetwork(rawValue: 0)
            case 1: ffiNet = FFINetwork(rawValue: 1)
            case 3: ffiNet = FFINetwork(rawValue: 3)
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

@MainActor
private class CallbackContext {
    weak var client: SPVClient?
    
    init(client: SPVClient) {
        self.client = client
    }

    func handleProgressUpdate(_ ffiProgress: FFIDetailedSyncProgress) {
        guard let client = self.client else { return }

        let overview = ffiProgress.overview
        client.peerCount = Int(overview.peer_count)

        var stage = SPVSyncStage(ffiStage: ffiProgress.stage)
        let estimatedTime: TimeInterval? = (ffiProgress.estimated_seconds_remaining > 0)
            ? TimeInterval(ffiProgress.estimated_seconds_remaining)
            : nil

        let syncStartTimestamp = ffiProgress.sync_start_timestamp
        var previous = client.syncProgress
        if syncStartTimestamp > 0 {
            if syncStartTimestamp != client.currentSyncStartTimestamp {
                client.currentSyncStartTimestamp = syncStartTimestamp
                previous = nil
            } else {
                client.currentSyncStartTimestamp = syncStartTimestamp
            }
        } else if client.currentSyncStartTimestamp != 0 {
            // Keep previous timestamp when FFI does not expose it
        }

        if client.swiftLoggingEnabled {
            let pct = max(0.0, min(ffiProgress.percentage, 100.0))
            let cur = overview.header_height
            let tot = ffiProgress.total_height
            let rate = ffiProgress.headers_per_second
            let eta = ffiProgress.estimated_seconds_remaining
            let filterHeaders = overview.filter_header_height
            let filters = overview.last_synced_filter_height
            print("[SPV][Progress] stage=\(stage.rawValue) header=\(cur)/\(tot) filterHeaders=\(filterHeaders) filters=\(filters) pct=\(pct) rate=\(rate) eta=\(eta)")
        }

        let safeBase: UInt32 = (client.startFromHeight > ffiProgress.total_height) ? 0 : client.startFromHeight

        let reportedHeader = overview.header_height
        let reportedTarget = max(ffiProgress.total_height, reportedHeader)
        let usesAbsolute = reportedHeader >= safeBase && reportedTarget >= safeBase

        let absoluteHeader: UInt32 = usesAbsolute ? max(reportedHeader, safeBase) : safeBase &+ reportedHeader
        let absoluteTarget: UInt32 = usesAbsolute ? max(reportedTarget, safeBase) : safeBase &+ reportedTarget

        let reportedFilterHeader = overview.filter_header_height
        var absoluteFilterHeader: UInt32 = usesAbsolute ? max(reportedFilterHeader, safeBase) : safeBase &+ reportedFilterHeader

        let reportedFilter = overview.last_synced_filter_height
        var absoluteFilter: UInt32 = usesAbsolute ? max(reportedFilter, safeBase) : safeBase &+ reportedFilter

        let range = max(1.0, Double(absoluteTarget) - Double(safeBase))
        var headerProgress = min(1.0, max(0.0, (Double(absoluteHeader) - Double(safeBase)) / range))
        let rawFilterHeaderProgress = min(1.0, max(0.0, (Double(absoluteFilterHeader) - Double(safeBase)) / range))
        let rawFilterProgress = min(1.0, max(0.0, (Double(absoluteFilter) - Double(safeBase)) / range))

        let filtersHeightAbsolute = absoluteFilter
        let nearTarget: (UInt32, UInt32) -> Bool = { current, target in
            guard target > 0 else { return false }
            if current >= target { return true }
            let remaining = target &- current
            return remaining <= 1
        }

        let headerDone = nearTarget(absoluteHeader, absoluteTarget)
        let filterHeadersDone = nearTarget(absoluteFilterHeader, absoluteTarget)
        let filtersStarted = (filtersHeightAbsolute > safeBase) || (overview.filters_downloaded > 0)
        let filtersDone = filtersStarted && nearTarget(filtersHeightAbsolute, absoluteTarget)

        if stage != .complete {
            if headerDone && filterHeadersDone && filtersDone {
                stage = .complete
            } else if headerDone && filterHeadersDone {
                stage = .transactions
            } else if headerDone {
                stage = .masternodes
            } else {
                stage = .headers
            }
        }

        if let prev = previous {
            headerProgress = max(prev.headerProgress, headerProgress)
        }
        if stage != .headers {
            headerProgress = 1.0
        }

        var filterHeaderProgress = rawFilterHeaderProgress
        var filterProgress = rawFilterProgress

        switch stage {
        case .headers:
            absoluteFilterHeader = safeBase
            absoluteFilter = safeBase
            filterHeaderProgress = 0.0
            filterProgress = 0.0
        case .masternodes:
            if filterHeadersDone {
                filterHeaderProgress = 1.0
                absoluteFilterHeader = max(absoluteFilterHeader, absoluteTarget)
            }
            absoluteFilter = safeBase
            filterProgress = 0.0
        case .transactions:
            if filterHeadersDone {
                filterHeaderProgress = 1.0
                absoluteFilterHeader = max(absoluteFilterHeader, absoluteTarget)
            }
            if !filtersStarted {
                absoluteFilter = safeBase
                filterProgress = 0.0
            }
        case .complete:
            if filterHeadersDone {
                filterHeaderProgress = 1.0
                absoluteFilterHeader = max(absoluteFilterHeader, absoluteTarget)
            }
            if filtersDone {
                filterProgress = 1.0
                absoluteFilter = max(absoluteFilter, absoluteTarget)
            }
        case .idle:
            absoluteFilterHeader = safeBase
            absoluteFilter = safeBase
            filterHeaderProgress = 0.0
            filterProgress = 0.0
        }

        let previousStage = previous?.stage ?? .idle
        let previousMasternode = (previousStage == .masternodes || previousStage == .transactions || previousStage == .complete) ? previous?.masternodeProgress ?? 0.0 : 0.0
        let previousTransaction = (previousStage == .transactions || previousStage == .complete) ? previous?.transactionProgress ?? 0.0 : 0.0

        let masternodeProgress = max(previousMasternode, filterHeaderProgress)
        let transactionProgress = max(previousTransaction, filterProgress)

        let progress = SPVSyncProgress(
            stage: stage,
            headerProgress: headerProgress,
            masternodeProgress: masternodeProgress,
            transactionProgress: transactionProgress,
            currentHeight: absoluteHeader,
            targetHeight: absoluteTarget,
            filterHeaderHeight: min(absoluteFilterHeader, absoluteTarget),
            filterHeight: min(absoluteFilter, absoluteTarget),
            syncStartedAt: TimeInterval(syncStartTimestamp > 0 ? syncStartTimestamp : client.currentSyncStartTimestamp),
            startHeight: safeBase,
            rate: ffiProgress.headers_per_second,
            estimatedTimeRemaining: estimatedTime
        )

        let now = Date().timeIntervalSince1970
        if now - client.lastProgressUIUpdate >= client.progressUICoalesceInterval {
            client.lastProgressUIUpdate = now
            client.syncProgress = progress
            client.delegate?.spvClient(client, didUpdateSyncProgress: progress)
        } else {
            client.syncProgress = progress
        }
    }
    func handleSyncCompletion(success: Bool, error: String?) {

        if client?.swiftLoggingEnabled == true {
            if success {
                print("[SPV][Complete] Sync finished successfully")
            } else {
                print("[SPV][Complete] Sync failed: \(error ?? "unknown error")")
            }
        }

        Task { @MainActor [weak self] in
            guard let client = self?.client else { return }
            if client.swiftLoggingEnabled {
                if success {
                    print("[SPV][Complete] Sync finished successfully")
                } else {
                    let errMsg = error ?? "unknown error"
                    print("[SPV][Complete] Sync failed: \(errMsg)")
                }
            }
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
                        filterHeaderHeight: client.syncProgress?.filterHeaderHeight ?? (client.syncProgress?.targetHeight ?? 0),
                        filterHeight: client.syncProgress?.filterHeight ?? (client.syncProgress?.targetHeight ?? 0),
                        syncStartedAt: client.syncProgress?.syncStartedAt ?? 0,
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

public struct SPVStats: Sendable {
    public let connectedPeers: Int
    public let headerHeight: Int
    public let filterHeight: Int
    public let filtersDownloaded: UInt64
    public let filterHeadersDownloaded: UInt64
    public let blocksProcessed: UInt64
    public let mempoolSize: Int
}

// A lightweight snapshot of sync progress from FFISyncProgress
public struct SPVSyncSnapshot: Sendable {
    public let headerHeight: UInt32
    public let filterHeaderHeight: UInt32
    public let masternodeHeight: UInt32
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

// MARK: - Private global state

@MainActor
private enum LogInitState {
    static var manualInitialized: Bool = false
}
