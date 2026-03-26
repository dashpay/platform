import DashSDKFFI
import Foundation

/// This class wraps the FFIDashSpvClient and provides a Swift interface for interacting with it
///
/// All dash_spv_ffi_* function calls are being wrapped in this class to avoid missusage
///
/// The FFI is still a work in progress, that means that some functionalities are not yet
/// implemented or may change in the future, this wrap was made with that in mind.
///
/// Important limitations:
///  - Once stopped, the SPVClient cannot be restarted without creating a new instance, FFI limitation
///  - The pointers are freed automatically but, to be able to create a new instance (with the same dataDir)
///    without causing the initialization to fail, you must call SPVClient::destroy manually, this is because,
///    FFIDashSpvClient locks the dir to avoid concurrency corruption and its only possible to
///    ensure is unlocked by freeing the pointer, FFI limitation
///  - Clearing the storage stops the SPVClient, a new one has to be created, FFI limitation
class SPVClient: @unchecked Sendable {
    private var progressUpdateEventHandler: SPVProgressUpdateEventHandler?
    private var syncEventsHandler: SPVSyncEventsHandler?
    private var networkEventsHandler: SPVNetworkEventsHandler?
    private var walletEventsHandler: SPVWalletEventsHandler?

    // FFI handles
    private var client: UnsafeMutablePointer<FFIDashSpvClient>?
    private var config: UnsafeMutablePointer<FFIClientConfig>?

    // Sync tracking

    fileprivate let swiftLoggingEnabled: Bool = {
        if let env = ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"], env.lowercased() == "1" || env.lowercased() == "true" {
            return true
        }
        return false
    }()

    init(
        network: Network = DashSDKNetwork(rawValue: 1),
        dataDir: String?,
        startHeight: UInt32,
        progressHandler: SPVProgressUpdateEventHandler? = nil,
        syncHandler: SPVSyncEventsHandler? = nil,
        networkHandler: SPVNetworkEventsHandler? = nil,
        walletHandler: SPVWalletEventsHandler? = nil
    ) throws {
        // Retain handlers so their pointers remain valid for the lifetime of the client
        self.progressUpdateEventHandler = progressHandler
        self.syncEventsHandler = syncHandler
        self.networkEventsHandler = networkHandler
        self.walletEventsHandler = walletHandler

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

        // Build unified event callbacks and create client
        let callbacks = buildFFIEventCallbacks(
            progressHandler: progressHandler,
            syncHandler: syncHandler,
            networkHandler: networkHandler,
            walletHandler: walletHandler
        )

        let client = dash_spv_ffi_client_new(configPtr, callbacks)
        guard let client = client else {
            print("[SPV][Init] Failed to create client: \(SPVClient.getLastDashFFIError())")
            throw SPVError.initializationFailed
        }

        self.client = client
        config = configPtr
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

    func getSyncProgress() -> SPVSyncProgress {
        guard let ptr = dash_spv_ffi_client_get_sync_progress(client) else {
            print("[SPV][GetSyncProgress] Failed to get sync progress (Should only fail if client is nil, but client is not nil)")
            return SPVSyncProgress.default()
        }
        defer { dash_spv_ffi_sync_progress_destroy(ptr) }
        let p = ptr.pointee

        return SPVSyncProgress(p)
    }

    static func getLastDashFFIError() -> String {
        guard let errorMsg = dash_spv_ffi_get_last_error() else { return "No error" }
        return String(cString: errorMsg)
    }

    // MARK: - Event Handlers
    //
    // Event handlers are now set at client creation time via FFIEventCallbacks.
    // The new dashcore FFI uses a unified EventHandler pattern instead of
    // individual callback setters. Pass handlers to the SPVClient init.

    /// Trigger a manual progress update from the current sync state.
    /// Useful for obtaining an initial progress snapshot after creation.
    func triggerProgressUpdate() {
        progressUpdateEventHandler?.onProgressUpdate(getSyncProgress())
    }

    /// Enable/disable masternode sync. If the client is running, apply the update immediately.
    func setMasternodeSyncEnabled(_ enabled: Bool) throws {
        var rc = dash_spv_ffi_config_set_masternode_sync_enabled(config, enabled)
        if rc != 0 { throw SPVError.configurationFailed }

        rc = dash_spv_ffi_client_update_config(client, config)
        if rc != 0 { throw SPVError.configurationFailed }
    }

    /// Clear all persisted SPV storage (headers, filters, metadata, sync state).
    func clearStorage() throws {
        let rc = dash_spv_ffi_client_clear_storage(client)
        if rc != 0 {
            throw SPVError.storageOperationFailed(SPVClient.getLastDashFFIError())
        }

        // TODO:
        // Manually calling the event doesn't look like the right approach,
        // if FFISPVClient could send us an event callback automatically...
        progressUpdateEventHandler?.onProgressUpdate(getSyncProgress())
    }

    func destroy() {
        dash_spv_ffi_client_destroy(client)
        dash_spv_ffi_config_destroy(config)
        
        client = nil
        config = nil
    }

    // MARK: - Synchronization

    func startSync() async throws {
        let result = dash_spv_ffi_client_run(
            client
        )

        if result != 0 {
            throw SPVError.syncFailed(SPVClient.getLastDashFFIError())
        }
    }

    func stopSync() {
        let cancelResult = dash_spv_ffi_client_stop(client)
        if cancelResult != 0 {
            let message = SPVClient.getLastDashFFIError()
            if swiftLoggingEnabled {
                print("[SPV][Cancel] client stop failed: \(message)")
            }
        }
    }

    // MARK: - Wallet Manager Access

    /// Produce a Swift wallet manager that shares the SPV client's underlying wallet state.
    /// Callers are responsible for retaining the returned instance for as long as needed.
    func getWalletManager() throws -> WalletManager {
        // This ffi call is expected to never fail
        let ffiWalletManager = dash_spv_ffi_client_get_wallet_manager(client)!

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
        case let .startFailed(reason):
            return "Failed to start SPV client: \(reason)"
        case .alreadySyncing:
            return "SPV client is already syncing"
        case let .syncFailed(reason):
            return "Sync failed: \(reason)"
        case let .storageOperationFailed(reason):
            return reason
        }
    }
}
