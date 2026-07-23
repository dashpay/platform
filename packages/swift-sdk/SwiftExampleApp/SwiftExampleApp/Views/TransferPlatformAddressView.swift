// TransferPlatformAddressView.swift
// SwiftExampleApp
//
// Production (wallet-signed) UI for ADDR-02: transfer credits between
// Platform (DIP-17) addresses. Mirrors the shape of
// `FundFromAssetLockPlatformAddressView` (Source → Destination → Amount
// → Submit) and drives `ManagedPlatformAddressWallet.transfer(...)`
// end-to-end with a `KeychainSigner`.
//
// No private keys are ever entered here. Input selection (Auto),
// the `Σ inputs == Σ outputs` balancing, fee strategy, nonce
// selection, and signing all happen inside the Rust `platform-wallet`
// crate via the FFI wrapper — the only thing this view decides is the
// source account, the amount, and the destination address. The
// credit-balance model leaves surplus on the source addresses, so
// there is no change address to pick. Contrast with the raw
// `TransferAddressFundsView` debug form, which pastes a 64-char
// private key.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct TransferPlatformAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    /// Wallet whose DIP-17 platform-payment accounts/addresses this
    /// transfer operates on.
    let wallet: PersistentWallet

    @Query private var allAccounts: [PersistentAccount]
    @Query private var allPlatformAddresses: [PersistentPlatformAddress]

    // MARK: - Selection state

    private enum DestinationMode: String, CaseIterable, Identifiable {
        case ownWallet = "My Wallet"
        case external = "External"
        var id: String { rawValue }
    }

    @State private var sourceAccountIndex: UInt32? = nil
    @State private var destinationMode: DestinationMode = .ownWallet
    /// Selected own-wallet recipient (20-byte hash) when mode == .ownWallet.
    @State private var selectedRecipientHash: Data? = nil
    /// Pasted/scanned external recipient hash, 40 hex chars (20 bytes).
    @State private var externalHashHex: String = ""
    @State private var amountDash: String = "0.0001"

    /// Per-input minimum credit amount (`min_input_amount`) the chain
    /// enforces for address-funds transitions, resolved from the wallet's
    /// current platform version via
    /// `ManagedPlatformAddressWallet.minInputAmount()` once on appear. The
    /// Rust Auto selector drops any funded address below this floor, so the
    /// per-account total and the submit gate must sum only balances `>=` it
    /// to match the input set Rust will actually consume.
    ///
    /// `nil` until resolved (or if resolution fails). We treat an
    /// unresolved floor as a closed gate (`canSubmit` requires it to be
    /// known) rather than substituting a numeric default: a fallback like
    /// `0` would re-introduce the over-permissive behavior this fixes
    /// (every dust row counted) and let the button enable an op Rust would
    /// reject as dust-only, while hardcoding the `100_000` protocol
    /// constant would violate the no-Swift-mirror rule. The view still
    /// renders fully when it's `nil`; only the spendable total reads `0`
    /// and submit stays disabled until the version-locked floor loads.
    @State private var minInputAmount: UInt64? = nil

    /// Per-output minimum credit amount (`min_output_amount`) the chain
    /// enforces for address-funds transitions, resolved from the wallet's
    /// current platform version via
    /// `ManagedPlatformAddressWallet.minOutputAmount()` once on appear. An
    /// address-funds transfer sends exactly one output, and DPP rejects any
    /// output below this floor (currently 500,000 credits), so a small amount
    /// that clears `parsedCredits > 0` would still fail structure validation
    /// after submit. The submit gate and the amount footer must enforce
    /// `credits >= this` so the button reflects what DPP will accept.
    ///
    /// `nil` until resolved (or if resolution fails). Same safe pattern as
    /// `minInputAmount`: an unresolved floor keeps the gate CLOSED
    /// (`canSubmit` requires it to be known) rather than substituting a
    /// numeric default — a fallback like `0` would re-open the gate for a
    /// sub-minimum amount DPP rejects, and hardcoding the `500_000` protocol
    /// constant would violate the no-Swift-mirror rule. This never
    /// *under*-gates: when unknown, submit simply stays disabled until the
    /// version-locked floor loads.
    @State private var minOutputAmount: UInt64? = nil

    // MARK: - Submit state

    @State private var submitError: SubmitError? = nil
    @State private var isSubmitting = false
    @State private var didSucceed = false
    /// Non-fatal caveat shown on the success screen when the transfer
    /// succeeded on-chain but the local SwiftData balance write failed.
    /// The transfer itself is NOT a failure (the `performSync()` that runs
    /// right after corrects balances regardless), so this must not be
    /// surfaced as `submitError` — but it must not be silently swallowed
    /// either.
    @State private var saveWarning: String? = nil

    /// 1e11 credits per DASH. Matches `CreateIdentityView`. Integer so
    /// the amount→credits conversion is exact — binary floating point
    /// can't represent every credit value at the 1e11 boundary, and a
    /// value-transfer path must not round the user's intended amount.
    private static let creditsPerDash: UInt64 = 100_000_000_000
    /// Number of fractional decimal digits in one DASH worth of credits
    /// (1e11 = 11 zeros). Anything finer than 1e-11 DASH is sub-credit
    /// and rejected rather than truncated.
    private static let creditFractionDigits = 11

    /// UI-only cushion: the Rust Auto path deducts the on-chain fee from
    /// the lex-smallest selected input's remaining balance
    /// (`[DeductFromInput(0)]`), so the source account must hold the
    /// transfer amount PLUS the fee. We hold back this cushion when
    /// gating the submit button so the button isn't enabled for an amount
    /// the account can't actually cover once the fee is taken. The Rust
    /// side computes the exact fee and returns a typed insufficient-
    /// balance error if this estimate is wrong; this is purely to avoid a
    /// dead-on-tap button. Observed fee for a small transfer is ~6.5M
    /// credits; this is intentionally an order of magnitude larger so
    /// estimation drift doesn't surprise the user.
    private static let feeBuffer: UInt64 = 100_000_000

    var body: some View {
        NavigationStack {
            Form {
                if didSucceed {
                    successSection
                } else {
                    walletSection
                    sourceAccountSection
                    destinationSection
                    amountSection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Transfer Platform Credits")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not transfer credits"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear {
                resolveMinInputAmount()
                resolveMinOutputAmount()
                autoSelectDefaults()
            }
            // Block swipe-to-dismiss while a transfer is in flight — only
            // the (disabled) Cancel button otherwise gates it, so a swipe
            // could tear the sheet down mid-submit.
            .interactiveDismissDisabled(isSubmitting)
        }
    }

    // MARK: - Sections

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
                .accessibilityIdentifier("transferPlatform.sourceAccountPicker")
                .onChange(of: sourceAccountIndex) { _, _ in
                    selectedRecipientHash = nil
                    autoSelectRecipient()
                }
            }
        } header: {
            Text("Source Account")
        } footer: {
            Text("Platform Payment account funding the transfer. Picker shows its current credit balance.")
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
            .accessibilityIdentifier("transferPlatform.destinationModePicker")

            switch destinationMode {
            case .ownWallet:
                let options = ownWalletRecipientCandidates
                if options.isEmpty {
                    Text("No other addresses available on this wallet to receive credits. Sync first or add funds.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Picker("Recipient", selection: $selectedRecipientHash) {
                        Text("Select…").tag(Optional<Data>.none)
                        ForEach(options, id: \.addressHash) { row in
                            Text("Addr #\(row.addressIndex) — \(row.address.prefix(12))…")
                                .tag(Optional(row.addressHash))
                        }
                    }
                    .accessibilityIdentifier("transferPlatform.recipientPicker")
                }
            case .external:
                VStack(alignment: .leading, spacing: 4) {
                    TextField("Recipient hash (40 hex chars = 20 bytes)", text: $externalHashHex)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .monospaced()
                        .accessibilityIdentifier("transferPlatform.externalHashField")
                    if !externalHashHex.isEmpty && parsedExternalHash == nil {
                        Text("Enter exactly 40 hexadecimal characters.")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }
        } header: {
            Text("Destination Address")
        } footer: {
            Text("Send to another address on this wallet, or paste a 20-byte P2PKH address hash. Surplus stays on the source addresses — there's no change address to pick.")
        }
    }

    @ViewBuilder
    private var amountSection: some View {
        Section {
            HStack {
                TextField("Amount", text: $amountDash)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(isSubmitting)
                    .accessibilityIdentifier("transferPlatform.amountField")
                Text("DASH")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Amount")
        } footer: {
            if let credits = parsedCredits {
                // Below-minimum takes precedence over the balance check: a tiny
                // amount can clear the balance check yet still be rejected by
                // DPP for falling under `min_output_amount`, so explain that
                // first. Only shown once the floor has resolved (`minOutputAmount`
                // non-nil) so we never claim a minimum we haven't read.
                let available = selectedSourceAccountCredits
                let needed = credits.addingReportingOverflow(Self.feeBuffer)
                if let minOutput = minOutputAmount, credits < minOutput {
                    Text("Minimum transfer is \(formatCredits(minOutput)). Increase the amount to at least that.")
                        .foregroundColor(.red)
                } else if needed.overflow || needed.partialValue > available {
                    Text("Insufficient balance: \(formatCredits(credits)) + fee exceeds the account's \(formatCredits(available)).")
                        .foregroundColor(.red)
                } else {
                    Text("\(formatCredits(credits)) will be transferred (plus a small on-chain fee taken from the source balance).")
                }
            } else {
                Text("Enter an amount in DASH.")
            }
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
                        Text("Transferring…")
                    } else {
                        Text("Transfer")
                    }
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
            .disabled(isSubmitting)
            .accessibilityIdentifier("transferPlatform.submitButton")
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Credits transferred", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                Text("The transfer was submitted and your balances are resyncing.")
                    .font(.callout)
                    .foregroundColor(.secondary)
                if let saveWarning {
                    Label(saveWarning, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundColor(.orange)
                        .accessibilityIdentifier("transferPlatform.saveWarning")
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

    /// Source accounts the transfer can actually spend from.
    ///
    /// Offers every DIP-17 platform-payment account (`accountType == 14`,
    /// key class 0) on this wallet. The Rust `platform-wallet` Auto selector
    /// resolves the chosen `accountIndex` via
    /// `platform_payment_managed_account_at_index(account_index)` (key class 0)
    /// and spends from that account, so the source matches the `accountIndex`
    /// the transfer persists/nonces against — the picker is multi-account,
    /// matching the withdraw flow.
    private var platformAccountOptions: [PlatformAccountOption] {
        // Spendable threshold: a funded address can only be an input if its
        // balance reaches the chain's `min_input_amount`. When the floor
        // hasn't resolved yet (`nil`), `UInt64.max` makes every row dust so
        // the spendable total is 0 and the submit gate stays closed — we
        // never count an unknown-floor balance as spendable. See the
        // `minInputAmount` doc comment for why we don't fall back to a
        // numeric default.
        let threshold = minInputAmount ?? UInt64.max
        let accounts = allAccounts
            .filter { $0.wallet.walletId == wallet.walletId }
            .filter { $0.accountType == 14 && $0.keyClass == 0 }
            .sorted { $0.accountIndex < $1.accountIndex }
        return accounts.map { acct in
            // Sum only addresses whose parent account is key class 0
            // (`account?.keyClass == 0`) AND whose balance clears the
            // per-input minimum (`balance >= threshold`). Rust's Auto
            // selector drops sub-`min_input_amount` dust before selecting
            // inputs (and returns `OnlyDustInputs` if nothing clears it), so
            // counting dust here would inflate the total and let `canSubmit`
            // promise more than Rust will spend — enabling a transfer Rust
            // then refuses. Summing every key-class-0 row regardless of key
            // class would likewise over-count.
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

    /// Funded addresses on the selected source account for this wallet that
    /// the Rust Auto selector could actually consume as inputs.
    /// The `AddressFundsTransferTransition` protocol forbids any output
    /// address from also being an input, and the selector excludes recipient
    /// addresses from its input set, so a recipient that collides with a real
    /// source input would enable the button here, then come up short Rust-side
    /// once that input is excluded. Gate on this set so the collision is caught
    /// up front.
    ///
    /// Floors on `min_input_amount`, NOT `balance > 0`: Rust's Auto selector
    /// only treats an address as a candidate input when its balance reaches
    /// `min_input_amount` (`build_auto_select_candidates` drops everything
    /// below it). A dust source-account address is therefore NOT an input, so
    /// sending TO it is structurally fine — excluding it on the old `> 0`
    /// floor wrongly removed legitimate dust recipients from the picker and
    /// rejected them as pasted externals. We use the same resolved
    /// `minInputAmount` the spendable-total/submit gate reads so this set
    /// matches the input set Rust will actually consume.
    ///
    /// When `minInputAmount` is unresolved (`nil`) we fall back to the prior
    /// `balance > 0` floor: with an unknown per-input minimum we cannot tell
    /// dust from a real input, so we conservatively treat every funded row as
    /// a possible input rather than risk UNDER-excluding (and offering a real
    /// input as a recipient). The submit gate is independently closed while
    /// `minInputAmount == nil`, so this only affects which recipients the
    /// picker offers.
    ///
    /// Scoped to DIP-17 platform-payment accounts at key class 0
    /// (`account?.accountType == 14 && account?.keyClass == 0`), matching
    /// `platformAccountOptions` and `selectedSourceAccountCredits`: Rust
    /// resolves the source via
    /// `platform_payment_managed_account_at_index(accountIndex)` (key
    /// class 0, account type 14) and only spends those rows, so a sibling
    /// row at the same `accountIndex` with a different account type or key
    /// class is not an input. Including it here would wrongly drop it as a
    /// destination candidate, blocking legitimate own-wallet/pasted
    /// recipients on multi-account-type / multi-key-class wallets.
    private var sourceInputHashes: Set<Data> {
        guard let acctIdx = sourceAccountIndex else { return [] }
        // Match Rust's candidate floor: an address is a possible input only
        // when its balance reaches `min_input_amount`. With the floor
        // unresolved, fall back to `> 0` so we never UNDER-exclude a real
        // input (offering it as a recipient would let Rust come up short).
        let isPossibleInput: (PersistentPlatformAddress) -> Bool = { addr in
            if let floor = minInputAmount {
                return addr.balance >= floor
            }
            return addr.balance > 0
        }
        return Set(
            allPlatformAddresses
                .filter {
                    $0.walletId == wallet.walletId
                        && $0.accountIndex == acctIdx
                        && isPossibleInput($0)
                        && $0.account?.accountType == 14
                        && $0.account?.keyClass == 0
                }
                .map { $0.addressHash }
        )
    }

    /// Own-wallet recipients: any address on the wallet that is NOT a
    /// funded source-account input. We surface unused (zero-balance)
    /// addresses on any platform-payment account so the user can send to a
    /// fresh address; the Rust Auto selector excludes recipients from its
    /// input set (DPP forbids the same address as both input and output),
    /// so a recipient that collides with a funded source input would be
    /// dropped from selection — we exclude those here so the button isn't
    /// enabled for a recipient Rust would refuse to fund against.
    ///
    /// Restricted to P2PKH rows (`addressType == 0`): the transfer FFI's
    /// `PlatformAddressFFI → PlatformAddress` conversion accepts P2PKH
    /// only (the P2PKH-only contract established earlier in this PR), so a
    /// persisted P2SH (`addressType == 1`) own-wallet row would parse here
    /// but only fail after submit. Filtering it out keeps the picker in
    /// step with what Rust will actually accept.
    private var ownWalletRecipientCandidates: [PersistentPlatformAddress] {
        let inputs = sourceInputHashes
        return allPlatformAddresses
            .filter { $0.walletId == wallet.walletId }
            .filter { $0.addressType == 0 }
            .filter { !inputs.contains($0.addressHash) }
            .sorted { ($0.accountIndex, $0.addressIndex) < ($1.accountIndex, $1.addressIndex) }
    }

    /// Parse the pasted external hash (40 hex chars → 20 bytes).
    private var parsedExternalHash: Data? {
        let raw = externalHashHex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard raw.count == 40 else { return nil }
        var bytes = [UInt8]()
        bytes.reserveCapacity(20)
        var idx = raw.startIndex
        while idx < raw.endIndex {
            let next = raw.index(idx, offsetBy: 2)
            guard let b = UInt8(raw[idx..<next], radix: 16) else { return nil }
            bytes.append(b)
            idx = next
        }
        return Data(bytes)
    }

    /// Resolved destination (addressType, 20-byte hash) for the current mode.
    private var resolvedDestination: (addressType: UInt8, hash: Data)? {
        switch destinationMode {
        case .ownWallet:
            // Scope by walletId AND hash: a hash-only lookup can match
            // another wallet's row in a multi-wallet store and route the
            // transfer to the wrong wallet's address.
            guard let hash = selectedRecipientHash,
                let row = allPlatformAddresses.first(where: {
                    $0.walletId == wallet.walletId && $0.addressHash == hash
                }),
                // P2PKH only: the transfer FFI rejects `addressType == 1`
                // (P2SH). `ownWalletRecipientCandidates` already filters
                // these out of the picker, but guard here too so a stale
                // `selectedRecipientHash` can't resolve to a P2SH row.
                row.addressType == 0
            else { return nil }
            return (row.addressType, row.addressHash)
        case .external:
            guard let hash = parsedExternalHash else { return nil }
            // External pasted hashes are treated as P2PKH (type 0).
            return (0, hash)
        }
    }

    /// Exact decimal→credits conversion. The amount is a value-transfer
    /// quantity, so it must NOT pass through `Double` (binary FP can't
    /// represent every credit value at the 1e11 boundary and would round
    /// the user's intended amount). Parse the decimal string directly:
    /// split on ".", require digits only, ≤11 fractional digits, then
    /// `whole * 1e11 + fractionalPaddedTo11` with overflow rejection.
    /// Returns nil for any malformed / zero / overflowing input.
    private var parsedCredits: UInt64? {
        let raw = amountDash.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !raw.isEmpty else { return nil }

        let parts = raw.split(separator: ".", omittingEmptySubsequences: false)
        guard parts.count <= 2 else { return nil }
        let wholeStr = String(parts.first ?? "")
        let fracStr = parts.count == 2 ? String(parts[1]) : ""

        // Reject empty/sign/non-digit components. An empty whole part is
        // allowed only when there's a fractional part (".5" → "0.5").
        guard wholeStr.allSatisfy(\.isNumber), fracStr.allSatisfy(\.isNumber) else { return nil }
        guard !(wholeStr.isEmpty && fracStr.isEmpty) else { return nil }
        guard fracStr.count <= Self.creditFractionDigits else { return nil }

        let whole = wholeStr.isEmpty ? 0 : UInt64(wholeStr)
        guard wholeStr.isEmpty || whole != nil else { return nil }

        // whole * 1e11
        let scaled = (whole ?? 0).multipliedReportingOverflow(by: Self.creditsPerDash)
        guard !scaled.overflow else { return nil }

        // Pad the fractional part to 11 digits so it expresses credits
        // directly, then add. (".5" → "50000000000" credits.)
        let paddedFrac = fracStr.padding(
            toLength: Self.creditFractionDigits, withPad: "0", startingAt: 0
        )
        guard let fracCredits = paddedFrac.isEmpty ? 0 : UInt64(paddedFrac) else { return nil }

        let total = scaled.partialValue.addingReportingOverflow(fracCredits)
        guard !total.overflow, total.partialValue > 0 else { return nil }
        return total.partialValue
    }

    private var canSubmit: Bool {
        guard
            !isSubmitting,
            // The per-input minimum must be known before we can promise the
            // account covers the transfer: `selectedSourceAccountCredits`
            // sums only balances ≥ this floor, and an unresolved floor makes
            // that figure 0. Keep the gate closed until it loads rather than
            // gating on an unknown/over-permissive spendable total.
            minInputAmount != nil,
            // The per-OUTPUT minimum must also be known before we enable
            // submit: an address-funds transfer sends one output, and DPP
            // rejects any output below `min_output_amount`. An unresolved
            // floor keeps the gate closed (never *under*-gates) rather than
            // letting a sub-minimum amount through to a post-submit failure.
            let minOutput = minOutputAmount,
            sourceAccountIndex != nil,
            let credits = parsedCredits, credits > 0,
            // The single output must reach `min_output_amount` or DPP rejects
            // the transition after submit — gate on it up front so the button
            // isn't enabled for an amount the chain will refuse.
            credits >= minOutput,
            let dest = resolvedDestination
        else { return false }
        // Reject a recipient that collides with a funded source input.
        // The Rust Auto selector excludes recipients from its input set,
        // so a recipient on a funded source input would be dropped from
        // selection and the transfer could come up short Rust-side.
        // Covers both own-wallet picks and pasted externals.
        if sourceInputHashes.contains(dest.hash) { return false }
        // Gate on amount + fee cushion <= account balance. The Auto path
        // deducts the on-chain fee from the source balance, so the account
        // must cover amount + fee; this is a conservative UI gate (Rust
        // computes the exact fee and rejects an over-spend with a typed
        // error).
        let needed = credits.addingReportingOverflow(Self.feeBuffer)
        if needed.overflow { return false }
        return selectedSourceAccountCredits >= needed.partialValue
    }

    // MARK: - Actions

    /// Resolve the chain's per-input minimum (`min_input_amount`) once from
    /// the wallet's current platform version (version-locked, read on the
    /// Rust side). Called on appear. On any failure we leave
    /// `minInputAmount == nil`, which keeps the spendable total at 0 and the
    /// submit gate closed — a deliberately conservative fallback that never
    /// *under*-gates (see the `minInputAmount` doc comment).
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

    /// Resolve the chain's per-output minimum (`min_output_amount`) once from
    /// the wallet's current platform version (version-locked, read on the
    /// Rust side). Called on appear. On any failure we leave
    /// `minOutputAmount == nil`, which keeps the submit gate closed until a
    /// later appearance resolves it — the same conservative fallback as
    /// `resolveMinInputAmount`, which never *under*-gates.
    private func resolveMinOutputAmount() {
        guard minOutputAmount == nil else { return }
        guard let managedHolder = walletManager.wallet(for: wallet.walletId) else { return }
        do {
            let addressWallet = try managedHolder.platformAddressWallet()
            minOutputAmount = try addressWallet.minOutputAmount()
        } catch {
            // Leave nil: gate stays closed until a later appearance resolves it.
            minOutputAmount = nil
        }
    }

    private func autoSelectDefaults() {
        if sourceAccountIndex == nil {
            sourceAccountIndex = platformAccountOptions
                .first(where: { $0.totalCredits > 0 })?.accountIndex
                ?? platformAccountOptions.first?.accountIndex
        }
        autoSelectRecipient()
    }

    private func autoSelectRecipient() {
        if destinationMode == .ownWallet && selectedRecipientHash == nil {
            selectedRecipientHash = ownWalletRecipientCandidates.first?.addressHash
        }
    }

    private func submit() {
        guard !isSubmitting else { return }
        guard
            let sourceAccount = sourceAccountIndex,
            let credits = parsedCredits,
            let dest = resolvedDestination
        else { return }

        guard !sourceInputHashes.contains(dest.hash) else {
            submitError = SubmitError(
                message: "The destination is a funded address on the source account, which the transfer uses as an input. Pick a different recipient."
            )
            return
        }

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
        let outputs = [
            ManagedPlatformAddressWallet.TransferOutput(
                addressType: dest.addressType,
                hash: dest.hash,
                credits: credits
            )
        ]

        isSubmitting = true
        Task {
            defer { isSubmitting = false }
            do {
                let updated = try await addressWallet.transfer(
                    accountIndex: sourceAccount,
                    outputs: outputs,
                    signer: signer
                )
                // The transfer has ALREADY succeeded on-chain here.
                // Persist the post-transfer balances Rust reported BEFORE
                // the resync so SwiftData doesn't show spent inputs as
                // spendable in the gap before `performSync()` catches up.
                // Mirrors the BLAST persister callback's upsert shape.
                //
                // A local save failure must NOT mark the transfer as failed
                // (it succeeded; `performSync()` below corrects balances
                // regardless) — but it must not be swallowed either. Surface
                // it as a non-fatal caveat on the success screen rather than
                // the hard error alert.
                do {
                    try persistUpdatedBalances(updated)
                } catch {
                    saveWarning = "Submitted successfully, but local balances "
                        + "couldn't be updated — they'll refresh on the next "
                        + "sync: \(error.localizedDescription)"
                }
                // Trigger a DIP-17 resync so balances + the unused-
                // address pool catch up after the transfer.
                await platformBalanceSyncService.performSync()
                didSucceed = true
            } catch {
                submitError = SubmitError(message: error.localizedDescription)
            }
        }
    }

    /// Apply the per-address `UpdatedBalance`s from a transfer's Rust
    /// changeset to the matching `PersistentPlatformAddress` rows. Scoped
    /// to this wallet and matched by 20-byte `addressHash`, mirroring the
    /// BLAST `persistAddressBalances` callback so the row state is
    /// consistent whether it lands from here or from the next sync round.
    ///
    /// Throws the SwiftData `save()` error to the caller rather than
    /// swallowing it with `try?`. The caller has already confirmed the
    /// on-chain transfer succeeded, so it routes this to a non-fatal
    /// caveat (NOT the failure path) — the transfer stands and the next
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
        // Display only — the Double divide here never feeds a transfer
        // amount, so the FP imprecision the parse path avoids is fine.
        let dash = Double(credits) / Double(Self.creditsPerDash)
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
