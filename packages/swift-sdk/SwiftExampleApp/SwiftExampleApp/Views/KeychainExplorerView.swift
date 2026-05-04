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

    /// Cached items keyed by service. Populated once by
    /// [`loadIfNeeded`] and only mutated via the toolbar refresh
    /// button. Avoiding `.task` / `.onAppear` reloads on every
    /// detail-view pop is deliberate: mutating this `@State` while
    /// a `NavigationLink` destination is animating in invalidates
    /// the link's identity and bounces the navigation straight back.
    @State private var itemsByService: [String: [KeychainItemSummary]] = [:]
    /// One-shot load gate. Flipped true after the first populate so
    /// re-appear events (returning from the detail view) don't
    /// trigger another reload.
    @State private var hasLoaded = false

    var body: some View {
        List {
            // Collapse every service's items into a single
            // `(category, rows)` stream, then render one Section
            // per category. This is the simplest shape the iOS 18
            // List diff is happy with: no nested `DisclosureGroup`,
            // no tuple-keyed `ForEach`, no conditional footers that
            // appear/disappear across renders.
            ForEach(Category.allCases, id: \.self) { cat in
                let rows = rowsForCategory(cat)
                if !rows.isEmpty {
                    Section {
                        ForEach(rows) { item in
                            NavigationLink {
                                KeychainItemDetailView(item: item)
                            } label: {
                                itemRow(item, category: cat)
                            }
                        }
                    } header: {
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

            if allItems.isEmpty && hasLoaded {
                Section {
                    Text("No items")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            // Service-info tail. Read-only; stays in a dedicated
            // bottom section so its re-render can't affect the
            // NavigationLinks above.
            Section("Services") {
                ForEach(services) { cfg in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(cfg.title)
                            .font(.subheadline)
                        Text(cfg.service)
                            .font(.caption2.monospaced())
                            .foregroundColor(.secondary)
                        Text(cfg.footer)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                    .padding(.vertical, 2)
                }
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
        // One-shot initial load. Using `.onAppear` with an explicit
        // gate instead of `.task` because `.task` restarts on every
        // reappear (detail-view pop); that restart fires a reload,
        // mutates `@State itemsByService`, and invalidates the
        // NavigationLink identity of a tap already mid-flight —
        // visible as "I tap a row and it bounces me back."
        .onAppear {
            guard !hasLoaded else { return }
            hasLoaded = true
            reload()
        }
    }

    // MARK: - Row aggregation

    /// Flatten every service's items into a single list. Services
    /// are already keyed on a unified namespace (`WalletStorage.
    /// keychainService`), so cross-service deduplication isn't
    /// needed in practice.
    private var allItems: [KeychainItemSummary] {
        itemsByService.values.flatMap { $0 }
    }

    /// Rows belonging to `cat`, sorted ascending by account name
    /// (the inspector already sorts within a service, but we join
    /// across services here so an explicit sort keeps the UI
    /// deterministic).
    private func rowsForCategory(_ cat: Category) -> [KeychainItemSummary] {
        allItems
            .filter { Category.from($0.account) == cat }
            .sorted { lhs, rhs in
                lhs.account.localizedCaseInsensitiveCompare(rhs.account) == .orderedAscending
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
    case walletMetadata
    case biometric
    case other

    var title: String {
        switch self {
        case .identityPrivateKey: return "Identity Private Keys"
        case .legacyIdentityPrivateKey: return "Identity Private Keys (legacy)"
        case .specialKey: return "Special Keys (Voting / Owner / Payout)"
        case .walletMnemonic: return "Per-Wallet Mnemonics"
        case .walletMetadata: return "Per-Wallet Metadata"
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
        case .walletMetadata: return "tag"
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
        // Order matters: `wallet.metadata.` must be matched before
        // `wallet.mnemonic.` because both share the `wallet.`
        // prefix; the explorer relies on the trailing namespace
        // segment to disambiguate.
        if account.hasPrefix("\(WalletStorage.metadataAccountPrefix).") { return .walletMetadata }
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
        case .walletMetadata:
            let prefix = "\(WalletStorage.metadataAccountPrefix)."
            let hex = String(account.dropFirst(prefix.count))
            return "Wallet \(shortHex(hex)) · metadata"
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

    /// Decoded `WalletKeychainMetadata` blob for `walletMetadata`
    /// rows. Pulled lazily on `.onAppear` so the rest of the detail
    /// view (which only renders attribute metadata) doesn't pay
    /// the cost of touching the keychain value path on every cell.
    /// Stays `nil` for non-metadata rows.
    @State private var walletMetadata: WalletMetadataPreview?

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

            // For wallet-metadata rows the stored value is the
            // user-typed name + description + networks + birth
            // height (NOT a secret), so we surface its decoded
            // form here. Mnemonic / private-key rows fall through
            // and are never read.
            if Category.from(item.account) == .walletMetadata,
               let preview = walletMetadata {
                Section("Wallet metadata") {
                    if let name = preview.name {
                        labeledRow("Name", name)
                    } else {
                        labeledRow("Name", "(none)")
                    }
                    if let desc = preview.walletDescription {
                        labeledRow("Description", desc)
                    } else {
                        labeledRow("Description", "(none)")
                    }
                    if let nets = preview.networks, !nets.isEmpty {
                        labeledRow("Networks", nets.joined(separator: ", "))
                    } else {
                        labeledRow("Networks", "(none)")
                    }
                    if let bh = preview.birthHeight {
                        labeledRow("Birth height", String(bh))
                    } else {
                        labeledRow("Birth height", "(none)")
                    }
                }
            }

            Section {
                Text(
                    "Secret material (mnemonics, private keys) is "
                    + "never read by this explorer — rows show keychain "
                    + "attribute metadata only. The wallet-metadata "
                    + "category surfaces its plain-text payload because "
                    + "the user typed those strings; everything else "
                    + "stays opaque."
                )
                .font(.caption2)
                .foregroundColor(.secondary)
            }
        }
        .navigationTitle("Keychain Item")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear(perform: loadWalletMetadataIfNeeded)
    }

    /// On first appear of a `walletMetadata` row, decode the value
    /// blob into `WalletKeychainMetadata` and snapshot it locally.
    /// Non-metadata rows are no-ops.
    private func loadWalletMetadataIfNeeded() {
        guard walletMetadata == nil,
              Category.from(item.account) == .walletMetadata else { return }
        let prefix = "\(WalletStorage.metadataAccountPrefix)."
        guard item.account.hasPrefix(prefix) else { return }
        let hex = String(item.account.dropFirst(prefix.count))
        guard let walletId = Self.dataFromHex(hex) else { return }
        let storage = WalletStorage()
        guard let stored = (try? storage.metadata(for: walletId)) ?? nil else { return }
        walletMetadata = WalletMetadataPreview(
            name: stored.name,
            walletDescription: stored.walletDescription,
            networks: stored.networks,
            birthHeight: stored.birthHeight
        )
    }

    /// Local hex decoder so the explorer doesn't depend on the
    /// private decoder inside `WalletStorage`. Safe to call on the
    /// trimmed account suffix because `Category.from` already
    /// validated the prefix.
    private static func dataFromHex(_ hex: String) -> Data? {
        guard hex.count % 2 == 0 else { return nil }
        var data = Data(capacity: hex.count / 2)
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            guard let byte = UInt8(hex[index..<next], radix: 16) else { return nil }
            data.append(byte)
            index = next
        }
        return data
    }

    /// Plain Swift mirror of the decoded metadata blob the detail
    /// view shows. Avoids passing a `WalletKeychainMetadata`
    /// directly through `@State` so the SDK type doesn't have to
    /// adopt `Hashable`.
    private struct WalletMetadataPreview: Equatable {
        let name: String?
        let walletDescription: String?
        let networks: [String]?
        let birthHeight: UInt32?
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
        let f = DateFormatter.gregorian()
        f.dateStyle = .medium
        f.timeStyle = .medium
        return f
    }
}
