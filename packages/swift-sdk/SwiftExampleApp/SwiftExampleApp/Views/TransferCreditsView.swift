// TransferCreditsView.swift
// SwiftExampleApp
//
// Credit-to-credit transfer between two Platform identities. The
// sender is fixed to the `identity` this flow opened from; the
// recipient is chosen via the shared `RecipientPickerView` (Local /
// Paste / DPNS). Structurally mirrors `TokenTransferActionView`
// (recipient picker + submit guard + `managedWallet` derivation) and
// `TopUpIdentityView` (DASH-amount entry, `creditsPerDash` divisor,
// NavigationStack + Cancel toolbar, target + success sections).
//
// All orchestration lives in Rust: this view only parses the amount
// and validates it against the sender's cached balance, then hands a
// fresh `KeychainSigner` to `ManagedPlatformWallet.transferCredits`.
// On success the Rust persister callback deducts the sender's
// `PersistentIdentity.balance`, so the parent view's `@Query`
// refreshes the displayed balance automatically — this view returns
// nothing from the SDK call.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct TransferCreditsView: View {
    /// Identity sending the credits. The owning wallet (and thus the
    /// signer + the FFI handle) is derived from `identity.wallet`.
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// Credits per DASH (1e11) — same divisor `TopUpIdentityView` and
    /// `PersistentIdentity.formattedBalance` use for credit amounts.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    // MARK: - Selection state

    @State private var recipient: RecipientSelection?
    @State private var amountDash: String = ""

    // MARK: - Submit state

    @State private var isSubmitting = false
    @State private var submitError: SubmitError?
    @State private var didComplete = false
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance
    /// after the user pops + repushes mid-broadcast. Mirrors
    /// `TokenTransferActionView.submitGeneration`.
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
                    recipientSection
                    amountSection
                    submitSection
                }
            }
            .navigationTitle("Transfer Credits")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Transfer failed"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
        }
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
    private var recipientSection: some View {
        Section("Recipient") {
            if let wallet = managedWallet {
                RecipientPickerView(
                    selection: $recipient,
                    wallet: wallet,
                    network: identity.network,
                    exclude: identity.identityId
                )
            } else {
                Text("The wallet that owns this identity isn't loaded.")
                    .font(.subheadline)
                    .foregroundColor(.red)
            }
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
            Text("Available: \(identity.formattedBalance). The amount entered here is deducted from this identity's credit balance and added to the recipient's.")
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
                        Text("Transferring…")
                    } else {
                        Text("Transfer Credits")
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
                Label("Transfer complete", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let credits = parsedCredits {
                    HStack {
                        Text("Transferred:")
                            .foregroundColor(.secondary)
                        Text(Self.formatDash(raw: credits))
                            .fontWeight(.medium)
                            .monospacedDigit()
                    }
                }
                if let recipient {
                    HStack {
                        Text("To:")
                            .foregroundColor(.secondary)
                        Text(recipient.label)
                            .lineLimit(1)
                            .truncationMode(.middle)
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

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    /// Parse the user's DASH input and scale it to credits. Mirrors
    /// `TopUpIdentityView.parsedAmountCredits`: require a finite,
    /// positive value, round to the nearest credit, and reject any
    /// result below 1 credit or beyond `UInt64.max`.
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

    /// Sender's balance in credits. `PersistentIdentity.balance` is an
    /// `Int64`; clamp any (unexpected) negative value to zero.
    private var senderBalanceCredits: UInt64 {
        identity.balance < 0 ? 0 : UInt64(identity.balance)
    }

    private var canSubmit: Bool {
        recipient != nil
            && managedWallet != nil
            && (parsedCredits.map { $0 > 0 && $0 <= senderBalanceCredits } ?? false)
    }

    // MARK: - Submit

    private func submit() {
        // Re-validate at submit time — the balance could have moved
        // (a concurrent sync) and the recipient/amount could have been
        // cleared between render and tap. Same shape as
        // `TokenTransferActionView.submit`.
        guard
            let wallet = managedWallet,
            let recipient = recipient,
            let credits = parsedCredits,
            credits > 0,
            credits <= senderBalanceCredits
        else {
            submitError = .init(message: "Amount is invalid or exceeds your balance.")
            return
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        // Fresh `KeychainSigner` per submit pass, same as
        // `TokenTransferActionView` / `RegisterNameView`: the address
        // signer trampoline derives the identity-state-transition
        // signing key on demand — no bytes leave Rust.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        // `Identifier` is a typealias for `Data`, so the 32-byte
        // `identityId` values pass straight through with no conversion.
        let fromId = identity.identityId
        let toId = recipient.identityId

        Task {
            do {
                try await wallet.transferCredits(
                    fromIdentityId: fromId,
                    toIdentityId: toId,
                    amount: credits,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    // No new balance is returned — the Rust persister
                    // callback deducts the sender's
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
    /// `TopUpIdentityView.formatDash`.
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
