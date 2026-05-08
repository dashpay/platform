import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for resuming a paused token.
///
/// Resume is an emergency action — it targets the token itself, so the
/// only inputs are an optional public note plus (when the
/// `emergencyAction` rule is gated on a group) a group-action banner
/// indicating the submission will be sent as a group-action proposal.
struct TokenResumeActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

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
                Text("Resuming re-enables all token operations after a pause.")
                    .font(.subheadline)
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This resume requires a group action.",
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
                            Text("Resume")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Resume")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Resume failed"),
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

    private var canSubmit: Bool {
        managedWallet != nil
    }

    private var inferredGroupPosition: Int? {
        guard let rule = token.emergencyActionRules else { return nil }
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
                try await wallet.tokenResume(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    publicNote: publicNoteOrNil,
                    groupAction: groupAction,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    // Single-signer submissions execute on this call;
                    // the chain is now unpaused, so flip the local
                    // flag so TokenActionPermissionsView's Pause gate
                    // unlocks immediately. Propose-mode just stores a
                    // pending group action — the resume takes effect
                    // when the threshold-crossing co-signer submits,
                    // so leave isPaused alone in that branch.
                    if case .none = groupAction {
                        token.isPaused = false
                        // The chain action has already succeeded, so a
                        // SwiftData persistence hiccup isn't user-facing
                        // (worst case the in-memory flip survives until
                        // the next contract re-parse reconciles). Log
                        // for diagnosability instead of swallowing.
                        do {
                            try modelContext.save()
                        } catch {
                            print("⚠️ TokenResumeActionView: failed to persist isPaused flip: \(error)")
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
