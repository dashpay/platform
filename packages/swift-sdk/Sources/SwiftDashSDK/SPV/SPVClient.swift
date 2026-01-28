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

private func spvProgressCallback(
    progressPtr: UnsafePointer<FFIDetailedSyncProgress>?,
    userData: UnsafeMutableRawPointer?
) {
    guard let progressPtr = progressPtr,
          let userData = userData else { return }
    let snapshot = progressPtr.pointee
    let ptrVal = UInt(bitPattern: userData)
    DispatchQueue.main.async {
        guard let userData = UnsafeMutableRawPointer(bitPattern: ptrVal) else { return }
        let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
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
    let ptrVal = UInt(bitPattern: userData)
    DispatchQueue.main.async {
        guard let userData = UnsafeMutableRawPointer(bitPattern: ptrVal) else { return }
        let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
        context.handleSyncCompletion(success: success, error: errorString)
    }
}

// Global C-compatible event callbacks that use userData context
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
    guard let userData = userData else { return }
    // Synchronously copy 32-byte hash into Swift-owned buffer to avoid TOCTOU
    var hashBytes: [UInt8] = []
    if let hashPtr = hashPtr {
        let raw = UnsafeRawPointer(hashPtr).assumingMemoryBound(to: UInt8.self)
        let buf = UnsafeBufferPointer(start: raw, count: 32)
        hashBytes = Array(buf)
    }
    let ctxAddr = UInt(bitPattern: userData)
    Task { @MainActor in
        guard let userData = UnsafeMutableRawPointer(bitPattern: ctxAddr) else { return }
        let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
        let hashData = Data(hashBytes)
        context.client?.handleBlockEvent(height: height, hash: hashData)
    }
}

private func onTransactionCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ confirmed: Bool,
    _ amount: Int64,
    _ addressesPtr: UnsafePointer<CChar>?,
    _ blockHeight: UInt32,
    _ userData: UnsafeMutableRawPointer?
) {
    guard let userData = userData else { return }
    // Synchronously copy 32-byte txid and address string to Swift-owned values
    var txidBytes: [UInt8] = []
    if let txidPtr = txidPtr {
        let raw = UnsafeRawPointer(txidPtr).assumingMemoryBound(to: UInt8.self)
        let buf = UnsafeBufferPointer(start: raw, count: 32)
        txidBytes = Array(buf)
    }
    var addresses: [String] = []
    if let addressesPtr = addressesPtr {
        let addressesStr = String(cString: addressesPtr)
        addresses = addressesStr.components(separatedBy: ",")
    }
    let ctxAddr = UInt(bitPattern: userData)
    Task { @MainActor in
        guard let userData = UnsafeMutableRawPointer(bitPattern: ctxAddr) else { return }
        let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
        let txid = Data(txidBytes)
        context.client?.handleTransactionEvent(
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
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    /// Absolute blockchain height reached for filter headers.
    public let filterHeaderHeight: UInt32
    /// Absolute blockchain height reached for compact filters.
    public let filterHeight: UInt32
    /// UNIX timestamp (seconds) when the current sync run started. 0 if unavailable.
    public let syncStartedAt: TimeInterval
    public let rate: Double // blocks per second
    public let estimatedTimeRemaining: TimeInterval?
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
    private var client: UnsafeMutablePointer<FFIDashSpvClient>
    private var config: UnsafeMutablePointer<FFIClientConfig>
    private var hasBeenFreed = false

    // Public accessor for client handle (needed for filter match queries)
    public var clientHandle: UnsafeMutablePointer<FFIDashSpvClient>? {
        return client
    }

    // Event polling task
    private var eventPollingTask: Task<Void, Never>?

    // Callback context
    private var callbackContext: CallbackContext?
    
    // Network
    private let network: Network
    private var masternodeSyncEnabled: Bool = true
    
    // Sync tracking
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
    
    public init(network: Network = DashSDKNetwork(rawValue: 1), dataDir: String?, masternodesEnabled: Bool, startHeight: UInt32) throws {
        self.network = network
        
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
                    if rc != 0, let err = dash_spv_ffi_get_last_error() {
                        let msg = String(cString: err)
                        print("[SPV][Config] add_peer failed for \(addr): \(msg)")
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
        self.masternodeSyncEnabled = masternodesEnabled

        _ = dash_spv_ffi_config_set_masternode_sync_enabled(configPtr, masternodeSyncEnabled)

        _ = dash_spv_ffi_config_set_start_from_height(configPtr, startHeight)
        
        // Create client
        let client = dash_spv_ffi_client_new(configPtr)
        guard let client = client else {
            if let errorMsg = dash_spv_ffi_get_last_error() {
              let error = String(cString: errorMsg)
              self.lastError = error
              print("[SPV][Init] Failed to create client: \(error)")
            }
            throw SPVError.initializationFailed
        }
        
        self.client = client
        
        // Store config for cleanup
        config = configPtr
        
        // Set up event callbacks with stable context
        setupEventCallbacks()
    }
    
    deinit {
        if !hasBeenFreed {
            print("[SPV][deinit] WARNING: SPVClient was not freed before deinit, call SPVClient::destroy")
        }
      
        // Stop event polling (synchronously cancel the task)
        eventPollingTask?.cancel()
        // Minimal teardown; prefer explicit stop() by callers.
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
    
    public func start() throws {
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

    /// Clear all persisted SPV storage (headers, filters, metadata, sync state).
    public func clearStorage() throws {
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
        // TODO: clear sync state doesnt exist anymore. Is it needed? Maybe wipe the directory?
//        let rc = dash_spv_ffi_client_clear_sync_state(client)
//        if rc != 0 {
//            if let errorMsg = dash_spv_ffi_get_last_error() {
//                let message = String(cString: errorMsg)
//                throw SPVError.storageOperationFailed(message)
//            } else {
//                throw SPVError.storageOperationFailed("Failed to clear sync state (code \(rc))")
//            }
//        }

        self.syncProgress = nil
        self.lastError = nil
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
        
        callbackContext = nil
        
        self.hasBeenFreed = true
    }
    
    // MARK: - Synchronization
    
    public func startSync() async throws {
        guard !isSyncing else {
            throw SPVError.alreadySyncing
        }
        
        self.isSyncing = true
        syncCancelled = false
        syncStartTime = Date()
        blocksHit = 0

        // Start event polling to drain Rust event queue
        startEventPolling()

        // Reset UI progress to known baseline (0%) before events arrive
        self.syncProgress = SPVSyncProgress(
            stage: .idle,
            currentHeight: 0,
            targetHeight: 0,
            filterHeaderHeight: 0,
            filterHeight: 0,
            syncStartedAt: 0,
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

        // Start sync in the background to avoid blocking the main thread
        // Copy pointer addresses to avoid capturing non-Sendable pointers inside the GCD closure
        let clientAddr = UInt(bitPattern: self.client)
        let ctxAddr = UInt(bitPattern: contextPtr)
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let client = UnsafeMutablePointer<FFIDashSpvClient>(bitPattern: clientAddr),
                  let contextPtr = UnsafeMutableRawPointer(bitPattern: ctxAddr) else { return }
            let result = dash_spv_ffi_client_sync_to_tip_with_progress(
                client,
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
    
    public func stopSync() {
        stopEventPolling()

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
    
    // MARK: - Event Callbacks
    
    private func setupEventCallbacks() {
        let context = CallbackContext(client: self)
        self.callbackContext = context
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()
        
        var callbacks = FFIEventCallbacks()

        // Assign C-compatible top-level functions which match the imported C signatures
        callbacks.on_block = onBlockCallbackC
        callbacks.on_transaction = onTransactionCallbackC

        callbacks.on_compact_filter_matched = { _blockHashPtr, _scripts, _wallet, userData in
            guard let userData = userData else { return }
            let ptrVal = UInt(bitPattern: userData)
            Task { @MainActor in
                guard let userData = UnsafeMutableRawPointer(bitPattern: ptrVal) else { return }
                let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
                guard let client = context.client else { return }
                client.blocksHit &+= 1
                client.delegate?.spvClient(client, didUpdateBlocksHit: client.blocksHit)
            }
        }

        // Mempool: unconfirmed transaction detected for any tracked address
        callbacks.on_mempool_transaction_added = { txidPtr, amount, addressesPtr, _isInstantSend, userData in
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
                    confirmed: false,
                    amount: amount,
                    addresses: addresses,
                    blockHeight: nil
                )
            }
        }

        // Mempool: transaction confirmed
        callbacks.on_mempool_transaction_confirmed = { txidPtr, blockHeight, _blockHashPtr, userData in
            guard let userData = userData else { return }
            let context = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()

            var txid = Data()
            if let txidPtr = txidPtr {
                txid = Data(bytes: txidPtr, count: 32)
            }

            // Amount and addresses are not provided here; emit a confirmation-only update
            let clientRef = context.client
            Task { @MainActor [weak clientRef] in
                clientRef?.handleTransactionEvent(
                    txid: txid,
                    confirmed: true,
                    amount: 0,
                    addresses: [],
                    blockHeight: blockHeight
                )
            }
        }

        // Mempool: transaction removed (expired/replaced/etc). No UI path yet; ignore for now.
        callbacks.on_mempool_transaction_removed = { _txidPtr, _reason, _userData in
            // Intentionally no-op; could surface to UI in future if needed
        }

        // Wallet-specific transaction callback (fires for our wallet, including mempool)
        callbacks.on_wallet_transaction = { _walletId, _accountIndex, txidPtr, confirmed, amount, addressesPtr, blockHeight, _isOurs, userData in
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
    }
    
    fileprivate func handleTransactionEvent(txid: Data, confirmed: Bool, amount: Int64, addresses: [String], blockHeight: UInt32?) {
        let transaction = SPVTransactionEvent(
            txid: txid,
            confirmed: confirmed,
            amount: amount,
            addresses: addresses,
            blockHeight: blockHeight
        )

        delegate?.spvClient(self, didReceiveTransaction: transaction)
    }

    // MARK: - Event Polling

    private func startEventPolling() {
        eventPollingTask?.cancel()

        eventPollingTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { break }
                dash_spv_ffi_client_drain_events(client)
                try? await Task.sleep(nanoseconds: 100_000_000)
            }
        }
    }

    private func stopEventPolling() {
        eventPollingTask?.cancel()
        eventPollingTask = nil
    }

    // MARK: - Wallet Manager Access
    
    public func getWalletManager() -> UnsafeMutablePointer<FFIWalletManager>? {
        return dash_spv_ffi_client_get_wallet_manager(client)
    }

    /// Produce a Swift wallet manager that shares the SPV client's underlying wallet state.
    /// Callers are responsible for retaining the returned instance for as long as needed.
    public func makeSharedWalletManager() throws -> WalletManager {
        return try WalletManager(fromSPVClient: client)
    }
    
    // MARK: - Statistics
    
    public func getStats() -> SPVStats? {
        
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
        var out: UInt32 = 0
        let rc = dash_spv_ffi_client_get_tip_height(client, &out)
        if rc == 0 { return out }
        return nil
    }

    /// Returns the current chain tip hash (32 bytes) known to the client, or nil if unavailable.
    public func getTipHash() -> Data? {
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
        case 2: ffiNet = FFINetwork(rawValue: 2)
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
        case 2: ffiNet = FFINetwork(rawValue: 2)
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
            case 2: ffiNet = FFINetwork(rawValue: 2)
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

        let tipHeight = ffiProgress.total_height;
        let currentBlockHeaderHeight = overview.header_height
        let currentFilterHeaderHeight = overview.filter_header_height
        let currentFilterHeight = overview.last_synced_filter_height

        let progress = SPVSyncProgress(
            stage: stage,
            currentHeight: currentBlockHeaderHeight,
            targetHeight: tipHeight,
            filterHeaderHeight: currentFilterHeaderHeight,
            filterHeight: currentFilterHeight,
            syncStartedAt: TimeInterval(syncStartTimestamp > 0 ? syncStartTimestamp : client.currentSyncStartTimestamp),
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
                        currentHeight: client.syncProgress?.targetHeight ?? 0,
                        targetHeight: client.syncProgress?.targetHeight ?? 0,
                        filterHeaderHeight: client.syncProgress?.filterHeaderHeight ?? 0,
                        filterHeight: client.syncProgress?.filterHeight ?? 0,
                        syncStartedAt: client.syncProgress?.syncStartedAt ?? 0,
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
