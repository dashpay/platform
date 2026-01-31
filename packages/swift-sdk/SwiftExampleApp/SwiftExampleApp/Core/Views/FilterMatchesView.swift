//
//  FilterMatchesView.swift
//  SwiftExampleApp
//
//  View for displaying compact filter matches with smooth scrolling and jump-to
//

import SwiftUI
import SwiftDashSDK

enum FilterDisplayMode: String, CaseIterable {
    case all = "All Filters"
    case matched = "Matched Filters"
}

struct FilterMatchesView: View {
    @EnvironmentObject var walletService: WalletService
    @StateObject private var service: FilterMatchService
    @State private var showJumpToAlert = false
    @State private var jumpToHeight = ""
    @State private var expandedHeights: Set<UInt32> = []
    @State private var displayMode: FilterDisplayMode = .all

    init(walletService: WalletService) {
        _service = StateObject(wrappedValue: FilterMatchService(walletService: walletService))
    }

    // Computed property to filter based on selected mode
    private var filteredFilters: [CompactFilter] {
        switch displayMode {
        case .all:
            return service.filters
        case .matched:
            return service.matchedFilters
        }
    }

    // Wait for sync to complete before loading filters
    private func waitForSyncToComplete() async {
        // If sync is not running, return immediately
        guard walletService.isSyncing else { return }

        print("⏳ Waiting for sync to complete before loading filters...")

        // Poll until sync completes (check every 0.5 seconds)
        while walletService.isSyncing {
            try? await Task.sleep(nanoseconds: 500_000_000) // 0.5 seconds
        }

        // Give extra time for client to fully release locks
        try? await Task.sleep(nanoseconds: 1_000_000_000) // 1 second

        print("✅ Sync completed, ready to load filters")
    }

    var body: some View {
        VStack(spacing: 0) {
            // Mode selector
            modeSelector

            // Jump-to bar
            jumpToBar

            // Main content
            if service.isLoading && service.filters.isEmpty {
                loadingView
            } else if let error = service.error {
                errorView(error)
            } else if filteredFilters.isEmpty {
                emptyView
            } else {
                filtersList
            }
        }
        .navigationTitle("Compact Filters")
        .navigationBarTitleDisplayMode(.inline)
        .task {
            // Wait for sync to complete before loading filters
            await waitForSyncToComplete()

            // Initialize with current sync height
            let currentHeight = UInt32(walletService.syncProgress.filterHeight)
            await service.initialize(endHeight: currentHeight)
        }
        .alert("Jump to Height", isPresented: $showJumpToAlert) {
            TextField("Block Height", text: $jumpToHeight)
                .keyboardType(.numberPad)

            Button("Cancel", role: .cancel) {
                jumpToHeight = ""
            }

            Button("Go") {
                if let height = UInt32(jumpToHeight) {
                    Task {
                        await service.jumpTo(height: height)
                    }
                }
                jumpToHeight = ""
            }
        } message: {
            if let range = service.heightRange {
                Text("Enter a height between \(range.lowerBound) and \(range.upperBound)")
            } else {
                Text("Enter a block height")
            }
        }
    }

    // MARK: - Mode Selector

    private var modeSelector: some View {
        Picker("Display Mode", selection: $displayMode) {
            ForEach(FilterDisplayMode.allCases, id: \.self) { mode in
                Text(mode.rawValue).tag(mode)
            }
        }
        .pickerStyle(.segmented)
        .padding(.horizontal)
        .padding(.vertical, 12)
        .background(Color(.systemBackground))
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundColor(Color(.separator)),
            alignment: .bottom
        )
    }

    // MARK: - Jump-to Bar

    private var jumpToBar: some View {
        HStack(spacing: 12) {
            Text(displayMode == .all ? "All Filters" : "Matched Filters")
                .font(.headline)
                .foregroundColor(.secondary)

            Spacer()

            if !filteredFilters.isEmpty {
                Text("\(filteredFilters.count) filter\(filteredFilters.count == 1 ? "" : "s")")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Button(action: {
                showJumpToAlert = true
            }) {
                HStack(spacing: 4) {
                    Image(systemName: "location.magnifyingglass")
                    Text("Jump to")
                }
                .font(.caption)
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(Color.blue)
                .foregroundColor(.white)
                .cornerRadius(8)
            }

            Button(action: {
                Task {
                    await service.reload()
                }
            }) {
                Image(systemName: "arrow.clockwise")
                    .font(.caption)
                    .padding(8)
                    .background(Color.gray.opacity(0.2))
                    .cornerRadius(8)
            }
        }
        .padding(.horizontal)
        .padding(.vertical, 12)
        .background(Color(.systemBackground))
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundColor(Color(.separator)),
            alignment: .bottom
        )
    }

    // MARK: - Filters List

    private var filtersList: some View {
        ScrollViewReader { proxy in
            List {
                ForEach(Array(filteredFilters.enumerated()), id: \.element.id) { index, filter in
                    FilterRow(filter: filter, isExpanded: expandedHeights.contains(filter.height), isMatched: displayMode == .matched || service.isFilterMatched(filter.height))
                        .onTapGesture {
                            withAnimation {
                                if expandedHeights.contains(filter.height) {
                                    expandedHeights.remove(filter.height)
                                } else {
                                    expandedHeights.insert(filter.height)
                                }
                            }
                        }
                        .onAppear {
                            // Trigger prefetch when this row appears
                            // Only prefetch in "All" mode since matched filters are already loaded
                            if displayMode == .all {
                                Task {
                                    await service.checkPrefetch(displayedIndex: index)
                                }
                            }
                        }
                }

                // Loading indicator at bottom
                if service.isLoading {
                    HStack {
                        Spacer()
                        ProgressView()
                            .padding()
                        Spacer()
                    }
                }
            }
            .listStyle(.plain)
        }
    }

    // MARK: - State Views

    private var loadingView: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)

            if walletService.isSyncing {
                VStack(spacing: 8) {
                    Text("Waiting for sync to complete...")
                        .font(.headline)
                        .foregroundColor(.secondary)

                    Text("Filters cannot be loaded while sync is in progress")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                }
            } else {
                Text("Loading compact filters...")
                    .font(.headline)
                    .foregroundColor(.secondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorView(_ error: FilterMatchError) -> some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 48))
                .foregroundColor(.red)

            Text("Error")
                .font(.headline)

            Text(error.localizedDescription)
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            Button("Retry") {
                Task {
                    // Wait for sync to complete before retrying
                    await waitForSyncToComplete()
                    await service.reload()
                }
            }
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyView: some View {
        VStack(spacing: 16) {
            Image(systemName: "tray")
                .font(.system(size: 48))
                .foregroundColor(.gray)

            if displayMode == .matched {
                Text("No Matched Filters")
                    .font(.headline)

                Text("No compact filters have matched any wallet addresses yet.\n\nFilters are checked during sync for relevant transactions.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
            } else {
                Text("No Compact Filters")
                    .font(.headline)

                Text("No compact filters have been downloaded yet.\n\nMake sure SPV sync is running.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Filter Row Component

struct FilterRow: View {
    let filter: CompactFilter
    let isExpanded: Bool
    let isMatched: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header row
            HStack {
                Image(systemName: isMatched ? "checkmark.circle.fill" : "line.3.horizontal.decrease.circle")
                    .foregroundColor(isMatched ? .green : .blue)
                    .font(.caption)

                Text("Height: \(filter.height)")
                    .font(.headline)

                Spacer()

                if isMatched {
                    Image(systemName: "star.fill")
                        .font(.caption2)
                        .foregroundColor(.orange)
                }

                Text("\(filter.sizeInBytes) bytes")
                    .font(.caption)
                    .foregroundColor(.secondary)

                Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            // Expanded filter data
            if isExpanded {
                VStack(alignment: .leading, spacing: 8) {
                    // Filter size info
                    HStack {
                        Text("Size:")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text("\(filter.sizeInBytes) bytes")
                            .font(.caption)
                            .foregroundColor(.primary)
                        Spacer()
                    }

                    // Filter data hex preview
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text("Filter Data:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Button(action: {
                                UIPasteboard.general.string = filter.data.hexEncodedString()
                            }) {
                                Image(systemName: "doc.on.doc")
                                    .font(.caption2)
                                    .foregroundColor(.blue)
                            }
                            .buttonStyle(.borderless)
                        }

                        // Show first 32 bytes as hex preview
                        let previewBytes = min(32, filter.data.count)
                        if previewBytes > 0 {
                            Text(filter.data.prefix(previewBytes).hexEncodedString() + (filter.data.count > previewBytes ? "..." : ""))
                                .font(.system(.caption2, design: .monospaced))
                                .foregroundColor(.primary)
                                .lineLimit(nil)
                                .padding(8)
                                .background(Color(.secondarySystemBackground))
                                .cornerRadius(4)
                        } else {
                            Text("(empty)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                                .italic()
                        }
                    }
                }
                .padding(.top, 4)
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Preview

#if DEBUG
struct FilterMatchesView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            FilterMatchesView(walletService: WalletService.shared)
                .environmentObject(WalletService.shared)
        }
    }
}
#endif
