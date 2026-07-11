// TopUpIdentityView.swift
// SwiftExampleApp
//
// Stepped UI for topping up an existing identity's credit balance.
// Mirrors `CreateIdentityView`'s shape but locks the wallet to the
// identity's owning wallet and skips the identity-pubkey / index
// sections — top-up is just a funding choice + amount, signed by the
// input addresses' private keys (no IdentityCreate to register).

import SwiftUI
import SwiftDashSDK
import SwiftData

struct TopUpIdentityView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Identity row this top-up flow targets. The owning wallet is
    /// derived from `identity.wallet`; the funding account picker is
    /// scoped to that wallet's accounts.
    let identity: PersistentIdentity

    /// Credits per DASH (1e11) — same divisor `CreateIdentityView`
    /// uses for Platform-side credit amounts.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    /// All persisted accounts. Filtered down to the identity's
    /// wallet inside `accountOptions`.
    @Query private var allAccounts: [PersistentAccount]

    // MARK: - Selection state

    @State private var fundingSelection: FundingSelection? = nil
    @State private var amountDash: String = ""

    // MARK: - Submit state

    @State private var isToppingUp: Bool = false
    @State private var submitError: SubmitError? = nil
    @State private var newBalance: UInt64? = nil

    var body: some View {
        NavigationStack {
            Form {
                if newBalance != nil {
                    successSection
                } else {
                    walletSection
                    fundingSection
                    amountSection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Top Up Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isToppingUp)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not top up identity"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
        }
    }

    // MARK: - Sections

    private var walletSection: some View {
        Section {
            HStack {
                Label("Wallet", systemImage: "wallet.pass")
                Spacer()
                Text(walletLabel)
                    .foregroundColor(.secondary)
            }
            HStack {
                Label("Identity", systemImage: "person.text.rectangle")
                Spacer()
                Text(identity.alias ?? identity.identityIdString)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.secondary)
            }
            HStack {
                Label("Current Balance", systemImage: "dollarsign.circle")
                Spacer()
                Text(identity.formattedBalance)
                    .foregroundColor(.blue)
                    .fontWeight(.medium)
            }
        } header: {
            Text("Target")
        }
    }

    @ViewBuilder
    private var fundingSection: some View {
        if let walletId = identity.wallet?.walletId {
            let options = accountOptions(for: walletId)
            Section {
                if options.isEmpty {
                    Text("No funded Platform Payment accounts on this wallet.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Picker("Funding Source", selection: $fundingSelection) {
                        Text("Select…")
                            .tag(Optional<FundingSelection>.none)
                            .accessibilityIdentifier("topup.fundingSource.none")
                        ForEach(options) { option in
                            Text("\(option.label) — \(option.balanceText)")
                                .tag(Optional(FundingSelection.account(id: option.persistentId)))
                                .accessibilityIdentifier("topup.fundingSource.account.\(option.accountIndex)")
                        }
                    }
                    .accessibleFormPicker("topup.fundingSourcePicker")
                    .onChange(of: fundingSelection) { _, newValue in
                        amountDash = defaultAmountString(for: newValue)
                    }
                }
            } header: {
                Text("Funding Source")
            } footer: {
                Text(
                    "Pick a Platform Payment account on the identity's wallet. "
                    + "The chosen account's addresses fund the top-up; their "
                    + "private keys sign the funding contributions."
                )
            }
        }
    }

    @ViewBuilder
    private var amountSection: some View {
        if let account = selectedPlatformAccount {
            Section {
                HStack {
                    TextField("Amount", text: $amountDash)
                        .keyboardType(.decimalPad)
                        .textFieldStyle(.roundedBorder)
                        .disabled(isToppingUp)
                    Text("DASH")
                        .foregroundColor(.secondary)
                }
            } header: {
                Text("Amount")
            } footer: {
                let available = Self.formatDash(
                    raw: accountBalance(account),
                    divisor: Double(Self.creditsPerDash)
                )
                Text("Available: \(available). The amount entered here will be added to the identity's credit balance.")
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isToppingUp {
                        ProgressView()
                            .controlSize(.small)
                            .tint(.white)
                        Text("Topping Up…")
                    } else {
                        Text("Top Up Balance")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isToppingUp)
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Top-up complete", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let newBalance {
                    HStack {
                        Text("New balance:")
                            .foregroundColor(.secondary)
                        Text(Self.formatDash(
                            raw: newBalance,
                            divisor: Double(Self.creditsPerDash)
                        ))
                        .fontWeight(.medium)
                        .monospacedDigit()
                    }
                }
                Button {
                    dismiss()
                } label: {
                    Text("Done")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Derived state

    private var canSubmit: Bool {
        guard
            identity.wallet?.walletId != nil,
            let account = selectedPlatformAccount,
            let credits = parsedAmountCredits
        else {
            return false
        }
        return credits > 0 && credits <= accountBalance(account)
    }

    private var walletLabel: String {
        guard let wallet = identity.wallet else { return "—" }
        let trimmed = wallet.label.trimmingCharacters(in: .whitespaces)
        return trimmed.isEmpty ? shortWalletId(wallet.walletId) : trimmed
    }

    private var selectedPlatformAccount: PersistentAccount? {
        guard
            case .account(let persistentId) = fundingSelection,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            }),
            account.accountType == 14
        else {
            return nil
        }
        return account
    }

    private func accountBalance(_ account: PersistentAccount) -> UInt64 {
        account.platformAddresses.reduce(0) { $0 + $1.balance }
    }

    private func defaultAmountString(for funding: FundingSelection?) -> String {
        guard
            case .account(let persistentId) = funding,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            }),
            account.accountType == 14
        else {
            return ""
        }
        let balance = accountBalance(account)
        if balance == 0 { return "" }
        let dash = Double(balance) / Double(Self.creditsPerDash)
        return String(format: "%g", dash)
    }

    private var parsedAmountCredits: UInt64? {
        let trimmed = amountDash.trimmingCharacters(in: .whitespaces)
        guard let dash = Double(trimmed), dash.isFinite, dash > 0 else {
            return nil
        }
        let credits = (dash * Double(Self.creditsPerDash)).rounded()
        guard credits >= 1, credits <= Double(UInt64.max) else { return nil }
        return UInt64(credits)
    }

    // MARK: - Submit

    private func submit() {
        guard
            let account = selectedPlatformAccount,
            let targetCredits = parsedAmountCredits,
            targetCredits > 0,
            let walletId = identity.wallet?.walletId
        else {
            submitError = .init(message: "Selection is incomplete.")
            return
        }
        guard let managedWallet = walletManager.wallet(for: walletId) else {
            submitError = .init(message: "The identity's wallet isn't loaded. Try restoring it from the wallets list first.")
            return
        }

        // Same KeychainSigner pattern as `CreateIdentityView` — the
        // address signer trampoline derives each input address's
        // private key on demand from `(mnemonic, derivation_path)`
        // and zeroes the buffer immediately. No bytes leave Rust.
        let signer = KeychainSigner(modelContainer: modelContext.container)

        let inputs = buildInputs(
            from: account,
            targetCredits: targetCredits
        )
        guard !inputs.isEmpty else {
            submitError = .init(message: "No funded Platform addresses available on this account.")
            return
        }

        isToppingUp = true
        let identityId = identity.identityId

        Task {
            do {
                let balance = try await managedWallet.topUpFromAddresses(
                    identityId: identityId,
                    inputs: inputs,
                    addressSigner: signer
                )

                await MainActor.run {
                    // The Rust persister callback writes the new
                    // balance onto the `PersistentIdentity` row, so
                    // the parent view updates automatically via
                    // SwiftData @Query. Stash a copy here for the
                    // success banner so we don't depend on the row
                    // re-fetch landing before this UI swap.
                    self.newBalance = balance
                    self.isToppingUp = false
                }
            } catch {
                await MainActor.run {
                    self.submitError = .init(
                        message: error.localizedDescription
                    )
                    self.isToppingUp = false
                }
            }
        }
    }

    private func buildInputs(
        from account: PersistentAccount,
        targetCredits: UInt64
    ) -> [ManagedPlatformWallet.IdentityAddressInput] {
        let candidates = account.platformAddresses
            .filter { $0.balance > 0 }
            .sorted { $0.balance > $1.balance }

        var remaining = targetCredits
        var inputs: [ManagedPlatformWallet.IdentityAddressInput] = []
        for addr in candidates {
            guard remaining > 0 else { break }
            let spend = min(addr.balance, remaining)
            inputs.append(
                ManagedPlatformWallet.IdentityAddressInput(
                    addressType: addr.addressType,
                    hash: addr.addressHash,
                    credits: spend
                )
            )
            remaining -= spend
        }
        return remaining == 0 ? inputs : []
    }

    // MARK: - Helpers

    private func shortWalletId(_ walletId: Data) -> String {
        let prefix = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return prefix.isEmpty ? "Wallet" : "Wallet \(prefix)…"
    }

    private func accountOptions(for walletId: Data) -> [FundingAccountOption] {
        allAccounts
            .filter { account in
                guard account.wallet.walletId == walletId else { return false }
                guard account.accountType == 14 else { return false }
                return account.platformAddresses.contains { $0.balance > 0 }
            }
            .sorted { $0.accountIndex < $1.accountIndex }
            .map { account in
                let balance = account.platformAddresses.reduce(0) { $0 + $1.balance }
                return FundingAccountOption(
                    persistentId: account.persistentModelID,
                    accountIndex: account.accountIndex,
                    label: "\(account.accountTypeName) #\(account.accountIndex)",
                    balanceText: Self.formatDash(
                        raw: balance,
                        divisor: Double(Self.creditsPerDash)
                    )
                )
            }
    }

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
}

// MARK: - Selection types

private enum FundingSelection: Hashable {
    case account(id: PersistentIdentifier)
}

private struct FundingAccountOption: Identifiable {
    let persistentId: PersistentIdentifier
    /// HD account index, carried only so the accessibility identifier
    /// for this row can be stable across launches (the
    /// `PersistentIdentifier` opaque id is not).
    let accountIndex: UInt32
    let label: String
    let balanceText: String
    var id: PersistentIdentifier { persistentId }
}

private struct SubmitError: Identifiable {
    let id = UUID()
    let message: String
}
