import SwiftUI
import SwiftData
import SwiftDashSDK

/// Ban, unban, suspend, unsuspend, warn or clear the warnings of one identity
/// on a contract (state transition type 24, protocol version 14), as an
/// identity of a loaded wallet that Rust named a moderator.
///
/// Who may act and which of the six moderations each may send are Rust's
/// answer (``ContractModerationOfferStore``); this sheet offers exactly
/// those. Whether the target admits the moderation (already banned, the most
/// warnings reached, protected) is Platform's: its refusal is shown with its
/// consensus code, and an accepted moderation shows the proved status.
struct ContractUserModerationSheet: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var offers: ContractModerationOfferStore
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss
    @Query private var identities: [PersistentIdentity]

    let scope: ModerationScope
    let target: ModerationTarget
    /// Called after a moderation went through, so the screen that opened
    /// the sheet reads again.
    var onModerated: () -> Void = {}

    @State private var moderatorSelection = ""
    @State private var actionSelection: ContractUserModerationAction?
    @State private var reasonText = ""
    @State private var reasonDocumentId = ""
    /// The suspension's end; a day from now until the moderator picks one.
    @State private var suspendUntil = Date().addingTimeInterval(86_400)
    @State private var isSending = false
    @State private var result: ModeratedContractUser?
    @State private var errorText: String?

    private var moderators: [ContractModerationOfferStore.Moderator] { offers.userModerators }

    private var moderator: ContractModerationOfferStore.Moderator? {
        ModeratorPicker.chosen(moderatorSelection, among: moderators)
    }

    /// The moderations Rust lists for the chosen moderator.
    private var actions: [ContractUserModerationAction] { moderator?.offer.userActions ?? [] }

    private var action: ContractUserModerationAction? {
        if let actionSelection, actions.contains(actionSelection) { return actionSelection }
        return actions.first
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Identity") {
                    ModerationFieldRow(label: "Target", value: target.identityId, monospaced: true)
                        .accessibilityIdentifier("contractModeration.moderate.target")
                }

                if moderators.isEmpty {
                    ModerationMessageRow(
                        kind: .empty,
                        text: "No identity of a loaded wallet moderates this contract's lists.",
                        identifier: "contractModeration.moderate.noModerator"
                    )
                } else {
                    Section {
                        ModeratorPicker(
                            moderators: moderators,
                            selection: $moderatorSelection,
                            label: label(forIdentity:),
                            identifier: "contractModeration.moderate.moderator"
                        )
                        Picker("Action", selection: actionBinding) {
                            ForEach(actions) { action in
                                Text(action.title).tag(action)
                            }
                        }
                        .accessibleFormPicker("contractModeration.moderate.action")
                        if action == .suspend {
                            DatePicker("Suspended until", selection: $suspendUntil, in: Date()...)
                                .accessibilityIdentifier("contractModeration.moderate.suspendUntil")
                        }
                    } header: {
                        Text("Moderation")
                    } footer: {
                        Text("Only the moderations of the lists this contract keeps, and for a seated team only those its declaration grants, are offered.")
                    }

                    Section {
                        ModerationReasonFields(
                            text: $reasonText,
                            reasonDocumentId: $reasonDocumentId,
                            identifierPrefix: "contractModeration.moderate"
                        )
                    } header: {
                        Text("Reason")
                    } footer: {
                        Text("Stored with a ban, a suspension and a warning. A seated team must name a reason document its proposal lists.")
                    }

                    Section {
                        Button {
                            send()
                        } label: {
                            HStack {
                                if isSending { ProgressView().controlSize(.small) }
                                Text(action.map { "\($0.title) Identity" } ?? "Moderate")
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                        .disabled(isSending || action == nil)
                        .accessibilityIdentifier("contractModeration.moderate.send")
                    }
                }

                if let errorText {
                    ModerationMessageRow(
                        kind: .error,
                        text: errorText,
                        identifier: "contractModeration.moderate.error"
                    )
                }
                if let result {
                    Section("Proved Status") {
                        Label("Moderation accepted", systemImage: "checkmark.seal.fill")
                            .foregroundColor(.green)
                        ContractModerationStatusResult(status: result.status)
                    }
                    .accessibilityIdentifier("contractModeration.moderate.result")
                }
            }
            .navigationTitle("Moderate Identity")
            .navigationBarTitleDisplayMode(.inline)
            // The result and the picked moderator belong to the contract and
            // the connection they were made on
            .onChange(of: scope) { _, _ in clearResult() }
            .onChange(of: appState.sdkGeneration) { _, _ in clearResult() }
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                        .accessibilityIdentifier("contractModeration.moderate.done")
                }
            }
        }
    }

    private func clearResult() {
        moderatorSelection = ""
        actionSelection = nil
        result = nil
        errorText = nil
    }

    private var actionBinding: Binding<ContractUserModerationAction> {
        Binding(
            get: { action ?? .ban },
            set: { actionSelection = $0 }
        )
    }

    /// The alias of a local identity, or its base58 id.
    private func label(forIdentity base58: String) -> String {
        identities.first { $0.identityIdBase58 == base58 }?.alias ?? base58
    }

    private func send() {
        guard let moderator, let action else { return }
        guard let wallet = walletManager.wallet(for: moderator.walletId),
              let moderatorId = Data.identifier(fromBase58: moderator.identityIdBase58),
              let contractId = Data.identifier(fromBase58: scope.contractId) else {
            errorText = DocumentActionError.noWallet.localizedDescription
            return
        }
        guard let targetId = Data.identifier(fromBase58: target.identityId) else {
            errorText = "The target is not a base58 identity id."
            return
        }
        guard let reason = ModerationReasonFields.input(text: reasonText, reasonDocumentId: reasonDocumentId) else {
            errorText = "The reason document id is not a base58 identifier."
            return
        }
        let suspendUntilMs = action == .suspend
            ? UInt64(max(0, suspendUntil.timeIntervalSince1970) * 1000)
            : nil
        // Fresh `KeychainSigner` per moderation, as `CreateDocumentView` does.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        isSending = true
        errorText = nil
        result = nil
        Task {
            do {
                result = try await wallet.moderateContractUser(
                    moderatorIdentityId: moderatorId,
                    contractId: contractId,
                    targetIdentityId: targetId,
                    action: action,
                    suspendUntilMs: suspendUntilMs,
                    reason: reason,
                    signer: signer
                )
                onModerated()
            } catch {
                errorText = moderationWriteErrorText(error)
            }
            isSending = false
        }
    }
}
