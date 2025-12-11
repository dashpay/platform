//
//  FilterMatch.swift
//  SwiftDashSDK
//
//  Models for compact filters from SPV client
//

import Foundation
import DashSDKFFI

/// A single compact filter with its height and data
public struct CompactFilter: Identifiable {
    public var id: UInt32 { height }

    /// Block height for this filter
    public let height: UInt32

    /// Filter data bytes
    public let data: Data

    public init(height: UInt32, data: Data) {
        self.height = height
        self.data = data
    }

    // NOTE: FFI initializer commented out - FFICompactFilter not available in current FFI
    // /// Initialize from FFI struct
    // public init(from ffiFilter: FFICompactFilter) {
    //     self.height = ffiFilter.height
    //
    //     if let dataPtr = ffiFilter.data, ffiFilter.data_len > 0 {
    //         self.data = Data(bytes: dataPtr, count: Int(ffiFilter.data_len))
    //     } else {
    //         self.data = Data()
    //     }
    // }

    /// Get filter size in bytes
    public var sizeInBytes: Int {
        data.count
    }
}

/// Collection of compact filters
public struct CompactFilters {
    public let filters: [CompactFilter]

    public init(filters: [CompactFilter]) {
        self.filters = filters
    }

    // NOTE: FFI initializer commented out - FFICompactFilters not available in current FFI
    // /// Initialize from FFI struct
    // public init(from ffiFilters: FFICompactFilters) {
    //     var filters: [CompactFilter] = []
    //
    //     if let filtersPtr = ffiFilters.filters {
    //         for i in 0..<ffiFilters.count {
    //             let ffiFilter = filtersPtr.advanced(by: Int(i)).pointee
    //             filters.append(CompactFilter(from: ffiFilter))
    //         }
    //     }
    //
    //     self.filters = filters
    // }

    /// Check if empty
    public var isEmpty: Bool {
        filters.isEmpty
    }

    /// Get total number of filters
    public var count: Int {
        filters.count
    }
}

/// A single filter match entry with height and wallet IDs
public struct FilterMatchEntry: Identifiable {
    public var id: UInt32 { height }

    /// Block height where filter matched
    public let height: UInt32

    /// Array of wallet IDs (32 bytes each) that matched at this height
    public let walletIds: [Data]

    public init(height: UInt32, walletIds: [Data]) {
        self.height = height
        self.walletIds = walletIds
    }

    // NOTE: FFI initializer commented out - FFIFilterMatchEntry not available in current FFI
    // /// Initialize from FFI struct
    // public init(from ffiEntry: FFIFilterMatchEntry) {
    //     self.height = ffiEntry.height
    //
    //     var walletIds: [Data] = []
    //     if let ptr = ffiEntry.wallet_ids {
    //         for i in 0..<ffiEntry.wallet_ids_count {
    //             let walletIdPtr = ptr.advanced(by: Int(i))
    //             // Convert tuple to Data by using withUnsafeBytes
    //             let data = withUnsafeBytes(of: walletIdPtr.pointee) { bytes in
    //                 Data(bytes)
    //             }
    //             walletIds.append(data)
    //         }
    //     }
    //     self.walletIds = walletIds
    // }
}

/// Collection of filter match entries
public struct FilterMatches {
    public let entries: [FilterMatchEntry]

    public init(entries: [FilterMatchEntry]) {
        self.entries = entries
    }

    // NOTE: FFI initializer commented out - FFIFilterMatches not available in current FFI
    // /// Initialize from FFI struct
    // public init(from ffiMatches: FFIFilterMatches) {
    //     var entries: [FilterMatchEntry] = []
    //
    //     if let entriesPtr = ffiMatches.entries {
    //         for i in 0..<ffiMatches.count {
    //             let ffiEntry = entriesPtr.advanced(by: Int(i)).pointee
    //             entries.append(FilterMatchEntry(from: ffiEntry))
    //         }
    //     }
    //
    //     self.entries = entries
    // }

    /// Check if empty
    public var isEmpty: Bool {
        entries.isEmpty
    }

    /// Get total number of heights with matches
    public var count: Int {
        entries.count
    }
}

/// Error types for filter operations
public enum FilterMatchError: Error, LocalizedError {
    case clientNotAvailable
    case invalidRange(String)
    case ffiError(String)
    case clientBusy
    case unknown

    public var errorDescription: String? {
        switch self {
        case .clientNotAvailable:
            return "SPV client not initialized. Please start sync from the Wallets screen first."
        case .invalidRange(let message):
            return "Invalid range: \(message)"
        case .ffiError(let message):
            // Check if it's the "Client not initialized" error which actually means busy
            if message.contains("Client not initialized") {
                return "SPV client is busy syncing. Please wait for sync to complete, then tap Retry."
            }
            return "Error loading filters: \(message)"
        case .clientBusy:
            return "SPV client is busy. Please wait for sync to complete."
        case .unknown:
            return "Unknown error occurred"
        }
    }
}
