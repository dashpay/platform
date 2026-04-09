import DashSDKFFI
import Foundation

/// This class wraps the FFIDashSpvClient and provides a Swift interface for interacting with it
///
/// Important limitations:
///  - Once stopped, the SPVClient cannot be restarted without creating a new instance, FFI limitation
///  - The pointers are freed automatically but, to be able to create a new instance (with the same dataDir)
///    without causing the initialization to fail, you must call SPVClient::destroy manually, this is because,
///    FFIDashSpvClient locks the dir to avoid concurrency corruption and its only possible to
///    ensure is unlocked by freeing the pointer, FFI limitation
///  - Clearing the storage stops the SPVClient, a new one has to be created, FFI limitation
public class SPVClient: @unchecked Sendable {
    private var spvEventHandlers: SPVEventHandlers?

    // FFI handle
    private var client: UnsafeMutablePointer<FFIDashSpvClient>?

    // FFIClientConfig wrapper
    private var config: SPVConfig

    fileprivate let swiftLoggingEnabled: Bool = {
        if let env = ProcessInfo.processInfo.environment["SPV_SWIFT_LOG"], env.lowercased() == "1" || env.lowercased() == "true" {
            return true
        }
        return false
    }()

    /// Its recomended to avoid reusing the same config for multiple clients,
    /// just in case the FFI layer stops cloning the config internally.
    public init(config: consuming SPVConfig, eventHandlers: SPVEventHandlers? = nil) throws {
        if swiftLoggingEnabled {
            let level = (ProcessInfo.processInfo.environment["SPV_LOG"] ?? "off")
            print("[SPV][Log] Initialized SPV logging level=\(level)")
        }

        self.config = config

        // Store event handlers so the pointers remain valid
        self.spvEventHandlers = eventHandlers

        // Create client with event callbacks
        let callbacks = eventHandlers?.intoFFIEventCallbacks() ?? FFIEventCallbacks()
        let client = dash_spv_ffi_client_new(self.config.config, callbacks)
        guard let client = client else {
            print("[SPV][Init] Failed to create client: \(SPVClient.getLastDashFFIError())")
            throw SPVError.initializationFailed
        }

        self.client = client
    }

    deinit {
        destroy()
    }

    public func getSyncProgress() -> SPVSyncProgress {
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

    public func updateConfig(_ config: consuming SPVConfig) {
        self.config = config;
        dash_spv_ffi_client_update_config(client, self.config.config)
    }

    /// Clear all persisted SPV storage (headers, filters, metadata, sync state).
    public func clearStorage() throws {
        let rc = dash_spv_ffi_client_clear_storage(client)
        if rc != 0 {
            throw SPVError.storageOperationFailed(SPVClient.getLastDashFFIError())
        }

        // TODO:
        // Manually calling the event doesn't look like the right approach,
        // if FFISPVClient could send us an event callback automatically...
        spvEventHandlers?.progress.onProgressUpdate(getSyncProgress())
    }

    public func destroy() {
        dash_spv_ffi_client_destroy(client)

        client = nil
    }

    // MARK: - Broadcast Transactions

    public func broadcastTransaction(_ transactionData: Data) throws {
        try transactionData.withUnsafeBytes { (ptr: UnsafeRawBufferPointer) in
            guard let txBytes = ptr.bindMemory(to: UInt8.self).baseAddress else {
                throw SPVError.transactionBroadcastFailed("Invalid transaction data pointer")
            }
            let result = dash_spv_ffi_client_broadcast_transaction(
                client,
                txBytes,
                UInt(transactionData.count)
            )

            if result != 0 {
                throw SPVError.transactionBroadcastFailed(SPVClient.getLastDashFFIError())
            }
        }
    }

    // MARK: - Synchronization

    public func startSync() async throws {
        let result = dash_spv_ffi_client_run(
            client
        )

        if result != 0 {
            throw SPVError.syncFailed(SPVClient.getLastDashFFIError())
        }
    }

    public func stopSync() {
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
    public func getWalletManager() throws -> WalletManager {
        // This ffi call is expected to never fail
        let ffiWalletManager = dash_spv_ffi_client_get_wallet_manager(client)!

        return try WalletManager(handle: ffiWalletManager)
    }
}

// MARK: - SPV Client Error handling

public enum SPVError: LocalizedError {
    case notInitialized
    case alreadyInitialized
    case initializationFailed
    case startFailed(String)
    case alreadySyncing
    case syncFailed(String)
    case storageOperationFailed(String)
    case transactionBroadcastFailed(String)

    public var errorDescription: String? {
        switch self {
        case .notInitialized:
            return "SPV client is not initialized"
        case .alreadyInitialized:
            return "SPV client is already initialized"
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
        case let .transactionBroadcastFailed(reason):
            return "Transaction broadcast failed: \(reason)"
        }
    }
}
