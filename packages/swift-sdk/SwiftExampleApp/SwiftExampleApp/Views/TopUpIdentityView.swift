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

import Combine
import SwiftUI
import SwiftDashSDK
import SwiftData

/// Submit gate for a shielded-pool top-up (Type 22): amount > 0, the
/// minimum fee resolved, the spendable credits known, and amount + minimum
/// fee within them (overflow-checked). The fee is spent from the notes on
/// top of the amount. The wallet may charge more than the minimum when it
/// combines more than two notes; it then refuses before building anything.
enum ShieldedTopUpAmountGate {
    static func canSubmit(amount: UInt64?, minimumFee: UInt64?, spendable: UInt64?) -> Bool {
        guard let amount, amount > 0, let minimumFee, let spendable else { return false }
        let (total, overflow) = amount.addingReportingOverflow(minimumFee)
        return !overflow && total <= spendable
    }
}

/// A shielded top-up exactly as the user authorized it at the tap. Captured
/// synchronously before any await, so later edits to the form (a different
/// funding source replacing the amount text, for example) cannot change
/// what is sent.
struct ShieldedTopUpRequest: Equatable {
    let walletId: Data
    let identityId: Data
    let amount: UInt64

    /// The request the form describes now, or `nil` when the form is not a
    /// complete shielded top-up (another funding source, no wallet, or an
    /// amount that does not parse to more than zero credits).
    static func capture(
        fundingIsShielded: Bool,
        amountText: String,
        walletId: Data?,
        identityId: Data
    ) -> ShieldedTopUpRequest? {
        guard fundingIsShielded,
              let walletId,
              let amount = parseTokenAmount(amountText, decimals: 11),
              amount > 0
        else { return nil }
        return ShieldedTopUpRequest(walletId: walletId, identityId: identityId, amount: amount)
    }

    /// Whether this captured request may be sent against the balance and
    /// fee read after the capture.
    func isAffordable(minimumFee: UInt64?, spendable: UInt64?) -> Bool {
        ShieldedTopUpAmountGate.canSubmit(amount: amount, minimumFee: minimumFee, spendable: spendable)
    }
}

/// Whether a captured top-up may be sent, decided from the spendable
/// balance the wallet pushes for the captured wallet, read at submit time.
/// An unknown balance never lets a payment through.
enum ShieldedTopUpSubmitCheck: Equatable {
    case send
    case refuse(String)

    static func decide(
        request: ShieldedTopUpRequest,
        minimumFee: UInt64?,
        pushedSpendable: UInt64?
    ) -> ShieldedTopUpSubmitCheck {
        guard let spendable = pushedSpendable else {
            return .refuse("The shielded balance is not known right now, so nothing was sent.")
        }
        guard request.isAffordable(minimumFee: minimumFee, spendable: spendable) else {
            return .refuse("The amount plus the minimum fee must fit the spendable "
                + "shielded balance. Nothing was sent.")
        }
        return .send
    }

    /// Result text after the wallet confirmed the spent notes. The proof does
    /// not show that this transition added the amount, so it is "requested".
    static func completedText(amount: UInt64) -> String {
        "The spent notes were confirmed. Requested: \(formatCreditsAsDash(amount))."
    }

    /// `pendingListedBelow`: this identity has pending top-ups shown under
    /// this result. None of them is claimed to be this submission (two can
    /// have the same amount), and without any the text promises no entry.
    static func unconfirmedText(amount: UInt64, pendingListedBelow: Bool) -> String {
        let warning = "The top-up of \(formatCreditsAsDash(amount)) could not be "
            + "confirmed. Do not retry: it may already be on chain. The wallet "
            + "checks it on each shielded sync."
        if pendingListedBelow {
            return warning + " Pending top-ups for this identity are listed below."
        }
        return warning
    }

    /// Wallet refusals of a top-up with wording for this screen; `nil` for
    /// any other error, whose own description is shown.
    static func refusalMessage(_ error: PlatformWalletError) -> String? {
        switch error {
        case .protocolVersionNotActive:
            return "This network has not activated topping up an identity from "
                + "the shielded pool yet. Nothing was sent."
        case .sdkPinnedBelowProtocolVersion:
            return "This app's SDK is pinned below the protocol version that tops "
                + "up an identity from the shielded pool. Nothing was sent."
        default:
            return nil
        }
    }
}

/// What the top-up screen knows about account 0's spendable shielded
/// credits, from the balance the wallet pushes
/// (`PlatformWalletManager.shieldedBalances`). Notes reserved for a pending
/// spend are already excluded by the wallet. "Unknown" is kept distinct
/// from a known zero: an unknown balance never offers the shielded source.
enum ShieldedSpendableCredits: Equatable {
    /// No known amount. `reason` is shown to the user when present; `nil`
    /// means nothing worth saying (no update yet, or no shielded pool).
    case unknown(reason: String?)
    case known(UInt64)

    /// The state for the wallet's latest pushed update, if any.
    static func from(_ update: ShieldedBalanceUpdate?) -> ShieldedSpendableCredits {
        guard let update else { return .unknown(reason: nil) }
        if case .unbound = update.state { return .unknown(reason: nil) }
        if let credits = update.accountBalance(0) { return .known(credits) }
        return .unknown(reason: "The shielded balance is not known yet.")
    }

    var credits: UInt64? {
        if case .known(let credits) = self { return credits }
        return nil
    }

    var offersShieldedSource: Bool { (credits ?? 0) > 0 }

    /// Whether a shielded funding selection must be dropped: the source is
    /// no longer offered and no submission is running (a running one keeps
    /// its selection and is rechecked when it ends).
    func dropsShieldedSelection(isToppingUp: Bool) -> Bool {
        !isToppingUp && !offersShieldedSource
    }

    var unavailableReason: String? {
        if case .unknown(let reason) = self { return reason }
        return nil
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

    /// This identity's shielded top-ups (kind 9) still pending, so an
    /// interrupted or unconfirmed one is visible when the screen reopens.
    @Query private var pendingShieldedTopUps: [PersistentShieldedActivity]

    init(identity: PersistentIdentity) {
        self.identity = identity
        // An identity without a wallet matches no rows.
        let walletId = identity.wallet?.walletId ?? Data()
        let identityId = identity.identityId
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

    /// Minimum fee of a shielded top-up (`.identityTopUpFromPool` at 2
    /// actions; the wallet prices max(notes selected, 2) actions), resolved
    /// on appear and again at submit. `nil` keeps submit closed.
    @State private var shieldedTopUpFee: UInt64? = nil


    // MARK: - Submit state

    @State private var isToppingUp: Bool = false
    @State private var submitError: SubmitError? = nil
    @State private var newBalance: UInt64? = nil
    @State private var shieldedOutcome: ShieldedTopUpOutcome? = nil
    @State private var checkStatus: ShieldedCheckNowStatus = .idle

    var body: some View {
        NavigationStack {
            Form {
                if newBalance != nil {
                    successSection
                } else if let shieldedOutcome {
                    shieldedOutcomeSection(shieldedOutcome)
                    if case .mayHaveGoneThrough = shieldedOutcome {
                        pendingShieldedTopUpsSection
                    }
                } else {
                    walletSection
                    pendingShieldedTopUpsSection
                    fundingSection
                    amountSection
                    // Kept while submitting: the spend reserves notes, so the
                    // pushed balance can close the gate mid-proof.
                    if canSubmit || isToppingUp {
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
            // The balance is pushed by the wallet; only the fee is resolved
            // here, again after a sync of this wallet (the network's version
            // can have moved).
            .onReceive(walletManager.$lastShieldedSyncEvent.dropFirst()) { event in
                if let walletId = identity.wallet?.walletId,
                   ShieldedSyncEventScope.covers(event, walletId: walletId) {
                    resolveShieldedTopUpFee()
                }
            }
            // A manager swapped during an SDK rebuild: re-resolve against it
            // so a sheet opened meanwhile recovers without being dismissed.
            .onChange(of: ObjectIdentifier(walletManager)) { _, _ in
                resolveShieldedTopUpFee()
            }
            // Drop a shielded selection the wallet can no longer fund, and
            // recheck when a submission that kept it ends.
            .onChange(of: shieldedSpendable) { _, _ in recheckShieldedSelection() }
            .onChange(of: isToppingUp) { _, _ in recheckShieldedSelection() }
            // A swipe-dismissed and reopened sheet would reset `isToppingUp`
            // and allow a second top-up while the first is still proving.
            .interactiveDismissDisabled(isToppingUp)
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
                Text(formatIdentityBalanceAsDash(identity.balance))
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
            let showShielded = shieldedSpendable.offersShieldedSource
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
                            Text("Shielded Balance: \(formatCreditsAsDash(shieldedSpendable.credits ?? 0))")
                                .tag(Optional(FundingSelection.shieldedBalance))
                                .accessibilityIdentifier("topup.fundingSource.shielded")
                        }
                    }
                    .accessibleFormPicker("topup.fundingSourcePicker")
                    // Locked while a top-up runs: picking another source
                    // replaces the amount text.
                    .disabled(isToppingUp)
                    .onChange(of: fundingSelection) { _, newValue in
                        amountDash = defaultAmountString(for: newValue)
                    }
                }
                if let reason = shieldedSpendable.unavailableReason {
                    Text("Shielded Balance unavailable. \(reason)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .accessibilityIdentifier("topup.fundingSource.shieldedUnavailable")
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
    /// reserved and checks them on each shielded sync.
    @ViewBuilder
    private var pendingShieldedTopUpsSection: some View {
        if !pendingShieldedTopUps.isEmpty {
            Section {
                ForEach(pendingShieldedTopUps, id: \.persistentModelID) { entry in
                    HStack {
                        Label("Top-up from shielded pool", systemImage: "clock")
                        Spacer()
                        Text(formatCreditsAsDash(entry.amount))
                        .monospacedDigit()
                    }
                    .accessibilityIdentifier("topup.shielded.pendingRow")
                }
                Button {
                    Task {
                        checkStatus = .checking
                        checkStatus = .after(await shieldedService.manualSync())
                    }
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
                ShieldedCheckNowStatusLine(status: checkStatus)
            } header: {
                Text("Pending Shielded Top-Ups")
            } footer: {
                Text(
                    "Not confirmed. They may already have gone through, so do "
                    + "not submit them again. The wallet checks them on each "
                    + "shielded sync."
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
                "Spendable shielded",
                value: shieldedSpendable.credits.map { formatCreditsAsDash($0) } ?? "Unknown"
            )
            LabeledContent(
                "Fee",
                value: shieldedTopUpFee.map {
                    "at least " + formatCreditsAsDash($0)
                } ?? "Not available"
            )
            .accessibilityIdentifier("topup.shielded.fee")
        } header: {
            Text("Amount")
        } footer: {
            Text(
                "The fee is spent from the shielded notes on top of the amount, "
                + "so amount plus fee must fit the spendable shielded balance. "
                + "The fee is more than the minimum if the wallet has to combine "
                + "more than two notes. Building the proof takes tens of seconds."
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
    /// resubmit straight away. The wallet's proof covers the spent notes and
    /// the identity's balance at a block, not that this transition added the
    /// amount, so the identity balance is shown on its own line.
    private func shieldedOutcomeSection(_ outcome: ShieldedTopUpOutcome) -> some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                switch outcome {
                case .completed(let amount):
                    Label("Top-up submitted", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(ShieldedTopUpSubmitCheck.completedText(amount: amount))
                        .foregroundColor(.secondary)
                case .mayHaveGoneThrough(let amount):
                    Label("May have gone through", systemImage: "clock.badge.questionmark")
                        .foregroundColor(.orange)
                        .font(.headline)
                    Text(ShieldedTopUpSubmitCheck.unconfirmedText(
                        amount: amount,
                        pendingListedBelow: !pendingShieldedTopUps.isEmpty
                    ))
                        .foregroundColor(.secondary)
                }
                HStack {
                    Text("Last saved identity balance:")
                        .foregroundColor(.secondary)
                    // Exact, so a one-credit change is visible.
                    Text(formatIdentityBalanceAsDash(identity.balance))
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
            // The same capture the tap uses, so the button and the send agree.
            return ShieldedTopUpRequest.capture(
                fundingIsShielded: true,
                amountText: amountDash,
                walletId: identity.wallet?.walletId,
                identityId: identity.identityId
            )?.isAffordable(
                minimumFee: shieldedTopUpFee,
                spendable: shieldedSpendable.credits
            ) ?? false
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

    /// Account 0's spendable shielded credits as the wallet pushes them for
    /// the identity's wallet. Re-renders when an update is published.
    private var shieldedSpendable: ShieldedSpendableCredits {
        guard let walletId = identity.wallet?.walletId else { return .unknown(reason: nil) }
        return .from(walletManager.shieldedBalances[walletId])
    }

    private func recheckShieldedSelection() {
        if fundingSelection == .shieldedBalance,
           shieldedSpendable.dropsShieldedSelection(isToppingUp: isToppingUp) {
            fundingSelection = nil
        }
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
        // Capture what the user authorized before any await; only the
        // captured values are used below, whatever the form shows later.
        guard let request = ShieldedTopUpRequest.capture(
            fundingIsShielded: fundingSelection == .shieldedBalance,
            amountText: amountDash,
            walletId: identity.wallet?.walletId,
            identityId: identity.identityId
        ) else {
            submitError = .init(message: "Selection is incomplete.")
            return
        }
        // Re-check the captured request against the fee and the balance the
        // wallet has pushed for the captured wallet, right now.
        resolveShieldedTopUpFee()
        if case .refuse(let message) = ShieldedTopUpSubmitCheck.decide(
            request: request,
            minimumFee: shieldedTopUpFee,
            pushedSpendable: walletManager.spendableShieldedCredits(
                walletId: request.walletId,
                account: 0
            )
        ) {
            submitError = .init(message: message)
            return
        }
        // Set before the async call so a second tap cannot start another
        // top-up, and so the funding picker and amount field lock.
        isToppingUp = true

        Task {
            let amount = request.amount
            do {
                try await walletManager.shieldedIdentityTopUpFromPool(
                    walletId: request.walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    identityId: request.identityId,
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
                let message = (error as? PlatformWalletError)
                    .flatMap(ShieldedTopUpSubmitCheck.refusalMessage)
                    ?? error.localizedDescription
                await MainActor.run {
                    self.submitError = .init(message: message)
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
