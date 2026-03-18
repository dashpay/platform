// AddressSyncTypes.swift
// SwiftDashSDK
//
// Swift types for address BLAST sync operations.
// These match the Rust #[repr(C)] structs in rs-sdk-ffi/src/address_sync/types.rs.

import Foundation
import DashSDKFFI

// MARK: - FFI Type Aliases
// These types are imported from the DashSDKFFI C header.

typealias FFIAddressSyncConfig = DashSDKAddressSyncConfig
typealias FFIAddressSyncResult = DashSDKAddressSyncResult
typealias FFIFoundAddress = DashSDKFoundAddress
typealias FFIAbsentAddress = DashSDKAbsentAddress
typealias FFIAddressSyncMetrics = DashSDKAddressSyncMetrics

// MARK: - High-Level Swift Models

/// Configuration for address BLAST sync.
public struct AddressSyncConfig: Sendable {
    /// Minimum privacy count -- subtrees smaller than this are expanded
    /// to include ancestor subtrees for better privacy.
    /// Higher values provide better privacy but increase returned elements.
    /// Default: 32
    public var minPrivacyCount: UInt64

    /// Maximum concurrent branch queries.
    /// Higher values speed up sync but increase memory and network usage.
    /// Default: 10
    public var maxConcurrentRequests: UInt32

    /// Maximum number of iterations (safety limit).
    /// Prevents infinite loops in case of unexpected behavior.
    /// Default: 50
    public var maxIterations: UInt32

    /// Maximum age in seconds before a full tree rescan is forced.
    /// When `lastSyncTimestamp` is provided, elapsed time is compared
    /// against this threshold. Set to 0 to always do a full tree scan.
    /// Default: 604800 (7 days)
    public var fullRescanAfterTimeSeconds: UInt64

    /// Create a config with default values.
    public init(
        minPrivacyCount: UInt64 = 32,
        maxConcurrentRequests: UInt32 = 10,
        maxIterations: UInt32 = 50,
        fullRescanAfterTimeSeconds: UInt64 = 7 * 24 * 60 * 60
    ) {
        self.minPrivacyCount = minPrivacyCount
        self.maxConcurrentRequests = maxConcurrentRequests
        self.maxIterations = maxIterations
        self.fullRescanAfterTimeSeconds = fullRescanAfterTimeSeconds
    }

    /// Convert to FFI struct.
    func toFFI() -> FFIAddressSyncConfig {
        return FFIAddressSyncConfig(
            min_privacy_count: minPrivacyCount,
            max_concurrent_requests: maxConcurrentRequests,
            max_iterations: maxIterations,
            full_rescan_after_time_s: fullRescanAfterTimeSeconds
        )
    }
}

/// An address found in the tree with its balance and nonce.
public struct FoundAddress: Sendable {
    /// The derivation index for this address.
    public let index: UInt32
    /// Address key bytes.
    public let key: Data
    /// Nonce associated with this address.
    public let nonce: UInt32
    /// Balance in credits at this address.
    public let balance: UInt64

    /// Convert key to hex string.
    public var keyHex: String {
        return key.map { String(format: "%02x", $0) }.joined()
    }
}

/// An address proven absent from the tree.
public struct AbsentAddress: Sendable {
    /// The derivation index for this address.
    public let index: UInt32
    /// Address key bytes.
    public let key: Data

    /// Convert key to hex string.
    public var keyHex: String {
        return key.map { String(format: "%02x", $0) }.joined()
    }
}

/// Metrics about the address synchronization process.
public struct AddressSyncMetrics: Sendable {
    /// Number of trunk queries (0 for incremental-only, 1 for full scan).
    public let trunkQueries: UInt32
    /// Number of branch queries.
    public let branchQueries: UInt32
    /// Total elements seen across all proofs (indicates anonymity set size).
    public let totalElementsSeen: UInt32
    /// Total proof bytes received.
    public let totalProofBytes: UInt32
    /// Number of iterations (0 = trunk only, 1+ = trunk plus branch rounds).
    public let iterations: UInt32
    /// Number of compacted incremental queries (historical aggregated changes).
    public let compactedQueries: UInt32
    /// Number of recent incremental queries (per-block changes).
    public let recentQueries: UInt32
    /// Total block entries returned by recent queries (all addresses, not just ours).
    public let recentEntriesReturned: UInt32
    /// Total block entries returned by compacted queries.
    public let compactedEntriesReturned: UInt32

    init(ffi: FFIAddressSyncMetrics) {
        self.trunkQueries = ffi.trunk_queries
        self.branchQueries = ffi.branch_queries
        self.totalElementsSeen = ffi.total_elements_seen
        self.totalProofBytes = ffi.total_proof_bytes
        self.iterations = ffi.iterations
        self.compactedQueries = ffi.compacted_queries
        self.recentQueries = ffi.recent_queries
        self.recentEntriesReturned = ffi.recent_entries_returned
        self.compactedEntriesReturned = ffi.compacted_entries_returned
    }
}

/// Result of an address BLAST sync operation.
public struct AddressSyncResult: Sendable {
    /// Addresses found in the tree with balances.
    public let found: [FoundAddress]
    /// Addresses proven absent from the tree.
    public let absent: [AbsentAddress]
    /// Highest found derivation index, or nil if no addresses were found.
    public let highestFoundIndex: UInt32?
    /// Checkpoint height from the trunk/branch tree scan (0 if incremental-only).
    public let checkpointHeight: UInt64
    /// New sync height to persist for the next incremental sync call.
    public let newSyncHeight: UInt64
    /// New sync timestamp to persist for the next sync call.
    public let newSyncTimestamp: UInt64
    /// Highest block from the most recent per-block balance changes.
    /// Pass this back as `lastKnownRecentBlock` on the next sync call
    /// to enable efficient compaction detection (0 if no recent blocks).
    public let lastKnownRecentBlock: UInt64
    /// Sync metrics.
    public let metrics: AddressSyncMetrics

    /// Raw GroveDB proof bytes from the most recent query (for debugging).
    public let recentProof: Data

    /// Total balance across all found addresses.
    public var totalBalance: UInt64 {
        return found.reduce(0) { $0 + $1.balance }
    }

    /// Number of found addresses with non-zero balance.
    public var nonZeroBalanceCount: Int {
        return found.filter { $0.balance > 0 }.count
    }

    /// Initialize from FFI result. Copies data out before the FFI pointer is freed.
    init(ffi: FFIAddressSyncResult) {
        // Copy found addresses
        var foundArr: [FoundAddress] = []
        if let ptr = ffi.found, ffi.found_count > 0 {
            for i in 0..<Int(ffi.found_count) {
                let ffiAddr = ptr[i]
                let keyData: Data
                if let keyPtr = ffiAddr.key, ffiAddr.key_len > 0 {
                    keyData = Data(bytes: keyPtr, count: Int(ffiAddr.key_len))
                } else {
                    keyData = Data()
                }
                foundArr.append(FoundAddress(
                    index: ffiAddr.index,
                    key: keyData,
                    nonce: ffiAddr.nonce,
                    balance: ffiAddr.balance
                ))
            }
        }
        self.found = foundArr

        // Copy absent addresses
        var absentArr: [AbsentAddress] = []
        if let ptr = ffi.absent, ffi.absent_count > 0 {
            for i in 0..<Int(ffi.absent_count) {
                let ffiAddr = ptr[i]
                let keyData: Data
                if let keyPtr = ffiAddr.key, ffiAddr.key_len > 0 {
                    keyData = Data(bytes: keyPtr, count: Int(ffiAddr.key_len))
                } else {
                    keyData = Data()
                }
                absentArr.append(AbsentAddress(
                    index: ffiAddr.index,
                    key: keyData
                ))
            }
        }
        self.absent = absentArr

        // Highest found index (optional)
        if ffi.has_highest_found_index {
            self.highestFoundIndex = ffi.highest_found_index
        } else {
            self.highestFoundIndex = nil
        }

        self.checkpointHeight = ffi.checkpoint_height
        self.newSyncHeight = ffi.new_sync_height
        self.newSyncTimestamp = ffi.new_sync_timestamp
        self.lastKnownRecentBlock = ffi.last_known_recent_block
        self.metrics = AddressSyncMetrics(ffi: ffi.metrics)

        // Copy recent proof bytes
        if let ptr = ffi.recent_proof, ffi.recent_proof_len > 0 {
            self.recentProof = Data(bytes: ptr, count: Int(ffi.recent_proof_len))
        } else {
            self.recentProof = Data()
        }
    }
}
