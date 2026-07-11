// SearchWalletsForIdentitiesView.swift
// SwiftExampleApp
//
// Sheet invoked from the Identities tab that drives the DIP-9
// identity-authentication gap-limit scan against a single user-
// selected wallet.
//
// On success the scan's discovered-identity count renders per the
// existing per-wallet row. On an empty scan the view also asks Rust
// for the identity-authentication keypairs the scan would have used
// (same derivation policy the scan applies — `previewIdentityRegistrationKeys`
// is a read-only view over the loaded wallet seed, no mnemonic needed
// on the Swift side), so the user can sanity-check the (public /
// private) keys being probed. Private keys are hidden by default and
// can be revealed per-row with a tap.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct SearchWalletsForIdentitiesView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @Environment(\.dismiss) private var dismiss

    /// Every persisted wallet, across all networks. Sorted by
    /// `createdAt` to match the ordering `CreateIdentityView` uses.
    /// Filtered down to the active network by `hdWallets` — the
    /// store keeps one row per (walletId, network), so without the
    /// filter the picker would list other networks' rows too.
    @Query(sort: \PersistentWallet.createdAt) private var allWallets: [PersistentWallet]

    /// Wallets on the currently-selected network — the only ones the
    /// picker should offer, since the scan runs against the active
    /// network's wallet manager. Mirrors the `networkRaw`-filter
    /// pattern used by `WalletsContentView` / `IdentitiesContentView`.
    private var hdWallets: [PersistentWallet] {
        let raw = platformState.currentNetwork.rawValue
        return allWallets.filter { $0.networkRaw == raw }
    }

    /// User-selected wallet id. Initialized to the first wallet on
    /// appear; always preselected even when the list only has one
    /// entry (so the picker stays visible and the UI is uniform).
    @State private var selectedWalletId: Data?

    @State private var isSearching: Bool = false
    @State private var result: WalletFinding?
    @State private var previewKeys: [ManagedPlatformWallet.IdentityRegistrationKeyPreview] = []
    @State private var previewError: String?
    @State private var revealedKeyIndex: UInt32?

    /// One wallet's outcome. Post-redesign we scan a single wallet
    /// at a time, so this is a flat struct rather than an array.
    struct WalletFinding: Equatable {
        let walletId: Data
        let label: String
        let foundCount: Int
        /// Rust-side error as reported by `discoverIdentities`.
        /// Rendered in full (no truncation) so path-derivation
        /// failures and similar long messages aren't cut off.
        let error: String?
    }

    /// Resolved runtime wallet for the current selection, or `nil`
    /// when the user's picked wallet isn't loaded in the manager
    /// (e.g. the app hasn't restored it from the persistor).
    private var selectedManagedWallet: ManagedPlatformWallet? {
        guard let id = selectedWalletId else { return nil }
        return walletManager.wallet(for: id)
    }

    /// Persisted row for the selected wallet — drives the label
    /// shown in the result / preview sections.
    private var selectedPersistentWallet: PersistentWallet? {
        guard let id = selectedWalletId else { return nil }
        return hdWallets.first { $0.walletId == id }
    }

    var body: some View {
        NavigationStack {
            Form {
                explainerSection
                walletPickerSection

                if let result {
                    resultSection(result)
                }

                if !previewKeys.isEmpty {
                    previewKeysSection
                }

                if let previewError {
                    Section {
                        Text(previewError)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }

                searchButtonSection
            }
            .navigationTitle("Re-scan for Identities")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                        .disabled(isSearching)
                }
            }
            .onAppear(perform: ensureDefaultSelection)
            .onChange(of: hdWallets) {
                ensureDefaultSelection()
            }
        }
    }

    // MARK: - Sections

    private var explainerSection: some View {
        Section {
            Text("Scan this wallet's DIP-9 identity authentication "
                + "tree and register any identities found on Platform.")
                .font(.callout)
                .padding(.vertical, 2)
        }
    }

    @ViewBuilder
    private var walletPickerSection: some View {
        Section("Wallet") {
            if hdWallets.isEmpty {
                Text("No wallets loaded. Create or import a wallet first.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker(
                    "Wallet",
                    selection: Binding<Data?>(
                        get: { selectedWalletId },
                        set: { newId in
                            selectedWalletId = newId
                            // A new wallet selection invalidates
                            // the previous scan's result and
                            // preview keys — they belong to a
                            // different seed.
                            result = nil
                            previewKeys = []
                            previewError = nil
                            revealedKeyIndex = nil
                        }
                    )
                ) {
                    ForEach(hdWallets, id: \.walletId) { wallet in
                        walletPickerRow(wallet).tag(Optional(wallet.walletId))
                    }
                }
                .pickerStyle(.menu)
                .disabled(isSearching || hdWallets.count < 1)
            }
        }
    }

    private func walletPickerRow(_ wallet: PersistentWallet) -> some View {
        HStack {
            Text(wallet.label)
            Text(labelFingerprint(wallet.walletId))
                .font(.caption.monospaced())
                .foregroundColor(.secondary)
        }
    }

    @ViewBuilder
    private func resultSection(_ finding: WalletFinding) -> some View {
        Section("Result") {
            HStack {
                Text("New identities")
                Spacer()
                Text("+\(finding.foundCount)")
                    .fontWeight(.semibold)
                    .foregroundColor(finding.foundCount > 0 ? .green : .secondary)
            }
            if let err = finding.error {
                // No `.lineLimit` — identity-derivation errors can
                // be long (full path + SECP error chain). Let
                // SwiftUI wrap so nothing's truncated.
                Text(err)
                    .font(.caption)
                    .foregroundColor(.red)
                    .textSelection(.enabled)
            } else if finding.foundCount == 0 {
                // The gap-limit window is owned by Rust; the
                // preview list below is sized against the same
                // policy, so we point the user at it rather than
                // naming a Swift-side constant.
                Text("No identities were registered under this wallet's "
                    + "DIP-9 identity tree within the gap-limit window. "
                    + "See the keypairs below for the exact keys the scan walked.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    @ViewBuilder
    private var previewKeysSection: some View {
        Section(
            header: Text("Identity Registration Keys"),
            footer: Text("These are the first \(previewKeys.count) "
                + "MASTER identity-authentication keys "
                + "(`m/9'/coin'/5'/0'/0'/identity_index'/0'`) "
                + "the discovery scan would use. Tap a row to reveal "
                + "its private key (WIF).")
                .font(.caption2)
        ) {
            ForEach(previewKeys) { preview in
                previewKeyRow(preview)
            }
        }
    }

    private func previewKeyRow(
        _ preview: ManagedPlatformWallet.IdentityRegistrationKeyPreview
    ) -> some View {
        let isRevealed = revealedKeyIndex == preview.identityIndex
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Index \(preview.identityIndex)")
                    .font(.subheadline.weight(.semibold))
                Spacer()
                Button {
                    revealedKeyIndex = isRevealed ? nil : preview.identityIndex
                } label: {
                    Label(
                        isRevealed ? "Hide Private Key" : "Reveal Private Key",
                        systemImage: isRevealed ? "eye.slash" : "eye"
                    )
                    .font(.caption)
                    .labelStyle(.titleAndIcon)
                }
                .buttonStyle(.borderless)
            }

            VStack(alignment: .leading, spacing: 2) {
                Text("Path")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(preview.derivationPath)
                    .font(.system(.caption2, design: .monospaced))
                    .textSelection(.enabled)
            }

            VStack(alignment: .leading, spacing: 2) {
                Text("Public Key")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Text(preview.publicKeyHex)
                    .font(.system(.caption2, design: .monospaced))
                    .textSelection(.enabled)
                    .lineLimit(2)
            }

            if isRevealed {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Private Key (WIF)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    Text(preview.privateKeyWIF)
                        .font(.system(.caption2, design: .monospaced))
                        .textSelection(.enabled)
                        .padding(6)
                        .background(Color(.secondarySystemBackground))
                        .cornerRadius(6)
                    Button {
                        UIPasteboard.general.string = preview.privateKeyWIF
                    } label: {
                        Label("Copy Private Key", systemImage: "doc.on.doc")
                            .font(.caption)
                    }
                    .buttonStyle(.borderless)
                }
            }
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder
    private var searchButtonSection: some View {
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
                        Text(result == nil ? "Re-scan Wallet" : "Re-scan Again")
                    }
                    Spacer()
                }
            }
            .disabled(
                isSearching
                    || selectedWalletId == nil
                    || selectedManagedWallet == nil
            )
            if selectedWalletId != nil && selectedManagedWallet == nil {
                Text("This wallet isn't loaded in the wallet manager yet. "
                    + "Restore it from the Wallets tab and try again.")
                    .font(.caption)
                    .foregroundColor(.orange)
            }
        }
    }

    // MARK: - Actions

    /// Preselect the first wallet (by `createdAt` order) once the
    /// query loads, and clear the selection if the underlying list
    /// shrank to exclude the previously-selected id.
    private func ensureDefaultSelection() {
        if let current = selectedWalletId,
           hdWallets.contains(where: { $0.walletId == current }) {
            return
        }
        selectedWalletId = hdWallets.first?.walletId
        result = nil
        previewKeys = []
        previewError = nil
        revealedKeyIndex = nil
    }

    /// Run discovery against the selected wallet. Resume-from-cache
    /// (the Rust default) — no full-rescan toggle post-redesign,
    /// since re-walking cached slots is the wrong tool for a "find
    /// my identities" button. Empty / errored results trigger the
    /// Rust-side preview keypair fetch.
    private func runScan() async {
        guard let walletId = selectedWalletId,
              let managed = walletManager.wallet(for: walletId),
              let persistent = selectedPersistentWallet
        else { return }

        isSearching = true
        defer { isSearching = false }

        // Wipe stale state before kicking off.
        result = nil
        previewKeys = []
        previewError = nil
        revealedKeyIndex = nil

        let label = persistent.label
        do {
            let found = try await managed.discoverIdentities(
                startIndex: nil, // resume from cache
                gapLimit: nil    // Rust default (IDENTITY_GAP_LIMIT)
            )
            result = WalletFinding(
                walletId: walletId,
                label: label,
                foundCount: found.count,
                error: nil
            )

            // Zero hits → ask Rust for the preview keypairs the
            // scan walked so the user can eyeball / copy them.
            if found.isEmpty {
                await loadPreviewKeys(on: managed)
            }
        } catch {
            result = WalletFinding(
                walletId: walletId,
                label: label,
                foundCount: 0,
                error: error.localizedDescription
            )

            // Even on error, show the candidate keys. The user's
            // reason for pulling this sheet up is usually "I don't
            // see my identity" — the keypairs let them confirm
            // whether the scan is probing the right derivation tree.
            await loadPreviewKeys(on: managed)
        }
    }

    /// Call the Rust-side preview FFI on the already-loaded wallet
    /// handle and publish the result. No Keychain / mnemonic
    /// access required on this path — derivation happens entirely
    /// in Rust against the wallet's resident seed. Runs off the
    /// main actor anyway because the call spins up a secp256k1
    /// context per row.
    @MainActor
    private func loadPreviewKeys(on managed: ManagedPlatformWallet) async {
        let previews: [ManagedPlatformWallet.IdentityRegistrationKeyPreview]
        do {
            previews = try await Task.detached(priority: .userInitiated) {
                // `count: nil` defers to the Rust-side
                // IDENTITY_GAP_LIMIT so the preview window tracks
                // whatever the discovery scan just walked.
                try managed.previewIdentityRegistrationKeys(
                    startIndex: 0,
                    count: nil
                )
            }.value
        } catch {
            previewError = "Couldn't derive identity-authentication keys: "
                + error.localizedDescription
            return
        }

        self.previewKeys = previews
    }

    /// Short human fingerprint for a wallet id when the list row
    /// needs a distinguishing suffix alongside the label.
    private func labelFingerprint(_ walletId: Data) -> String {
        let hex = walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
        return hex.isEmpty ? "" : "(\(hex))"
    }
}
