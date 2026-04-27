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

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var recipient: RecipientSelection?
    @State private var amountText: String = ""
    @State private var publicNote: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?

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
                    .keyboardType(.numberPad)
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
        guard let balance = matchingBalance else { return 0 }
        // PersistentTokenBalance stores Int64; we treat it as
        // a UInt64 here (token amounts are non-negative on Platform).
        return balance.balance < 0 ? 0 : UInt64(balance.balance)
    }

    private var balanceDisplay: String {
        guard let balance = matchingBalance else { return "0" }
        return balance.displayBalance
    }

    private var matchingBalance: PersistentTokenBalance? {
        identity.tokenBalances.first { tb in
            tb.tokenId == token.id.toBase58String()
                || tb.token?.id == token.id
        }
    }

    private var parsedAmount: UInt64? {
        let trimmed = amountText.trimmingCharacters(in: .whitespacesAndNewlines)
        return UInt64(trimmed)
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
        guard
            let wallet = managedWallet,
            let recipient = recipient,
            let amount = parsedAmount,
            amount > 0
        else {
            submitError = .init(message: "Selection is incomplete.")
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let position = UInt16(token.position)
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
                    self.isSubmitting = false
                    self.dismiss()
                }
            } catch {
                await MainActor.run {
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }
}
