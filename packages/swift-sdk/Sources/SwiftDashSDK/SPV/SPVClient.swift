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

    public static func from(_ ffiProgress: FFIDetailedSyncProgress) -> SPVSyncProgress {
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
public class SPVClient {
    public var isConnected = false
    public var isSyncing = false
    var blocksHit: Int = 0

    // Delegate for callbacks
    public let delegate: SPVClientDelegate

    // FFI handles
    private let client: UnsafeMutablePointer<FFIDashSpvClient>
    private let config: UnsafeMutablePointer<FFIClientConfig>
    private var hasBeenFreed = false

    // Public accessor for client handle (needed for filter match queries)
    public var clientHandle: UnsafeMutablePointer<FFIDashSpvClient>? {
        return client
    }

    // Callback context
    private var callbackContext: CallbackContext?

    // Sync tracking
    internal var syncCancelled = false
    fileprivate let swiftLoggingEnabled: Bool = {
        if let env = ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"], env.lowercased() == "1" || env.lowercased() == "true" {
            return true
        }
        return false
    }()

    // Removed: Temporary poller for filter header progress (now event-driven via FFI)

    public init(network: Network = DashSDKNetwork(rawValue: 1), dataDir: String?, startHeight: UInt32, delegate: SPVClientDelegate) throws {
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
        self.delegate = delegate
        self.config = configPtr

        self.delegate.spvClient(self, didUpdateSyncProgress: self.getSyncProgress())
        
        // Set up event callbacks with stable context
        setupEventCallbacks()
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
        self.delegate.spvClient(self, didUpdateSyncProgress: SPVSyncProgress.default())
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
        blocksHit = 0

        // Use a stable callback context; create if needed
        let context: CallbackContext
        if let existing = self.callbackContext {
            context = existing
        } else {
            context = CallbackContext(client: self)
            self.callbackContext = context
        }
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()
        
        let clientAddr = UInt(bitPattern: self.client)
        let ctxAddr = UInt(bitPattern: contextPtr)
        
        guard let client = UnsafeMutablePointer<FFIDashSpvClient>(bitPattern: clientAddr),
              let contextPtr = UnsafeMutableRawPointer(bitPattern: ctxAddr) else { return }
        let result = dash_spv_ffi_client_sync_to_tip_with_progress(
            client,
            spvProgressCallback,
            spvCompletionCallback,
            contextPtr
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
                client.delegate.spvClient(client, didUpdateBlocksHit: client.blocksHit)
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

        delegate.spvClient(self, didReceiveBlock: block)
    }

    fileprivate func handleTransactionEvent(txid: Data, confirmed: Bool, amount: Int64, addresses: [String], blockHeight: UInt32?) {
        let transaction = SPVTransactionEvent(
            txid: txid,
            confirmed: confirmed,
            amount: amount,
            addresses: addresses,
            blockHeight: blockHeight
        )

        delegate.spvClient(self, didReceiveTransaction: transaction)
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

// MARK: - Callback Context

@MainActor
private class CallbackContext {
    weak var client: SPVClient?

    init(client: SPVClient) {
        self.client = client
    }

    func handleProgressUpdate(_ ffiProgress: FFIDetailedSyncProgress) {
        guard let client = self.client else { return }

        let spvSyncProgress = SPVSyncProgress.from(ffiProgress)
        client.delegate.spvClient(client, didUpdateSyncProgress: spvSyncProgress)
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

            client.delegate.spvClient(client, didCompleteSync: success, error: error)
        }
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
