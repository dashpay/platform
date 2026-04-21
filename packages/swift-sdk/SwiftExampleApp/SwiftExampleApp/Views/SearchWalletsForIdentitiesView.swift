// SearchWalletsForIdentitiesView.swift
// SwiftExampleApp
//
// Sheet invoked from the Identities tab that runs the HD
// gap-limit identity discovery scan over every loaded platform
// wallet, surfacing the count of newly-discovered identities on
// completion.
//
// Per the platform-wallet proposal: by default the scan resumes
// from each wallet's cached `last_scanned_index`; a "Full rescan
// from index 0" toggle flips to a cold scan. Heavy lifting
// (derivation, Platform lookup, cache writes) all happens on the
// Rust side — this view just drives the async call and reports
// aggregate results.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct SearchWalletsForIdentitiesView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss

    /// Local persisted wallet list drives the "wallets to scan"
    /// count shown in the explainer copy. The actual scan walks
    /// `walletManager.wallets` because that's where the runtime
    /// `ManagedPlatformWallet` handles live.
    @Query private var hdWallets: [HDWallet]

    /// Toggle wired to the `startIndex` argument: `false` → resume
    /// from cache (nil); `true` → start from 0.
    @State private var fullRescan: Bool = false

    @State private var isSearching: Bool = false
    @State private var result: SearchOutcome?

    /// Result bucket. `perWallet` drives the per-wallet breakdown
    /// list that renders after a successful scan.
    struct SearchOutcome: Equatable {
        struct WalletFinding: Equatable, Identifiable {
            let walletId: Data
            let label: String
            let foundCount: Int
            let error: String?
            var id: Data { walletId }
        }
        var perWallet: [WalletFinding]
        var totalFound: Int {
            perWallet.reduce(0) { $0 + $1.foundCount }
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Scan every loaded wallet's DIP-9 identity "
                            + "authentication tree and register any "
                            + "identities found on Platform.")
                            .font(.callout)
                        Text("\(hdWallets.count) wallet\(hdWallets.count == 1 ? "" : "s") will be scanned.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.vertical, 2)
                }

                Section("Options") {
                    Toggle("Full rescan from index 0", isOn: $fullRescan)
                        .disabled(isSearching)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(fullRescan
                            ? "Every wallet is scanned from identity index 0. Slower."
                            : "Each wallet resumes from its cached last-scanned index. Fast path.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }

                if let result {
                    Section("Result") {
                        HStack {
                            Text("Total new identities")
                            Spacer()
                            Text("\(result.totalFound)")
                                .fontWeight(.semibold)
                                .foregroundColor(result.totalFound > 0 ? .green : .secondary)
                        }
                        ForEach(result.perWallet) { wallet in
                            HStack(alignment: .top) {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(wallet.label)
                                        .font(.subheadline)
                                    if let err = wallet.error {
                                        Text(err)
                                            .font(.caption)
                                            .foregroundColor(.red)
                                            .lineLimit(2)
                                    }
                                }
                                Spacer()
                                Text("+\(wallet.foundCount)")
                                    .foregroundColor(wallet.foundCount > 0 ? .green : .secondary)
                            }
                        }
                    }
                }

                Section {
                    Button {
                        Task { await runScan() }
                    } label: {
                        HStack {
                            if isSearching {
                                ProgressView()
                                    .scaleEffect(0.8)
                                    .padding(.trailing, 4)
                                Text("Scanning…")
                            } else {
                                Image(systemName: "magnifyingglass")
                                Text(result == nil ? "Search Wallets" : "Search Again")
                            }
                            Spacer()
                        }
                    }
                    .disabled(isSearching || hdWallets.isEmpty)
                }
            }
            .navigationTitle("Search Wallets")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                        .disabled(isSearching)
                }
            }
        }
    }

    /// Iterate every runtime wallet, kick off `discoverIdentities`
    /// for each, and aggregate the outcome. Failures for individual
    /// wallets are collected into `perWallet[i].error` rather than
    /// aborting — partial progress is still useful.
    private func runScan() async {
        isSearching = true
        defer { isSearching = false }

        // Snapshot the map so we don't hold the publisher mid-iteration.
        let runtimeWallets = walletManager.wallets

        // Join runtime wallets against the HDWallet table so we can
        // render the user-visible label. A runtime wallet that's
        // missing from SwiftData still gets scanned; we just fall
        // back to its id fingerprint for the label.
        let labelsById = Dictionary(
            uniqueKeysWithValues: hdWallets.map { ($0.walletId, $0.label) }
        )

        var findings: [SearchOutcome.WalletFinding] = []
        for (walletId, wallet) in runtimeWallets {
            let label = labelsById[walletId] ?? labelFingerprint(walletId)
            do {
                let found = try await wallet.discoverIdentities(
                    startIndex: fullRescan ? 0 : nil,
                    gapLimit: nil
                )
                findings.append(
                    SearchOutcome.WalletFinding(
                        walletId: walletId,
                        label: label,
                        foundCount: found.count,
                        error: nil
                    )
                )
            } catch {
                findings.append(
                    SearchOutcome.WalletFinding(
                        walletId: walletId,
                        label: label,
                        foundCount: 0,
                        error: error.localizedDescription
                    )
                )
            }
        }

        // Sort: wallets with hits first, then by label for stability.
        findings.sort { lhs, rhs in
            if lhs.foundCount != rhs.foundCount {
                return lhs.foundCount > rhs.foundCount
            }
            return lhs.label.localizedCaseInsensitiveCompare(rhs.label) == .orderedAscending
        }

        result = SearchOutcome(perWallet: findings)
    }

    /// Short human fingerprint for a wallet id when we have no
    /// persisted label to render (e.g. a runtime-only wallet that
    /// hasn't been saved yet).
    private func labelFingerprint(_ walletId: Data) -> String {
        let hex = walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
        return "Wallet \(hex)"
    }
}
