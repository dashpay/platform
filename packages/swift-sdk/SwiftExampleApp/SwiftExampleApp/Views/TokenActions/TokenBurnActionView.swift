import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for burning tokens.
///
/// Inputs: amount, optional public note. Validates `amount > 0` and
/// `amount <= balance`. Surfaces a group-action banner whenever the
/// caller's authorization to burn is gated on a `MainGroup` /
/// `Group:<position>` rule — in that case the submission is sent as a
/// group-action proposal (`GroupActionMode.propose`).
struct TokenBurnActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

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

            Section {
                Label(
                    "Burning is irreversible — the tokens are permanently removed from circulation.",
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.subheadline)
                .foregroundColor(.orange)
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This burn requires a group action.",
                        systemImage: "person.3.fill"
                    )
                    .font(.subheadline)
                    LabeledContent("Group position", value: "\(groupPosition)")
                    Text(
                        "Submitting will propose a new group action. Other group members "
                        + "must sign before it broadcasts."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
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
                            Text("Burn")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Burn")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Burn failed"),
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
        guard let amount = parsedAmount, amount > 0, amount <= balanceValue else {
            return false
        }
        return managedWallet != nil
    }

    /// If the burn rule on this token is gated on a group, return the
    /// group position the caller should propose under. Otherwise nil
    /// (single-signer burn).
    ///
    /// Mirrors the resolver logic in `TokenActionEvaluator` — we only
    /// look at the explicit `MainGroup` / `Group:<position>` cases.
    /// More exotic cases land here as nil for Wave 1; the underlying
    /// `_with_signer` path will reject them server-side and surface a
    /// readable error.
    private var inferredGroupPosition: Int? {
        guard let rule = token.manualBurningRules else { return nil }
        let authorized = rule.authorizedToMakeChange

        if authorized == AuthorizedActionTakers.mainGroup.rawValue {
            return token.mainControlGroupPosition
        }
        if authorized.hasPrefix("Group:"),
           let pos = Int(authorized.dropFirst("Group:".count)) {
            return pos
        }
        return nil
    }

    // MARK: - Submit

    private func submit() {
        guard
            let wallet = managedWallet,
            let amount = parsedAmount,
            amount > 0
        else {
            submitError = .init(message: "Amount is invalid.")
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let position = UInt16(token.position)
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note
        let groupAction: GroupActionMode = inferredGroupPosition.map {
            .propose(position: UInt16($0))
        } ?? .none

        Task {
            do {
                try await wallet.tokenBurn(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    amount: amount,
                    publicNote: publicNoteOrNil,
                    groupAction: groupAction,
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
