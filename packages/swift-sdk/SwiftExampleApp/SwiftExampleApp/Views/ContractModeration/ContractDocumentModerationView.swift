import SwiftUI
import SwiftData
import SwiftDashSDK

/// Moderate one document of a contract's document type (state transition
/// type 24, protocol version 14): delete it, change the fields the type
/// keeps for its moderators, or, as a member of an elected contract's seated
/// team, propose the deletion of a settled document.
///
/// Who may act and what each may do on this type are Rust's answer
/// (``ContractModerationOfferStore``); a control shows only for what it
/// lists. Whether the document exists, is still inside its window or is
/// settled is Platform's: its refusal is shown with its consensus code. A
/// deletion keeps the document as it was on this device
/// (``ModeratedDocumentArchive``), so the removal records screen can offer
/// to restore it.
struct ContractDocumentModerationView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var offers: ContractModerationOfferStore
    @Environment(\.modelContext) private var modelContext
    @Query private var identities: [PersistentIdentity]

    let scope: ModerationScope
    let documentTypeName: String

    @State private var documentId = ""
    @State private var moderatorSelection = ""
    @State private var reasonText = ""
    @State private var reasonDocumentId = ""
    @State private var fieldsJSON = ""
    @State private var running: Operation?
    @State private var errorText: String?
    @State private var deletion: ContractDocumentDeletion?
    /// Whether the archive kept the deleted document's bytes: the result of
    /// ``ModeratedDocumentArchive/save(_:contractId:documentTypeName:documentId:)``.
    @State private var keptForRestore = false
    @State private var fieldsChange: ContractDocumentFieldsChange?
    @State private var proposal: ContractTeamActionSignature?

    private enum Operation {
        case delete
        case changeFields
        case proposeSettledDeletion
    }

    private var moderators: [ContractModerationOfferStore.Moderator] {
        offers.documentModerators(of: documentTypeName)
    }

    private var moderator: ContractModerationOfferStore.Moderator? {
        ModeratorPicker.chosen(moderatorSelection, among: moderators)
    }

    /// What Rust offers the chosen moderator on this type.
    private var typeOffer: ContractDocumentTypeModerationOffer? {
        moderator?.offer.documentType(named: documentTypeName)
    }

    var body: some View {
        Form {
            Section {
                TextField("Document ID (base58)", text: $documentId)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                    .accessibilityIdentifier("contractModeration.document.documentId")
            } header: {
                Text("Document")
            }

            if moderators.isEmpty {
                ModerationMessageRow(
                    kind: .empty,
                    text: "No identity of a loaded wallet may moderate \(documentTypeName) documents.",
                    identifier: "contractModeration.document.noModerator"
                )
            } else {
                Section {
                    ModeratorPicker(
                        moderators: moderators,
                        selection: $moderatorSelection,
                        label: label(forIdentity:),
                        identifier: "contractModeration.document.moderator"
                    )
                    ModerationReasonFields(
                        text: $reasonText,
                        reasonDocumentId: $reasonDocumentId,
                        identifierPrefix: "contractModeration.document"
                    )
                } header: {
                    Text("Moderator and Reason")
                } footer: {
                    Text("A seated team must name a reason document its proposal lists.")
                }

                if let typeOffer {
                    actionSections(typeOffer)
                }
            }

            if let errorText {
                ModerationMessageRow(
                    kind: .error,
                    text: errorText,
                    identifier: "contractModeration.document.error"
                )
            }
            results
        }
        .navigationTitle("Moderate \(documentTypeName)")
        .navigationBarTitleDisplayMode(.inline)
        // Results and the picked moderator belong to the contract and the
        // connection they were made on
        .onChange(of: scope) { _, _ in clearResults() }
        .onChange(of: appState.sdkGeneration) { _, _ in clearResults() }
    }

    private func clearResults() {
        moderatorSelection = ""
        errorText = nil
        deletion = nil
        fieldsChange = nil
        proposal = nil
    }

    @ViewBuilder
    private func actionSections(_ offer: ContractDocumentTypeModerationOffer) -> some View {
        if offer.delete {
            Section {
                actionButton("Delete Document", .delete, identifier: "contractModeration.document.delete") {
                    delete()
                }
            } header: {
                Text("Delete")
            } footer: {
                if let seconds = offer.deleteWithinSeconds {
                    Text("Platform refuses a document last modified more than \(seconds) seconds ago: it is settled.")
                } else {
                    Text("The type sets no window.")
                }
            }
        }
        if !offer.changeableFields.isEmpty {
            Section {
                Text("Fields: \(offer.changeableFields.joined(separator: ", "))")
                    .font(.caption)
                    .accessibilityIdentifier("contractModeration.document.changeableFields")
                PlainTextEditor(
                    text: $fieldsJSON,
                    textStyle: .caption1,
                    accessibilityIdentifier: "contractModeration.document.fieldsJSON"
                )
                .frame(minHeight: 80)
                .onAppear {
                    if fieldsJSON.isEmpty { fieldsJSON = Self.fieldsTemplate(offer.changeableFields) }
                }
                actionButton("Change Fields", .changeFields, identifier: "contractModeration.document.changeFields") {
                    changeFields()
                }
            } header: {
                Text("Change Moderator Fields")
            } footer: {
                Text("A JSON object of the fields to set; null removes a field.")
            }
        }
        if let rule = offer.settledDeletion {
            Section {
                actionButton(
                    "Propose Settled Deletion",
                    .proposeSettledDeletion,
                    identifier: "contractModeration.document.proposeSettledDeletion"
                ) {
                    proposeSettledDeletion()
                }
            } header: {
                Text("Settled Deletion")
            } footer: {
                Text("The seated team deletes a settled document together: \(rule.approvals) approval(s)\(rule.leader ? ", the leader's among them" : ""). The proposal is this member's approval.")
            }
        }
    }

    private func actionButton(
        _ title: String,
        _ operation: Operation,
        identifier: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack {
                if running == operation { ProgressView().controlSize(.small) }
                Text(title)
            }
            .frame(maxWidth: .infinity)
        }
        .disabled(running != nil || trimmedDocumentId.isEmpty)
        .accessibilityIdentifier(identifier)
    }

    @ViewBuilder
    private var results: some View {
        if let deletion {
            Section("Deleted") {
                Label("Document deleted", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                if let removal = deletion.removal {
                    ContractDocumentRemovalRow(removal: removal)
                } else {
                    Text("The type keeps no removal record.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Text(keptForRestore
                     ? "The document as it was is kept on this device for restore."
                     : "The document's bytes could not be kept on this device, so a restore will not be possible from this device.")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            .accessibilityIdentifier("contractModeration.document.deleteResult")
        }
        if let fieldsChange {
            Section("Changed Document") {
                Label("Fields changed", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                ModerationFieldRow(label: "Document", value: fieldsChange.documentId, monospaced: true)
                if let revision = fieldsChange.revision {
                    ModerationFieldRow(label: "Revision", value: "\(revision)")
                }
                Text(BidiControls.jsonEscaped(fieldsChange.documentJSON))
                    .font(.system(.caption2, design: .monospaced))
                    .textSelection(.enabled)
            }
            .accessibilityIdentifier("contractModeration.document.changeResult")
        }
        if let proposal {
            Section("Team Action") {
                ModerationFieldRow(label: "Action ID", value: proposal.actionId, monospaced: true)
                ModerationFieldRow(
                    label: "Status",
                    value: proposal.status == .closed
                        ? "Closed: the document is deleted"
                        : "Active: other members must approve"
                )
            }
            .accessibilityIdentifier("contractModeration.document.proposalResult")
        }
    }

    private var trimmedDocumentId: String {
        documentId.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// A JSON object naming each changeable field, for the moderator to fill.
    private static func fieldsTemplate(_ fields: [String]) -> String {
        let members = fields.map { "  \(ContractKeptFieldValue.quoted($0)): null" }
        return "{\n" + members.joined(separator: ",\n") + "\n}"
    }

    /// The alias of a local identity, or its base58 id.
    private func label(forIdentity base58: String) -> String {
        identities.first { $0.identityIdBase58 == base58 }?.alias ?? base58
    }

    /// What every operation needs, or `nil` with the reason shown.
    private func prepare() -> (ManagedPlatformWallet, Data, Data, Data, ContractModerationReasonInput)? {
        guard let moderator,
              let wallet = walletManager.wallet(for: moderator.walletId),
              let moderatorId = Data.identifier(fromBase58: moderator.identityIdBase58),
              let contractId = Data.identifier(fromBase58: scope.contractId) else {
            errorText = DocumentActionError.noWallet.localizedDescription
            return nil
        }
        guard let documentIdBytes = Data.identifier(fromBase58: trimmedDocumentId) else {
            errorText = "The document id is not a base58 identifier."
            return nil
        }
        guard let reason = ModerationReasonFields.input(text: reasonText, reasonDocumentId: reasonDocumentId) else {
            errorText = "The reason document id is not a base58 identifier."
            return nil
        }
        errorText = nil
        deletion = nil
        fieldsChange = nil
        proposal = nil
        return (wallet, moderatorId, contractId, documentIdBytes, reason)
    }

    private func run(_ operation: Operation, _ body: @escaping @MainActor () async throws -> Void) {
        running = operation
        Task { @MainActor in
            do {
                try await body()
            } catch {
                errorText = moderationWriteErrorText(error)
            }
            running = nil
        }
    }

    private func delete() {
        guard case let (wallet, moderatorId, contractId, documentIdBytes, reason)? = prepare() else { return }
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let contractIdBase58 = scope.contractId
        let documentTypeName = documentTypeName
        run(.delete) {
            let result = try await wallet.deleteContractDocument(
                moderatorIdentityId: moderatorId,
                contractId: contractId,
                documentTypeName: documentTypeName,
                documentId: documentIdBytes,
                reason: reason,
                signer: signer
            )
            keptForRestore = result.documentData.map {
                ModeratedDocumentArchive.standard.save(
                    $0,
                    contractId: contractIdBase58,
                    documentTypeName: documentTypeName,
                    documentId: result.documentId
                )
            } ?? false
            deletion = result
        }
    }

    private func changeFields() {
        guard case let (wallet, moderatorId, contractId, documentIdBytes, reason)? = prepare() else { return }
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let documentTypeName = documentTypeName
        let fieldsJSON = fieldsJSON
        run(.changeFields) {
            fieldsChange = try await wallet.changeContractDocumentFields(
                moderatorIdentityId: moderatorId,
                contractId: contractId,
                documentTypeName: documentTypeName,
                documentId: documentIdBytes,
                fieldsJSON: fieldsJSON,
                reason: reason,
                signer: signer
            )
        }
    }

    private func proposeSettledDeletion() {
        guard case let (wallet, moderatorId, contractId, documentIdBytes, reason)? = prepare() else { return }
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let documentTypeName = documentTypeName
        run(.proposeSettledDeletion) {
            proposal = try await wallet.proposeSettledContractDocumentDeletion(
                moderatorIdentityId: moderatorId,
                contractId: contractId,
                documentTypeName: documentTypeName,
                documentId: documentIdBytes,
                reason: reason,
                signer: signer
            )
        }
    }
}
