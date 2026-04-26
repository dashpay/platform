import Foundation

/// SPV sync state mirroring Rust's `dash_spv::sync::SyncState`.
public enum PlatformSpvSyncState: UInt32, Sendable {
    case waitForEvents = 0
    case waitingForConnections = 1
    case syncing = 2
    case synced = 3
    case error = 4

    public var isRunning: Bool {
        switch self {
        case .waitingForConnections, .syncing: return true
        case .waitForEvents, .synced, .error: return false
        }
    }
}

/// Progress for one of the SPV sub-managers (headers, filters, masternodes, etc.).
public struct PlatformSpvSubProgress: Sendable {
    public let state: PlatformSpvSyncState
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let percentage: Double
}

/// Aggregate SPV sync progress snapshot.
public struct PlatformSpvSyncProgress: Sendable {
    public let overallState: PlatformSpvSyncState
    public let overallPercentage: Double

    public let headers: PlatformSpvSubProgress?
    public let filterHeaders: PlatformSpvSubProgress?
    public let filters: PlatformSpvSubProgress?
    public let masternodes: PlatformSpvSubProgress?

    public static let empty = PlatformSpvSyncProgress(
        overallState: .waitForEvents,
        overallPercentage: 0.0,
        headers: nil,
        filterHeaders: nil,
        filters: nil,
        masternodes: nil
    )

    init(_ ffi: FFISpvSyncProgress) {
        self.overallState = PlatformSpvSyncState(rawValue: ffi.overall_state) ?? .waitForEvents
        self.overallPercentage = ffi.overall_percentage
        self.headers = ffi.has_headers
            ? PlatformSpvSubProgress(
                state: PlatformSpvSyncState(rawValue: ffi.headers_state) ?? .waitForEvents,
                currentHeight: ffi.headers_current,
                targetHeight: ffi.headers_target,
                percentage: ffi.headers_percentage
            )
            : nil
        self.filterHeaders = ffi.has_filter_headers
            ? PlatformSpvSubProgress(
                state: PlatformSpvSyncState(rawValue: ffi.filter_headers_state) ?? .waitForEvents,
                currentHeight: ffi.filter_headers_current,
                targetHeight: ffi.filter_headers_target,
                percentage: ffi.filter_headers_percentage
            )
            : nil
        self.filters = ffi.has_filters
            ? PlatformSpvSubProgress(
                state: PlatformSpvSyncState(rawValue: ffi.filters_state) ?? .waitForEvents,
                currentHeight: ffi.filters_current,
                targetHeight: ffi.filters_target,
                percentage: ffi.filters_percentage
            )
            : nil
        self.masternodes = ffi.has_masternodes
            ? PlatformSpvSubProgress(
                state: PlatformSpvSyncState(rawValue: ffi.masternodes_state) ?? .waitForEvents,
                currentHeight: ffi.masternodes_current,
                targetHeight: ffi.masternodes_target,
                percentage: ffi.masternodes_percentage
            )
            : nil
    }

    internal init(
        overallState: PlatformSpvSyncState,
        overallPercentage: Double,
        headers: PlatformSpvSubProgress?,
        filterHeaders: PlatformSpvSubProgress?,
        filters: PlatformSpvSubProgress?,
        masternodes: PlatformSpvSubProgress?
    ) {
        self.overallState = overallState
        self.overallPercentage = overallPercentage
        self.headers = headers
        self.filterHeaders = filterHeaders
        self.filters = filters
        self.masternodes = masternodes
    }
}

/// Config for starting the SPV sync.
public struct PlatformSpvStartConfig {
    public var dataDir: String
    public var network: PlatformNetwork
    public var userAgent: String?
    public var peers: [String]
    public var restrictToConfiguredPeers: Bool
    public var startFromHeight: UInt32
    public var masternodeSyncEnabled: Bool

    public init(
        dataDir: String,
        network: PlatformNetwork,
        userAgent: String? = nil,
        peers: [String] = [],
        restrictToConfiguredPeers: Bool = false,
        startFromHeight: UInt32 = 0,
        masternodeSyncEnabled: Bool = true
    ) {
        self.dataDir = dataDir
        self.network = network
        self.userAgent = userAgent
        self.peers = peers
        self.restrictToConfiguredPeers = restrictToConfiguredPeers
        self.startFromHeight = startFromHeight
        self.masternodeSyncEnabled = masternodeSyncEnabled
    }
}

// MARK: - PlatformWalletManager SPV extensions

extension PlatformWalletManager {
    /// Poll the current SPV sync progress.
    ///
    /// Returns a snapshot with all sub-progresses set to `nil` if the
    /// SPV client is not yet running.
    public func syncProgress() throws -> PlatformSpvSyncProgress {
        var ffi = FFISpvSyncProgress(
            overall_state: 0, overall_percentage: 0,
            has_headers: false, headers_state: 0, headers_current: 0,
            headers_target: 0, headers_percentage: 0,
            has_filter_headers: false, filter_headers_state: 0,
            filter_headers_current: 0, filter_headers_target: 0,
            filter_headers_percentage: 0,
            has_filters: false, filters_state: 0, filters_current: 0,
            filters_target: 0, filters_percentage: 0,
            has_masternodes: false, masternodes_state: 0,
            masternodes_current: 0, masternodes_target: 0,
            masternodes_percentage: 0
        )
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_sync_progress(handle, &ffi, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return PlatformSpvSyncProgress(ffi)
    }

    /// Whether the SPV client is currently running.
    public func isSpvRunning() throws -> Bool {
        var running: Bool = false
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_spv_is_running(handle, &running, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return running
    }

    /// Start the SPV client in the background.
    ///
    /// Spawns the sync loop on the shared tokio runtime and returns
    /// immediately. Use [`stopSpv`] to cancel.
    public func startSpv(config: PlatformSpvStartConfig) throws {
        var error = PlatformWalletFFIError()

        // Peer array: allocate contiguous C strings.
        let peerCStrings: [UnsafeMutablePointer<CChar>?] = config.peers.map { strdup($0) }
        defer { peerCStrings.forEach { if let p = $0 { free(p) } } }

        let result = config.dataDir.withCString { dataDirPtr -> PlatformWalletFFIResult in
            let userAgentPtr = config.userAgent.flatMap { strdup($0) }
            defer { if let p = userAgentPtr { free(p) } }

            return peerCStrings.withUnsafeBufferPointer { peersBuf in
                let peersPtr: UnsafePointer<UnsafePointer<CChar>?>? = peersBuf.baseAddress.map {
                    UnsafeRawPointer($0).assumingMemoryBound(to: UnsafePointer<CChar>?.self)
                }
                return platform_wallet_manager_spv_start(
                    handle,
                    dataDirPtr,
                    config.network.rawValue,
                    userAgentPtr,
                    peersPtr,
                    peerCStrings.count,
                    config.restrictToConfiguredPeers,
                    config.startFromHeight,
                    config.masternodeSyncEnabled,
                    &error
                )
            }
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Stop the SPV client (idempotent).
    public func stopSpv() throws {
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_spv_stop(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Clear all persisted SPV storage (headers, filters, state).
    ///
    /// The SPV client must be running.
    public func clearSpvStorage() throws {
        var error = PlatformWalletFFIError()
        let result = platform_wallet_manager_spv_clear_storage(handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }
}
