// WithdrawPlatformAddressView.swift
// SwiftExampleApp
//
// Production (wallet-signed) UI for ADDR-04: withdraw a Platform
// payment account's credits back to a Core L1 address. Mirrors
// `FundFromAssetLockPlatformAddressView`'s shape and drives
// `ManagedPlatformAddressWallet.withdraw(accountIndex:coreAddress:
// coreFeePerByte:signer:)` with a `KeychainSigner`.
//
// Withdrawals consume the FULL funded balance of the account (no
// per-address amount, no change output), so this view shows the
// computed total rather than an amount field. The Core destination
// can be one of the wallet's own receive addresses ("My Wallet") or a
// pasted/scanned external address; the address is network-checked on
// the Rust side. No private keys are entered here — contrast with the
// raw `WithdrawAddressFundsView` debug form.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct WithdrawPlatformAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    let wallet: PersistentWallet

    @Query private var allAccounts: [PersistentAccount]
    @Query private var allPlatformAddresses: [PersistentPlatformAddress]

    // MARK: - Selection state

    private enum DestinationMode: String, CaseIterable, Identifiable {
        case myWallet = "My Wallet"
        case external = "External"
        var id: String { rawValue }
    }

    @State private var sourceAccountIndex: UInt32? = nil
    @State private var destinationMode: DestinationMode = .myWallet
    /// Core L1 address derived from this wallet's Core receive pool
    /// (mode == .myWallet). Resolved lazily in `resolveMyWalletAddress`.
    @State private var myWalletAddress: String? = nil
    /// Pasted/scanned external Core address (mode == .external).
    @State private var externalAddress: String = ""
    /// Selected Core L1 fee rate (duffs/byte). Constrained by the picker
    /// to a protocol-valid value (see `validFeeRates`); defaults to 1.
    @State private var coreFeePerByte: UInt32 = 1

    /// Per-input minimum credit amount (`min_input_amount`) the chain
    /// enforces for address-funds transitions, resolved from the wallet's
    /// current platform version via
    /// `ManagedPlatformAddressWallet.minInputAmount()` once on appear. The
    /// Rust withdraw selector (`select_withdrawable_inputs`) keeps only
    /// addresses whose balance reaches this floor and returns
    /// `OnlyDustInputs` when none do, so the "Total to Withdraw" figure and
    /// the submit gate must sum only balances `>=` it to reflect the
    /// *withdrawable* balance Rust will actually take.
    ///
    /// `nil` until resolved (or if resolution fails). We treat an
    /// unresolved floor as a closed gate (`canSubmit` requires it to be
    /// known) rather than substituting a numeric default: a fallback like
    /// `0` would re-introduce the over-permissive behavior this fixes (every
    /// dust row counted) and let the button enable a dust-only withdrawal
    /// Rust would reject, while hardcoding the `100_000` protocol constant
    /// would violate the no-Swift-mirror rule. The view still renders fully
    /// when it's `nil`; only the withdrawable total reads `0` and submit
    /// stays disabled until the version-locked floor loads.
    @State private var minInputAmount: UInt64? = nil

    // MARK: - Core readiness

    /// nil = not yet checked, true/false = Core wallet usable.
    @State private var coreReady: Bool? = nil
    @State private var coreNotReadyReason: String? = nil

    // MARK: - Submit state

    @State private var submitError: SubmitError? = nil
    @State private var isSubmitting = false
    @State private var didSucceed = false
    /// Non-fatal caveat shown on the success screen when the withdrawal
    /// succeeded on-chain but the local SwiftData balance write failed.
    /// The withdrawal itself is NOT a failure (the `performSync()` that
    /// runs right after corrects balances regardless), so this must not be
    /// surfaced as `submitError` — but it must not be silently swallowed
    /// either.
    @State private var saveWarning: String? = nil

    private static let creditsPerDash: Double = 100_000_000_000.0

    /// Upper bound on the Core L1 fee rate (duffs/byte). The normal rate
    /// is 1; even heavy congestion rarely exceeds a few hundred. Because a
    /// withdrawal is full-balance with the fee deducted from inputs, a
    /// fat-fingered rate could eat the entire payout, so we cap well above
    /// any legitimate manual override (10_000 = 10,000× the default) while
    /// still rejecting obviously destructive values. This is an app-side
    /// ceiling only — the protocol imposes no upper bound.
    private static let maxFeePerByte: UInt32 = 10_000

    /// The Core fee rates the protocol accepts, offered by the picker.
    ///
    /// DPP's `AddressCreditWithdrawalTransitionV0::validate_structure`
    /// rejects any `core_fee_per_byte` that is not a NON-ZERO Fibonacci
    /// number, so non-Fibonacci rates (4, 6, 7, 9, 10, 100, …)
    /// deterministically fail structure validation on submit. The set is
    /// generated by `WithdrawalCoreFeeRates` (a Fibonacci walk that mirrors
    /// the validator) and capped at the app-side `maxFeePerByte` ceiling.
    private static let validFeeRates: [UInt32] =
        WithdrawalCoreFeeRates.rates(upTo: maxFeePerByte)

    var body: some View {
        NavigationStack {
            Form {
                if didSucceed {
                    successSection
                } else if coreReady == false {
                    coreNotReadySection
                } else {
                    walletSection
                    sourceAccountSection
                    destinationSection
                    feeSection
                    summarySection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Withdraw to Core")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not withdraw"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear {
                checkCoreReady()
                resolveMinInputAmount()
                autoSelectDefaults()
            }
            .onChange(of: destinationMode) { _, mode in
                if mode == .myWallet { resolveMyWalletAddress() }
            }
            // Block swipe-to-dismiss while a withdrawal is in flight —
            // only the (disabled) Cancel button otherwise gates it, so a
            // swipe could tear the sheet down mid-submit.
            .interactiveDismissDisabled(isSubmitting)
        }
    }

    // MARK: - Sections

    private var coreNotReadySection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Core wallet not ready", systemImage: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                    .font(.headline)
                Text(coreNotReadyReason
                    ?? "The Core (SPV) wallet must be initialized before you can withdraw to an L1 address. Sync the Core wallet and try again.")
                    .font(.callout)
                    .foregroundColor(.secondary)
                Button("Close") { dismiss() }
                    .padding(.top, 4)
            }
        }
        .accessibilityIdentifier("withdrawPlatform.coreNotReadySection")
    }

    private var walletSection: some View {
        Section {
            HStack {
                Label("Wallet", systemImage: "wallet.pass")
                Spacer()
                Text(wallet.name ?? hexShort(wallet.walletId))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Source")
        }
    }

    @ViewBuilder
    private var sourceAccountSection: some View {
        let options = platformAccountOptions
        Section {
            if options.isEmpty {
                Text("No DIP-17 Platform Payment accounts on this wallet yet.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker("Source Account", selection: $sourceAccountIndex) {
                    Text("Select…").tag(Optional<UInt32>.none)
                    ForEach(options, id: \.accountIndex) { opt in
                        Text("Account #\(opt.accountIndex) — \(formatCredits(opt.totalCredits))")
                            .tag(Optional(opt.accountIndex))
                    }
                }
                .accessibilityIdentifier("withdrawPlatform.sourceAccountPicker")
            }
        } header: {
            Text("Source Account")
        } footer: {
            Text("The full credit balance of this account is withdrawn — there is no partial amount.")
        }
    }

    @ViewBuilder
    private var destinationSection: some View {
        Section {
            Picker("Destination", selection: $destinationMode) {
                ForEach(DestinationMode.allCases) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("withdrawPlatform.destinationModePicker")

            switch destinationMode {
            case .myWallet:
                if let addr = myWalletAddress {
                    HStack {
                        Label("Receive Address", systemImage: "arrow.down.circle")
                        Spacer()
                        Text("\(addr.prefix(10))…\(addr.suffix(6))")
                            .font(.system(.body, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                } else {
                    Text("Resolving a Core receive address…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            case .external:
                VStack(alignment: .leading, spacing: 4) {
                    TextField("Core L1 address", text: $externalAddress)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .monospaced()
                        .accessibilityIdentifier("withdrawPlatform.externalAddressField")
                    Text("The address is validated for this network on submit.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        } header: {
            Text("Core L1 Destination")
        } footer: {
            Text("Withdraw to one of your own Core receive addresses, or paste an external Core address.")
        }
    }

    @ViewBuilder
    private var feeSection: some View {
        Section {
            Picker("Fee per byte", selection: $coreFeePerByte) {
                ForEach(Self.validFeeRates, id: \.self) { rate in
                    Text("\(rate) duffs/byte")
                        .tag(rate)
                        .accessibilityIdentifier("withdrawPlatform.feeRate.\(rate)")
                }
            }
            .disabled(isSubmitting)
            .accessibleFormPicker("withdrawPlatform.feePerBytePicker")
        } header: {
            Text("Core Fee Rate")
        } footer: {
            Text("Fee rate for the eventual L1 payout transaction. The protocol only accepts non-zero Fibonacci rates (1, 2, 3, 5, 8, …), so the picker offers exactly those. Default is 1.")
        }
    }

    private var summarySection: some View {
        Section {
            HStack {
                Label("Total to Withdraw", systemImage: "dollarsign.circle")
                Spacer()
                Text(formatCredits(selectedSourceAccountCredits))
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Summary")
        } footer: {
            Text("The platform-side fee is deducted from these inputs. The full remaining balance is converted to Core duffs and paid out on L1 (minus the L1 fee).")
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView()
                        Text("Withdrawing…")
                    } else {
                        Text("Withdraw")
                    }
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
            .disabled(isSubmitting)
            .accessibilityIdentifier("withdrawPlatform.submitButton")
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Withdrawal submitted", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                Text("The withdrawal was submitted. Credits will arrive on L1 once the payout is processed; balances are resyncing.")
                    .font(.callout)
                    .foregroundColor(.secondary)
                if let saveWarning {
                    Label(saveWarning, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundColor(.orange)
                        .accessibilityIdentifier("withdrawPlatform.saveWarning")
                }
                Button {
                    dismiss()
                } label: {
                    Text("Done").frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Derived

    private struct PlatformAccountOption {
        let accountIndex: UInt32
        let totalCredits: UInt64
    }

    /// Source accounts: only DIP-17 PlatformPayment (`accountType == 14`)
    /// accounts on the **default key class** (`keyClass == 0`). The Rust
    /// `platform-wallet` crate resolves the withdraw source via
    /// `platform_payment_managed_account_at_index(account_index)` = key
    /// class 0, so a key-class-other account at the same index would never
    /// be the spent source. Mirrors `TransferPlatformAddressView`.
    ///
    /// The displayed per-account balance sums only addresses whose parent
    /// account is key class 0 (`account?.keyClass == 0`) AND whose balance
    /// clears the chain's per-input minimum (`balance >= threshold`). The
    /// Rust withdraw selector keeps only inputs that reach
    /// `min_input_amount` and withdraws that *withdrawable* balance (dropping
    /// sub-minimum dust, or failing with `OnlyDustInputs` if none clear it),
    /// so this is the figure actually paid out. Summing every key-class-0
    /// row regardless of key class or balance would inflate the total and
    /// let `canSubmit` enable a withdrawal Rust then refuses as dust-only.
    private var platformAccountOptions: [PlatformAccountOption] {
        // Withdrawable threshold: an address can only be a withdrawal input
        // if its balance reaches the chain's `min_input_amount`. When the
        // floor hasn't resolved yet (`nil`), `UInt64.max` makes every row
        // dust so the withdrawable total is 0 and the submit gate stays
        // closed — we never count an unknown-floor balance as withdrawable.
        // See the `minInputAmount` doc comment for why we don't fall back to
        // a numeric default.
        let threshold = minInputAmount ?? UInt64.max
        let accounts = allAccounts
            .filter { $0.wallet.walletId == wallet.walletId }
            .filter { $0.accountType == 14 && $0.keyClass == 0 }
            .sorted { $0.accountIndex < $1.accountIndex }
        return accounts.map { acct in
            let total = allPlatformAddresses
                .filter {
                    $0.walletId == wallet.walletId
                        && $0.accountIndex == acct.accountIndex
                        && $0.account?.keyClass == 0
                        && $0.balance >= threshold
                }
                .reduce(into: UInt64(0)) { acc, addr in acc &+= addr.balance }
            return PlatformAccountOption(accountIndex: acct.accountIndex, totalCredits: total)
        }
    }

    private var selectedSourceAccountCredits: UInt64 {
        guard let idx = sourceAccountIndex else { return 0 }
        return platformAccountOptions.first(where: { $0.accountIndex == idx })?.totalCredits ?? 0
    }

    /// The fee rate to submit. The picker constrains `coreFeePerByte` to a
    /// protocol-valid (non-zero Fibonacci) value within the app ceiling, so
    /// this is always non-nil; kept Optional so `canSubmit`/`submit()` read
    /// unchanged and stay robust if the binding is ever widened.
    private var parsedFeePerByte: UInt32? {
        Self.validFeeRates.contains(coreFeePerByte) ? coreFeePerByte : nil
    }

    /// Resolved Core destination address for the current mode.
    private var resolvedCoreAddress: String? {
        switch destinationMode {
        case .myWallet:
            return myWalletAddress
        case .external:
            let trimmed = externalAddress.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : trimmed
        }
    }

    private var canSubmit: Bool {
        guard
            !isSubmitting,
            coreReady == true,
            // The per-input minimum must be known before we can promise the
            // account has anything withdrawable: `selectedSourceAccountCredits`
            // sums only balances ≥ this floor, and an unresolved floor makes
            // that figure 0. The `> 0` check below already closes the gate in
            // that case; this makes the dependency explicit.
            minInputAmount != nil,
            sourceAccountIndex != nil,
            // Require the dust-FILTERED (withdrawable) total > 0, not the raw
            // total: the Rust selector returns `OnlyDustInputs` when no
            // address clears `min_input_amount`, so a purely-dust account
            // (raw balance > 0) must not enable the button.
            selectedSourceAccountCredits > 0,
            parsedFeePerByte != nil,
            let addr = resolvedCoreAddress, !addr.isEmpty
        else { return false }
        return true
    }

    // MARK: - Actions

    private func autoSelectDefaults() {
        if sourceAccountIndex == nil {
            sourceAccountIndex = platformAccountOptions
                .first(where: { $0.totalCredits > 0 })?.accountIndex
                ?? platformAccountOptions.first?.accountIndex
        }
        if destinationMode == .myWallet {
            resolveMyWalletAddress()
        }
    }

    /// Gate the whole flow on the Core (SPV) wallet being usable.
    ///
    /// This must be a NON-mutating probe: it runs on every sheet open, so
    /// anything with a side effect would churn wallet state just from the
    /// user glancing at the sheet. `coreWallet()` (`platform_wallet_get_core`)
    /// already throws if the Core side isn't initialized, and `network()` is
    /// a lock-free read that succeeds whenever the handle is live — together
    /// they confirm the Core wallet is acquirable and answering without
    /// touching the BIP-44 receive pool.
    ///
    /// The earlier implementation probed `nextReceiveAddress`, but that FFI
    /// passes `advance = true` (`CoreWallet::next_receive_address_for_account`,
    /// rs-platform-wallet/src/wallet/core/wallet.rs:103), so every readiness
    /// check ADVANCED the external pool — opening the sheet repeatedly burned
    /// receive addresses. The Core wallet FFI surface has no non-advancing
    /// "peek" or "is account present" call, so we gate on `network()` here
    /// and only consume an address when the user actually needs a My-Wallet
    /// destination (see `resolveMyWalletAddress`, which caches its one fetch).
    private func checkCoreReady() {
        guard let managedHolder = walletManager.wallet(for: wallet.walletId) else {
            coreReady = false
            coreNotReadyReason = "Wallet handle not found in the wallet manager."
            return
        }
        do {
            let core = try managedHolder.coreWallet()
            _ = try core.network()
            coreReady = true
        } catch {
            coreReady = false
            coreNotReadyReason = "Core wallet is not ready: \(error.localizedDescription)"
        }
    }

    /// Resolve the chain's per-input minimum (`min_input_amount`) once from
    /// the wallet's current platform version (version-locked, read on the
    /// Rust side). Called on appear. On any failure we leave
    /// `minInputAmount == nil`, which keeps the withdrawable total at 0 and
    /// the submit gate closed — a deliberately conservative fallback that
    /// never *under*-gates (see the `minInputAmount` doc comment).
    private func resolveMinInputAmount() {
        guard minInputAmount == nil else { return }
        guard let managedHolder = walletManager.wallet(for: wallet.walletId) else { return }
        do {
            let addressWallet = try managedHolder.platformAddressWallet()
            minInputAmount = try addressWallet.minInputAmount()
        } catch {
            // Leave nil: gate stays closed until a later appearance resolves it.
            minInputAmount = nil
        }
    }

    /// Resolve a Core receive address for the "My Wallet" destination and
    /// cache it in `myWalletAddress` for the sheet's lifetime.
    ///
    /// `core.nextReceiveAddress(accountIndex:)` ADVANCES the BIP-44 external
    /// pool (`advance = true` on the Rust side), so it must be called at most
    /// once per sheet session and only when the user actually needs a
    /// My-Wallet destination — never as a readiness probe (see
    /// `checkCoreReady`). The `myWalletAddress == nil` guard makes repeated
    /// calls (e.g. toggling the destination segment back to My Wallet)
    /// no-ops, so open/cancel/toggle consumes exactly one receive address,
    /// not one per interaction. Only invoked from `autoSelectDefaults`
    /// (when the default My-Wallet mode is active) and the destination-mode
    /// `onChange` when switching back to My Wallet.
    private func resolveMyWalletAddress() {
        guard myWalletAddress == nil else { return }
        guard let managedHolder = walletManager.wallet(for: wallet.walletId) else { return }
        do {
            let core = try managedHolder.coreWallet()
            myWalletAddress = try core.nextReceiveAddress(accountIndex: 0)
        } catch {
            // Leave nil; the destination section shows the resolving
            // placeholder and Core-readiness gating handles the rest.
            myWalletAddress = nil
        }
    }

    private func submit() {
        guard !isSubmitting else { return }
        guard
            let sourceAccount = sourceAccountIndex,
            let feePerByte = parsedFeePerByte,
            let coreAddress = resolvedCoreAddress
        else { return }

        let managedHolder = walletManager.wallet(for: wallet.walletId)
        guard let managedHolder else {
            submitError = SubmitError(message: "Wallet handle not found in the wallet manager.")
            return
        }
        let addressWallet: ManagedPlatformAddressWallet
        do {
            addressWallet = try managedHolder.platformAddressWallet()
        } catch {
            submitError = SubmitError(message: "Couldn't acquire platform-address wallet: \(error.localizedDescription)")
            return
        }

        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            defer { isSubmitting = false }
            do {
                let updated = try await addressWallet.withdraw(
                    accountIndex: sourceAccount,
                    coreAddress: coreAddress,
                    coreFeePerByte: feePerByte,
                    signer: signer
                )
                // The withdrawal has ALREADY succeeded on-chain here.
                // Persist the drained balances Rust just reported BEFORE
                // the resync so SwiftData stops showing the consumed
                // inputs as spendable in the gap before `performSync()`
                // catches up. Mirrors the BLAST persister callback's
                // upsert shape (`persistAddressBalances`).
                //
                // A local save failure must NOT mark the withdrawal as
                // failed (it succeeded; `performSync()` below corrects
                // balances regardless) — but it must not be swallowed
                // either. Surface it as a non-fatal caveat on the success
                // screen rather than the hard error alert.
                do {
                    try persistUpdatedBalances(updated)
                } catch {
                    saveWarning = "Submitted successfully, but local balances "
                        + "couldn't be updated — they'll refresh on the next "
                        + "sync: \(error.localizedDescription)"
                }
                await platformBalanceSyncService.performSync()
                didSucceed = true
            } catch {
                submitError = SubmitError(message: error.localizedDescription)
            }
        }
    }

    /// Apply the per-address `UpdatedBalance`s from a withdrawal's Rust
    /// changeset to the matching `PersistentPlatformAddress` rows. Scoped
    /// to this wallet and matched by 20-byte `addressHash`, mirroring the
    /// BLAST `persistAddressBalances` callback so the row state is
    /// consistent whether it lands from here or from the next sync round.
    ///
    /// Throws the SwiftData `save()` error to the caller rather than
    /// swallowing it with `try?`. The caller has already confirmed the
    /// on-chain withdrawal succeeded, so it routes this to a non-fatal
    /// caveat (NOT the failure path) — the withdrawal stands and the next
    /// sync reconciles balances regardless.
    private func persistUpdatedBalances(
        _ updated: [ManagedPlatformAddressWallet.UpdatedBalance]
    ) throws {
        guard !updated.isEmpty else { return }
        let walletId = wallet.walletId
        for entry in updated {
            let hash = entry.hash
            let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                predicate: #Predicate {
                    $0.walletId == walletId && $0.addressHash == hash
                }
            )
            guard let row = try? modelContext.fetch(descriptor).first else { continue }
            row.balance = entry.balance
            row.nonce = entry.nonce
            if entry.balance > 0 || entry.nonce > 0 {
                row.isUsed = true
            }
            row.lastUpdated = Date()
        }
        try modelContext.save()
    }

    // MARK: - Helpers

    private func formatCredits(_ credits: UInt64) -> String {
        let dash = Double(credits) / Self.creditsPerDash
        return String(format: "%.6f DASH", dash)
    }

    private func hexShort(_ data: Data) -> String {
        let hex = data.map { String(format: "%02x", $0) }.joined()
        return hex.count > 12 ? "\(hex.prefix(6))…\(hex.suffix(6))" : hex
    }
}

// MARK: - Submit error wrapper

private struct SubmitError: Identifiable {
    let id = UUID()
    let message: String
}
