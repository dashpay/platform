// TopUpIdentityView.swift
// SwiftExampleApp
//
// Stepped UI for topping up an existing identity's credit balance.
// Mirrors `CreateIdentityView`'s shape but locks the wallet to the
// identity's owning wallet and skips the identity-pubkey / index
// sections — top-up is just a funding choice + amount, signed by the
// input addresses' private keys (no IdentityCreate to register).
//
// A second funding source, "Shielded Balance", spends notes from the
// wallet's shielded pool (Type 22 IdentityTopUpFromShieldedPool) for a
// free-form amount.

import SwiftUI
import SwiftDashSDK
import SwiftData

/// Submit gate for a shielded-pool top-up (Type 22): amount > 0, the flat
/// fee resolved, and amount + fee within the pool balance (overflow-checked).
/// The fee is spent from the notes on top of the amount.
enum ShieldedTopUpAmountGate {
    static func canSubmit(amount: UInt64?, fee: UInt64?, poolBalance: UInt64) -> Bool {
        guard let amount, amount > 0, let fee else { return false }
        let (total, overflow) = amount.addingReportingOverflow(fee)
        return !overflow && total <= poolBalance
    }
}

struct TopUpIdentityView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var shieldedService: ShieldedService

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

    /// The identity's wallet's unspent account-0 shielded notes.
    @Query private var shieldedNotes: [PersistentShieldedNote]

    /// This identity's shielded top-ups (kind 9) still pending, so an
    /// interrupted or unconfirmed one is visible when the screen reopens.
    @Query private var pendingShieldedTopUps: [PersistentShieldedActivity]

    init(identity: PersistentIdentity) {
        self.identity = identity
        // An identity without a wallet matches no rows.
        let walletId = identity.wallet?.walletId ?? Data()
        let identityId = identity.identityId
        _shieldedNotes = Query(
            filter: SendTransactionView.shieldedSourcePredicate(walletId: walletId)
        )
        _pendingShieldedTopUps = Query(
            filter: #Predicate<PersistentShieldedActivity> {
                $0.walletId == walletId && $0.kindTag == 9 && $0.status == 0
                    && $0.identityId == identityId
            },
            sort: [SortDescriptor(\PersistentShieldedActivity.createdAtMs, order: .reverse)]
        )
    }

    // MARK: - Selection state

    @State private var fundingSelection: FundingSelection? = nil
    @State private var amountDash: String = ""

    /// Flat fee of a shielded top-up (`.identityTopUpFromPool`, 2 actions),
    /// resolved on appear and again at submit. `nil` keeps submit closed.
    @State private var shieldedTopUpFee: UInt64? = nil

    // MARK: - Submit state

    @State private var isToppingUp: Bool = false
    @State private var submitError: SubmitError? = nil
    @State private var newBalance: UInt64? = nil
    @State private var shieldedOutcome: ShieldedTopUpOutcome? = nil

    var body: some View {
        NavigationStack {
            Form {
                if newBalance != nil {
                    successSection
                } else if let shieldedOutcome {
                    shieldedOutcomeSection(shieldedOutcome)
                } else {
                    walletSection
                    pendingShieldedTopUpsSection
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
            .onAppear { resolveShieldedTopUpFee() }
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
            let showShielded = shieldedOptionAvailable(for: walletId)
            Section {
                if options.isEmpty && !showShielded {
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
                        if showShielded {
                            Text("Shielded Balance: \(Self.formatDash(raw: shieldedPoolBalance, divisor: Double(Self.creditsPerDash)))")
                                .tag(Optional(FundingSelection.shieldedBalance))
                                .accessibilityIdentifier("topup.fundingSource.shielded")
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
                Text(Self.fundingFooterText(showShielded: showShielded))
            }
        }
    }

    private static func fundingFooterText(showShielded: Bool) -> String {
        var text = "Pick a Platform Payment account on the identity's wallet. "
            + "The chosen account's addresses fund the top-up; their "
            + "private keys sign the funding contributions."
        if showShielded {
            text += " Shielded Balance spends notes from this wallet's "
                + "shielded pool instead; no identity key signs."
        }
        return text
    }

    /// Pending shielded top-ups of this identity. Rust keeps their notes
    /// reserved and resolves them on a later shielded sync.
    @ViewBuilder
    private var pendingShieldedTopUpsSection: some View {
        if !pendingShieldedTopUps.isEmpty {
            Section {
                ForEach(pendingShieldedTopUps, id: \.persistentModelID) { entry in
                    HStack {
                        Label("Top-up from shielded pool", systemImage: "clock")
                        Spacer()
                        Text(Self.formatDash(
                            raw: entry.amount,
                            divisor: Double(Self.creditsPerDash)
                        ))
                        .monospacedDigit()
                    }
                    .accessibilityIdentifier("topup.shielded.pendingRow")
                }
                Button {
                    Task { await shieldedService.manualSync() }
                } label: {
                    HStack {
                        Label("Check Now", systemImage: "arrow.clockwise")
                        if shieldedService.isSyncing {
                            Spacer()
                            ProgressView().controlSize(.small)
                        }
                    }
                }
                .disabled(shieldedService.isSyncing || !shieldedService.canResume)
                .accessibilityIdentifier("topup.shielded.checkNow")
            } header: {
                Text("Pending Shielded Top-Ups")
            } footer: {
                Text(
                    "Not confirmed yet. They may already have gone through, so "
                    + "do not submit them again. The wallet resolves them on a "
                    + "later shielded sync."
                )
            }
        }
    }

    @ViewBuilder
    private var amountSection: some View {
        if fundingSelection == .shieldedBalance {
            shieldedAmountSection
        } else if let account = selectedPlatformAccount {
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

    /// Free-form amount for the shielded path. Not prefilled with the
    /// whole balance: the fee is spent from the notes on top of it.
    private var shieldedAmountSection: some View {
        Section {
            HStack {
                TextField("Amount", text: $amountDash)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(isToppingUp)
                    .accessibilityIdentifier("topup.shielded.amount")
                Text("DASH")
                    .foregroundColor(.secondary)
            }
            LabeledContent(
                "Shielded balance",
                value: Self.formatDash(raw: shieldedPoolBalance, divisor: Double(Self.creditsPerDash))
            )
            LabeledContent(
                "Estimated fee",
                value: shieldedTopUpFee.map {
                    Self.formatDash(raw: $0, divisor: Double(Self.creditsPerDash))
                } ?? "Not available"
            )
            .accessibilityIdentifier("topup.shielded.fee")
        } header: {
            Text("Amount")
        } footer: {
            Text(
                "The identity receives the amount. The fee is spent from the "
                + "shielded notes on top of it, so amount plus fee must fit the "
                + "shielded balance. Building the proof takes tens of seconds."
            )
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                if fundingSelection == .shieldedBalance {
                    submitShielded()
                } else {
                    submit()
                }
            } label: {
                HStack {
                    if isToppingUp {
                        ProgressView()
                            .controlSize(.small)
                            .tint(.white)
                        Text(fundingSelection == .shieldedBalance
                             ? "Building proof, this takes tens of seconds…"
                             : "Topping Up…")
                    } else {
                        Text("Top Up Balance")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isToppingUp)
            .accessibilityIdentifier("topup.submit")
        }
    }

    /// Result of a shielded top-up. Replaces the form so the user cannot
    /// resubmit straight away; the call returns no balance, so this shows
    /// the amount and the identity row's balance as the persister updates it.
    private func shieldedOutcomeSection(_ outcome: ShieldedTopUpOutcome) -> some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                switch outcome {
                case .completed(let amount):
                    Label("Top-up complete", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text("Added \(Self.formatDash(raw: amount, divisor: Double(Self.creditsPerDash))) from the shielded pool.")
                        .foregroundColor(.secondary)
                case .mayHaveGoneThrough(let amount):
                    Label("May have gone through", systemImage: "clock.badge.questionmark")
                        .foregroundColor(.orange)
                        .font(.headline)
                    Text(
                        "The top-up of \(Self.formatDash(raw: amount, divisor: Double(Self.creditsPerDash))) "
                        + "could not be confirmed. Do not retry: it may already be on "
                        + "chain. It is listed as pending on this screen and in "
                        + "Shielded Activity until a later shielded sync resolves it."
                    )
                    .foregroundColor(.secondary)
                }
                HStack {
                    Text("Identity balance:")
                        .foregroundColor(.secondary)
                    Text(identity.formattedBalance)
                        .fontWeight(.medium)
                        .monospacedDigit()
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
            .accessibilityIdentifier("topup.shielded.outcome")
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
        if fundingSelection == .shieldedBalance {
            return identity.wallet?.walletId != nil
                && ShieldedTopUpAmountGate.canSubmit(
                    amount: parsedShieldedAmountCredits,
                    fee: shieldedTopUpFee,
                    poolBalance: shieldedPoolBalance
                )
        }
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

    /// Whether "Shielded Balance" is offered: the wallet is bound to the
    /// shielded pool (its default Orchard address resolves) and holds
    /// unspent account-0 notes.
    private func shieldedOptionAvailable(for walletId: Data) -> Bool {
        let engineBound = (try? walletManager.shieldedDefaultAddress(
            walletId: walletId,
            account: 0
        )) != nil
        return engineBound && shieldedPoolBalance > 0
    }

    /// Sum of the identity's wallet's unspent account-0 notes.
    private var shieldedPoolBalance: UInt64 {
        shieldedNotes.reduce(0) { $0 + $1.value }
    }

    /// Shielded amount in credits through the exact `Decimal` parser.
    private var parsedShieldedAmountCredits: UInt64? {
        parseTokenAmount(amountDash, decimals: 11)
    }

    private func resolveShieldedTopUpFee() {
        shieldedTopUpFee = try? walletManager.estimateShieldedFee(
            kind: .identityTopUpFromPool,
            numActions: 2
        )
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

    /// Type 22 top-up from the identity's wallet's shielded pool. Note
    /// selection, the fee and the proof are all Rust-side; Rust also
    /// applies the proven balance to the identity row through the persister.
    private func submitShielded() {
        // Re-read the fee at submit, as the Send screen does.
        resolveShieldedTopUpFee()
        guard
            let walletId = identity.wallet?.walletId,
            let amount = parsedShieldedAmountCredits,
            ShieldedTopUpAmountGate.canSubmit(
                amount: amount,
                fee: shieldedTopUpFee,
                poolBalance: shieldedPoolBalance
            )
        else {
            submitError = .init(message: "The amount plus the estimated fee must fit the shielded balance.")
            return
        }

        isToppingUp = true
        let identityId = identity.identityId

        Task {
            do {
                try await walletManager.shieldedIdentityTopUpFromPool(
                    walletId: walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    identityId: identityId,
                    amount: amount
                )
                await MainActor.run {
                    self.shieldedOutcome = .completed(amount: amount)
                    self.isToppingUp = false
                }
            } catch let unconfirmed as PlatformWalletError
                        where SendViewModel.mayHaveGoneThrough(unconfirmed) {
                // Rust keeps the notes reserved and a pending activity row;
                // a retry would sign a second top-up.
                await MainActor.run {
                    self.shieldedOutcome = .mayHaveGoneThrough(amount: amount)
                    self.isToppingUp = false
                }
            } catch {
                await MainActor.run {
                    self.submitError = .init(message: error.localizedDescription)
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
    case shieldedBalance
}

private enum ShieldedTopUpOutcome {
    case completed(amount: UInt64)
    case mayHaveGoneThrough(amount: UInt64)
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
