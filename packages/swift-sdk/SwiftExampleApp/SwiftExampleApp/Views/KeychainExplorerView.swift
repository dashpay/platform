// KeychainExplorerView.swift
// SwiftExampleApp
//
// Read-only diagnostic surface listing every generic-password item
// this app has written to the iOS Keychain. Mirrors the
// `StorageExplorerView` spirit for SwiftData but targets the two
// services the app uses:
//
//   * `com.dash.swiftexampleapp.keys` — identity private keys
//     (`privkey_{hex}_{idx}`) and special keys (voting / owner /
//     payout), plus anything the app stashes via
//     `KeychainManager.storeKeyData`.
//   * `org.dash.wallet`               — wallet-level seed + per-wallet
//     mnemonics + auth material stored through `WalletStorage`.
//
// The inspector never pulls secret values into memory — every row
// is populated from attribute metadata only. Taps drill into
// `KeychainItemDetailView` which renders the same metadata in
// full. No delete / reveal actions in v1 to keep the surface
// obviously non-destructive.

import SwiftUI
import SwiftDashSDK
import Security

struct KeychainExplorerView: View {
    /// Every SDK / app keychain write now goes through the unified
    /// `WalletStorage.keychainService` namespace. Legacy services
    /// (`org.dash.wallet`, `com.dash.sdk.keys`,
    /// `com.dash.swiftexampleapp.keys`) are wiped on launch by
    /// `WalletStorage.cleanupLegacyItems`, so this single-row list
    /// is all the explorer needs to show.
    private let services: [ServiceConfig] = [
        ServiceConfig(
            id: "unified",
            service: WalletStorage.keychainService,
            title: "Keychain Items",
            footer:
                "Identity private keys, special (voting/owner/payout) keys, "
                + "per-wallet mnemonics, and any biometric-protected material "
                + "the app has written."
        ),
    ]

    /// Cached items keyed by service. `reload()` repopulates on
    /// appear and after any manual refresh.
    @State private var itemsByService: [String: [KeychainItemSummary]] = [:]

    var body: some View {
        List {
            ForEach(services) { cfg in
                serviceSection(cfg)
            }
        }
        .navigationTitle("Keychain Explorer")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    reload()
                } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Re-query the keychain")
            }
        }
        .onAppear { reload() }
    }

    // MARK: - Sections

    /// One top-level service section, subdivided into rows by
    /// identifier-prefix category. Categories are derived from the
    /// account name since neither service tags items with a
    /// discoverable "kind" field.
    @ViewBuilder
    private func serviceSection(_ cfg: ServiceConfig) -> some View {
        let items = itemsByService[cfg.service] ?? []
        let grouped = Dictionary(grouping: items) { Category.from($0.account) }

        Section {
            if items.isEmpty {
                Text("No items")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                ForEach(Category.allCases, id: \.self) { cat in
                    if let rows = grouped[cat], !rows.isEmpty {
                        DisclosureGroup {
                            ForEach(rows) { item in
                                NavigationLink {
                                    KeychainItemDetailView(item: item)
                                } label: {
                                    itemRow(item, category: cat)
                                }
                            }
                        } label: {
                            HStack {
                                Label(cat.title, systemImage: cat.symbol)
                                Spacer()
                                Text("\(rows.count)")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                }
            }
        } header: {
            HStack {
                Text(cfg.title)
                Spacer()
                Text("\(items.count)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } footer: {
            VStack(alignment: .leading, spacing: 4) {
                Text(cfg.service)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(cfg.footer)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    @ViewBuilder
    private func itemRow(_ item: KeychainItemSummary, category: Category) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(category.displayName(for: item.account))
                .font(.subheadline)
                .lineLimit(1)
                .truncationMode(.middle)
            Text(item.account)
                .font(.caption2.monospaced())
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
            if let created = item.createdAt {
                Text("Created \(created, style: .relative) ago")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
    }

    // MARK: - Data loading

    private func reload() {
        let inspector = KeychainInspector()
        var fresh: [String: [KeychainItemSummary]] = [:]
        for cfg in services {
            fresh[cfg.service] = inspector.listItems(service: cfg.service)
        }
        itemsByService = fresh
    }

    // MARK: - Supporting types

    /// Static per-service config rendered as a section.
    private struct ServiceConfig: Identifiable {
        let id: String
        let service: String
        let title: String
        let footer: String
    }
}

// MARK: - Categorization

/// Logical grouping of keychain items inside a service. Derived
/// from the `kSecAttrAccount` prefix since the SDK stamps every
/// identifier with a well-known format string in
/// `KeychainManager.generateKeyIdentifier`,
/// `generateSpecialKeyIdentifier`, and the per-wallet mnemonic
/// account builder in `WalletStorage`.
enum Category: CaseIterable, Hashable {
    case identityPrivateKey
    case legacyIdentityPrivateKey
    case specialKey
    case walletMnemonic
    case biometric
    case other

    var title: String {
        switch self {
        case .identityPrivateKey: return "Identity Private Keys"
        case .legacyIdentityPrivateKey: return "Identity Private Keys (legacy)"
        case .specialKey: return "Special Keys (Voting / Owner / Payout)"
        case .walletMnemonic: return "Per-Wallet Mnemonics"
        case .biometric: return "Biometric Material"
        case .other: return "Other"
        }
    }

    var symbol: String {
        switch self {
        case .identityPrivateKey: return "key.fill"
        case .legacyIdentityPrivateKey: return "key"
        case .specialKey: return "key.icloud"
        case .walletMnemonic: return "doc.text"
        case .biometric: return "faceid"
        case .other: return "questionmark.square.dashed"
        }
    }

    static func from(_ account: String) -> Category {
        // New persister-callback path: `identity_privkey.<m/9'/...>`.
        if account.hasPrefix("identity_privkey.") { return .identityPrivateKey }
        // Legacy `KeychainManager.storePrivateKey` path —
        // `privkey_<identityHex>_<keyIndex>`. Still supported for
        // direct `KeychainManager` callers but superseded by the
        // derivation-path-keyed layout above.
        if account.hasPrefix("privkey_") { return .legacyIdentityPrivateKey }
        if account.hasPrefix("specialkey_") { return .specialKey }
        if account.hasPrefix("wallet.mnemonic.") { return .walletMnemonic }
        if account == "wallet.biometric" { return .biometric }
        return .other
    }

    /// Friendly name for a row. For identity / special keys pulls
    /// the identity hex + key id out of the account string so the
    /// row label reads "Identity abc…def key 0" instead of the raw
    /// fifty-character identifier.
    func displayName(for account: String) -> String {
        switch self {
        case .identityPrivateKey:
            // Format: "identity_privkey.<derivation-path>" — drop the
            // prefix and surface the path itself as the row label.
            // Rich metadata (identity, key index, wallet) sits in the
            // kSecAttrGeneric JSON payload rendered on the detail view.
            let path = account.dropFirst("identity_privkey.".count)
            return String(path)
        case .legacyIdentityPrivateKey:
            // Format: "privkey_{identityHex}_{keyIndex}"
            let parts = account.dropFirst("privkey_".count).split(separator: "_")
            if parts.count == 2 {
                let hex = String(parts[0])
                let idx = String(parts[1])
                return "Identity \(shortHex(hex)) · key \(idx)"
            }
            return account
        case .specialKey:
            // Format: "specialkey_{identityHex}_{voting|owner|payout}"
            let parts = account.dropFirst("specialkey_".count).split(separator: "_")
            if parts.count == 2 {
                let hex = String(parts[0])
                let kind = String(parts[1])
                return "Identity \(shortHex(hex)) · \(kind)"
            }
            return account
        case .walletMnemonic:
            let hex = String(account.dropFirst("wallet.mnemonic.".count))
            return "Wallet \(shortHex(hex))"
        case .biometric:
            return "Biometric"
        case .other:
            return account
        }
    }

    /// 64-char hex → "abcdef…123456" (6/6). Shorter inputs pass
    /// through unchanged so we never over-truncate.
    private func shortHex(_ hex: String) -> String {
        guard hex.count > 14 else { return hex }
        return "\(hex.prefix(6))…\(hex.suffix(6))"
    }
}

// MARK: - Detail view

struct KeychainItemDetailView: View {
    let item: KeychainItemSummary

    var body: some View {
        List {
            Section("Identity") {
                labeledRow("Service", item.service)
                labeledRow("Account", item.account, monospaced: true)
                labeledRow("Category", Category.from(item.account).title)
            }

            Section("Timestamps") {
                if let created = item.createdAt {
                    labeledRow("Created", dateFormatter.string(from: created))
                }
                if let modified = item.modifiedAt {
                    labeledRow("Modified", dateFormatter.string(from: modified))
                }
            }

            Section("Access") {
                if let level = item.accessibleLevel {
                    labeledRow("Accessible", friendlyAccessible(level))
                }
                labeledRow("iCloud sync", item.synchronizable ? "Yes" : "No")
                if let creator = item.creator {
                    labeledRow("Creator", creator)
                }
            }

            if item.label != nil || item.itemDescription != nil || item.comment != nil {
                Section("Annotations") {
                    if let label = item.label { labeledRow("Label", label) }
                    if let desc = item.itemDescription { labeledRow("Description", desc) }
                    if let comment = item.comment { labeledRow("Comment", comment) }
                }
            }

            if let generic = item.genericMetadata {
                Section("Generic metadata (\(item.genericDataBytes) bytes)") {
                    Text(generic)
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                }
            }

            Section {
                Text(
                    "Key material is never read by this explorer — rows "
                    + "show keychain attribute metadata only. To extract a "
                    + "value you'd have to call the owning API path "
                    + "(KeychainManager / WalletStorage) directly."
                )
                .font(.caption2)
                .foregroundColor(.secondary)
            }
        }
        .navigationTitle("Keychain Item")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func labeledRow(_ label: String, _ value: String, monospaced: Bool = false) -> some View {
        HStack(alignment: .top) {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(monospaced ? .body.monospaced() : .body)
                .multilineTextAlignment(.trailing)
                .textSelection(.enabled)
        }
    }

    /// Translate the four-char `kSecAttrAccessible` constants into
    /// something an engineer can skim without opening the Security
    /// framework headers.
    ///
    /// Uses a dictionary rather than `switch` because the Security
    /// constants are `CFString` — Swift's `case` pattern can't coerce
    /// them through `as String`, and the dictionary-lookup pattern
    /// dodges that entirely.
    private func friendlyAccessible(_ raw: String) -> String {
        let mapping: [String: String] = [
            kSecAttrAccessibleWhenUnlocked as String: "WhenUnlocked",
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String: "WhenUnlockedThisDeviceOnly",
            kSecAttrAccessibleAfterFirstUnlock as String: "AfterFirstUnlock",
            kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String: "AfterFirstUnlockThisDeviceOnly",
            kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly as String: "WhenPasscodeSetThisDeviceOnly",
        ]
        return mapping[raw] ?? raw
    }

    private var dateFormatter: DateFormatter {
        let f = DateFormatter()
        f.dateStyle = .medium
        f.timeStyle = .medium
        return f
    }
}
