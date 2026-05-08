import SwiftUI
import SwiftDashSDK

/// One row in the multi-wallet recovery sheet: the orphan wallet's
/// stable id, its display name (from keychain metadata), the network
/// it was originally created on (best-effort), and the user's choice
/// for whether it shares the primary auth prompt with its siblings.
///
/// `samePinCode == true` (default) means "fold this wallet into the
/// shared authorization round"; `false` means "prompt separately
/// for this one before re-deriving".
struct OrphanWalletEntry: Identifiable, Hashable {
    let walletId: Data
    let displayName: String
    let network: Network?
    var samePinCode: Bool

    var id: Data { walletId }
}

/// Sheet that consolidates orphan-mnemonic recovery into a single
/// surface. Lists every orphan side-by-side with a `Same PIN` toggle
/// so the user can authorize once for the wallets that share a
/// passcode, and only get a separate prompt for the rows they
/// uncheck.
///
/// All authentication and re-derivation logic lives on the parent
/// (`ContentView`) — the sheet only collects the user's choices and
/// hands them back through `onAuthorize`. That keeps `LAContext` /
/// `WalletStorage` / `walletManager.createWallet` plumbing in one
/// place and lets the sheet stay a pure SwiftUI struct.
struct RecoverWalletsSheet: View {
    @Environment(\.dismiss) private var dismiss

    /// Initial set of orphans surfaced to the user. The sheet edits
    /// the `samePinCode` flag locally on a copy so toggling rows
    /// doesn't have to round-trip through the parent until Authorize
    /// is tapped.
    let entries: [OrphanWalletEntry]

    /// Called with the user's per-wallet choices when they tap the
    /// primary Authorize button. The sheet dismisses immediately and
    /// the parent runs the auth + re-derivation pipeline.
    let onAuthorize: ([OrphanWalletEntry]) -> Void

    /// Called when the user taps Cancel. Parent decides whether to
    /// route into the keep/delete prompt or just leave the orphans
    /// pending.
    let onCancel: () -> Void

    @State private var localEntries: [OrphanWalletEntry]

    init(
        entries: [OrphanWalletEntry],
        onAuthorize: @escaping ([OrphanWalletEntry]) -> Void,
        onCancel: @escaping () -> Void
    ) {
        self.entries = entries
        self.onAuthorize = onAuthorize
        self.onCancel = onCancel
        _localEntries = State(initialValue: entries)
    }

    /// Hide the per-row "Same PIN" toggle when there's only one
    /// orphan — the consolidation choice is meaningless for a single
    /// wallet and the toggle would just clutter the row.
    private var isMultiWallet: Bool { localEntries.count > 1 }

    private var sharedCount: Int {
        localEntries.filter(\.samePinCode).count
    }

    private var separateCount: Int {
        localEntries.count - sharedCount
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Text(headerMessage)
                        .font(.callout)
                } header: {
                    Text("Wallets to Recover")
                }

                Section {
                    ForEach($localEntries) { $entry in
                        walletRow(entry: $entry)
                    }
                } footer: {
                    if isMultiWallet {
                        Text(footerHelp)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Recover Wallets")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        onCancel()
                        dismiss()
                    }
                    .accessibilityIdentifier("recoverWallets.cancelButton")
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Authorize") {
                        onAuthorize(localEntries)
                        dismiss()
                    }
                    .fontWeight(.semibold)
                    .accessibilityIdentifier("recoverWallets.authorizeButton")
                }
            }
        }
    }

    @ViewBuilder
    private func walletRow(entry: Binding<OrphanWalletEntry>) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .center, spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.wrappedValue.displayName)
                        .font(.body)
                        .lineLimit(1)
                    HStack(spacing: 6) {
                        Text(entry.wrappedValue.network?.displayName ?? "Unknown network")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text("·")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(walletShortId(entry.wrappedValue.walletId))
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                }
                Spacer()
            }
            if isMultiWallet {
                Toggle(isOn: entry.samePinCode) {
                    Text("Same PIN")
                        .font(.callout)
                }
                .toggleStyle(.switch)
            }
        }
        .padding(.vertical, 2)
    }

    private var headerMessage: String {
        let count = localEntries.count
        if count == 1 {
            return "A wallet recovery phrase is stored on this device, "
                + "but no matching wallet data was found. Authorize to "
                + "re-derive its public keys from the stored phrase."
        }
        return "\(count) wallet recovery phrases are stored on this device, "
            + "but no matching wallet data was found. Authorize to re-derive "
            + "their public keys from the stored phrases."
    }

    private var footerHelp: String {
        if separateCount == 0 {
            return "All \(sharedCount) wallets will be authorized in one prompt. "
                + "Uncheck \"Same PIN\" on any wallet that uses a different code."
        }
        if sharedCount == 0 {
            return "Each wallet will prompt for its own authorization."
        }
        return "\(sharedCount) wallet\(sharedCount == 1 ? "" : "s") share a prompt; "
            + "\(separateCount) will prompt separately."
    }

    /// 4-byte hex prefix of the 32-byte wallet id — short enough for a
    /// caption row, distinguishable across the orphan set.
    private func walletShortId(_ walletId: Data) -> String {
        guard walletId.count >= 4 else {
            return walletId.map { String(format: "%02x", $0) }.joined()
        }
        return walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
            + "…"
    }
}
