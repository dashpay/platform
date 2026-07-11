import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for configuring the direct-purchase price of a token.
///
/// Inputs: a single flat `pricePerToken` (credits-per-token) and an
/// optional public note. `0` disables direct purchase. Tiered
/// (`SetPrices`) schedules are deliberately deferred — surface a TODO
/// and only wire the flat-price path.
///
/// Set Price is group-capable. When the rule that governs pricing
/// changes (`distributionChangeRules.changeDirectPurchasePricingRules`)
/// is gated on a `MainGroup` / `Group:<position>`, the submission is
/// sent as a group-action proposal — same banner shape as Burn /
/// Mint / Freeze.
///
/// TODO: PersistentToken doesn't yet carry the *current* configured
/// price for the token; once it does, surface it as helper text and
/// pre-fill the field.
/// TODO: support the tiered (`SetPrices`) schedule shape.
struct TokenSetPriceActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var priceText: String = ""
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
            }

            Section {
                Text("Set the per-token price for direct purchase. Buyers pay credits at this price; enter 0 to disable direct purchase.")
                    .font(.subheadline)
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This price change requires a group action.",
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

            Section("Price per token") {
                TextField("Credits per token", text: $priceText)
                    .keyboardType(.numberPad)
                Text("Credits per token. Enter 0 to disable direct purchase.")
                    .font(.caption)
                    .foregroundColor(.secondary)
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
                            Text("Set Price")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Set Price")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Set price failed"),
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

    /// `0` is a valid value (disables direct purchase). Treat empty
    /// or non-numeric input as invalid; everything else parses.
    private var parsedPrice: UInt64? {
        let trimmed = priceText.trimmingCharacters(in: .whitespacesAndNewlines)
        return UInt64(trimmed)
    }

    private var canSubmit: Bool {
        guard parsedPrice != nil else { return false }
        return managedWallet != nil
    }

    /// If the change-direct-purchase-pricing rule is gated on a
    /// group, return the group position the caller should propose
    /// under. Otherwise nil (single-signer set-price). Mirrors the
    /// Burn / Pause resolver but reads the pricing-change rule.
    private var inferredGroupPosition: Int? {
        guard let rule = token.distributionChangeRules?.changeDirectPurchasePricingRules else {
            return nil
        }
        let authorized = rule.authorizedToMakeChange

        if authorized == AuthorizedActionTakers.mainGroup.rawValue {
            guard let main = token.mainControlGroupPosition,
                  UInt16(exactly: main) != nil
            else { return nil }
            return main
        }
        if authorized.hasPrefix("Group:"),
           let pos = Int(authorized.dropFirst("Group:".count)),
           UInt16(exactly: pos) != nil {
            return pos
        }
        return nil
    }

    // MARK: - Submit

    private func submit() {
        guard let wallet = managedWallet else {
            submitError = .init(message: "The wallet that owns this identity isn't loaded.")
            return
        }
        guard let price = parsedPrice else {
            submitError = .init(message: "Price is invalid.")
            return
        }

        guard let position = UInt16(exactly: token.position) else {
            submitError = .init(message: "Invalid token position.")
            return
        }

        let groupAction: GroupActionMode
        if let rawGroupPosition = inferredGroupPosition {
            guard let groupPosition = UInt16(exactly: rawGroupPosition) else {
                submitError = .init(message: "Group position is out of range.")
                return
            }
            groupAction = .propose(position: groupPosition)
        } else {
            groupAction = .none
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note

        Task {
            do {
                try await wallet.tokenSetPrice(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    pricePerToken: price,
                    publicNote: publicNoteOrNil,
                    groupAction: groupAction,
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
