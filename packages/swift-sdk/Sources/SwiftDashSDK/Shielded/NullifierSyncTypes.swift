// NullifierSyncTypes.swift
// SwiftDashSDK
//
// Swift types for nullifier BLAST sync operations.
// These match the Rust #[repr(C)] structs in rs-sdk-ffi/src/nullifier_sync/types.rs.

import Foundation

// MARK: - FFI Type Aliases
// These types are imported from the DashSDKFFI C header.

typealias FFINullifierSyncConfig = DashSDKNullifierSyncConfig
typealias FFINullifierSyncMetrics = DashSDKNullifierSyncMetrics
typealias FFINullifierSyncResult = DashSDKNullifierSyncResult

// MARK: - High-Level Swift Models

/// Configuration for nullifier BLAST sync.
public struct NullifierSyncConfig: Sendable {
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
    /// - Throws: If `poolType` is invalid, or if `poolType == 2` without a valid `poolIdentifier`.
    func toFFI() throws -> FFINullifierSyncConfig {
        // Validate poolType: 0=credit, 1=main token, 2=individual token
        guard poolType <= 2 else {
            throw SDKError.invalidParameter("poolType must be 0, 1, or 2, got \(poolType)")
        }

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
            // poolType 2 (individual token) requires a poolIdentifier
            if poolType == 2 {
                throw SDKError.invalidParameter(
                    "poolType 2 (individual token) requires a poolIdentifier"
                )
            }
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
public struct NullifierSyncMetrics: Sendable {
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
public struct NullifierSyncResult: Sendable {
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
