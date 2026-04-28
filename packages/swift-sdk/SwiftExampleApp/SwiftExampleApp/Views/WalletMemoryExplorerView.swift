// WalletMemoryExplorerView.swift
// SwiftExampleApp
//
// Read-only diagnostic surface that surfaces the in-memory state of
// every loaded platform-wallet — the Rust-side `ManagedPlatformWallet`
// / `PlatformWalletInfo` / `IdentityManager`. Mirrors the spirit of
// `StorageExplorerView` (SwiftData) and `KeychainExplorerView`
// (Keychain), but the source of truth is `PlatformWalletManager
// .wallets`.
//
// The whole point is "what does Rust hold *right now*" — every
// section is backed by a thin FFI enumerator on the Rust side and
// rendered as a flat list on the Swift side. No syncs, no edits, no
// background refreshes. Useful when SwiftData says an identity exists
// but the wallet's `IdentityManager` doesn't (the bug that motivated
// the view).

import SwiftUI
import SwiftDashSDK

// MARK: - Helpers

/// Truncated base58 view of a 32-byte identifier. Matches the
/// shorthand used elsewhere in the example app for compact lists.
private func shortBase58(_ id: Identifier) -> String {
    let b58 = id.toBase58()
    if b58.count <= 16 {
        return b58
    }
    return "\(b58.prefix(8))…\(b58.suffix(6))"
}

/// Full base58 of a 32-byte identifier.
private func fullBase58(_ id: Identifier) -> String {
    id.toBase58()
}

/// Pretty label for a loaded wallet — prefers the SwiftData
/// `PersistentWallet.name`, falls back to the first 8 hex chars of
/// the wallet id. Kept private so each view file can have its own
/// fallback policy.
private func walletDisplayLabel(_ walletId: Data, fromPersistent name: String?) -> String {
    if let name, !name.isEmpty {
        return name
    }
    let hex = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
    return hex.isEmpty ? "Unknown wallet" : "Wallet \(hex)…"
}

/// One-line key-value row. Long values are middle-truncated and made
/// `textSelection`-able so users can copy ids from the explorer.
private struct KVRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value)
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .font(.system(.body, design: .monospaced))
        }
    }
}

// MARK: - Top-level view: list of wallets

struct WalletMemoryExplorerView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext

    var body: some View {
        List {
            if walletManager.wallets.isEmpty {
                Section {
                    Text("No wallets currently loaded.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else {
                Section {
                    ForEach(sortedWalletIds, id: \.self) { walletId in
                        if let wallet = walletManager.wallets[walletId] {
                            NavigationLink {
                                WalletMemoryDetailView(
                                    wallet: wallet,
                                    walletLabel: walletDisplayLabel(walletId, fromPersistent: nil)
                                )
                            } label: {
                                walletRow(walletId: walletId, wallet: wallet)
                            }
                        }
                    }
                } header: {
                    Text("Loaded Wallets")
                } footer: {
                    Text(
                        "Read-only snapshot of what each wallet's "
                        + "Rust-side `IdentityManager` currently holds. "
                        + "Compare with the Storage Explorer to spot "
                        + "drift between SwiftData and the in-memory "
                        + "wallet state."
                    )
                    .font(.caption)
                }
            }
        }
        .navigationTitle("Wallet Memory Explorer")
        .navigationBarTitleDisplayMode(.inline)
    }

    /// Stable display order — sort by raw walletId bytes so the list
    /// position is deterministic across launches.
    private var sortedWalletIds: [Data] {
        walletManager.wallets.keys.sorted { lhs, rhs in
            lhs.lexicographicallyPrecedes(rhs)
        }
    }

    @ViewBuilder
    private func walletRow(walletId: Data, wallet: ManagedPlatformWallet) -> some View {
        // Pull the summary inline so the row shows the same counts
        // the detail view confirms — keeps the diagnostic value of
        // the listing front-loaded without an extra tap.
        let summary = (try? wallet.inMemorySummary()) ?? InMemoryWalletSummary(
            identitiesCount: 0,
            watchedCount: 0,
            lastScannedIndex: 0,
            primaryIdentityId: nil,
            trackedAssetLocksCount: 0,
            tokenBalancesCount: 0
        )
        VStack(alignment: .leading, spacing: 4) {
            Text(walletDisplayLabel(walletId, fromPersistent: nil))
                .font(.headline)
            HStack(spacing: 4) {
                Text("\(summary.identitiesCount) identities")
                Text("·")
                Text("\(summary.watchedCount) watched")
                Text("·")
                Text("\(summary.trackedAssetLocksCount) asset locks")
            }
            .font(.caption)
            .foregroundColor(.secondary)
        }
        .padding(.vertical, 2)
    }
}

// MARK: - Per-wallet detail view

struct WalletMemoryDetailView: View {
    let wallet: ManagedPlatformWallet
    let walletLabel: String

    @State private var summary: InMemoryWalletSummary?
    @State private var summaryError: String?
    @State private var identityIds: [Identifier] = []
    @State private var watchedIds: [Identifier] = []
    @State private var idLabels: [Identifier: String] = [:]
    @State private var loadError: String?

    var body: some View {
        Form {
            Section("Summary") {
                if let summaryError {
                    Text(summaryError)
                        .font(.caption)
                        .foregroundColor(.red)
                } else if let summary {
                    KVRow(label: "Identities", value: "\(summary.identitiesCount)")
                    KVRow(label: "Watched", value: "\(summary.watchedCount)")
                    KVRow(label: "Last Scanned Index", value: "\(summary.lastScannedIndex)")
                    KVRow(
                        label: "Tracked Asset Locks",
                        value: "\(summary.trackedAssetLocksCount)"
                    )
                    KVRow(
                        label: "Token Balances",
                        value: "\(summary.tokenBalancesCount)"
                    )
                    if let primary = summary.primaryIdentityId {
                        KVRow(label: "Primary Identity", value: shortBase58(primary))
                        Text(fullBase58(primary))
                            .font(.caption2.monospaced())
                            .foregroundColor(.secondary)
                            .textSelection(.enabled)
                    } else {
                        KVRow(label: "Primary Identity", value: "—")
                    }
                } else {
                    HStack {
                        ProgressView().scaleEffect(0.8)
                        Text("Loading…").font(.caption).foregroundColor(.secondary)
                    }
                }
            }

            Section {
                if identityIds.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(identityIds, id: \.self) { id in
                        NavigationLink {
                            WalletMemoryIdentityDetailView(wallet: wallet, identityId: id)
                        } label: {
                            identityRow(id: id)
                        }
                    }
                }
            } header: {
                HStack {
                    Text("Identities")
                    Spacer()
                    Text("\(identityIds.count)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } footer: {
                Text("Tap a row to dump its in-memory state.")
                    .font(.caption2)
            }

            Section {
                if watchedIds.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(watchedIds, id: \.self) { id in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(shortBase58(id))
                                .font(.system(.body, design: .monospaced))
                            Text(fullBase58(id))
                                .font(.caption2.monospaced())
                                .foregroundColor(.secondary)
                                .textSelection(.enabled)
                        }
                    }
                }
            } header: {
                HStack {
                    Text("Watched Identities")
                    Spacer()
                    Text("\(watchedIds.count)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            if let loadError {
                Section {
                    Text(loadError)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
        }
        .navigationTitle(walletLabel)
        .navigationBarTitleDisplayMode(.inline)
        // Single one-shot load — the explorer is read-only, refreshing
        // would mask the drift between SwiftData and in-memory state
        // we are trying to surface.
        .onAppear { loadOnce() }
    }

    @ViewBuilder
    private func identityRow(id: Identifier) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text(shortBase58(id))
                    .font(.system(.body, design: .monospaced))
                Spacer()
                if let label = idLabels[id], !label.isEmpty {
                    Text(label)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            Text(fullBase58(id))
                .font(.caption2.monospaced())
                .foregroundColor(.secondary)
                .textSelection(.enabled)
        }
    }

    private func loadOnce() {
        guard summary == nil else { return }
        do {
            summary = try wallet.inMemorySummary()
        } catch {
            summaryError = "Summary failed: \(error.localizedDescription)"
        }
        do {
            identityIds = try wallet.inMemoryIdentityIds()
        } catch {
            loadError = "Identity list failed: \(error.localizedDescription)"
            identityIds = []
        }
        do {
            watchedIds = try wallet.inMemoryWatchedIdentityIds()
        } catch {
            // Combined error message — same diagnostic surface either way.
            loadError = "\(loadError ?? "")\nWatched list failed: "
                + "\(error.localizedDescription)"
            watchedIds = []
        }
        // Best-effort labels for the identity rows. Failures here are
        // not fatal — the row still shows the truncated id.
        for id in identityIds {
            if let mi = try? wallet.managedIdentity(identityId: id),
               let label = try? mi.getLabel(), !label.isEmpty {
                idLabels[id] = label
            }
        }
    }
}

// MARK: - Per-identity detail view

struct WalletMemoryIdentityDetailView: View {
    let wallet: ManagedPlatformWallet
    let identityId: Identifier

    @State private var loaded = false
    @State private var loadError: String?

    @State private var balance: UInt64 = 0
    @State private var revision: UInt64 = 0
    @State private var label: String?
    @State private var identityIndex: UInt32? = nil
    @State private var status: IdentityStatus = .unknown

    @State private var publicKeys: [ManagedIdentity.IdentityPublicKeyInfo] = []
    @State private var sentRequestIds: [Identifier] = []
    @State private var incomingRequestIds: [Identifier] = []
    @State private var establishedContactIds: [Identifier] = []
    @State private var dpnsNames: [String] = []
    @State private var contestedDpnsNames: [String] = []

    @State private var dashpayProfile: DashPayProfile?
    @State private var dashpayProfileMissing: Bool = false

    var body: some View {
        Form {
            Section("Identity") {
                KVRow(label: "Id (short)", value: shortBase58(identityId))
                Text(fullBase58(identityId))
                    .font(.caption2.monospaced())
                    .foregroundColor(.secondary)
                    .textSelection(.enabled)
                if loaded {
                    KVRow(label: "Label", value: label ?? "—")
                    KVRow(label: "Balance", value: "\(balance)")
                    KVRow(label: "Revision", value: "\(revision)")
                    KVRow(
                        label: "Identity Index",
                        value: identityIndex.map { "\($0)" } ?? "— (out of wallet)"
                    )
                    KVRow(label: "Status", value: status.displayName)
                }
            }

            if loaded {
                Section {
                    if publicKeys.isEmpty {
                        Text("None")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(publicKeys, id: \.keyId) { key in
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Key \(key.keyId)")
                                    .font(.system(.body, design: .monospaced))
                                Text(
                                    "\(key.purpose.name) · \(key.securityLevel.name) · "
                                    + "\(key.keyType.name)"
                                )
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            }
                        }
                    }
                } header: {
                    sectionHeader("Public Keys", count: publicKeys.count)
                }

                Section {
                    if sentRequestIds.isEmpty {
                        Text("None")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(sentRequestIds, id: \.self) { id in
                            idRow(id)
                        }
                    }
                } header: {
                    sectionHeader("Sent Contact Requests", count: sentRequestIds.count)
                }

                Section {
                    if incomingRequestIds.isEmpty {
                        Text("None")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(incomingRequestIds, id: \.self) { id in
                            idRow(id)
                        }
                    }
                } header: {
                    sectionHeader(
                        "Incoming Contact Requests",
                        count: incomingRequestIds.count
                    )
                }

                Section {
                    if establishedContactIds.isEmpty {
                        Text("None")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(establishedContactIds, id: \.self) { id in
                            idRow(id)
                        }
                    }
                } header: {
                    sectionHeader(
                        "Established Contacts",
                        count: establishedContactIds.count
                    )
                }

                Section {
                    if dpnsNames.isEmpty {
                        Text("None")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(dpnsNames, id: \.self) { name in
                            Text(name)
                                .font(.system(.body, design: .monospaced))
                                .textSelection(.enabled)
                        }
                    }
                } header: {
                    sectionHeader("DPNS Names", count: dpnsNames.count)
                }

                Section {
                    if contestedDpnsNames.isEmpty {
                        Text("None")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(contestedDpnsNames, id: \.self) { name in
                            Text(name)
                                .font(.system(.body, design: .monospaced))
                                .textSelection(.enabled)
                        }
                    }
                } header: {
                    sectionHeader(
                        "Contested DPNS Names",
                        count: contestedDpnsNames.count
                    )
                }

                Section("DashPay Profile") {
                    if let profile = dashpayProfile {
                        KVRow(label: "Display Name", value: profile.displayName ?? "—")
                        KVRow(label: "Public Message", value: profile.publicMessage ?? "—")
                        KVRow(label: "Avatar URL", value: profile.avatarUrl ?? "—")
                    } else if dashpayProfileMissing {
                        Text("No profile cached.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        // Profile read errored but we surface the
                        // overall error in the loadError section
                        // below; show a neutral placeholder here so
                        // section count stays stable.
                        Text("—")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }

            if let loadError {
                Section {
                    Text(loadError)
                        .font(.caption)
                        .foregroundColor(.red)
                        .textSelection(.enabled)
                }
            }
        }
        .navigationTitle("Identity")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { loadOnce() }
    }

    private func sectionHeader(_ title: String, count: Int) -> some View {
        HStack {
            Text(title)
            Spacer()
            Text("\(count)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    @ViewBuilder
    private func idRow(_ id: Identifier) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(shortBase58(id))
                .font(.system(.body, design: .monospaced))
            Text(fullBase58(id))
                .font(.caption2.monospaced())
                .foregroundColor(.secondary)
                .textSelection(.enabled)
        }
    }

    private func loadOnce() {
        guard !loaded else { return }
        var errors: [String] = []
        do {
            let mi = try wallet.managedIdentity(identityId: identityId)
            balance = (try? mi.getBalance()) ?? 0
            revision = (try? mi.getRevision()) ?? 0
            label = (try? mi.getLabel()).flatMap { $0 }
            identityIndex = (try? mi.getIdentityIndex()) ?? nil
            status = (try? mi.getStatus()) ?? .unknown
            publicKeys = (try? mi.getPublicKeys()) ?? []
            sentRequestIds = (try? mi.getSentContactRequestIds()) ?? []
            incomingRequestIds = (try? mi.getIncomingContactRequestIds()) ?? []
            establishedContactIds = (try? mi.getEstablishedContactIds()) ?? []
            dpnsNames = (try? mi.getDpnsNames()) ?? []
            contestedDpnsNames = (try? mi.getContestedDpnsNames()) ?? []
            do {
                dashpayProfile = try mi.getDashPayProfile()
                dashpayProfileMissing = (dashpayProfile == nil)
            } catch let err as PlatformWalletError {
                if case .identityNotFound = err {
                    dashpayProfileMissing = true
                } else {
                    errors.append("DashPay profile: \(err.localizedDescription)")
                }
            } catch {
                errors.append("DashPay profile: \(error.localizedDescription)")
            }
        } catch {
            errors.append(
                "Identity \(shortBase58(identityId)) not found in wallet: "
                + error.localizedDescription
            )
        }
        if !errors.isEmpty {
            loadError = errors.joined(separator: "\n")
        }
        loaded = true
    }
}
