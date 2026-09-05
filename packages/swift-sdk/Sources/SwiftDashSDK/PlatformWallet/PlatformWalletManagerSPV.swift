import Foundation
import DashSDKFFI

/// SPV sync state mirroring Rust's `dash_spv::sync::SyncState`.
public enum PlatformSpvSyncState: UInt32, Sendable {
    case waitForEvents = 0
    case waitingForConnections = 1
    case syncing = 2
    case synced = 3
    case error = 4
}

/// Progress for one of the SPV sub-managers (headers, filters, masternodes, etc.).
public struct PlatformSpvSubProgress: Sendable, Equatable {
    public let state: PlatformSpvSyncState
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let percentage: Double
}

/// Aggregate SPV sync progress snapshot.
public struct PlatformSpvSyncProgress: Sendable, Equatable {
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

enum CoreRescanDiagnosticResult: String, Sendable, Equatable {
    case armed
    case acceptedNoRewind = "accepted_no_rewind"
    case noOp = "no_op"
}

/// Classifies only what can be proven from the checkpoint visible before the
/// accepted FFI call. A missing checkpoint is not evidence of a rewind.
func coreRescanDiagnosticResult(
    previousSyncedHeight: UInt32?,
    requestedStartHeight: UInt32
) -> CoreRescanDiagnosticResult {
    guard let previousSyncedHeight else { return .acceptedNoRewind }
    if requestedStartHeight < previousSyncedHeight { return .armed }
    if requestedStartHeight == previousSyncedHeight { return .noOp }
    return .acceptedNoRewind
}

/// Node type of a connected SPV peer, classified against the masternode
/// list. Mirrors Rust's `SpvPeerNodeType` / the `SPV_PEER_NODE_TYPE_*`
/// FFI constants.
public enum PlatformSpvPeerNodeType: UInt32, Sendable {
    /// The masternode list hasn't synced yet, so the peer can't be
    /// classified.
    case unknown = 0
    /// Not present in the masternode list.
    case normal = 1
    /// A regular masternode.
    case masternode = 2
    /// A high-performance (evo) masternode.
    case evonode = 3
}

/// One connected SPV peer.
public struct PlatformSpvPeerInfo: Sendable, Equatable, Identifiable {
    /// `ip:port` the client dialed.
    public let address: String
    public let nodeType: PlatformSpvPeerNodeType

    public var id: String { address }
}

/// Config for starting the SPV sync.
///
/// Masternode sync (and the IS/CL P2P subscriptions that come with it)
/// is always enabled — `AssetLockManager::wait_for_proof` requires it
/// to receive InstantSend and ChainLock signatures from peers, so
/// exposing a toggle here would silently break asset-lock-funded
/// identity registration in trusted-SDK setups.
public struct PlatformSpvStartConfig {
    public var dataDir: String
    public var network: Network
    public var userAgent: String?
    public var peers: [String]
    public var restrictToConfiguredPeers: Bool
    public var startFromHeight: UInt32
    /// Devnet name (e.g. `"333"` for `devnet-333`). Required when
    /// `network == .devnet`, must be `nil` on every other network.
    /// The FFI rebuilds the user agent to embed
    /// `devnet.devnet-<name>` so Dash Core devnet peers accept the
    /// handshake.
    public var devnetName: String?
    /// LLMQ_DEVNET quorum size override (matches Dash Core's
    /// `-llmqdevnetparams=<size>:<threshold>`). `0` means "no
    /// override". Must be paired with `llmqDevnetThreshold` and only
    /// valid on devnet.
    public var llmqDevnetSize: UInt32
    /// LLMQ_DEVNET signing threshold override. See
    /// `llmqDevnetSize`.
    public var llmqDevnetThreshold: UInt32

    public init(
        dataDir: String,
        network: Network,
        userAgent: String? = nil,
        peers: [String] = [],
        restrictToConfiguredPeers: Bool = false,
        startFromHeight: UInt32 = 0,
        devnetName: String? = nil,
        llmqDevnetSize: UInt32 = 0,
        llmqDevnetThreshold: UInt32 = 0
    ) {
        self.dataDir = dataDir
        self.network = network
        self.userAgent = userAgent
        self.peers = peers
        self.restrictToConfiguredPeers = restrictToConfiguredPeers
        self.startFromHeight = startFromHeight
        self.devnetName = devnetName
        self.llmqDevnetSize = llmqDevnetSize
        self.llmqDevnetThreshold = llmqDevnetThreshold
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
        try platform_wallet_manager_sync_progress(handle, &ffi).check()
        return PlatformSpvSyncProgress(ffi)
    }

    /// Poll the peers the SPV client is currently connected to, each
    /// classified against the masternode list. Returns an empty array
    /// when the client isn't running or no peers are connected.
    ///
    /// Distinct from the `@Published var spvPeers` mirror on the
    /// manager: this is a one-shot FFI query, the published property
    /// is the cached value the 1 Hz progress poll feeds.
    public func connectedSpvPeers() throws -> [PlatformSpvPeerInfo] {
        var entries: UnsafePointer<FFISpvPeerInfo>? = nil
        var count: UInt = 0
        try platform_wallet_manager_spv_connected_peers(handle, &entries, &count).check()
        guard let entries, count > 0 else { return [] }
        defer {
            platform_wallet_manager_spv_connected_peers_free(
                UnsafeMutablePointer(mutating: entries), count)
        }
        return UnsafeBufferPointer(start: entries, count: Int(count)).map { entry in
            PlatformSpvPeerInfo(
                address: entry.address.map { String(cString: $0) } ?? "",
                nodeType: PlatformSpvPeerNodeType(rawValue: entry.node_type) ?? .unknown
            )
        }
    }

    /// Whether the SPV client is currently running.
    public func isSpvRunning() throws -> Bool {
        var running: Bool = false
        try platform_wallet_manager_spv_is_running(handle, &running).check()
        return running
    }

    /// Read the unix-seconds block time of the SPV header storage's
    /// current tip. Returns `nil` when no tip is available — i.e.
    /// the SPV client isn't running, no headers have been stored
    /// yet, or the tip header isn't readable.
    ///
    /// Distinct from the `@Published var spvTipBlockTime` mirror on
    /// the manager: this is a one-shot FFI query, the published
    /// property is the cached value the 1 Hz progress poll feeds.
    ///
    /// Useful as a "is core producing blocks?" indicator: a stale
    /// stamp across multiple polls means the chain has stalled
    /// even though the local SPV client is healthy.
    public func currentSpvTipBlockTime() throws -> Date? {
        var unixSeconds: UInt64 = 0
        try platform_wallet_manager_spv_tip_unix_seconds(handle, &unixSeconds).check()
        guard unixSeconds > 0 else { return nil }
        return Date(timeIntervalSince1970: TimeInterval(unixSeconds))
    }

    /// Start the SPV client in the background.
    ///
    /// Spawns the sync loop on the shared tokio runtime and returns
    /// immediately. Use [`stopSpv`] to cancel.
    public func startSpv(config: PlatformSpvStartConfig) throws {
        // Peer array: allocate contiguous C strings.
        let peerCStrings: [UnsafeMutablePointer<CChar>?] = config.peers.map { strdup($0) }
        defer { peerCStrings.forEach { if let p = $0 { free(p) } } }

        try config.dataDir.withCString { dataDirPtr in
            let userAgentPtr = config.userAgent.flatMap { strdup($0) }
            defer { if let p = userAgentPtr { free(p) } }

            let devnetNamePtr = config.devnetName.flatMap { strdup($0) }
            defer { if let p = devnetNamePtr { free(p) } }

            try peerCStrings.withUnsafeBufferPointer { peersBuf in
                let peersPtr: UnsafePointer<UnsafePointer<CChar>?>? = peersBuf.baseAddress.map {
                    UnsafeRawPointer($0).assumingMemoryBound(to: UnsafePointer<CChar>?.self)
                }
                try platform_wallet_manager_spv_start(
                    handle,
                    dataDirPtr,
                    config.network.ffiValue,
                    userAgentPtr,
                    peersPtr,
                    UInt(peerCStrings.count),
                    config.restrictToConfiguredPeers,
                    config.startFromHeight,
                    devnetNamePtr,
                    config.llmqDevnetSize,
                    config.llmqDevnetThreshold
                ).check()
            }
        }
    }

    /// Stop the SPV client (idempotent).
    public func stopSpv() throws {
        try platform_wallet_manager_spv_stop(handle).check()
    }

    /// Clear all persisted SPV storage (headers, filters, state).
    public func clearSpvStorage() throws {
        try platform_wallet_manager_spv_clear_storage(handle).check()
    }

    /// Arm an organic compact-filter rescan for one wallet by rewinding
    /// its SPV filter-scan checkpoint (`synced_height`) to `fromHeight`.
    ///
    /// This does not scan directly. It lowers the wallet's synced height
    /// on the shared wallet state the running filter sync reads; on that
    /// sync's next tick the filter manager detects the wallet is behind,
    /// resets its committed height to the rewound value, and re-downloads
    /// and re-matches compact filters from there — picking up scripts
    /// (e.g. addresses added after those blocks were first scanned) that
    /// were not in the watch set the first time.
    ///
    /// Requires SPV running for an immediate effect; otherwise the
    /// rewound checkpoint takes effect when SPV next starts and its
    /// filter loop first ticks. A `fromHeight` at or above the wallet's
    /// current checkpoint is stored but arms no rescan (the sync only
    /// rescans wallets that are strictly behind). Throws `.notFound`
    /// (via `.check()`) when no loaded wallet matches `walletId`.
    ///
    /// - Parameters:
    ///   - walletId: the 32-byte wallet identifier.
    ///   - fromHeight: the core block height to rewind the filter scan to.
    public func spvRescanFilters(walletId: Data, fromHeight: UInt32) throws {
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be exactly 32 bytes"
            )
        }
        let previousHeight = coreWalletState(for: walletId)?.syncedHeight
        do {
            try walletId.withUnsafeBytes { widRaw in
                guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try platform_wallet_manager_spv_rescan_filters(handle, widPtr, fromHeight).check()
            }
            let diagnosticResult = coreRescanDiagnosticResult(
                previousSyncedHeight: previousHeight,
                requestedStartHeight: fromHeight
            )
            var fields: [String: SDKLogValue] = [
                "from_height": .unsignedInteger(UInt64(fromHeight)),
                "result": .publicText(diagnosticResult.rawValue),
                "wallet_reference": .reference(walletId),
            ]
            if let previousHeight {
                fields["previous_synced_height"] = .unsignedInteger(UInt64(previousHeight))
            }
            SDKLogger.event(
                "core_rescan_armed",
                category: .persistence,
                fields: fields
            )
        } catch {
            var fields: [String: SDKLogValue] = [
                "from_height": .unsignedInteger(UInt64(fromHeight)),
                "result": .publicText("failed"),
                "wallet_reference": .reference(walletId),
            ]
            if let previousHeight {
                fields["previous_synced_height"] = .unsignedInteger(UInt64(previousHeight))
            }
            SDKLogger.event(
                "core_rescan_armed",
                category: .persistence,
                severity: .error,
                fields: fields
            )
            throw error
        }
    }
}
