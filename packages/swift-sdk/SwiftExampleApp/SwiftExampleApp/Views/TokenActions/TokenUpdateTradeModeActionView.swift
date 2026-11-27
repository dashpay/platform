import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for changing a token's marketplace trade mode: whether holders may
/// list the token for sale and make offers for it.
///
/// The submission is one `TokenConfigChange.marketplaceTradeMode`, which Rust
/// turns into `TokenConfigurationChangeItem::MarketplaceTradeMode`. Platform
/// decides whether the token may take the mode (a tradeable token must be
/// transferable, and the network at protocol version 16 or later); this form
/// does not check either and shows Platform's refusal as it arrives.
///
/// Group-capable, like the max supply form: when
/// `token.tradeModeChangeRules` names `MainGroup` or `Group:<position>`, the
/// submission is sent as a group-action proposal.
struct TokenUpdateTradeModeActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var newTradeMode: TokenTradeMode
    @State private var publicNote: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?
    /// Shown after a confirmed update whose contract could not be stored.
    @State private var refreshNotice: AlertMessage?
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance.
    @State private var submitGeneration: Int = 0

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    init(token: PersistentToken, identity: PersistentIdentity) {
        self.token = token
        self.identity = identity
        // Start on the mode the token does not have.
        _newTradeMode = State(
            initialValue: token.tradeMode == .tradeable ? .notTradeable : .tradeable
        )
    }

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
                LabeledContent("Current trade mode", value: token.tradeMode.displayName)
                    .accessibilityIdentifier("tokenTradeMode.current")
            }

            Section {
                Text("A tradeable token can be listed for sale and offered for through Platform. Platform refuses the change if the token is not transferable or the network does not support token trading yet.")
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

            Section("New trade mode") {
                Picker("Trade mode", selection: $newTradeMode) {
                    ForEach(TokenTradeMode.allCases, id: \.self) { mode in
                        Text(mode.displayName).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("tokenTradeMode.picker")
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
                            Text("Update Trade Mode")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(managedWallet == nil || isSubmitting)
                .accessibilityIdentifier("tokenTradeMode.submitButton")
            }
        }
        .navigationTitle("Update Trade Mode")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Update failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
        .alert(item: $refreshNotice) { msg in
            Alert(
                title: Text("Trade mode updated"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK")) { dismiss() }
            )
        }
    }

    // MARK: - Derived state

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    private var inferredGroupPosition: Int? {
        TokenConfigChangeGroup.proposalPosition(rule: token.tradeModeChangeRules, token: token)
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
        let change = TokenConfigChange.marketplaceTradeMode(newTradeMode)
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note
        let sdk = appState.sdk

        Task {
            do {
                let outcome = try await wallet.tokenUpdateConfig(
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
                    // Rust says whether the update took effect and hands back
                    // the contract it produced; storing that shows the new
                    // mode. A pending group action changes nothing yet.
                    if let reason = ContractDownloader.storeConfigUpdateOutcome(
                        outcome,
                        contractId: contractId,
                        sdk: sdk,
                        modelContext: modelContext
                    ) {
                        self.refreshNotice = .init(
                            message: "The update was confirmed, but the updated contract could not be "
                                + "saved on this device (\(reason)). It keeps showing the old trade "
                                + "mode until the contract is refreshed."
                        )
                    } else {
                        self.dismiss()
                    }
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

/// Which group a token configuration change is proposed under.
enum TokenConfigChangeGroup {
    /// If `rule` (the change rule of the item being changed) is gated on a
    /// group, the group position to propose under; otherwise nil (a
    /// single-signer config update). Mirrors the resolver pattern used by
    /// Burn / Pause / SetPrice.
    static func proposalPosition(rule: ChangeControlRules?, token: PersistentToken) -> Int? {
        guard let rule else { return nil }
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
}
