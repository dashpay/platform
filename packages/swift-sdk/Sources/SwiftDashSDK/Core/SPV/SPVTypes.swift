/* 
 This file contains wrappers and helpers to interact with the SPV FFI
 structs used in this SDK. Freeing FFI structs is handled always by the caller
 */

import DashSDKFFI
import Foundation

// SyncProgress.swift
// Swift wrappers for Rust FFI sync progress
// All types are Sendable & public

import Foundation

// MARK: - SPVSyncState

public enum SPVSyncState: UInt32, Sendable {
    case initializing = 0
    case waitingForConnections = 1
    case waitForEvents = 2
    case syncing = 3
    case synced = 4
    case error = 5

    // Custom states, not in FFI
    case idle = 998
    case unknown = 999

    public func isSyncing() -> Bool {
        switch self {
        case .waitingForConnections, .waitForEvents, .syncing:
            return true
        case .initializing, .synced, .unknown, .idle, .error:
            return false
        }
    }

    public func isRunning() -> Bool {
        switch self {
        case .waitingForConnections, .waitForEvents, .syncing, .synced:
            return true
        case .initializing, .unknown, .idle, .error:
            return false
        }
    }

    public func isComplete() -> Bool {
        return self == .synced
    }
}

// MARK: - Block Headers Progress

public struct SPVBlockHeadersProgress: Sendable {
    public let state: SPVSyncState
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let processed: UInt32
    public let buffered: UInt32
    public let percentage: Double
    public let lastActivity: UInt64

    public init(_ ffi: FFIBlockHeadersProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        currentHeight = ffi.tip_height
        targetHeight = ffi.target_height
        processed = ffi.processed
        buffered = ffi.buffered
        percentage = ffi.percentage
        lastActivity = ffi.last_activity
    }
}

// MARK: - Filter Headers Progress

public struct SPVFilterHeadersProgress: Sendable {
    public let state: SPVSyncState
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let blockHeaderTipHeight: UInt32
    public let processed: UInt32
    public let percentage: Double
    public let lastActivity: UInt64

    public init(_ ffi: FFIFilterHeadersProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        currentHeight = ffi.current_height
        targetHeight = ffi.target_height
        blockHeaderTipHeight = ffi.block_header_tip_height
        processed = ffi.processed
        percentage = ffi.percentage
        lastActivity = ffi.last_activity
    }
}

// MARK: - Filters Progress

public struct SPVFiltersProgress: Sendable {
    public let state: SPVSyncState
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let filterHeaderTipHeight: UInt32
    public let downloaded: UInt32
    public let processed: UInt32
    public let matched: UInt32
    public let percentage: Double
    public let lastActivity: UInt64

    public init(_ ffi: FFIFiltersProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        currentHeight = ffi.current_height
        targetHeight = ffi.target_height
        filterHeaderTipHeight = ffi.filter_header_tip_height
        downloaded = ffi.downloaded
        processed = ffi.processed
        matched = ffi.matched
        percentage = ffi.percentage
        lastActivity = ffi.last_activity
    }
}

// MARK: - Blocks Progress

public struct SPVBlocksProgress: Sendable {
    public let state: SPVSyncState
    public let lastProcessed: UInt32
    public let requested: UInt32
    public let fromStorage: UInt32
    public let downloaded: UInt32
    public let processed: UInt32
    public let relevant: UInt32
    public let transactions: UInt32
    public let lastActivity: UInt64

    public init(_ ffi: FFIBlocksProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        lastProcessed = ffi.last_processed
        requested = ffi.requested
        fromStorage = ffi.from_storage
        downloaded = ffi.downloaded
        processed = ffi.processed
        relevant = ffi.relevant
        transactions = ffi.transactions
        lastActivity = ffi.last_activity
    }
}

// MARK: - Masternodes Progress

public struct SPVMasternodesProgress: Sendable {
    public let state: SPVSyncState
    public let currentHeight: UInt32
    public let targetHeight: UInt32
    public let blockHeaderTipHeight: UInt32
    public let diffsProcessed: UInt32
    public let lastActivity: UInt64

    public init(_ ffi: FFIMasternodesProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        currentHeight = ffi.current_height
        targetHeight = ffi.target_height
        blockHeaderTipHeight = ffi.block_header_tip_height
        diffsProcessed = ffi.diffs_processed
        lastActivity = ffi.last_activity
    }
}

// MARK: - ChainLock Progress

public struct SPVChainLockProgress: Sendable {
    public let state: SPVSyncState
    public let bestValidatedHeight: UInt32
    public let valid: UInt32
    public let invalid: UInt32
    public let lastActivity: UInt64

    public init(_ ffi: FFIChainLockProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        bestValidatedHeight = ffi.best_validated_height
        valid = ffi.valid
        invalid = ffi.invalid
        lastActivity = ffi.last_activity
    }
}

// MARK: - InstantSend Progress

public struct SPVInstantSendProgress: Sendable {
    public let state: SPVSyncState
    public let pending: UInt32
    public let valid: UInt32
    public let invalid: UInt32
    public let lastActivity: UInt64

    public init(_ ffi: FFIInstantSendProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        pending = ffi.pending
        valid = ffi.valid
        invalid = ffi.invalid
        lastActivity = ffi.last_activity
    }
}

// MARK: - Aggregate Sync Progress

public struct SPVSyncProgress: Sendable {
    public let state: SPVSyncState
    public let percentage: Double

    public let headers: SPVBlockHeadersProgress?
    public let filterHeaders: SPVFilterHeadersProgress?
    public let filters: SPVFiltersProgress?
    public let blocks: SPVBlocksProgress?
    public let masternodes: SPVMasternodesProgress?
    public let chainLocks: SPVChainLockProgress?
    public let instantSend: SPVInstantSendProgress?

    public static func `default`() -> SPVSyncProgress {
        SPVSyncProgress(
            state: .idle,
            percentage: 0.0,
            headers: nil,
            filterHeaders: nil,
            filters: nil,
            blocks: nil,
            masternodes: nil,
            chainLocks: nil,
            instantSend: nil
        )
    }

    private init(state: SPVSyncState, percentage: Double, headers: SPVBlockHeadersProgress?, filterHeaders: SPVFilterHeadersProgress?,
                 filters: SPVFiltersProgress?, blocks: SPVBlocksProgress?, masternodes: SPVMasternodesProgress?, chainLocks: SPVChainLockProgress?,
                 instantSend: SPVInstantSendProgress?)
    {
        self.state = state
        self.percentage = percentage
        self.headers = headers
        self.filterHeaders = filterHeaders
        self.filters = filters
        self.blocks = blocks
        self.masternodes = masternodes
        self.chainLocks = chainLocks
        self.instantSend = instantSend
    }

    public init(_ ffi: FFISyncProgress) {
        state = SPVSyncState(rawValue: ffi.state.rawValue) ?? .unknown
        percentage = ffi.percentage

        if let headersPtr = ffi.headers {
            headers = SPVBlockHeadersProgress(headersPtr.pointee)
        } else {
            headers = nil
        }

        if let filterHeadersPtr = ffi.filter_headers {
            filterHeaders = SPVFilterHeadersProgress(filterHeadersPtr.pointee)
        } else {
            filterHeaders = nil
        }

        if let filtersPtr = ffi.filters {
            filters = SPVFiltersProgress(filtersPtr.pointee)
        } else {
            filters = nil
        }

        if let blocksPtr = ffi.blocks {
            blocks = SPVBlocksProgress(blocksPtr.pointee)
        } else {
            blocks = nil
        }

        if let masternodesPtr = ffi.masternodes {
            masternodes = SPVMasternodesProgress(masternodesPtr.pointee)
        } else {
            masternodes = nil
        }

        if let chainLocksPtr = ffi.chainlocks {
            chainLocks = SPVChainLockProgress(chainLocksPtr.pointee)
        } else {
            chainLocks = nil
        }

        if let instantSendPtr = ffi.instantsend {
            instantSend = SPVInstantSendProgress(instantSendPtr.pointee)
        } else {
            instantSend = nil
        }
    }
}
