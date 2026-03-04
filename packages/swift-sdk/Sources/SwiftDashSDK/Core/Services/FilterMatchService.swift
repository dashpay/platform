//
//  FilterMatchService.swift
//  SwiftExampleApp
//
//  Service for querying and batch-loading filter matches from SPV client
//

import Foundation
import DashSDKFFI

/// Service for managing compact filter queries with batch loading and caching
@MainActor
public class FilterMatchService: ObservableObject {
    // MARK: - Published Properties

    /// All loaded compact filters (sorted by height descending)
    @Published public private(set) var filters: [CompactFilter] = []

    /// Matched filter heights (filters that matched wallet addresses)
    @Published public private(set) var matchedHeights: Set<UInt32> = []

    /// Loading state
    @Published public private(set) var isLoading = false

    /// Error state
    @Published public private(set) var error: FilterMatchError?

    /// Total height range available
    @Published public private(set) var heightRange: ClosedRange<UInt32>?

    // MARK: - Private Properties

    /// Reference to wallet service for SPV client access
    private weak var walletService: WalletService?

    /// Batch size for loading (must be ≤ 10,000 per FFI constraint)
    private let batchSize: UInt32 = 1_000

    /// Pre-fetch threshold (load more when this many rows from end)
    private let prefetchThreshold: Int = 50

    /// Cache of loaded height ranges to avoid duplicate queries
    private var loadedRanges: [ClosedRange<UInt32>] = []

    // MARK: - Initialization

    public init(walletService: WalletService) {
        self.walletService = walletService
    }

    // MARK: - Computed Properties

    /// Filters that matched wallet addresses
    public var matchedFilters: [CompactFilter] {
        filters.filter { matchedHeights.contains($0.height) }
    }

    /// Check if a specific height has a matched filter
    public func isFilterMatched(_ height: UInt32) -> Bool {
        matchedHeights.contains(height)
    }

    // MARK: - Public Methods

    /// Initialize the service and load the initial batch
    public func initialize(endHeight: UInt32) async {
        print("🔍 FilterMatchService: Initializing with endHeight=\(endHeight)")
        self.heightRange = 0...endHeight
        await loadMatchedHeights()
        await loadInitialBatch()
    }

    /// Update the height range (when sync progresses)
    public func updateHeightRange(endHeight: UInt32) {
        self.heightRange = 0...endHeight
    }

    /// Jump to a specific height and load surrounding data
    public func jumpTo(height: UInt32) async {
        guard let range = heightRange,
              range.contains(height) else {
            error = .invalidRange("Height \(height) is outside valid range")
            return
        }

        // Clear existing filters and load around the target height
        filters = []
        loadedRanges = []

        // Load batch containing the target height, avoiding underflow
        let startHeight: UInt32
        if height >= batchSize / 2 {
            startHeight = height - batchSize / 2
        } else {
            startHeight = range.lowerBound
        }
        await loadBatch(startHeight: startHeight)
    }

    /// Check if we need to prefetch more data based on scroll position
    public func checkPrefetch(displayedIndex: Int) async {
        guard !isLoading,
              let range = heightRange else {
            return
        }

        // Check if we're near the end of loaded data
        if displayedIndex >= filters.count - prefetchThreshold {
            // Load more data before the oldest loaded height
            if let oldestLoaded = filters.last?.height,
               oldestLoaded > range.lowerBound {
                // Avoid underflow when calculating startHeight
                let startHeight: UInt32
                if oldestLoaded >= batchSize {
                    startHeight = max(range.lowerBound, oldestLoaded - batchSize)
                } else {
                    startHeight = range.lowerBound
                }
                await loadBatch(startHeight: startHeight)
            }
        }

        // Check if we're near the beginning and need newer data
        if displayedIndex < prefetchThreshold {
            if let newestLoaded = filters.first?.height,
               newestLoaded < range.upperBound {
                let startHeight = min(range.upperBound - batchSize + 1, newestLoaded + 1)
                await loadBatch(startHeight: startHeight)
            }
        }
    }

    /// Reload all data (useful after sync completes)
    public func reload() async {
        filters = []
        loadedRanges = []
        await loadInitialBatch()
    }

    // MARK: - Private Methods

    /// Load matched filter heights from FFI
    /// NOTE: FFI functions for filter matching are not yet available in current FFI
    private func loadMatchedHeights() async {
        guard walletService != nil,
              heightRange != nil else {
            print("❌ FilterMatchService: Cannot load matched heights - client not available")
            return
        }

        print("🔍 FilterMatchService: Filter matching FFI not available in current build")

        // NOTE: The following FFI functions are not available in the current FFI:
        // - dash_spv_ffi_client_get_filter_matched_heights
        // - dash_spv_ffi_filter_matches_destroy
        // When these become available, uncomment and use:
        //
        // guard let client = walletService.spvClientHandle else { return }
        // let matchesPtr = dash_spv_ffi_client_get_filter_matched_heights(client, range.lowerBound, range.upperBound + 1)
        // defer { if let ptr = matchesPtr { dash_spv_ffi_filter_matches_destroy(ptr) } }
        // guard let ptr = matchesPtr else { return }
        // let ffiMatches = ptr.pointee
        // let filterMatches = FilterMatches(from: ffiMatches)
        // var heights = Set<UInt32>()
        // for entry in filterMatches.entries { heights.insert(entry.height) }
        // matchedHeights = heights

        matchedHeights = Set<UInt32>()
    }

    private func loadInitialBatch() async {
        guard let range = heightRange else { return }

        // Load the most recent batch, avoiding underflow
        let startHeight: UInt32
        if range.upperBound >= batchSize - 1 {
            startHeight = max(range.lowerBound, range.upperBound - batchSize + 1)
        } else {
            startHeight = range.lowerBound
        }
        await loadBatch(startHeight: startHeight)
    }

    /// NOTE: FFI functions for loading compact filters are not yet available in current FFI
    private func loadBatch(startHeight: UInt32) async {
        guard walletService != nil else {
            print("❌ FilterMatchService: WalletService not available")
            error = .clientNotAvailable
            return
        }

        guard let range = heightRange else {
            print("❌ FilterMatchService: Height range not set")
            error = .clientNotAvailable
            return
        }

        // Calculate end height (exclusive, max batchSize)
        let endHeight = min(range.upperBound + 1, startHeight + batchSize)

        print("🔍 FilterMatchService: Loading filters from \(startHeight) to \(endHeight)")
        print("🔍 FilterMatchService: Compact filter loading FFI not available in current build")

        // Check if this range is already loaded
        let requestedRange = startHeight...endHeight
        if loadedRanges.contains(where: { $0.overlaps(requestedRange) }) {
            return
        }

        isLoading = true
        error = nil

        // NOTE: The following FFI functions are not available in the current FFI:
        // - dash_spv_ffi_client_load_filters
        // - dash_spv_ffi_compact_filters_destroy
        // When these become available, uncomment and use:
        //
        // guard let client = walletService.spvClientHandle else {
        //     error = .clientNotAvailable
        //     isLoading = false
        //     return
        // }
        // let filtersPtr = dash_spv_ffi_client_load_filters(client, startHeight, endHeight)
        // defer { if let ptr = filtersPtr { dash_spv_ffi_compact_filters_destroy(ptr) } }
        // guard let ptr = filtersPtr else {
        //     if let errorCStr = dash_spv_ffi_get_last_error() {
        //         error = .ffiError(String(cString: errorCStr))
        //     } else {
        //         error = .unknown
        //     }
        //     isLoading = false
        //     return
        // }
        // let ffiFilters = ptr.pointee
        // let compactFilters = CompactFilters(from: ffiFilters)
        // var allFilters = filters + compactFilters.filters
        // allFilters.sort { $0.height > $1.height }
        // var seenHeights = Set<UInt32>()
        // allFilters = allFilters.filter { filter in
        //     if seenHeights.contains(filter.height) { return false }
        //     seenHeights.insert(filter.height)
        //     return true
        // }
        // filters = allFilters

        // Currently return empty filters since FFI is not available
        loadedRanges.append(startHeight...(endHeight - 1))
        print("🔍 FilterMatchService: Stubbed - no filters loaded (FFI not available)")

        isLoading = false
    }
}

// MARK: - CompactFilter Hashable Conformance

extension CompactFilter: Hashable {
    public static func == (lhs: CompactFilter, rhs: CompactFilter) -> Bool {
        lhs.height == rhs.height
    }

    public func hash(into hasher: inout Hasher) {
        hasher.combine(height)
    }
}

// MARK: - Helper Extensions

extension Data {
    /// Convert Data to hex string for display
    public func hexEncodedString() -> String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
