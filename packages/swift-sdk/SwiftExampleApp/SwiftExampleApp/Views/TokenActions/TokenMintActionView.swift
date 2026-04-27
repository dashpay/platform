import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for minting tokens.
///
/// Inputs: amount, optional recipient (defaults to self), optional
/// public note. Validates `amount > 0` and — when the token has a
/// `maxSupply` configured — that `amount <= maxSupply`. The screen
/// surfaces a group-action banner whenever the caller's authorization
/// to mint is gated on a `MainGroup` / `Group:<position>` rule, in
/// which case the submission is sent as a group-action proposal
/// (`GroupActionMode.propose`).
struct TokenMintActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var amountText: String = ""
    @State private var publicNote: String = ""
    @State private var mintToSelf: Bool = true
    @State private var recipient: RecipientSelection?
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
                if let maxSupply = token.maxSupply {
                    LabeledContent("Max supply", value: maxSupply)
                }
            }

            if !token.mintingAllowChoosingDestination {
                Section {
                    Label(
                        "This token does not allow choosing a mint destination — tokens are issued to the configured recipient.",
                        systemImage: "info.circle"
                    )
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                }
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This mint requires a group action.",
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

            Section("Recipient") {
                Toggle("Mint to self", isOn: $mintToSelf)
                    .disabled(!token.mintingAllowChoosingDestination)
                if !mintToSelf {
                    if let wallet = managedWallet {
                        RecipientPickerView(
                            selection: $recipient,
                            wallet: wallet,
                            network: identity.network,
                            exclude: nil
                        )
                    } else {
                        Text("The wallet that owns this identity isn't loaded.")
                            .font(.subheadline)
                            .foregroundColor(.red)
                    }
                }
            }

            Section("Amount") {
                TextField("Amount", text: $amountText)
                    .keyboardType(.numberPad)
                if let amountValue = parsedAmount,
                   let maxSupplyValue = parsedMaxSupply,
                   amountValue > maxSupplyValue {
                    Text("Amount exceeds the configured max supply.")
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
                            Text("Mint")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Mint")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Mint failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
        .onAppear {
            // Token forbids choosing the destination -> force mint-to-self.
            if !token.mintingAllowChoosingDestination {
                mintToSelf = true
            }
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

    private var parsedMaxSupply: UInt64? {
        guard let raw = token.maxSupply else { return nil }
        return UInt64(raw)
    }

    private var canSubmit: Bool {
        guard let amount = parsedAmount, amount > 0 else { return false }
        if let maxSupply = parsedMaxSupply, amount > maxSupply {
            return false
        }
        if !mintToSelf, recipient == nil { return false }
        return managedWallet != nil
    }

    /// If the mint rule on this token is gated on a group, return the
    /// group position the caller should propose under. Otherwise nil
    /// (single-signer mint). Mirrors Wave 1's burn evaluator.
    private var inferredGroupPosition: Int? {
        guard let rule = token.manualMintingRules else { return nil }
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

        let recipientId: Data?
        if mintToSelf {
            recipientId = nil
        } else {
            guard let chosen = recipient else {
                submitError = .init(message: "Pick a recipient or enable mint-to-self.")
                return
            }
            recipientId = chosen.identityId
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
                try await wallet.tokenMint(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    recipient: recipientId,
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
