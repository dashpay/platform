// CreateIdentityView.swift
// SwiftExampleApp
//
// Stepped UI for spinning up a new Dash Platform identity. The
// workflow chooses a funding source in two passes:
//
//   1. Source Wallet — one of the local HDWallet rows, or
//      "Create without Wallet" for the advanced path where the caller
//      supplies a raw asset-lock proof.
//   2. When a wallet is chosen: either a PersistentAccount on that
//      wallet (any type — Core pools and Platform Payment both work)
//      or "Fund from unused Asset Lock".
//
// This file is UI-only — the submit button is a stub. Wiring to the
// actual create-identity FFI comes next.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct CreateIdentityView: View {
    @Environment(\.dismiss) private var dismiss

    /// All locally-persisted wallets. Drives the Source Wallet
    /// picker along with the synthetic "no wallet" sentinel.
    @Query(sort: \HDWallet.createdAt) private var wallets: [HDWallet]

    /// All persisted accounts across wallets. Filtered per-selection
    /// inside `accountOptions(for:)` so switching wallets doesn't
    /// re-fire a SwiftData query.
    @Query private var allAccounts: [PersistentAccount]

    // MARK: - Selection state

    /// The source wallet selection. `nil` encodes "pick nothing yet";
    /// `.walletless` encodes the explicit "Create without Wallet"
    /// choice that switches step 2 to the raw asset-lock path.
    @State private var walletSelection: WalletSelection? = nil

    /// Chosen funding source when a wallet is selected.
    @State private var fundingSelection: FundingSelection? = nil

    /// Raw asset-lock proof text, used only in the walletless path.
    /// Accepted encoding is base64 or lowercase hex — the submit
    /// logic (future) will detect + decode.
    @State private var walletlessProof: String = ""

    var body: some View {
        NavigationStack {
            Form {
                sourceWalletSection
                fundingSection
                if canSubmit {
                    submitSection
                }
            }
            .navigationTitle("Create Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }

    // MARK: - Sections

    private var sourceWalletSection: some View {
        Section {
            Picker("Source Wallet", selection: $walletSelection) {
                Text("Select…")
                    .tag(Optional<WalletSelection>.none)
                ForEach(wallets) { wallet in
                    Text(walletLabel(for: wallet))
                        .tag(Optional(WalletSelection.wallet(id: wallet.walletId)))
                }
                Divider()
                Text("Create without Wallet")
                    .tag(Optional(WalletSelection.walletless))
            }
            .onChange(of: walletSelection) { _, _ in
                // Reset downstream selection whenever the wallet
                // changes so a stale account / proof can't leak
                // through.
                fundingSelection = nil
                walletlessProof = ""
            }
        } header: {
            Text("Source Wallet")
        } footer: {
            Text(
                "Pick a wallet to fund the identity from one of its accounts, "
                + "or Create without Wallet to supply a raw asset-lock proof."
            )
        }
    }

    @ViewBuilder
    private var fundingSection: some View {
        switch walletSelection {
        case .none:
            EmptyView()
        case .walletless:
            walletlessSection
        case .wallet(let walletId):
            walletAccountSection(for: walletId)
        }
    }

    @ViewBuilder
    private func walletAccountSection(for walletId: Data) -> some View {
        let options = accountOptions(for: walletId)
        Section {
            Picker("Funding Source", selection: $fundingSelection) {
                Text("Select…")
                    .tag(Optional<FundingSelection>.none)
                ForEach(options) { option in
                    fundingOptionLabel(option)
                        .tag(Optional(FundingSelection.account(id: option.persistentId)))
                }
                Divider()
                Text("Fund from unused Asset Lock")
                    .tag(Optional(FundingSelection.unusedAssetLock))
            }
            .onChange(of: fundingSelection) { _, newValue in
                // SwiftUI's menu-style Picker doesn't have a per-row
                // `disabled` hook we can trust, so enforce "can't
                // pick a zero-balance account" by reverting the
                // selection if the user taps one.
                guard case let .account(persistentId) = newValue,
                      let option = options.first(where: { $0.persistentId == persistentId }),
                      !option.hasBalance
                else { return }
                fundingSelection = nil
            }
        } header: {
            Text("Funding Source")
        } footer: {
            Text(
                "Any account on the selected wallet can fund the identity — "
                + "Core or Platform Payment. Accounts with no balance are "
                + "greyed out and can't be selected. \"Fund from unused "
                + "Asset Lock\" picks an existing tracked asset lock instead."
            )
        }
    }

    /// Picker row renderer. We build the label as an
    /// `AttributedString` because SwiftUI's menu-style Picker strips
    /// `.foregroundStyle` / `.opacity` modifiers on the outer `Text`,
    /// but honours per-run `foregroundColor` attributes inside an
    /// attributed value. The balance suffix (` — 0.01 DASH` / ` —
    /// empty`) makes the state obvious even if the colour gets
    /// overridden on some iOS variants.
    private func fundingOptionLabel(_ option: FundingAccountOption) -> Text {
        var attr = AttributedString("\(option.label) — \(option.balanceText)")
        if !option.hasBalance {
            attr.foregroundColor = .secondary
        }
        return Text(attr)
    }

    private var walletlessSection: some View {
        Section {
            TextEditor(text: $walletlessProof)
                .font(.system(.footnote, design: .monospaced))
                .frame(minHeight: 120)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
        } header: {
            Text("Asset Lock Proof")
        } footer: {
            Text("Paste the raw proof as base64 or hex.")
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                // TODO(platform-wallet): wire up to the actual
                // create-identity FFI path. For now the button is
                // a stub so we can iterate the UI.
            } label: {
                Text("Create Identity")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit)
        }
    }

    // MARK: - Derived state

    /// Whether the current selection is complete enough that the
    /// submit button should light up. Non-empty hex / base64 content
    /// in the walletless path, or a concrete account + funding choice
    /// otherwise.
    private var canSubmit: Bool {
        switch (walletSelection, fundingSelection) {
        case (.walletless, _):
            return !walletlessProof
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty
        case (.wallet, .some):
            return true
        default:
            return false
        }
    }

    // MARK: - Helpers

    private func walletLabel(for wallet: HDWallet) -> String {
        let trimmed = wallet.label.trimmingCharacters(in: .whitespaces)
        let base = trimmed.isEmpty ? shortWalletId(wallet.walletId) : trimmed
        return "\(base) (\(wallet.network.rawValue))"
    }

    private func shortWalletId(_ walletId: Data) -> String {
        let prefix = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return prefix.isEmpty ? "Wallet" : "Wallet \(prefix)…"
    }

    /// Turn a wallet's PersistentAccounts into the funding-picker
    /// rows. Restricted to accounts that actually hold spendable
    /// funds — Core Standard (BIP44 / BIP32), CoinJoin, and
    /// PlatformPayment. Identity / provider / asset-lock-topup
    /// accounts are intentionally excluded; they aren't sources of
    /// funds for a new identity. Ordering matches
    /// `AccountListView`: BIP44 → PlatformPayment → BIP32 →
    /// CoinJoin.
    private func accountOptions(for walletId: Data) -> [FundingAccountOption] {
        allAccounts
            .filter { account in
                guard account.wallet?.walletId == walletId else { return false }
                return CreateIdentityView.isFundingAccount(account)
            }
            .sorted { lhs, rhs in
                let lhsKey = CreateIdentityView.sortKey(for: lhs)
                let rhsKey = CreateIdentityView.sortKey(for: rhs)
                return lhsKey < rhsKey
            }
            .map { account in
                let (hasBalance, balanceText) = Self.accountBalanceSummary(account)
                return FundingAccountOption(
                    persistentId: account.persistentModelID,
                    label: Self.fundingLabel(for: account),
                    hasBalance: hasBalance,
                    balanceText: balanceText
                )
            }
    }

    /// Account types eligible to fund a new identity.
    private static func isFundingAccount(_ account: PersistentAccount) -> Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
        }
    }

    /// Formatted balance for the picker row and the disabled flag.
    /// Core / CoinJoin use the SPV-maintained
    /// `balanceConfirmed + balanceUnconfirmed` duffs (1e8/DASH);
    /// PlatformPayment sums the BLAST-synced credit balances across
    /// its addresses (1e11/DASH).
    private static func accountBalanceSummary(
        _ account: PersistentAccount
    ) -> (hasBalance: Bool, balanceText: String) {
        switch account.accountType {
        case 14:
            let credits = account.platformAddresses.reduce(0) { $0 + $1.balance }
            return (
                credits > 0,
                credits > 0 ? formatDash(raw: credits, divisor: 100_000_000_000.0) : "empty"
            )
        default:
            let duffs = account.balanceConfirmed + account.balanceUnconfirmed
            return (
                duffs > 0,
                duffs > 0 ? formatDash(raw: duffs, divisor: 100_000_000.0) : "empty"
            )
        }
    }

    /// `"0.01 DASH"` — stripped of trailing zeros, uses up to 8 decimals.
    private static func formatDash(raw: UInt64, divisor: Double) -> String {
        let dash = Double(raw) / divisor
        let fmt = NumberFormatter()
        fmt.minimumFractionDigits = 0
        fmt.maximumFractionDigits = 8
        fmt.numberStyle = .decimal
        fmt.groupingSeparator = ","
        fmt.decimalSeparator = "."
        return (fmt.string(from: NSNumber(value: dash)) ?? String(format: "%.8f", dash)) + " DASH"
    }

    private static func fundingLabel(for account: PersistentAccount) -> String {
        "\(account.accountTypeName) #\(account.accountIndex)"
    }

    private static func sortKey(
        for account: PersistentAccount
    ) -> (UInt8, UInt32, UInt8, UInt32) {
        let group: UInt8
        switch account.accountType {
        case 0: group = account.standardTag == 0 ? 0 : 2
        case 14: group = 1
        case 1: group = 3
        default: group = 4
        }
        return (group, account.accountType, account.standardTag, account.accountIndex)
    }
}

// MARK: - Selection types

private enum WalletSelection: Hashable {
    case wallet(id: Data)
    case walletless
}

private enum FundingSelection: Hashable {
    case account(id: PersistentIdentifier)
    case unusedAssetLock
}

private struct FundingAccountOption: Identifiable {
    let persistentId: PersistentIdentifier
    let label: String
    /// `true` if the account currently holds spendable funds. Drives
    /// the greyed-out picker row for zero-balance sources.
    let hasBalance: Bool
    var id: PersistentIdentifier { persistentId }
}
