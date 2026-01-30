import Foundation
import DashSDKFFI

internal class SPVClient<T: SPVEventHandler & Sendable>: @unchecked Sendable {
    // SPVEventHandler for callbacks
    private let spvEventHandler: T

    // FFI handles
    private let client: UnsafeMutablePointer<FFIDashSpvClient>
    private let config: UnsafeMutablePointer<FFIClientConfig>

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

        self.client = client
        self.config = configPtr
        
        // Set up events callbacks
        self.spvEventHandler = spvEventHandler
        dash_spv_ffi_client_set_event_callbacks(client, self.spvEventHandler.intoFFIEventCallbacks())
        
        // Call the event handler to notify about the initial sync progress
        self.spvEventHandler.spvClient(didUpdateSyncProgress: self.getSyncProgress())
    }

    deinit {
        self.destroy()
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
    
    public func destroy() {
        dash_spv_ffi_client_destroy(client)
        dash_spv_ffi_config_destroy(config)
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

    // MARK: - Synchronization

    public func startSync() async throws {
        let spvEventHandlerPtr = Unmanaged.passUnretained(spvEventHandler).toOpaque()
        
        let result = dash_spv_ffi_client_sync_to_tip_with_progress(
            self.client,
            onSpvProgressCallbackC,
            onSpvCompletionCallbackC,
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

// MARK: - SPV Client Error handling

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
