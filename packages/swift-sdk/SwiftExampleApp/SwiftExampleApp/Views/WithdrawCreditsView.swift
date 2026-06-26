// WithdrawCreditsView.swift
// SwiftExampleApp
//
// Withdraw Platform credits from an identity to an L1 Dash address.
// The source is fixed to the `identity` this flow opened from; the
// destination is a network-aware L1 Dash address the user types in.
// Structurally mirrors `TransferCreditsView` (DASH-amount entry,
// `creditsPerDash` divisor, NavigationStack + Cancel toolbar, submit
// guard + `submitGeneration`, target + success sections) — the only
// material difference is a destination-address `TextField` in place of
// the recipient identity picker.
//
// All orchestration lives in Rust: this view only parses the amount,
// validates it against the source's cached balance, lightly sanity-
// checks the address (non-empty), then hands a fresh `KeychainSigner`
// to `ManagedPlatformWallet.withdrawCredits`. Authoritative Base58 +
// network validation happens in the FFI/Rust layer — any rejection is
// surfaced in the error UI. On success the Rust persister callback
// deducts this identity's `PersistentIdentity.balance`, so the parent
// view's `@Query` refreshes the displayed balance automatically — this
// view returns nothing from the SDK call.
//
// On the L1 side there is NO immediate transaction id: Platform
// withdrawals are pooled and processed asynchronously by the network,
// and the withdraw FFI path returns only the void / new balance. The
// success screen therefore shows the destination address + amount plus
// a note that the L1 payout is processed asynchronously, rather than a
// (non-existent) txid.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct WithdrawCreditsView: View {
    /// Identity the credits are withdrawn from. The owning wallet (and
    /// thus the signer + the FFI handle) is derived from `identity.wallet`.
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// Credits per DASH (1e11) — same divisor `TransferCreditsView` and
    /// `PersistentIdentity.formattedBalance` use for credit amounts.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    // MARK: - Selection state

    @State private var toAddress: String = ""
    @State private var amountDash: String = ""

    // MARK: - Submit state

    @State private var isSubmitting = false
    @State private var submitError: SubmitError?
    @State private var didComplete = false
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance
    /// after the user pops + repushes mid-broadcast. Mirrors
    /// `TransferCreditsView.submitGeneration`.
    @State private var submitGeneration = 0

    private struct SubmitError: Identifiable {
        let id = UUID()
        let message: String
    }

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    successSection
                } else {
                    targetSection
                    addressSection
                    amountSection
                    submitSection
                }
            }
            .navigationTitle("Withdraw Credits")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Withdrawal failed"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
        }
        // Block swipe-to-dismiss while the withdrawal is in flight — the
        // Cancel button is already disabled, but interactive dismissal
        // would otherwise let the user drop the sheet mid-broadcast,
        // reopen it, and fire a second withdrawal on this write path.
        .interactiveDismissDisabled(isSubmitting)
    }

    // MARK: - Sections

    private var targetSection: some View {
        Section {
            HStack {
                Label("Identity", systemImage: "person.text.rectangle")
                Spacer()
                Text(identity.displayName)
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
            Text("From")
        }
    }

    @ViewBuilder
    private var addressSection: some View {
        Section {
            if managedWallet != nil {
                TextField("Dash address", text: $toAddress)
                    .textFieldStyle(.roundedBorder)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled(true)
                    .disabled(isSubmitting)
                if !trimmedAddress.isEmpty && !isValidDestinationAddress {
                    Text("Not a valid \(identity.network.displayName) Dash address.")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            } else {
                Text("The wallet that owns this identity isn't loaded.")
                    .font(.subheadline)
                    .foregroundColor(.red)
            }
        } header: {
            Text("Destination L1 Address")
        } footer: {
            Text("Enter a Dash (Layer 1) address to receive the withdrawn funds. The address is validated against this wallet's network when you submit.")
        }
    }

    private var amountSection: some View {
        Section {
            HStack {
                TextField("Amount", text: $amountDash)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(isSubmitting)
                Text("DASH")
                    .foregroundColor(.secondary)
            }
            if let credits = parsedCredits, credits > senderBalanceCredits {
                Text("Amount exceeds your balance.")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        } header: {
            Text("Amount")
        } footer: {
            Text("Available: \(identity.formattedBalance). The amount entered here is deducted from this identity's credit balance and paid out to the destination address on Layer 1.")
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
                            .controlSize(.small)
                            .tint(.white)
                        Text("Withdrawing…")
                    } else {
                        Text("Withdraw Credits")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isSubmitting)
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Withdrawal submitted", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let credits = parsedCredits {
                    HStack {
                        Text("Withdrawn:")
                            .foregroundColor(.secondary)
                        Text(Self.formatDash(raw: credits))
                            .fontWeight(.medium)
                            .monospacedDigit()
                    }
                }
                HStack(alignment: .top) {
                    Text("To:")
                        .foregroundColor(.secondary)
                    Text(trimmedAddress)
                        .lineLimit(2)
                        .truncationMode(.middle)
                        .monospaced()
                }
                // Platform withdrawals are pooled and broadcast to L1 by
                // the network asynchronously, so there is no immediate
                // L1 transaction id to show here. The credit-balance
                // decrease reflects automatically via the
                // persister → SwiftData → @Query path.
                Text("The Layer 1 payout is processed asynchronously by the network and may take a while to appear on-chain.")
                    .font(.caption)
                    .foregroundColor(.secondary)
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

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    /// Destination address with surrounding whitespace removed. The
    /// authoritative Base58 + network check is performed in Rust.
    private var trimmedAddress: String {
        toAddress.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// Parse the user's DASH input and scale it to credits. Mirrors
    /// `TransferCreditsView.parsedCredits`: require a finite, positive
    /// value, round to the nearest credit, and reject any result below
    /// 1 credit or beyond `UInt64.max`.
    private var parsedCredits: UInt64? {
        let trimmed = amountDash.trimmingCharacters(in: .whitespaces)
        guard let dash = Double(trimmed), dash.isFinite, dash > 0 else {
            return nil
        }
        let credits = (dash * Double(Self.creditsPerDash)).rounded()
        // Strict `<`: `Double(UInt64.max)` isn't exactly representable and
        // rounds up to 2^64, so a `<=` bound would admit 2^64 and trap on
        // the `UInt64(credits)` cast (this recomputes on every keystroke,
        // before submit-time re-validation). `<` keeps it in range.
        guard credits >= 1, credits < Double(UInt64.max) else { return nil }
        return UInt64(credits)
    }

    /// Source balance in credits. `PersistentIdentity.balance` is an
    /// `Int64`; clamp any (unexpected) negative value to zero.
    private var senderBalanceCredits: UInt64 {
        identity.balance < 0 ? 0 : UInt64(identity.balance)
    }

    /// Local pre-validation of the destination so the submit button
    /// doesn't light up for obviously malformed / wrong-network strings.
    /// Uses the existing `DashAddress.parse` bridge (Base58Check + network
    /// check in Rust); a `.core` result is a valid L1 address on this
    /// identity's network. Authoritative validation still happens in Rust
    /// on submit — this only tightens the UI.
    private var isValidDestinationAddress: Bool {
        guard !trimmedAddress.isEmpty else { return false }
        if case .core = DashAddress.parse(trimmedAddress, network: identity.network).type {
            return true
        }
        return false
    }

    private var canSubmit: Bool {
        isValidDestinationAddress
            && managedWallet != nil
            && (parsedCredits.map { $0 > 0 && $0 <= senderBalanceCredits } ?? false)
    }

    // MARK: - Submit

    private func submit() {
        // Re-validate at submit time — the balance could have moved
        // (a concurrent sync) and the address/amount could have been
        // cleared between render and tap. Same shape as
        // `TransferCreditsView.submit`.
        let address = trimmedAddress
        guard
            let wallet = managedWallet,
            isValidDestinationAddress,
            let credits = parsedCredits,
            credits > 0,
            credits <= senderBalanceCredits
        else {
            submitError = .init(message: "Amount or address is invalid, or the amount exceeds your balance.")
            return
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        // Fresh `KeychainSigner` per submit pass, same as
        // `TransferCreditsView`: the address signer trampoline derives
        // the identity-state-transition signing key on demand — no
        // bytes leave Rust.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        // `Identifier` is a typealias for `Data`, so the 32-byte
        // `identityId` value passes straight through with no conversion.
        let fromId = identity.identityId

        Task {
            do {
                try await wallet.withdrawCredits(
                    identityId: fromId,
                    amount: credits,
                    toAddress: address,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    // No new balance is returned — the Rust persister
                    // callback deducts this identity's
                    // `PersistentIdentity.balance`, which the parent
                    // view's @Query reflects automatically.
                    self.didComplete = true
                }
            } catch {
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }

    // MARK: - Helpers

    /// Format a raw credit amount as a `… DASH` string. Mirrors
    /// `TransferCreditsView.formatDash`.
    private static func formatDash(raw: UInt64) -> String {
        let dash = Double(raw) / Double(creditsPerDash)
        let fmt = NumberFormatter()
        fmt.minimumFractionDigits = 0
        fmt.maximumFractionDigits = 8
        fmt.numberStyle = .decimal
        fmt.groupingSeparator = ","
        fmt.decimalSeparator = "."
        return (fmt.string(from: NSNumber(value: dash)) ?? String(format: "%.8f", dash)) + " DASH"
    }
}
