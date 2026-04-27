import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for buying tokens at the configured direct-purchase price.
///
/// Inputs: amount of tokens to buy. The total cost is shown
/// read-only when a price is locally known; otherwise the user is
/// told the price isn't surfaced yet and a sentinel
/// (`UInt64.max`) is sent for `expectedTotalCost` so the Rust side
/// validates against the actual schedule. Direct Purchase is not
/// group-gated, so there's no group-action banner.
///
/// TODO: PersistentToken doesn't yet carry the *current* configured
/// price. Once it does, surface it as helper text, compute the total
/// cost client-side, and replace the `UInt64.max` sentinel with the
/// real expected total.
struct TokenPurchaseActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var amountText: String = ""
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
            }

            Section {
                Text("Direct purchase debits credits from this identity in exchange for the requested tokens. The buyer pays the configured price set by the token's pricing controller.")
                    .font(.subheadline)
            }

            Section("Amount") {
                TextField("Tokens to buy", text: $amountText)
                    .keyboardType(.numberPad)
                if let amount = parsedAmount, amount == 0 {
                    Text("Amount must be greater than zero.")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }

            Section("Total cost") {
                Text("Price not yet surfaced locally — submit to fetch the configured price from Platform.")
                    .font(.caption)
                    .foregroundColor(.secondary)
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
                            Text("Buy")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Direct Purchase")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Purchase failed"),
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

    private var parsedAmount: UInt64? {
        let trimmed = amountText.trimmingCharacters(in: .whitespacesAndNewlines)
        return UInt64(trimmed)
    }

    private var canSubmit: Bool {
        guard let amount = parsedAmount, amount > 0 else { return false }
        return managedWallet != nil
    }

    // MARK: - Submit

    private func submit() {
        guard let wallet = managedWallet else {
            submitError = .init(message: "The wallet that owns this identity isn't loaded.")
            return
        }
        guard let amount = parsedAmount, amount > 0 else {
            submitError = .init(message: "Amount must be greater than zero.")
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let position = UInt16(token.position)
        // The local `PersistentToken` row doesn't yet expose the
        // direct-purchase price, so we can't compute the buyer's
        // expected total here. Send `UInt64.max` as a sentinel so the
        // Rust side fails with a readable "agreed price doesn't match
        // schedule" error rather than authorising an arbitrary debit.
        // TODO: replace once the price field lands on PersistentToken.
        let expectedTotalCost = UInt64.max

        Task {
            do {
                try await wallet.tokenPurchase(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    amount: amount,
                    expectedTotalCost: expectedTotalCost,
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
