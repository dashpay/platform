import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for buying tokens at the configured direct-purchase price.
///
/// Inputs: amount of tokens to buy. The Buy button stays disabled
/// until `PersistentToken` exposes the configured direct-purchase
/// price; submitting a known-failing state transition (with a sentinel
/// total) is worse than telling the user to wait. Direct Purchase is
/// not group-gated, so there's no group-action banner.
///
/// TODO: PersistentToken doesn't yet carry the *current* configured
/// price. Once it does, surface it as helper text, compute the total
/// cost client-side, and flip `priceKnown` so Buy enables.
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
                Text("Price not yet known — waiting for sync.")
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
                if !priceKnown {
                    Text("Buy is disabled until the configured direct-purchase price is available locally.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
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

    /// `PersistentToken` doesn't yet carry a direct-purchase price
    /// field, so we can't compute the buyer's expected total. Until it
    /// does, Buy stays disabled rather than dispatching a known-failing
    /// state transition with a sentinel total.
    /// TODO: surface the configured price on PersistentToken and gate
    /// Buy on that instead.
    private var priceKnown: Bool { false }

    private var canSubmit: Bool {
        guard let amount = parsedAmount, amount > 0 else { return false }
        guard priceKnown else { return false }
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
        guard priceKnown else {
            submitError = .init(message: "Direct-purchase price isn't known locally yet.")
            return
        }

        guard let position = UInt16(exactly: token.position) else {
            submitError = .init(message: "Invalid token position.")
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        // TODO: replace once the price field lands on PersistentToken.
        // Buy is gated on `priceKnown` above, so this fallback is
        // unreachable until that gate flips.
        let expectedTotalCost: UInt64 = 0

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
