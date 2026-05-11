import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for updating the configured max supply of a token.
///
/// Inputs: a numeric `newMaxSupply` field plus a "Remove cap" toggle.
/// When the toggle is ON the input is disabled and the submission
/// uses `TokenConfigChange.maxSupply(newValue: nil)`, which Rust
/// translates to `TokenConfigurationChangeItem::MaxSupply(None)` —
/// removing the cap entirely.
///
/// Group-capable. The rule that governs max-supply changes
/// (`token.maxSupplyChangeRules`) can name `MainGroup` or
/// `Group:<position>`; in that case the submission is sent as a
/// group-action proposal — same banner shape as Burn / Mint.
///
/// TODO: PersistentToken doesn't yet expose the *current circulating*
/// supply locally, so the form can't pre-validate `newMaxSupply >=
/// circulating`. Rust / Platform reject the case server-side; once
/// circulating supply lands on the model, surface it as helper text
/// and gate the submit button.
struct TokenUpdateMaxSupplyActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var newMaxSupplyText: String = ""
    @State private var removeCap: Bool = false
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
                LabeledContent("Current max supply", value: currentMaxSupplyDisplay)
            }

            Section {
                Text("Set a new ceiling on the token's total supply, or remove the cap entirely. Platform rejects the change if the new ceiling is below the current circulating supply.")
                    .font(.subheadline)
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This config change requires a group action.",
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

            Section("New max supply") {
                Toggle("Remove cap", isOn: $removeCap)
                TextField("Amount", text: $newMaxSupplyText)
                    .keyboardType(.decimalPad)
                    .disabled(removeCap)
                    .foregroundColor(removeCap ? .secondary : .primary)
                if !removeCap, let parsed = parsedNewMaxSupply, parsed == 0 {
                    Text("Use the 'Remove cap' toggle to disable the supply ceiling — '0' is not a valid cap.")
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
                            Text("Update Max Supply")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Update Max Supply")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Update failed"),
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

    /// Display the current max supply scaled to display units, so the
    /// number the user sees here is in the same unit they're about to
    /// type in the input below. `token.maxSupply` is a string-encoded
    /// raw u64; missing means "uncapped".
    private var currentMaxSupplyDisplay: String {
        guard let raw = token.maxSupply, let value = UInt64(raw) else {
            return "Uncapped"
        }
        return formatTokenAmount(value, decimals: token.decimals)
    }

    /// User input is in display units; scale to raw on-chain units for
    /// the FFI / config-change payload.
    private var parsedNewMaxSupply: UInt64? {
        parseTokenAmount(newMaxSupplyText, decimals: token.decimals)
    }

    private var canSubmit: Bool {
        guard managedWallet != nil else { return false }
        if removeCap { return true }
        guard let parsed = parsedNewMaxSupply, parsed > 0 else { return false }
        return true
    }

    /// If the max-supply-change rule on this token is gated on a
    /// group, return the group position the caller should propose
    /// under. Otherwise nil (single-signer config update).
    /// Mirrors the resolver pattern used by Burn / Pause / SetPrice.
    private var inferredGroupPosition: Int? {
        guard let rule = token.maxSupplyChangeRules else { return nil }
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

        let change: TokenConfigChange
        if removeCap {
            change = .maxSupply(newValue: nil)
        } else {
            guard let parsed = parsedNewMaxSupply, parsed > 0 else {
                submitError = .init(message: "New max supply is invalid.")
                return
            }
            change = .maxSupply(newValue: parsed)
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
        // `PersistentToken.maxSupply` is a string-encoded raw u64 (see
        // `currentMaxSupplyDisplay` — it parses via `UInt64(raw)`),
        // with `nil` representing "no cap" / Unlimited. Materialize the
        // post-success target value here so the Task closure can write
        // it back without re-touching @State from a non-main context.
        let newMaxSupplyValue: String? = removeCap
            ? nil
            : parsedNewMaxSupply.map(String.init)

        Task {
            do {
                try await wallet.tokenUpdateConfig(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    change: change,
                    publicNote: publicNoteOrNil,
                    groupAction: groupAction,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    // Single-signer submissions execute on this call —
                    // flip the local maxSupply so the view (and any
                    // surfaces reading `token.maxSupply`) reflects the
                    // new cap without waiting for a manual contract
                    // refetch. Propose-mode just stores a pending
                    // group action; the chain config doesn't change
                    // until the threshold-crossing co-signer submits,
                    // so leave the field alone in that branch.
                    if case .none = groupAction {
                        token.maxSupply = newMaxSupplyValue
                        // The chain action has already succeeded, so a
                        // SwiftData persistence hiccup isn't user-facing
                        // (worst case the in-memory write survives until
                        // the next contract re-parse reconciles). Log
                        // for diagnosability instead of swallowing.
                        do {
                            try modelContext.save()
                        } catch {
                            print("⚠️ TokenUpdateMaxSupplyActionView: failed to persist maxSupply: \(error)")
                        }
                    }
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
