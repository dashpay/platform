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

@MainActor
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
    private var config: OpaquePointer?
    
    // Callback context
    private var callbackContext: CallbackContext?
    
    // Network
    private let network: Network
    
    // Sync tracking
    private var syncStartTime: Date?
    private var lastBlockHeight: UInt32 = 0
    internal var syncCancelled = false
    
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
    
    public func initialize(dataDir: String? = nil) throws {
        guard client == nil else {
            throw SPVError.alreadyInitialized
        }
        
        // Create configuration based on network raw value
        let rawConfigPtr: UnsafeMutableRawPointer? = {
            switch network {
            case DashSDKNetwork(rawValue: 0):
                return UnsafeMutableRawPointer(dash_spv_ffi_config_mainnet())
            case DashSDKNetwork(rawValue: 1):
                return UnsafeMutableRawPointer(dash_spv_ffi_config_testnet())
            case DashSDKNetwork(rawValue: 2):
                // Map devnet to custom FFINetwork value 3
                return UnsafeMutableRawPointer(dash_spv_ffi_config_new(FFINetwork(rawValue: 3)))
            default:
                return UnsafeMutableRawPointer(dash_spv_ffi_config_testnet())
            }
        }()
        
        guard let rawConfigPtr = rawConfigPtr else {
            throw SPVError.configurationFailed
        }
        
        let configPtr = OpaquePointer(rawConfigPtr)
        
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
        
        // Create client
        client = dash_spv_ffi_client_new(configPtr)
        guard client != nil else {
            throw SPVError.initializationFailed
        }
        
        // Store config for cleanup
        config = configPtr
        
        // Set up event callbacks
        setupEventCallbacks()
    }
    
    public func start() throws {
        guard let client = client else {
            throw SPVError.notInitialized
        }
        
        let result = dash_spv_ffi_client_start(client)
        if result != 0 {
            if let errorMsg = dash_spv_ffi_get_last_error() {
                let error = String(cString: errorMsg)
                lastError = error
                throw SPVError.startFailed(error)
            }
            throw SPVError.startFailed("Unknown error")
        }
        
        isConnected = true
    }
    
    public func stop() {
        guard let client = client else { return }
        
        dash_spv_ffi_client_stop(client)
        isConnected = false
        isSyncing = false
        syncProgress = nil
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
        
        isSyncing = true
        syncCancelled = false
        syncStartTime = Date()
        
        // Create callback context that captures self weakly
        let context = CallbackContext(client: self)
        self.callbackContext = context
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()
        
        // Start sync with progress callbacks
        // Use global C callbacks that can access context via userData
        let result = dash_spv_ffi_client_sync_to_tip_with_progress(
            client,
            spvProgressCallback,
            spvCompletionCallback,
            contextPtr
        )
        
        if result != 0 {
            isSyncing = false
            throw SPVError.syncFailed(lastError ?? "Unknown error")
        }
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
        
        delegate?.spvClient(self, didReceiveTransaction: transaction)
    }
    
    // MARK: - Wallet Manager Access
    
    public func getWalletManager() -> OpaquePointer? {
        guard let client = client else { return nil }
        
        let managerPtr = dash_spv_ffi_client_get_wallet_manager(client)
        return OpaquePointer(managerPtr)
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
        
        let progress = SPVSyncProgress(
            stage: stage,
            headerProgress: min(ffiProgress.percentage / 0.3, 1.0),
            masternodeProgress: min(max((ffiProgress.percentage - 0.3) / 0.4, 0), 1.0),
            transactionProgress: min(max((ffiProgress.percentage - 0.7) / 0.3, 0), 1.0),
            currentHeight: ffiProgress.current_height,
            targetHeight: ffiProgress.total_height,
            rate: ffiProgress.headers_per_second,
            estimatedTimeRemaining: estimatedTime
        )
        
        Task { @MainActor in
            guard let client = self.client else { return }
            client.syncProgress = progress
            client.delegate?.spvClient(client, didUpdateSyncProgress: progress)
        }
    }
    
    func handleSyncCompletion(success: Bool, errorMsg: UnsafePointer<CChar>?) {
        var error: String? = nil
        if let errorMsg = errorMsg {
            error = String(cString: errorMsg)
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
