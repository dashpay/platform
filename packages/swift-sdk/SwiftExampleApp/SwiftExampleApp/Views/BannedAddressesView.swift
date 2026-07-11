// BannedAddressesView.swift
// SwiftExampleApp
//
// Read-only diagnostic surface for the DAPI address ban list. Renders
// the synchronous snapshot returned by
// `PlatformWalletManager.addressBanInfo()` — the Rust-side DAPI client's
// per-address ban state, including the reason each address was banned
// (when recorded).
//
// Complements the other Data-section explorers (Storage / Keychain /
// Wallet Memory). Like those, this view only READS via the existing FFI
// wrapper and renders; it makes no policy decisions, performs no
// iteration logic, and writes nothing back.

import SwiftUI
import SwiftDashSDK

struct BannedAddressesView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Snapshot loaded imperatively from `addressBanInfo()`. The wrapper
    /// is a synchronous one-shot read (not a SwiftData `@Query`), so the
    /// view loads it on appear and re-loads on manual refresh rather
    /// than observing reactively.
    @State private var entries: [PlatformWalletManager.AddressBanInfo] = []
    @State private var hasLoaded = false

    /// Stable formatter for the `bannedUntil` instant. Built once so we
    /// don't allocate a `DateFormatter` per row.
    private static let dateFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateStyle = .short
        f.timeStyle = .medium
        return f
    }()

    var body: some View {
        Group {
            if entries.isEmpty {
                emptyState
            } else {
                List {
                    Section {
                        ForEach(Array(entries.enumerated()), id: \.offset) { _, entry in
                            BanRow(entry: entry, dateFormatter: Self.dateFormatter)
                        }
                    } header: {
                        Text("Addresses (\(entries.count))")
                    } footer: {
                        sessionCaption
                    }
                }
            }
        }
        .navigationTitle("Banned Addresses")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    load()
                } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .accessibilityLabel("Refresh")
            }
        }
        .refreshable { load() }
        .onAppear {
            // Load once on first appearance; pull-to-refresh and the
            // toolbar button drive subsequent reloads.
            guard !hasLoaded else { return }
            load()
        }
    }

    // MARK: - Empty state

    @ViewBuilder
    private var emptyState: some View {
        ContentUnavailableView {
            Label("No Banned Addresses", systemImage: "nosign")
        } description: {
            Text(emptyStateMessage)
        } actions: {
            Button("Refresh") { load() }
        }
    }

    private var emptyStateMessage: String {
        "This list reflects the current SDK session. An empty list can "
        + "mean either that no DAPI addresses have been banned, or that "
        + "the address pool has not yet been seeded."
    }

    private var sessionCaption: some View {
        Text(
            "Reflects the current SDK session. An empty list can mean "
            + "either no bans or an unseeded address pool."
        )
        .font(.caption)
        .foregroundColor(.secondary)
    }

    // MARK: - Load

    private func load() {
        entries = walletManager.addressBanInfo()
        hasLoaded = true
    }
}

// MARK: - Row

private struct BanRow: View {
    let entry: PlatformWalletManager.AddressBanInfo
    let dateFormatter: DateFormatter

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(entry.address)
                    .font(.system(.body, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
                Spacer(minLength: 8)
                BanStatusBadge(banned: entry.banned)
            }

            KVLine(label: "Ban Count", value: "\(entry.banCount)")
            KVLine(
                label: "Banned Until",
                value: entry.bannedUntil.map { dateFormatter.string(from: $0) } ?? "—"
            )

            VStack(alignment: .leading, spacing: 2) {
                Text("Reason")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(entry.reason ?? "—")
                    .font(.callout)
                    .fixedSize(horizontal: false, vertical: true)
                    .textSelection(.enabled)
            }
        }
        .padding(.vertical, 2)
    }
}

/// A label/value line where the value is monospaced and trailing-aligned.
private struct KVLine: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(.system(.caption, design: .monospaced))
        }
    }
}

/// Red "Banned" capsule vs green "Live" capsule, mirroring the badge
/// style used by `AccountVariantBadge` in the Wallet Memory explorer.
private struct BanStatusBadge: View {
    let banned: Bool

    var body: some View {
        Text(banned ? "Banned" : "Live")
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
            .background((banned ? Color.red : Color.green).opacity(0.18))
            .foregroundColor(banned ? .red : .green)
            .clipShape(Capsule())
    }
}
