// WalletMemoryExplorerView.swift
// SwiftExampleApp
//
// Read-only diagnostic surface that mirrors the complete in-memory
// state of `PlatformWalletManager` — the Rust-side wallet manager,
// SPV runtime, platform-address sync manager, identity-token sync
// manager, and every loaded wallet's balance / identity / asset-lock
// state.
//
// Complements `StorageExplorerView` (SwiftData) and
// `KeychainExplorerView` (Keychain). Every section is backed by a
// thin FFI query or a `@Published` property on `PlatformWalletManager`.

import SwiftUI
import SwiftDashSDK

// MARK: - Helpers

private func shortBase58(_ id: Identifier) -> String {
    let b58 = id.toBase58()
    if b58.count <= 16 { return b58 }
    return "\(b58.prefix(8))…\(b58.suffix(6))"
}

private func fullBase58(_ id: Identifier) -> String {
    id.toBase58()
}

private func walletDisplayLabel(_ walletId: Data, fromPersistent name: String?) -> String {
    if let name, !name.isEmpty { return name }
    let hex = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
    return hex.isEmpty ? "Unknown wallet" : "Wallet \(hex)…"
}

private func formatTimestamp(_ unix: UInt64) -> String {
    guard unix > 0 else { return "never" }
    let date = Date(timeIntervalSince1970: TimeInterval(unix))
    let formatter = RelativeDateTimeFormatter()
    formatter.unitsStyle = .abbreviated
    return formatter.localizedString(for: date, relativeTo: Date())
}

private func formatDuffs(_ duffs: UInt64) -> String {
    let dash = Double(duffs) / 100_000_000.0
    let f = NumberFormatter()
    f.numberStyle = .decimal
    f.minimumFractionDigits = 0
    f.maximumFractionDigits = 8
    return f.string(from: NSNumber(value: dash)).map { "\($0) DASH" }
        ?? String(format: "%.8f DASH", dash)
}

private func accountTypeName(typeTag: UInt8, standardTag: UInt8) -> String {
    switch typeTag {
    case 0: return standardTag == 0 ? "BIP44" : "BIP32"
    case 1: return "CoinJoin"
    case 2: return "IdentityRegistration"
    case 3: return "IdentityTopUp"
    case 4: return "IdentityTopUpUnbound"
    case 5: return "IdentityInvitation"
    case 6: return "AssetLockAddressTopUp"
    case 7: return "AssetLockShieldedTopUp"
    case 8: return "ProviderVotingKeys"
    case 9: return "ProviderOwnerKeys"
    case 10: return "ProviderOperatorKeys"
    case 11: return "ProviderPlatformKeys"
    case 12: return "DashpayReceiving"
    case 13: return "DashpayExternal"
    case 14: return "PlatformPayment"
    case 15: return "IdentityAuthECDSA"
    case 16: return "IdentityAuthBLS"
    default: return "Unknown(\(typeTag))"
    }
}

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

// MARK: - Top-level view

struct WalletMemoryExplorerView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager

    @State private var addressSyncRunning = false
    @State private var addressSyncing = false
    @State private var addressSyncLastUnix: UInt64 = 0
    @State private var identitySyncRunning = false
    @State private var identitySyncing = false
    @State private var identityTokenRows: [IdentityTokenSyncRow] = []
    @State private var loadError: String?

    var body: some View {
        List {
            spvSection
            addressSyncSection
            identityTokenSyncSection
            walletsSection
            if let loadError {
                Section {
                    Text(loadError)
                        .font(.caption)
                        .foregroundColor(.red)
                        .textSelection(.enabled)
                }
            }
        }
        .navigationTitle("Wallet Memory Explorer")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { loadManagerState() }
    }

    // MARK: - SPV Sync

    private var spvSection: some View {
        Section("SPV Sync") {
            let p = walletManager.spvProgress
            KVRow(label: "State", value: p.overallState.label)
            KVRow(label: "Progress", value: String(format: "%.1f%%", p.overallPercentage))
            if let h = p.headers {
                KVRow(label: "Headers", value: "\(h.currentHeight)/\(h.targetHeight)")
            }
            if let fh = p.filterHeaders {
                KVRow(label: "Filter Headers", value: "\(fh.currentHeight)/\(fh.targetHeight)")
            }
            if let f = p.filters {
                KVRow(label: "Filters", value: "\(f.currentHeight)/\(f.targetHeight)")
            }
            if let m = p.masternodes {
                KVRow(label: "Masternodes", value: "\(m.currentHeight)/\(m.targetHeight)")
            }
        }
    }

    // MARK: - Platform Address Sync

    private var addressSyncSection: some View {
        Section("Platform Address Sync") {
            KVRow(label: "Running", value: addressSyncRunning ? "yes" : "no")
            KVRow(label: "Syncing", value: addressSyncing ? "yes" : "no")
            KVRow(label: "Last Sync", value: formatTimestamp(addressSyncLastUnix))
            if let event = walletManager.lastPlatformAddressSyncEvent {
                KVRow(
                    label: "Last Event",
                    value: "\(event.walletResults.count) wallet(s)"
                )
            }
        }
    }

    // MARK: - Identity Token Sync

    private var identityTokenSyncSection: some View {
        Section {
            KVRow(label: "Running", value: identitySyncRunning ? "yes" : "no")
            KVRow(label: "Syncing", value: identitySyncing ? "yes" : "no")
            KVRow(label: "Cached Rows", value: "\(identityTokenRows.count)")
            if !identityTokenRows.isEmpty {
                let byIdentity = Dictionary(grouping: identityTokenRows, by: \.identityId)
                ForEach(Array(byIdentity.keys.sorted(by: { $0.lexicographicallyPrecedes($1) })), id: \.self) { identityId in
                    let rows = byIdentity[identityId] ?? []
                    DisclosureGroup {
                        ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                            VStack(alignment: .leading, spacing: 2) {
                                KVRow(label: "Token", value: shortBase58(row.tokenId))
                                KVRow(label: "Balance", value: "\(row.balance)")
                                KVRow(label: "Nonce", value: "\(row.identityContractNonce)")
                            }
                        }
                    } label: {
                        Text(shortBase58(identityId))
                            .font(.system(.caption, design: .monospaced))
                    }
                }
            }
        } header: {
            Text("Identity Token Sync")
        }
    }

    // MARK: - Wallets

    private var walletsSection: some View {
        Section {
            if walletManager.wallets.isEmpty {
                Text("No wallets loaded.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(sortedWalletIds, id: \.self) { walletId in
                    if let wallet = walletManager.wallets[walletId] {
                        NavigationLink {
                            WalletMemoryDetailView(
                                wallet: wallet,
                                walletId: walletId,
                                walletLabel: walletDisplayLabel(walletId, fromPersistent: nil)
                            )
                        } label: {
                            walletRow(walletId: walletId, wallet: wallet)
                        }
                    }
                }
            }
        } header: {
            Text("Wallets (\(walletManager.wallets.count))")
        }
    }

    private var sortedWalletIds: [Data] {
        walletManager.wallets.keys.sorted { $0.lexicographicallyPrecedes($1) }
    }

    @ViewBuilder
    private func walletRow(walletId: Data, wallet: ManagedPlatformWallet) -> some View {
        let summary = (try? wallet.inMemorySummary()) ?? InMemoryWalletSummary(
            identitiesCount: 0, watchedCount: 0, lastScannedIndex: 0,
            primaryIdentityId: nil, trackedAssetLocksCount: 0
        )
        let bal = try? wallet.balance()
        VStack(alignment: .leading, spacing: 4) {
            Text(walletDisplayLabel(walletId, fromPersistent: nil))
                .font(.headline)
            HStack(spacing: 4) {
                Text("\(summary.identitiesCount) identities")
                Text("·")
                Text("\(summary.trackedAssetLocksCount) asset locks")
                if let bal {
                    Text("·")
                    Text(formatDuffs(bal.total))
                }
            }
            .font(.caption)
            .foregroundColor(.secondary)
        }
        .padding(.vertical, 2)
    }

    // MARK: - Load

    private func loadManagerState() {
        loadError = nil
        var errors: [String] = []
        do {
            addressSyncRunning = try walletManager.isPlatformAddressSyncRunning()
        } catch {
            errors.append("Address sync running: \(error.localizedDescription)")
        }
        do {
            addressSyncing = try walletManager.isPlatformAddressSyncing()
        } catch {
            errors.append("Address syncing: \(error.localizedDescription)")
        }
        do {
            addressSyncLastUnix = try walletManager.lastPlatformAddressSyncUnixSeconds()
        } catch {
            errors.append("Address last sync: \(error.localizedDescription)")
        }
        do {
            identitySyncRunning = try walletManager.isIdentityTokenSyncRunning()
        } catch {
            errors.append("Identity sync running: \(error.localizedDescription)")
        }
        do {
            identitySyncing = try walletManager.isIdentityTokenSyncing()
        } catch {
            errors.append("Identity syncing: \(error.localizedDescription)")
        }
        do {
            identityTokenRows = try walletManager.allIdentityTokenSyncRows()
        } catch {
            errors.append("Token sync state: \(error.localizedDescription)")
        }
        if !errors.isEmpty {
            loadError = errors.joined(separator: "\n")
        }
    }
}

// MARK: - Per-wallet detail view

struct WalletMemoryDetailView: View {
    let wallet: ManagedPlatformWallet
    let walletId: Data
    let walletLabel: String
    @EnvironmentObject var walletManager: PlatformWalletManager

    @State private var summary: InMemoryWalletSummary?
    @State private var summaryError: String?
    @State private var walletBalance: ManagedPlatformWallet.WalletBalance?
    @State private var accountBalances: [PlatformWalletManager.AccountBalance] = []
    @State private var identityIds: [Identifier] = []
    @State private var watchedIds: [Identifier] = []
    @State private var idLabels: [Identifier: String] = [:]
    @State private var loadError: String?

    var body: some View {
        Form {
            walletInfoSection
            balanceSection
            accountBalancesSection
            summarySection
            identitiesSection
            watchedSection
            if let loadError {
                Section {
                    Text(loadError)
                        .font(.caption)
                        .foregroundColor(.red)
                        .textSelection(.enabled)
                }
            }
        }
        .navigationTitle(walletLabel)
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { loadOnce() }
    }

    // MARK: - Wallet Info

    private var walletInfoSection: some View {
        Section("Wallet") {
            KVRow(
                label: "ID",
                value: walletId.map { String(format: "%02x", $0) }.joined()
            )
        }
    }

    // MARK: - Balance

    private var balanceSection: some View {
        Section("Wallet Balance") {
            if let bal = walletBalance {
                KVRow(label: "Confirmed", value: formatDuffs(bal.spendable))
                KVRow(label: "Unconfirmed", value: formatDuffs(bal.unconfirmed))
                KVRow(label: "Immature", value: formatDuffs(bal.immature))
                KVRow(label: "Locked", value: formatDuffs(bal.locked))
                KVRow(label: "Total", value: formatDuffs(bal.total))
            } else {
                Text("Unavailable")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Account Balances

    private var accountBalancesSection: some View {
        Section {
            if accountBalances.isEmpty {
                Text("No accounts")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Array(accountBalances.enumerated()), id: \.offset) { _, acct in
                    let name = accountTypeName(
                        typeTag: acct.typeTag,
                        standardTag: acct.standardTag
                    )
                    let total = acct.confirmed + acct.unconfirmed
                        + acct.immature + acct.locked
                    DisclosureGroup {
                        KVRow(label: "Confirmed", value: formatDuffs(acct.confirmed))
                        KVRow(label: "Unconfirmed", value: formatDuffs(acct.unconfirmed))
                        KVRow(label: "Immature", value: formatDuffs(acct.immature))
                        KVRow(label: "Locked", value: formatDuffs(acct.locked))
                    } label: {
                        HStack {
                            Text("\(name) #\(acct.index)")
                                .font(.system(.body, design: .monospaced))
                            Spacer()
                            Text(formatDuffs(total))
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
        } header: {
            Text("Core Account Balances (\(accountBalances.count))")
        }
    }

    // MARK: - Summary

    private var summarySection: some View {
        Section("Identity Manager") {
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
            } else {
                HStack {
                    ProgressView().scaleEffect(0.8)
                    Text("Loading…").font(.caption).foregroundColor(.secondary)
                }
            }
        }
    }

    // MARK: - Identities

    private var identitiesSection: some View {
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
        }
    }

    // MARK: - Watched

    private var watchedSection: some View {
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
    }

    // MARK: - Helpers

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
        summaryError = nil
        loadError = nil
        var errors: [String] = []
        do {
            summary = try wallet.inMemorySummary()
        } catch {
            summaryError = "Summary failed: \(error.localizedDescription)"
        }
        walletBalance = try? wallet.balance()
        accountBalances = walletManager.accountBalances(for: walletId)
        do {
            identityIds = try wallet.inMemoryIdentityIds()
        } catch {
            errors.append("Identity list failed: \(error.localizedDescription)")
            identityIds = []
        }
        do {
            watchedIds = try wallet.inMemoryWatchedIdentityIds()
        } catch {
            errors.append("Watched list failed: \(error.localizedDescription)")
            watchedIds = []
        }
        for id in identityIds {
            if let mi = try? wallet.managedIdentity(identityId: id),
               let label = try? mi.getLabel(), !label.isEmpty {
                idLabels[id] = label
            }
        }
        if !errors.isEmpty {
            loadError = errors.joined(separator: "\n")
        }
    }
}

// MARK: - Per-identity detail view

struct WalletMemoryIdentityDetailView: View {
    let wallet: ManagedPlatformWallet
    let identityId: Identifier
    @EnvironmentObject var walletManager: PlatformWalletManager

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

    @State private var tokenSyncSnapshot: IdentityTokenSyncSnapshot?

    var body: some View {
        Form {
            identitySection
            if loaded {
                publicKeysSection
                tokenSyncSection
                sentRequestsSection
                incomingRequestsSection
                contactsSection
                dpnsSection
                contestedDpnsSection
                dashpayProfileSection
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

    // MARK: - Identity

    private var identitySection: some View {
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
    }

    // MARK: - Public Keys

    private var publicKeysSection: some View {
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
    }

    // MARK: - Token Sync State

    private var tokenSyncSection: some View {
        Section {
            if let snapshot = tokenSyncSnapshot {
                KVRow(
                    label: "Last Sync",
                    value: formatTimestamp(snapshot.lastSyncUnixSeconds)
                )
                if snapshot.rows.isEmpty {
                    Text("No tokens tracked")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(Array(snapshot.rows.enumerated()), id: \.offset) { _, row in
                        VStack(alignment: .leading, spacing: 2) {
                            KVRow(label: "Token", value: shortBase58(row.tokenId))
                            KVRow(label: "Balance", value: "\(row.balance)")
                            KVRow(label: "Nonce", value: "\(row.identityContractNonce)")
                        }
                    }
                }
            } else {
                Text("No sync state cached")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } header: {
            let count = tokenSyncSnapshot?.rows.count ?? 0
            sectionHeader("Token Sync State", count: count)
        }
    }

    // MARK: - Contacts / Requests

    private var sentRequestsSection: some View {
        Section {
            if sentRequestIds.isEmpty {
                Text("None").font(.caption).foregroundColor(.secondary)
            } else {
                ForEach(sentRequestIds, id: \.self) { id in idRow(id) }
            }
        } header: {
            sectionHeader("Sent Contact Requests", count: sentRequestIds.count)
        }
    }

    private var incomingRequestsSection: some View {
        Section {
            if incomingRequestIds.isEmpty {
                Text("None").font(.caption).foregroundColor(.secondary)
            } else {
                ForEach(incomingRequestIds, id: \.self) { id in idRow(id) }
            }
        } header: {
            sectionHeader("Incoming Contact Requests", count: incomingRequestIds.count)
        }
    }

    private var contactsSection: some View {
        Section {
            if establishedContactIds.isEmpty {
                Text("None").font(.caption).foregroundColor(.secondary)
            } else {
                ForEach(establishedContactIds, id: \.self) { id in idRow(id) }
            }
        } header: {
            sectionHeader("Established Contacts", count: establishedContactIds.count)
        }
    }

    // MARK: - DPNS

    private var dpnsSection: some View {
        Section {
            if dpnsNames.isEmpty {
                Text("None").font(.caption).foregroundColor(.secondary)
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
    }

    private var contestedDpnsSection: some View {
        Section {
            if contestedDpnsNames.isEmpty {
                Text("None").font(.caption).foregroundColor(.secondary)
            } else {
                ForEach(contestedDpnsNames, id: \.self) { name in
                    Text(name)
                        .font(.system(.body, design: .monospaced))
                        .textSelection(.enabled)
                }
            }
        } header: {
            sectionHeader("Contested DPNS Names", count: contestedDpnsNames.count)
        }
    }

    // MARK: - DashPay Profile

    private var dashpayProfileSection: some View {
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
                Text("—")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Helpers

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
        tokenSyncSnapshot = try? walletManager.identityTokenSyncState(for: identityId)
        if !errors.isEmpty {
            loadError = errors.joined(separator: "\n")
        }
        loaded = true
    }
}

private extension PlatformSpvSyncState {
    var label: String {
        switch self {
        case .waitForEvents: return "Waiting"
        case .waitingForConnections: return "Connecting"
        case .syncing: return "Syncing"
        case .synced: return "Synced"
        case .error: return "Error"
        @unknown default: return "Unknown"
        }
    }
}
