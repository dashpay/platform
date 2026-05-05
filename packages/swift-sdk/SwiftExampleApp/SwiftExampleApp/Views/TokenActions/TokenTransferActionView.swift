import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for transferring tokens between identities.
///
/// Inputs: recipient (via `RecipientPickerView`), amount, optional
/// public note. The form refuses to submit unless the recipient is
/// set and the amount is in `(0, balance]`. On success, the view
/// dismisses; the parent screen relies on the next balance sync to
/// pick up the change (Wave 1 doesn't push a synchronous local
/// `PersistentTokenBalance` update — that's a TODO once a token-sync
/// helper exists in `platform-wallet-ffi`).
struct TokenTransferActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity
    /// Balance the parent screen already fetched for this token via
    /// `sdk.getIdentityTokenBalances`. When non-nil, it's the source of
    /// truth; when nil, we fall back to the `PersistentTokenBalance`
    /// row, which may not be populated yet (see file header). Declared
    /// as `var` (not `let`) so the synthesized memberwise init exposes
    /// it with the default of `nil`.
    var initialBalance: UInt64? = nil

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var recipient: RecipientSelection?
    @State private var amountText: String = ""
    @State private var publicNote: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance
    /// after the user pops + repushes mid-broadcast.
    @State private var submitGeneration: Int = 0

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
                LabeledContent("Balance", value: balanceDisplay)
            }

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

            Section("Amount") {
                TextField("Amount", text: $amountText)
                    .keyboardType(.decimalPad)
                if let amountValue = parsedAmount, amountValue > balanceValue {
                    Text("Amount exceeds your balance.")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }

            Section("Public note (optional)") {
                TextField("Note", text: $publicNote, axis: .vertical)
                    .lineLimit(1...3)
            }

            Section {
                Button {
                    submit()
                } label: {
                    HStack {
                        if isSubmitting {
                            ProgressView().controlSize(.small)
                            Text("Submitting…")
                        } else {
                            Text("Transfer")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Transfer")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Transfer failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
    }

    // MARK: - Derived state

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    private var balanceValue: UInt64 {
        if let initialBalance { return initialBalance }
        guard let balance = matchingBalance else { return 0 }
        // PersistentTokenBalance stores Int64; we treat it as
        // a UInt64 here (token amounts are non-negative on Platform).
        return balance.balance < 0 ? 0 : UInt64(balance.balance)
    }

    private var balanceDisplay: String {
        if let initialBalance {
            return formatTokenAmount(initialBalance, decimals: token.decimals)
        }
        guard let balance = matchingBalance else { return "0" }
        return balance.displayBalance
    }

    /// Match against the SwiftData relationship key. The earlier
    /// `tb.tokenId == token.id.toBase58String()` arm of this matcher
    /// was always false: `tb.tokenId` holds the canonical on-chain
    /// token id while `token.id` is a `contractId + position` SwiftData
    /// uniqueness key.
    private var matchingBalance: PersistentTokenBalance? {
        identity.tokenBalances.first { $0.token?.id == token.id }
    }

    /// Parse the user's input as a decimal number in display units and
    /// scale it to raw on-chain units using `token.decimals`. Without
    /// this, typing "5" against a token with 8 decimals would submit
    /// 5 raw units (0.00000005 of a token) and silently sneak past the
    /// balance check, since the displayed balance is also in display
    /// units.
    private var parsedAmount: UInt64? {
        parseTokenAmount(amountText, decimals: token.decimals)
    }

    private var canSubmit: Bool {
        guard recipient != nil else { return false }
        guard let amount = parsedAmount, amount > 0, amount <= balanceValue else {
            return false
        }
        return managedWallet != nil
    }

    // MARK: - Submit

    private func submit() {
        // Re-check balance at submit time: the user could have spent
        // the underlying tokens between render and tap. Mirror the
        // shape used by `TokenBurnActionView.submit`.
        guard
            let wallet = managedWallet,
            let recipient = recipient,
            let amount = parsedAmount,
            amount > 0,
            amount <= balanceValue
        else {
            submitError = .init(message: "Amount is invalid or exceeds your balance.")
            return
        }

        // Guard before flipping `isSubmitting` so an out-of-range
        // position doesn't leave the spinner stuck.
        guard let position = UInt16(exactly: token.position) else {
            submitError = .init(message: "Invalid token position.")
            return
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let recipientId = recipient.identityId
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note

        Task {
            do {
                try await wallet.tokenTransfer(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    recipient: recipientId,
                    amount: amount,
                    publicNote: publicNoteOrNil,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.dismiss()
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
}

