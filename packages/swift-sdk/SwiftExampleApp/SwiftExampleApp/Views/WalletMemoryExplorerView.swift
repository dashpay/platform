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
import SwiftData

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

/// Coarse classification of which underlying Rust variant carries the
/// account: `ManagedCoreFundsAccount`, `ManagedCoreKeysAccount`, or the
/// separate `ManagedPlatformAccount`. Drives the row badge so the
/// natural emptiness of keys-only rows (no balance, no UTXOs) reads as
/// intentional rather than a missing-data bug.
///
/// Mapping mirrors the post-split account-collection layout in
/// `key-wallet/src/managed_account/managed_account_collection.rs`:
/// Standard BIP44/BIP32, CoinJoin, and DashPay receive/external sit in
/// the funds variant; identity / asset-lock / provider account slots
/// were promoted to the keys variant; PlatformPayment is its own type.
private enum AccountVariantKind {
    case funds
    case keys
    case platform

    var label: String {
        switch self {
        case .funds: return "Funds"
        case .keys: return "Keys"
        case .platform: return "Platform"
        }
    }

    var color: Color {
        switch self {
        case .funds: return .green
        case .keys: return .blue
        case .platform: return .purple
        }
    }
}

private func accountVariantKind(typeTag: UInt8) -> AccountVariantKind {
    switch typeTag {
    // 0 Standard, 1 CoinJoin, 12 DashpayReceiving, 13 DashpayExternal
    case 0, 1, 12, 13: return .funds
    // 14 PlatformPayment lives on `ManagedPlatformAccount`, a distinct
    // type from the core funds/keys split.
    case 14: return .platform
    // Everything else (identity registration / topup / invitation /
    // asset-lock / provider keys / identity auth) is keys-only — no
    // UTXOs, no balance, by construction.
    default: return .keys
    }
}

/// Capsule badge rendered alongside the account row label. Color
/// coding matches `AccountVariantKind.color`.
private struct AccountVariantBadge: View {
    let kind: AccountVariantKind

    var body: some View {
        Text(kind.label)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
            .background(kind.color.opacity(0.18))
            .foregroundColor(kind.color)
            .clipShape(Capsule())
    }
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
    /// Optional override for the value text color. Used by the
    /// SwiftData-counterpart diagnostic to paint mismatch rows red
    /// (or orange for "linked weakly") so reviewers can scan a long
    /// list without expanding every entry. Defaults to the system
    /// foreground color when nil so prior call sites stay unchanged.
    var valueColor: Color? = nil

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value)
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .font(.system(.body, design: .monospaced))
                .foregroundColor(valueColor)
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
    @State private var atomicWalletIds: [Data] = []
    @State private var addressSyncConfig: PlatformWalletManager.PlatformAddressSyncConfigSnapshot?
    @State private var identitySyncConfig: PlatformWalletManager.IdentitySyncConfigSnapshot?
    @State private var loadError: String?

    var body: some View {
        List {
            spvSection
            addressSyncSection
            identityTokenSyncSection
            managerLevelSection
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
            // `overallPercentage` is a 0.0–1.0 fraction (the same value
            // ContentView feeds straight into `ProgressView(value:)`),
            // so multiply by 100 before formatting as a percent.
            KVRow(
                label: "Progress",
                value: String(format: "%.1f%%", p.overallPercentage * 100)
            )
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

    // MARK: - Manager-level diagnostics

    private var managerLevelSection: some View {
        Section {
            DisclosureGroup("Atomic Wallet IDs (\(atomicWalletIds.count))") {
                if atomicWalletIds.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(atomicWalletIds, id: \.self) { wid in
                        Text(wid.map { String(format: "%02x", $0) }.joined())
                            .font(.caption2.monospaced())
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                    }
                }
            }
            DisclosureGroup("PlatformAddressSyncManager Config") {
                if let cfg = addressSyncConfig {
                    KVRow(label: "Interval (s)", value: "\(cfg.intervalSeconds)")
                    KVRow(label: "Watch List Size", value: "\(cfg.watchListSize)")
                    KVRow(
                        label: "Last Event",
                        value: formatTimestamp(cfg.lastEventUnixSeconds)
                    )
                } else {
                    Text("Unavailable")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            DisclosureGroup("IdentitySyncManager Config") {
                if let cfg = identitySyncConfig {
                    KVRow(label: "Interval (s)", value: "\(cfg.intervalSeconds)")
                    KVRow(label: "Queue Depth", value: "\(cfg.queueDepth)")
                } else {
                    Text("Unavailable")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        } header: {
            Text("Manager State")
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
        atomicWalletIds = walletManager.listWalletIdsAtomic()
        addressSyncConfig = walletManager.platformAddressSyncConfigSnapshot()
        identitySyncConfig = walletManager.identitySyncConfigSnapshot()
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

    // Diagnostic sections (Phases 3, 4, 7).
    @State private var coreState: PlatformWalletManager.CoreWalletStateSnapshot?
    @State private var identityWalletState: PlatformWalletManager.IdentityWalletStateSnapshot?
    @State private var providerState: PlatformWalletManager.PlatformAddressProviderStateSnapshot?
    @State private var trackedAssetLocks: [PlatformWalletManager.TrackedAssetLockSnapshot] = []
    @State private var instantSendLocks: [Data] = []
    @State private var outOfWalletIds: [Data] = []
    @State private var walletIdentityRows: [PlatformWalletManager.WalletIdentityRow] = []

    var body: some View {
        Form {
            walletInfoSection
            // PlatformWalletInfo metadata block (name / description /
            // birth+synced+last-processed heights / total transactions /
            // first loaded at) was removed: every meaningful field
            // either duplicates `Core Wallet State` or reads "0/never"
            // because nothing populates it (total_transactions is
            // event-driven, first_loaded_at isn't stamped on this
            // path). The Rust accessor + FFI wrapper are gone too.
            coreStateSection
            identityWalletStateSection
            platformAddressProviderSection
            balanceSection
            fundsAccountBalancesSection
            keysAccountBalancesSection
            summarySection
            identityManagerSection
            trackedAssetLocksSection
            instantSendLocksSection
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

    // MARK: - Core wallet state

    private var coreStateSection: some View {
        Section("Core Wallet State") {
            if let s = coreState {
                KVRow(label: "Synced Height", value: "\(s.syncedHeight)")
                KVRow(label: "Last Processed", value: "\(s.lastProcessedHeight)")
                KVRow(label: "Monitor Revision (max)", value: "\(s.monitorRevision)")
            } else {
                Text("Unavailable")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Identity wallet scan state

    private var identityWalletStateSection: some View {
        Section("Identity Wallet Scan State") {
            if let s = identityWalletState {
                KVRow(label: "Last Scanned Index", value: "\(s.lastScannedIndex)")
                KVRow(label: "Scan Pending", value: s.scanPending ? "yes" : "no")
            } else {
                Text("Unavailable")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Platform Address Provider state

    private var platformAddressProviderSection: some View {
        Section("Platform Address Provider") {
            if let s = providerState {
                KVRow(label: "Initialized", value: s.initialized ? "yes" : "no")
                KVRow(label: "Accounts Watched", value: "\(s.accountsWatched)")
                KVRow(label: "Found Count", value: "\(s.foundCount)")
                KVRow(label: "Known Balances", value: "\(s.knownBalancesCount)")
                KVRow(label: "Watermark Height", value: "\(s.watermarkHeight)")
            } else {
                Text("Unavailable")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
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
    //
    // Funds and Keys variants are split into separate sections so the
    // headline number on each row reads correctly: balance for funds
    // (real money, summable), keys-used for keys (no balance by
    // construction — the Rust-side `ManagedCoreKeysAccount` doesn't
    // carry UTXOs). Platform-payment accounts (the third variant on
    // `ManagedAccountCollection`) ride along on the funds section
    // because they DO carry balance, just under a different in-memory
    // type.

    /// Funds + Platform-payment accounts; rendered with the C/U/I/L
    /// balance breakdown.
    private var fundsAccountBalancesSection: some View {
        let rows = accountBalances.filter {
            accountVariantKind(typeTag: $0.typeTag) != .keys
        }
        return Section {
            if rows.isEmpty {
                Text("None")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Array(rows.enumerated()), id: \.offset) { _, acct in
                    NavigationLink {
                        AccountDrillDownView(walletId: walletId, balance: acct)
                    } label: {
                        fundsAccountRow(acct: acct)
                    }
                }
            }
        } header: {
            Text("Core Funds Accounts (\(rows.count))")
        }
    }

    /// Keys-only accounts (identity / asset-lock / provider). The
    /// headline number is `keysUsed / keysTotal` rather than balance —
    /// these accounts derive special-purpose keys and never carry
    /// UTXOs.
    private var keysAccountBalancesSection: some View {
        let rows = accountBalances.filter {
            accountVariantKind(typeTag: $0.typeTag) == .keys
        }
        return Section {
            if rows.isEmpty {
                Text("None")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Array(rows.enumerated()), id: \.offset) { _, acct in
                    NavigationLink {
                        AccountDrillDownView(walletId: walletId, balance: acct)
                    } label: {
                        keysAccountRow(acct: acct)
                    }
                }
            }
        } header: {
            Text("Core Keys Accounts (\(rows.count))")
        }
    }

    @ViewBuilder
    private func fundsAccountRow(
        acct: PlatformWalletManager.AccountBalance
    ) -> some View {
        let name = accountTypeName(
            typeTag: acct.typeTag,
            standardTag: acct.standardTag
        )
        let kind = accountVariantKind(typeTag: acct.typeTag)
        let total = acct.confirmed + acct.unconfirmed + acct.immature + acct.locked
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 6) {
                Text("\(name) #\(acct.index)")
                    .font(.system(.body, design: .monospaced))
                AccountVariantBadge(kind: kind)
                Spacer()
                Text(formatDuffs(total))
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            HStack(spacing: 6) {
                Text("C: \(formatDuffs(acct.confirmed))")
                Text("·")
                Text("U: \(formatDuffs(acct.unconfirmed))")
                Text("·")
                Text("I: \(formatDuffs(acct.immature))")
                Text("·")
                Text("L: \(formatDuffs(acct.locked))")
            }
            .font(.caption2.monospaced())
            .foregroundColor(.secondary)
        }
    }

    @ViewBuilder
    private func keysAccountRow(
        acct: PlatformWalletManager.AccountBalance
    ) -> some View {
        let name = accountTypeName(
            typeTag: acct.typeTag,
            standardTag: acct.standardTag
        )
        // Keys variants always badge as `Keys`; pinning the kind here
        // avoids re-classifying inside the row.
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 6) {
                Text("\(name) #\(acct.index)")
                    .font(.system(.body, design: .monospaced))
                AccountVariantBadge(kind: .keys)
                Spacer()
                Text("\(acct.keysUsed) / \(acct.keysTotal) keys")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Tracked asset locks

    private var trackedAssetLocksSection: some View {
        Section {
            if trackedAssetLocks.isEmpty {
                Text("None")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Array(trackedAssetLocks.enumerated()), id: \.offset) { _, lock in
                    DisclosureGroup {
                        KVRow(
                            label: "Outpoint",
                            value: lock.outpointTxid.prefix(8).map {
                                String(format: "%02x", $0)
                            }.joined() + ":" + "\(lock.outpointVout)"
                        )
                        KVRow(label: "Lock Type", value: trackedAssetLockTypeLabel(lock.lockType))
                        KVRow(label: "Status", value: trackedAssetLockStatusLabel(lock.status))
                        KVRow(label: "Reg Index", value: "\(lock.registrationIndex)")
                        KVRow(label: "InstantLock", value: lock.instantLockPresent ? "yes" : "no")
                        KVRow(label: "ChainLock Height", value: "\(lock.chainLockHeight)")
                    } label: {
                        Text("Lock #\(trackedAssetLocks.firstIndex(where: { $0.outpointTxid == lock.outpointTxid && $0.outpointVout == lock.outpointVout }) ?? 0)")
                            .font(.system(.body, design: .monospaced))
                    }
                }
            }
        } header: {
            Text("Tracked Asset Locks (\(trackedAssetLocks.count))")
        }
    }

    // MARK: - InstantSend lock txids

    private var instantSendLocksSection: some View {
        Section {
            if instantSendLocks.isEmpty {
                Text("None")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(instantSendLocks, id: \.self) { txid in
                    Text(txid.map { String(format: "%02x", $0) }.joined())
                        .font(.caption2.monospaced())
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                }
            }
        } header: {
            Text("InstantSend Locks (\(instantSendLocks.count))")
        }
    }

    // MARK: - Identity Manager structure

    private var identityManagerSection: some View {
        Section {
            DisclosureGroup("Wallet Identities (\(walletIdentityRows.count))") {
                if walletIdentityRows.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(Array(walletIdentityRows.enumerated()), id: \.offset) { _, row in
                        HStack {
                            Text("#\(row.registrationIndex)")
                                .font(.system(.body, design: .monospaced))
                            Spacer()
                            Text(row.identityId.map { String(format: "%02x", $0) }.joined())
                                .font(.caption2.monospaced())
                                .lineLimit(1)
                                .truncationMode(.middle)
                                .textSelection(.enabled)
                        }
                    }
                }
            }
            DisclosureGroup("Out-of-Wallet Identities (\(outOfWalletIds.count))") {
                if outOfWalletIds.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(outOfWalletIds, id: \.self) { id in
                        Text(id.map { String(format: "%02x", $0) }.joined())
                            .font(.caption2.monospaced())
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                    }
                }
            }
        } header: {
            Text("Identity Manager Structure")
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
        coreState = walletManager.coreWalletState(for: walletId)
        identityWalletState = walletManager.identityWalletState(for: walletId)
        providerState = walletManager.platformAddressProviderState(for: walletId)
        trackedAssetLocks = walletManager.trackedAssetLocks(for: walletId)
        instantSendLocks = walletManager.instantSendLockTxids(for: walletId)
        outOfWalletIds = walletManager.identityManagerOutOfWalletIds(for: walletId)
        walletIdentityRows = walletManager.identityManagerWalletIdentities(for: walletId)
        if !errors.isEmpty {
            loadError = errors.joined(separator: "\n")
        }
    }
}

// MARK: - Per-account drill-down view

struct AccountDrillDownView: View {
    let walletId: Data
    let balance: PlatformWalletManager.AccountBalance
    @EnvironmentObject var walletManager: PlatformWalletManager
    /// SwiftData context — used to cross-check every in-memory UTXO
    /// against its persisted `PersistentTxo` counterpart and surface
    /// the side-by-side diff in the explorer. Mismatches point at
    /// real bugs (orphan rows from incomplete cascades, lingering
    /// `isSpent == false` rows that the wallet has already evicted,
    /// out-of-sync amount / height fields, etc.).
    @Environment(\.modelContext) private var modelContext

    @State private var metadata: PlatformWalletManager.AccountMetadataSnapshot?
    @State private var pools: [PlatformWalletManager.AccountAddressPool] = []
    @State private var utxos: [PlatformWalletManager.AccountUtxo] = []
    /// Snapshot of `PersistentTxo` rows for this wallet+account that
    /// SwiftData reports as unspent. Keyed by 36-byte outpoint
    /// (`PersistentTxo.makeOutpoint`) so each in-memory UTXO can do
    /// an O(1) lookup. Refreshed alongside `utxos` in `load()`.
    @State private var persistedTxosByOutpoint: [Data: TxoSnapshot] = [:]
    /// Persisted rows the wallet doesn't currently claim — i.e.,
    /// `PersistentTxo` rows for this wallet+account where
    /// `isSpent == false` but the outpoint isn't in the in-memory
    /// UTXO set. The orphan signature for the persistence /
    /// cascade-delete bug surfaced during the run-1 → fresh-load
    /// regression diagnosis.
    @State private var orphanPersistedTxos: [TxoSnapshot] = []

    /// Whether this account is the keys-only variant — drives whether
    /// UTXO-related surfaces are shown. UTXOs are exclusive to the
    /// `ManagedCoreFundsAccount` Rust variant; keys-only accounts
    /// (identity / asset-lock / provider) never carry them.
    private var isKeysAccount: Bool {
        accountVariantKind(typeTag: balance.typeTag) == .keys
    }

    var body: some View {
        Form {
            // Balance + UTXOs both live on the funds variant only.
            // Suppress them on keys-only accounts so the drill-down
            // doesn't render five zero rows that look like missing
            // data rather than "by design".
            if !isKeysAccount {
                balanceHeaderSection
            }
            metadataSection
            addressPoolsSection
            if !isKeysAccount {
                utxosSection
                orphanPersistedSection
            }
            // Per-account in-memory transaction list intentionally
            // omitted: `keep_txs_in_memory` is off and tx history is
            // delivered through the event channel rather than stored
            // on `ManagedCoreFundsAccount.transactions`. The Rust-side
            // `account_transactions_blocking` accessor and its FFI /
            // Swift wrapper still exist (return empty by design) for
            // builds that flip the feature on.
        }
        .navigationTitle(
            accountTypeName(typeTag: balance.typeTag, standardTag: balance.standardTag)
            + " #\(balance.index)"
        )
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { load() }
    }

    private var balanceHeaderSection: some View {
        Section("Balance") {
            KVRow(label: "Confirmed", value: formatDuffs(balance.confirmed))
            KVRow(label: "Unconfirmed", value: formatDuffs(balance.unconfirmed))
            KVRow(label: "Immature", value: formatDuffs(balance.immature))
            KVRow(label: "Locked", value: formatDuffs(balance.locked))
            KVRow(
                label: "Total",
                value: formatDuffs(
                    balance.confirmed + balance.unconfirmed
                    + balance.immature + balance.locked
                )
            )
        }
    }

    private var metadataSection: some View {
        Section("Account Metadata") {
            if let m = metadata {
                // `totalTransactions` is intentionally not surfaced —
                // it counts the in-memory transaction map, which is
                // empty by design when `keep_txs_in_memory` is off.
                // "Watch Only" and "Custom Name" rows were dropped in
                // lockstep with upstream removing those fields from
                // the underlying `ManagedCore*Account` variants —
                // watch-only is wallet-level now, custom names are
                // gone entirely.
                if !isKeysAccount {
                    // Hide "Total UTXOs" on keys-only accounts: they
                    // never carry UTXOs, so the row would always read
                    // 0 and add noise.
                    KVRow(label: "Total UTXOs", value: "\(m.totalUtxos)")
                }
                KVRow(label: "Monitor Revision", value: "\(m.monitorRevision)")
            } else {
                Text("Unavailable")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    private var addressPoolsSection: some View {
        Section {
            if pools.isEmpty {
                Text("No pools")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Array(pools.enumerated()), id: \.offset) { idx, pool in
                    DisclosureGroup {
                        KVRow(label: "Gap Limit", value: "\(pool.gapLimit)")
                        KVRow(
                            label: "Last Used Index",
                            value: pool.lastUsedIndex < 0
                                ? "—"
                                : "\(pool.lastUsedIndex)"
                        )
                        KVRow(label: "Address Count", value: "\(pool.addresses.count)")
                        ForEach(Array(pool.addresses.enumerated()), id: \.offset) { _, info in
                            VStack(alignment: .leading, spacing: 2) {
                                HStack {
                                    Text("idx \(info.addressIndex)")
                                        .font(.caption2.monospaced())
                                        .foregroundColor(.secondary)
                                    Spacer()
                                    Text(info.isUsed ? "used" : "unused")
                                        .font(.caption2)
                                        .foregroundColor(info.isUsed ? .accentColor : .secondary)
                                }
                                // Encoded address — the prominent line
                                // for the row.
                                Text(info.address.isEmpty ? "—" : info.address)
                                    .font(.system(.caption, design: .monospaced))
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                    .textSelection(.enabled)
                                // Public-key bytes (hex). Empty when
                                // the pool didn't retain the
                                // derivation source — falls back to
                                // the 20-byte pubkey-hash so the row
                                // always carries some cryptographic
                                // identity for the user.
                                let pkHex = (info.publicKeyBytes.isEmpty
                                    ? info.pubkeyHash
                                    : info.publicKeyBytes
                                ).map { String(format: "%02x", $0) }.joined()
                                let pkLabel = info.publicKeyBytes.isEmpty
                                    ? "hash160: \(pkHex)"
                                    : "pubkey: \(pkHex)"
                                Text(pkLabel)
                                    .font(.caption2.monospaced())
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                    .foregroundColor(.secondary)
                                    .textSelection(.enabled)
                            }
                        }
                    } label: {
                        Text("Pool \(idx) (\(addressPoolTypeLabel(pool.poolType)))")
                            .font(.system(.body, design: .monospaced))
                    }
                }
            }
        } header: {
            Text("Address Pools (\(pools.count))")
        }
    }

    private var utxosSection: some View {
        Section {
            if utxos.isEmpty {
                Text("None")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Array(utxos.enumerated()), id: \.offset) { _, u in
                    DisclosureGroup {
                        // In-memory side (Rust-owned, what the wallet
                        // currently believes about this UTXO).
                        Text("In-memory (Rust)")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        KVRow(label: "Value", value: formatDuffs(u.valueDuffs))
                        KVRow(label: "Height", value: "\(u.height)")
                        KVRow(label: "Locked", value: u.isLocked ? "yes" : "no")
                        KVRow(label: "Script Len", value: "\(u.scriptPubkey.count)")

                        Divider()

                        // SwiftData side. The persistence handler
                        // upserts a `PersistentTxo` for each emit; if
                        // the row is missing here the in-memory wallet
                        // is ahead of disk (recent receive that
                        // hasn't flushed yet). If the row is present
                        // but flagged `isSpent`, that's a real
                        // disagreement worth investigating.
                        Text("SwiftData (PersistentTxo)")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        let outpointKey = PersistentTxo.makeOutpoint(
                            txid: u.outpointTxid,
                            vout: u.outpointVout
                        )
                        if let p = persistedTxosByOutpoint[outpointKey] {
                            KVRow(
                                label: "Amount",
                                value: formatDuffs(p.amount),
                                valueColor: p.amount == u.valueDuffs ? nil : .red
                            )
                            KVRow(
                                label: "Height",
                                value: "\(p.height)",
                                valueColor: p.height == u.height ? nil : .red
                            )
                            KVRow(
                                label: "isSpent",
                                value: p.isSpent ? "yes (DISAGREE)" : "no",
                                valueColor: p.isSpent ? .red : nil
                            )
                            KVRow(label: "isConfirmed", value: p.isConfirmed ? "yes" : "no")
                            KVRow(label: "isCoinbase", value: p.isCoinbase ? "yes" : "no")
                            KVRow(label: "isInstantLocked", value: p.isInstantLocked ? "yes" : "no")
                            KVRow(label: "isLocked", value: p.isLocked ? "yes" : "no")
                            KVRow(
                                label: "Address",
                                value: p.address.isEmpty ? "(none)" : p.address
                            )
                            KVRow(
                                label: "wallet match",
                                value: p.walletIdMatches ? "yes" : "no",
                                valueColor: p.walletIdMatches ? nil : .red
                            )
                            KVRow(
                                label: "account linked",
                                value: p.hasAccountLink ? "yes" : "no",
                                valueColor: p.hasAccountLink ? nil : .orange
                            )
                            KVRow(
                                label: "coreAddress linked",
                                value: p.hasCoreAddressLink ? "yes" : "no",
                                valueColor: p.hasCoreAddressLink ? nil : .orange
                            )
                        } else {
                            Text("Not in SwiftData (in-memory ahead of disk, or never persisted)")
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    } label: {
                        let outpointKey = PersistentTxo.makeOutpoint(
                            txid: u.outpointTxid,
                            vout: u.outpointVout
                        )
                        let mismatchKind: String? = persistedTxosByOutpoint[outpointKey]
                            .map { snap in mismatchSummary(persisted: snap, against: u) }
                        VStack(alignment: .leading, spacing: 1) {
                            HStack(spacing: 6) {
                                Text(
                                    u.outpointTxid.map { String(format: "%02x", $0) }.joined()
                                    + ":" + "\(u.outpointVout)"
                                )
                                .font(.caption2.monospaced())
                                .lineLimit(1)
                                .truncationMode(.middle)
                                if persistedTxosByOutpoint[outpointKey] == nil {
                                    Text("⚠︎ no row")
                                        .font(.caption2)
                                        .foregroundColor(.red)
                                } else if let kind = mismatchKind, !kind.isEmpty {
                                    Text("⚠︎ \(kind)")
                                        .font(.caption2)
                                        .foregroundColor(.red)
                                }
                            }
                            Text(formatDuffs(u.valueDuffs))
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
        } header: {
            Text("UTXOs (\(utxos.count))")
        }
    }

    /// SwiftData rows the in-memory wallet doesn't claim. Surfaces
    /// the cascade / spent-flag / orphan-row class of bugs where
    /// `load_from_persistor` would over-restore on next launch.
    @ViewBuilder
    private var orphanPersistedSection: some View {
        if !orphanPersistedTxos.isEmpty {
            Section {
                Text(
                    "These PersistentTxo rows are unspent on disk but "
                    + "the in-memory wallet doesn't list them. They "
                    + "would surface on the next load_from_persistor "
                    + "and inflate the restored balance."
                )
                .font(.caption2)
                .foregroundColor(.secondary)
                ForEach(Array(orphanPersistedTxos.enumerated()), id: \.offset) { _, p in
                    DisclosureGroup {
                        KVRow(label: "Amount", value: formatDuffs(p.amount))
                        KVRow(label: "Height", value: "\(p.height)")
                        KVRow(label: "isConfirmed", value: p.isConfirmed ? "yes" : "no")
                        KVRow(label: "isCoinbase", value: p.isCoinbase ? "yes" : "no")
                        KVRow(label: "isInstantLocked", value: p.isInstantLocked ? "yes" : "no")
                        KVRow(label: "Address", value: p.address.isEmpty ? "(none)" : p.address)
                        KVRow(
                            label: "wallet match",
                            value: p.walletIdMatches ? "yes" : "no",
                            valueColor: p.walletIdMatches ? nil : .red
                        )
                        KVRow(
                            label: "account linked",
                            value: p.hasAccountLink ? "yes" : "no",
                            valueColor: p.hasAccountLink ? nil : .orange
                        )
                        KVRow(
                            label: "coreAddress linked",
                            value: p.hasCoreAddressLink ? "yes" : "no",
                            valueColor: p.hasCoreAddressLink ? nil : .orange
                        )
                    } label: {
                        VStack(alignment: .leading, spacing: 1) {
                            Text(p.outpointHex)
                                .font(.caption2.monospaced())
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Text(formatDuffs(p.amount))
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }
                }
            } header: {
                Text("Orphan persisted UTXOs (\(orphanPersistedTxos.count))")
            }
        }
    }

    /// Compose a one-word summary of the most-prominent disagreement
    /// between a `TxoSnapshot` and the in-memory `AccountUtxo`. The
    /// label badge shows this so reviewers can scan a long list
    /// without expanding every row. Returns an empty string when
    /// every checked field agrees.
    private func mismatchSummary(
        persisted p: TxoSnapshot,
        against m: PlatformWalletManager.AccountUtxo
    ) -> String {
        if p.isSpent { return "spent on disk" }
        if !p.walletIdMatches { return "wallet id mismatch" }
        if p.amount != m.valueDuffs { return "amount mismatch" }
        if p.height != m.height { return "height mismatch" }
        if !p.hasAccountLink { return "no account link" }
        if !p.hasCoreAddressLink { return "no coreAddress link" }
        return ""
    }

    private func load() {
        metadata = walletManager.accountMetadata(for: walletId, balance: balance)
        pools = walletManager.accountAddressPools(for: walletId, balance: balance)
        utxos = walletManager.accountUtxos(for: walletId, balance: balance)
        // Tx history is event-driven and not held in memory; skip the
        // accessor here — see the comment on the body's omitted
        // `transactionsSection`.

        // Refresh the SwiftData side. Fetch every unspent
        // `PersistentTxo` for this wallet, then narrow to rows
        // routed to this account by tag tuple — `PersistentTxo`
        // links to `PersistentAccount` directly (line 85 in the
        // model), so we filter on `account.accountType /
        // accountIndex / standardTag / registrationIndex / keyClass`
        // in Swift (SwiftData `#Predicate` doesn't traverse
        // `account?.…` nicely). Result is keyed by 36-byte outpoint
        // for the per-row comparison loop.
        let walletIdLocal = walletId
        let typeTag = balance.typeTag
        let standardTag = balance.standardTag
        let accountIdx = balance.index
        let regIdx = balance.registrationIndex
        let keyClass = balance.keyClass
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { txo in
                txo.walletId == walletIdLocal && txo.isSpent == false
            }
        )
        let rows = (try? modelContext.fetch(descriptor)) ?? []
        var byOutpoint: [Data: TxoSnapshot] = [:]
        var orphanCandidates: [TxoSnapshot] = []
        let inMemoryOutpoints: Set<Data> = Set(utxos.map { u in
            PersistentTxo.makeOutpoint(txid: u.outpointTxid, vout: u.outpointVout)
        })
        for row in rows {
            guard let acc = row.account else {
                // No account link — definitely orphan, surface in
                // the orphan section regardless of tag matching.
                let snap = TxoSnapshot(from: row, expectedWalletId: walletIdLocal)
                orphanCandidates.append(snap)
                continue
            }
            // Filter on the same account tag tuple the manager uses
            // when routing in-memory UTXOs into accounts. A row that
            // doesn't match this account — even if its walletId
            // matches — belongs to a sibling account in this view's
            // sibling drill-downs.
            let matchesThisAccount =
                UInt8(exactly: acc.accountType) == typeTag
                && acc.standardTag == standardTag
                && acc.accountIndex == accountIdx
                && acc.registrationIndex == regIdx
                && acc.keyClass == keyClass
            guard matchesThisAccount else { continue }
            let snap = TxoSnapshot(from: row, expectedWalletId: walletIdLocal)
            byOutpoint[row.outpoint] = snap
            if !inMemoryOutpoints.contains(row.outpoint) {
                orphanCandidates.append(snap)
            }
        }
        persistedTxosByOutpoint = byOutpoint
        orphanPersistedTxos = orphanCandidates
    }
}

/// Plain-Swift snapshot of the `PersistentTxo` fields the explorer
/// reads. Decouples the view from the SwiftData @Model so we don't
/// hand a managed object across `@State` (which fights with
/// SwiftUI's value-semantics expectations) and so the comparison
/// helpers don't have to walk the relationship graph mid-render.
private struct TxoSnapshot: Equatable {
    let outpoint: Data
    let outpointHex: String
    let amount: UInt64
    let height: UInt32
    let isConfirmed: Bool
    let isCoinbase: Bool
    let isInstantLocked: Bool
    let isLocked: Bool
    let isSpent: Bool
    let address: String
    let walletIdMatches: Bool
    let hasAccountLink: Bool
    let hasCoreAddressLink: Bool

    init(from row: PersistentTxo, expectedWalletId: Data) {
        self.outpoint = row.outpoint
        self.outpointHex = row.outpointHex
        self.amount = row.amount
        self.height = row.height
        self.isConfirmed = row.isConfirmed
        self.isCoinbase = row.isCoinbase
        self.isInstantLocked = row.isInstantLocked
        self.isLocked = row.isLocked
        self.isSpent = row.isSpent
        self.address = row.address
        self.walletIdMatches = row.walletId == expectedWalletId
        self.hasAccountLink = row.account != nil
        self.hasCoreAddressLink = row.coreAddress != nil
    }
}

// MARK: - Helper labels

private func addressPoolTypeLabel(_ tag: UInt8) -> String {
    // Mirrors `PersistentCoreAddress.poolTypeName` — tags 2/3 are the
    // on-demand "Additional" pools (provider keys etc.), not Rust's
    // internal "Absent" naming.
    switch tag {
    case 0: return "External"
    case 1: return "Internal"
    case 2: return "Additional"
    case 3: return "Additional (Hardened)"
    default: return "Unknown(\(tag))"
    }
}

private func trackedAssetLockTypeLabel(_ tag: UInt8) -> String {
    switch tag {
    case 0: return "IdentityRegistration"
    case 1: return "IdentityTopUp"
    case 2: return "IdentityTopUpNotBound"
    case 3: return "IdentityInvitation"
    case 4: return "AssetLockAddressTopUp"
    case 5: return "AssetLockShieldedAddressTopUp"
    default: return "Unknown(\(tag))"
    }
}

private func trackedAssetLockStatusLabel(_ tag: UInt8) -> String {
    switch tag {
    case 0: return "Built"
    case 1: return "Broadcast"
    case 2: return "InstantSendLocked"
    case 3: return "ChainLocked"
    case 4: return "Consumed"
    default: return "Unknown(\(tag))"
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
    @State private var dashpaySyncState: ManagedIdentity.DashPaySyncState?

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
                dashpaySyncStateSection
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

    /// Live in-memory DashPay sync state — the counts let you compare against
    /// the persisted SwiftData rows (Storage Explorer), and the high-water
    /// cursors are visible NOWHERE else (they aren't persisted; they reset to
    /// "not advanced" on every cold restart).
    private var dashpaySyncStateSection: some View {
        Section("DashPay Sync State (live)") {
            if let s = dashpaySyncState {
                KVRow(label: "Established contacts", value: "\(s.establishedContacts)")
                KVRow(label: "Incoming requests", value: "\(s.incomingRequests)")
                KVRow(label: "Sent requests", value: "\(s.sentRequests)")
                KVRow(label: "Ignored senders", value: "\(s.ignoredSenders)")
                KVRow(
                    label: "Contact profiles",
                    value: "\(s.presentContactProfiles) present / \(s.contactProfiles) cached"
                )
                KVRow(label: "Payments", value: "\(s.dashpayPayments)")
                KVRow(label: "Own profile", value: s.hasDashPayProfile ? "yes" : "no")
                KVRow(label: "High-water received", value: cursorLabel(s.highWaterReceivedMs))
                KVRow(label: "High-water sent", value: cursorLabel(s.highWaterSentMs))
            } else {
                Text("—")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    private func cursorLabel(_ ms: UInt64?) -> String {
        guard let ms else { return "not advanced" }
        let date = Date(timeIntervalSince1970: TimeInterval(ms) / 1000)
        return "\(ms) (\(date.formatted(date: .abbreviated, time: .standard)))"
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
            dashpaySyncState = try? mi.getDashPaySyncState()
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
