// NullifierSyncTypes.swift
// SwiftDashSDK
//
// Swift types for nullifier BLAST sync operations.
// These match the Rust #[repr(C)] structs in rs-sdk-ffi/src/nullifier_sync/types.rs.

import Foundation

// MARK: - FFI C-Compatible Structs

/// Matches `DashSDKNullifierSyncConfig` in Rust.
/// Configuration for nullifier BLAST sync. Pass nil to use defaults.
struct FFINullifierSyncConfig {
    /// Minimum privacy count -- subtrees smaller than this are expanded.
    /// Default: 32
    var min_privacy_count: UInt64

    /// Maximum concurrent branch queries.
    /// Default: 10
    var max_concurrent_requests: UInt32

    /// Maximum number of iterations (safety limit).
    /// Default: 50
    var max_iterations: UInt32

    /// Shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    /// Default: 0
    var pool_type: UInt32

    /// Optional 32-byte pool identifier for individual token pools.
    /// Only used when `has_pool_identifier` is true.
    var pool_identifier: Bytes32Tuple

    /// Whether `pool_identifier` is valid.
    var has_pool_identifier: Bool

    /// Maximum age in seconds before a full tree rescan is forced.
    /// Default: 604800 (7 days)
    var full_rescan_after_time_s: UInt64
}

/// Matches `DashSDKNullifierSyncMetrics` in Rust.
/// Metrics about the nullifier sync process.
struct FFINullifierSyncMetrics {
    var trunk_queries: UInt32
    var branch_queries: UInt32
    var total_elements_seen: UInt32
    var total_proof_bytes: UInt32
    var branch_query_failures: UInt32
    var iterations: UInt32
    var compacted_queries: UInt32
    var recent_queries: UInt32
}

/// Matches `DashSDKNullifierSyncResult` in Rust.
/// Result of nullifier BLAST sync. Free with `dash_sdk_nullifier_sync_result_free`.
struct FFINullifierSyncResult {
    /// Contiguous array of found (spent) nullifiers, each 32 bytes.
    var found: UnsafeMutablePointer<UInt8>?
    /// Number of found nullifiers.
    var found_count: UInt32
    /// Contiguous array of absent (unspent) nullifiers, each 32 bytes.
    var absent: UnsafeMutablePointer<UInt8>?
    /// Number of absent nullifiers.
    var absent_count: UInt32
    /// Block height of the tree snapshot.
    var checkpoint_height: UInt64
    /// Highest block height seen -- persist for next sync call.
    var new_sync_height: UInt64
    /// Block time at the latest response -- persist for next sync call.
    var new_sync_timestamp: UInt64
    /// Sync metrics.
    var metrics: FFINullifierSyncMetrics
}

// MARK: - High-Level Swift Models

/// Configuration for nullifier BLAST sync.
public struct NullifierSyncConfig {
    /// Minimum privacy count -- subtrees smaller than this are expanded.
    public var minPrivacyCount: UInt64
    /// Maximum concurrent branch queries.
    public var maxConcurrentRequests: UInt32
    /// Maximum number of iterations (safety limit).
    public var maxIterations: UInt32
    /// Shielded pool type (0 = credit, 1 = main token, 2 = individual token).
    public var poolType: UInt32
    /// Optional 32-byte pool identifier for individual token pools.
    public var poolIdentifier: Data?
    /// Maximum age in seconds before a full tree rescan is forced.
    public var fullRescanAfterTimeSeconds: UInt64

    /// Create a config with default values.
    public init(
        minPrivacyCount: UInt64 = 32,
        maxConcurrentRequests: UInt32 = 10,
        maxIterations: UInt32 = 50,
        poolType: UInt32 = 0,
        poolIdentifier: Data? = nil,
        fullRescanAfterTimeSeconds: UInt64 = 7 * 24 * 60 * 60
    ) {
        self.minPrivacyCount = minPrivacyCount
        self.maxConcurrentRequests = maxConcurrentRequests
        self.maxIterations = maxIterations
        self.poolType = poolType
        self.poolIdentifier = poolIdentifier
        self.fullRescanAfterTimeSeconds = fullRescanAfterTimeSeconds
    }

    /// Convert to FFI struct.
    /// - Throws: If `poolIdentifier` is set but not exactly 32 bytes.
    func toFFI() throws -> FFINullifierSyncConfig {
        let hasPoolId: Bool
        let poolIdTuple: Bytes32Tuple
        if let pid = poolIdentifier {
            guard pid.count == 32 else {
                throw SDKError.invalidParameter(
                    "poolIdentifier must be exactly 32 bytes, got \(pid.count)"
                )
            }
            hasPoolId = true
            poolIdTuple = dataToBytes32(pid)
        } else {
            hasPoolId = false
            poolIdTuple = dataToBytes32(Data(count: 32))
        }

        return FFINullifierSyncConfig(
            min_privacy_count: minPrivacyCount,
            max_concurrent_requests: maxConcurrentRequests,
            max_iterations: maxIterations,
            pool_type: poolType,
            pool_identifier: poolIdTuple,
            has_pool_identifier: hasPoolId,
            full_rescan_after_time_s: fullRescanAfterTimeSeconds
        )
    }
}

/// Metrics about a nullifier sync operation.
public struct NullifierSyncMetrics {
    public let trunkQueries: UInt32
    public let branchQueries: UInt32
    public let totalElementsSeen: UInt32
    public let totalProofBytes: UInt32
    public let branchQueryFailures: UInt32
    public let iterations: UInt32
    public let compactedQueries: UInt32
    public let recentQueries: UInt32

    init(ffi: FFINullifierSyncMetrics) {
        self.trunkQueries = ffi.trunk_queries
        self.branchQueries = ffi.branch_queries
        self.totalElementsSeen = ffi.total_elements_seen
        self.totalProofBytes = ffi.total_proof_bytes
        self.branchQueryFailures = ffi.branch_query_failures
        self.iterations = ffi.iterations
        self.compactedQueries = ffi.compacted_queries
        self.recentQueries = ffi.recent_queries
    }
}

/// Result of a nullifier BLAST sync operation.
public struct NullifierSyncResult {
    /// Nullifiers that were found (spent) in the shielded pool.
    public let found: [Data]
    /// Nullifiers that were absent (unspent) in the shielded pool.
    public let absent: [Data]
    /// Block height of the tree snapshot.
    public let checkpointHeight: UInt64
    /// Highest block height seen -- persist for next sync call.
    public let newSyncHeight: UInt64
    /// Block time at the latest response -- persist for next sync call.
    public let newSyncTimestamp: UInt64
    /// Sync metrics.
    public let metrics: NullifierSyncMetrics

    /// Initialize from FFI result. Copies data out before the FFI pointer is freed.
    init(ffi: FFINullifierSyncResult) {
        var foundArr: [Data] = []
        if let ptr = ffi.found, ffi.found_count > 0 {
            for i in 0..<Int(ffi.found_count) {
                let offset = i * 32
                foundArr.append(Data(bytes: ptr.advanced(by: offset), count: 32))
            }
        }
        self.found = foundArr

        var absentArr: [Data] = []
        if let ptr = ffi.absent, ffi.absent_count > 0 {
            for i in 0..<Int(ffi.absent_count) {
                let offset = i * 32
                absentArr.append(Data(bytes: ptr.advanced(by: offset), count: 32))
            }
        }
        self.absent = absentArr

        self.checkpointHeight = ffi.checkpoint_height
        self.newSyncHeight = ffi.new_sync_height
        self.newSyncTimestamp = ffi.new_sync_timestamp
        self.metrics = NullifierSyncMetrics(ffi: ffi.metrics)
    }
}
