// PlatformAddressBalanceHistoryView.swift
// SwiftExampleApp
//
// ADDR-10: balance change history of the wallet's DIP-17 Platform addresses,
// opened from the WalletDetailView Platform Balance row's ⋯ menu.
//
// Both sections come from one `ManagedPlatformAddressWallet.balanceHistory`
// call: the first load asks for recent changes only, "Load older history"
// asks for both and replaces both sections from that one result, so a block
// compacted in between never shows twice. Paging, the filter to this
// wallet's addresses (every platform payment account), the reconciliation
// of the two zones, the newest-first order and each address's bech32m form
// all happen in Rust (`platform-wallet`); this view only renders what it
// gets back.

import SwiftUI
import SwiftDashSDK

struct PlatformAddressBalanceHistoryView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    let wallet: PersistentWallet

    @State private var recent: [ManagedPlatformAddressWallet.BalanceHistoryEntry] = []
    @State private var hasLoadedRecent = false
    @State private var isLoadingRecent = false
    @State private var recentError: String?

    /// `nil` until the user asks for older history.
    @State private var compacted: [ManagedPlatformAddressWallet.CompactedBalanceHistoryEntry]?
    @State private var isLoadingCompacted = false
    @State private var compactedError: String?

    /// One load at a time: the first load, Load older and pull to refresh
    /// all replace the same sections.
    private var isBusy: Bool { isLoadingRecent || isLoadingCompacted }

    var body: some View {
        NavigationStack {
            List {
                recentSection
                olderSection
            }
            .accessibilityIdentifier("balanceHistory.list")
            .navigationTitle("Balance History")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                        .accessibilityIdentifier("balanceHistory.done")
                }
            }
            .refreshable { await refresh() }
            .task {
                if !hasLoadedRecent { await loadRecent() }
            }
        }
    }

    // MARK: - Sections

    @ViewBuilder
    private var recentSection: some View {
        Section {
            if let recentError {
                errorRow(recentError, identifier: "balanceHistory.recent.error")
            } else if !hasLoadedRecent || (isLoadingRecent && recent.isEmpty) {
                HStack {
                    ProgressView()
                    Text("Loading recent changes…")
                        .foregroundColor(.secondary)
                }
                .accessibilityIdentifier("balanceHistory.recent.loading")
            } else if recent.isEmpty {
                Text("No balance changes in recent blocks.")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("balanceHistory.recent.empty")
            } else {
                ForEach(Array(recent.enumerated()), id: \.offset) { index, entry in
                    recentRow(entry)
                        .accessibilityIdentifier("balanceHistory.recent.row.\(index)")
                }
            }
        } header: {
            Text("Recent")
        } footer: {
            Text("Changes in blocks Platform has not compacted yet. Pull to refresh.")
        }
    }

    @ViewBuilder
    private var olderSection: some View {
        Section {
            if let compacted {
                if compacted.isEmpty {
                    Text("No older balance changes.")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .accessibilityIdentifier("balanceHistory.compacted.empty")
                } else {
                    ForEach(Array(compacted.enumerated()), id: \.offset) { index, entry in
                        compactedRow(entry)
                            .accessibilityIdentifier("balanceHistory.compacted.row.\(index)")
                    }
                }
            }
            if let compactedError {
                errorRow(compactedError, identifier: "balanceHistory.compacted.error")
            }
            if compacted == nil || compactedError != nil {
                Button {
                    Task { await loadOlder() }
                } label: {
                    HStack {
                        if isLoadingCompacted {
                            ProgressView()
                        } else {
                            Image(systemName: "clock.arrow.circlepath")
                        }
                        Text(isLoadingCompacted ? "Loading older history…" : "Load older history")
                    }
                }
                .disabled(isBusy)
                .accessibilityIdentifier("balanceHistory.loadOlder")
            }
        } header: {
            Text("Older (compacted)")
        } footer: {
            Text("Platform merges older blocks into ranges and keeps them for about a week.")
        }
    }

    // MARK: - Rows

    private func recentRow(_ entry: ManagedPlatformAddressWallet.BalanceHistoryEntry) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Block \(entry.blockHeight)")
                    .font(.subheadline)
                    .fontWeight(.medium)
                Spacer()
                changeText(entry.change)
            }
            addressText(entry.address)
        }
        .padding(.vertical, 2)
    }

    private func compactedRow(
        _ entry: ManagedPlatformAddressWallet.CompactedBalanceHistoryEntry
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Blocks \(entry.startBlockHeight) to \(entry.endBlockHeight)")
                    .font(.subheadline)
                    .fontWeight(.medium)
                Spacer()
                switch entry.change {
                case .setFinal(let credits):
                    Text("set to \(formatCredits(credits))")
                        .font(.subheadline)
                        .foregroundColor(.orange)
                case .additions(let total, _):
                    Text("+\(formatCredits(total))")
                        .font(.subheadline)
                        .foregroundColor(.green)
                }
            }
            addressText(entry.address)
            if case .additions(_, let additions) = entry.change, !additions.isEmpty {
                ForEach(Array(additions.enumerated()), id: \.offset) { _, addition in
                    HStack {
                        Text("Block \(addition.blockHeight)")
                        Spacer()
                        Text("+\(formatCredits(addition.credits))")
                    }
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .padding(.leading, 12)
                }
            }
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder
    private func changeText(_ change: ManagedPlatformAddressWallet.BalanceHistoryEntry.Change) -> some View {
        switch change {
        case .added(let credits):
            Text("+\(formatCredits(credits))")
                .font(.subheadline)
                .foregroundColor(.green)
        case .set(let credits):
            Text("set to \(formatCredits(credits))")
                .font(.subheadline)
                .foregroundColor(.orange)
        }
    }

    private func addressText(_ address: String) -> some View {
        Text(address)
            .font(.system(.caption, design: .monospaced))
            .foregroundColor(.secondary)
            .lineLimit(1)
            .truncationMode(.middle)
    }

    private func errorRow(_ message: String, identifier: String) -> some View {
        Label(message, systemImage: "exclamationmark.triangle")
            .font(.caption)
            .foregroundColor(.red)
            .accessibilityIdentifier(identifier)
    }

    // MARK: - Loading

    private func addressWallet() throws -> ManagedPlatformAddressWallet {
        guard let managed = walletManager.wallet(for: wallet.walletId) else {
            throw PlatformWalletError.walletOperation("Wallet handle not found in the wallet manager.")
        }
        return try managed.platformAddressWallet()
    }

    /// Pull to refresh: reload what is shown, both sections from one call
    /// when older history is shown.
    private func refresh() async {
        if compacted != nil {
            await loadOlder()
        } else {
            await loadRecent()
        }
    }

    /// First load: the recent zone only.
    private func loadRecent() async {
        guard !isBusy else { return }
        isLoadingRecent = true
        defer {
            isLoadingRecent = false
            hasLoadedRecent = true
        }
        do {
            let history = try await addressWallet().balanceHistory(includeCompacted: false)
            recent = history.recent
            recentError = nil
        } catch {
            recentError = error.localizedDescription
        }
    }

    /// "Load older history": both zones from one call, replacing both
    /// sections, so no row shows in both.
    private func loadOlder() async {
        guard !isBusy else { return }
        isLoadingCompacted = true
        defer { isLoadingCompacted = false }
        do {
            let history = try await addressWallet().balanceHistory(includeCompacted: true)
            recent = history.recent
            compacted = history.compacted
            hasLoadedRecent = true
            recentError = nil
            compactedError = nil
        } catch {
            compactedError = error.localizedDescription
        }
    }

    // MARK: - Formatting

    /// Credits as DASH at full credit precision (exact integer arithmetic),
    /// the formatter the key screens use: 400 credits is "0.000000004 DASH".
    private func formatCredits(_ credits: UInt64) -> String {
        KeyLimitsFormatting.dash(credits)
    }
}
