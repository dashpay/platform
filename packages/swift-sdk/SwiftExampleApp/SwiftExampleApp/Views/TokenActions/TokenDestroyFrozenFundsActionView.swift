import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for destroying another identity's frozen token balance.
///
/// Inputs: target identity (via `RecipientPickerView`), required reason
/// (carried as the public note — submission is blocked until non-empty).
/// Surfaces a group-action banner whenever the caller's authorization
/// to destroy frozen funds is gated on a `MainGroup` /
/// `Group:<position>` rule.
///
/// The Rust builder takes no `amount` — destroying always affects the
/// full frozen balance of the target.
struct TokenDestroyFrozenFundsActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var target: RecipientSelection?
    @State private var reason: String = ""
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
                Label(
                    "⚠️ This action permanently destroys frozen tokens. It cannot be undone.",
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.subheadline)
                .foregroundColor(.red)
            }

            Section("Identity whose frozen funds will be destroyed") {
                if let wallet = managedWallet {
                    RecipientPickerView(
                        selection: $target,
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

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This destroy requires a group action.",
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

            Section("Reason (required)") {
                TextField("Why are you destroying these funds?", text: $reason, axis: .vertical)
                    .lineLimit(2...4)
                if trimmedReason.isEmpty {
                    Text("A reason is required for this destructive action.")
                        .font(.caption)
                        .foregroundColor(.red)
                }
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
                            Text("Destroy Frozen Funds")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Destroy Frozen Funds")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Destroy failed"),
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

    private var trimmedReason: String {
        reason.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var canSubmit: Bool {
        target != nil && !trimmedReason.isEmpty && managedWallet != nil
    }

    private var inferredGroupPosition: Int? {
        guard let rule = token.destroyFrozenFundsRules else { return nil }
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
        guard
            let wallet = managedWallet,
            let target = target,
            !trimmedReason.isEmpty
        else {
            submitError = .init(message: "Selection or reason is incomplete.")
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
        let frozenId = target.identityId
        let publicNoteOrNil: String? = trimmedReason

        Task {
            do {
                try await wallet.tokenDestroyFrozenFunds(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    frozenIdentityId: frozenId,
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
