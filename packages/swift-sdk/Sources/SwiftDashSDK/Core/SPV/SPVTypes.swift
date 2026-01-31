/**
 This file contains wrappers and helpers to interact with the SPV FFI 
 structs used in this SDK. Freeing FFI structs is handled always by the caller
*/

import Foundation
import DashSDKFFI

public struct SPVSyncProgress: Sendable {
    public let stage: SPVSyncStage
    public let currentHeight: UInt32
    public let masternodeHeight: UInt32
    public let targetHeight: UInt32
    public let filterHeaderHeight: UInt32
    public let filterHeight: UInt32
    public let syncStartedAt: TimeInterval
    public let rate: Double // blocks per second
    public let estimatedTimeRemaining: TimeInterval?
    public let peerCount: UInt32

    public static func `default`() -> SPVSyncProgress {
        SPVSyncProgress(
            stage: .idle,
            currentHeight: 0,
            masternodeHeight: 0,
            targetHeight: 0,
            filterHeaderHeight: 0,
            filterHeight: 0,
            syncStartedAt: 0,
            rate: 0,
            estimatedTimeRemaining: nil,
            peerCount: 0
        )
    }

    internal static func from(_ detailed: FFIDetailedSyncProgress) -> SPVSyncProgress {
        let overview = detailed.overview
    
        return SPVSyncProgress(
            stage: SPVSyncStage(ffiStage: detailed.stage),
            currentHeight: overview.header_height,
            masternodeHeight: overview.masternode_height,
            targetHeight: detailed.total_height,
            filterHeaderHeight: overview.filter_header_height,
            filterHeight: overview.last_synced_filter_height,
            syncStartedAt: TimeInterval(detailed.sync_start_timestamp),
            rate: detailed.headers_per_second,
            estimatedTimeRemaining: detailed.estimated_seconds_remaining > 0
                ? TimeInterval(detailed.estimated_seconds_remaining)
                : nil,
            peerCount: overview.peer_count
        )
    }
    
    internal static func from(_ overview: FFISyncProgress) -> SPVSyncProgress {
        SPVSyncProgress(
            stage: .idle,
            currentHeight: overview.header_height,
            masternodeHeight: overview.masternode_height,
            targetHeight: 0,
            filterHeaderHeight: overview.filter_header_height,
            filterHeight: overview.last_synced_filter_height,
            syncStartedAt: 0, // TODO: This field exists in the Rust struct but the FFI does not provide it yet
            rate: 0,
            estimatedTimeRemaining: nil,
            peerCount: overview.peer_count
        )
    }
}

public enum SPVSyncStage: String, Sendable {
    case idle = "Idle"
    case connecting = "Connecting"
    case queryingHeight = "Querying Height"
    case downloading = "Downloading"
    case validating = "Validating"
    case storing = "Storing"
    case downloadingFilterHeaders = "Downloading Filter Headers"
    case downloadingFilters = "Downloading Filters"
    case downloadingBlocks = "Downloading Blocks"
    case complete = "Complete"
    case failed = "Failed"
    case unknown = "Unknown"
}

extension SPVSyncStage {
    init(ffiStage: FFISyncStage) {
        switch ffiStage.rawValue {
            case 0: self = .connecting
            case 1: self = .queryingHeight
            case 2: self = .downloading
            case 3: self = .validating
            case 4: self = .storing
            case 5: self = .downloadingFilterHeaders
            case 6: self = .downloadingFilters
            case 7: self = .downloadingBlocks
            case 8: self = .complete
            case 9: self = .failed
            default: self = .unknown
        }
    }
}