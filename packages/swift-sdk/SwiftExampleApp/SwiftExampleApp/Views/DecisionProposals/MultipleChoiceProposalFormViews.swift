import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Shared input

/// An amount of credits as Dash, for display (1 Dash = 100,000,000,000
/// credits).
func multipleChoiceProposalDash(_ credits: UInt64) -> String {
    let dash = Double(credits) / 100_000_000_000
    return "\(dash.formatted(.number.precision(.fractionLength(0...8)))) Dash"
}

/// The three text fields naming a description document, as typed.
struct DescriptionDocumentInput: Equatable {
    var id = ""
    var contractId = ""
    var documentType = ""

    init() {}

    init(_ document: MultipleChoiceProposal.DescriptionDocument?) {
        id = document?.id ?? ""
        contractId = document?.contractId ?? ""
        documentType = document?.documentType ?? ""
    }

    private var trimmed: (id: String, contractId: String, documentType: String) {
        (
            id.trimmingCharacters(in: .whitespacesAndNewlines),
            contractId.trimmingCharacters(in: .whitespacesAndNewlines),
            documentType.trimmingCharacters(in: .whitespacesAndNewlines)
        )
    }

    /// Whether every field is empty: no description document.
    var isEmpty: Bool {
        let fields = trimmed
        return fields.id.isEmpty && fields.contractId.isEmpty && fields.documentType.isEmpty
    }

    /// Whether the fields name `document`.
    func matches(_ document: MultipleChoiceProposal.DescriptionDocument?) -> Bool {
        let fields = trimmed
        guard let document else { return isEmpty }
        return fields.id == document.id
            && fields.contractId == document.contractId
            && fields.documentType == document.documentType
    }

    /// The fields as the SDK params, `nil` when they are all empty. Throws
    /// when an id is not base58; Platform judges the rest.
    func params() throws -> DescriptionDocumentParams? {
        guard !isEmpty else { return nil }
        let fields = trimmed
        guard let id = Data.identifier(fromBase58: fields.id) else {
            throw DescriptionDocumentInputError.notBase58("document id")
        }
        guard let contractId = Data.identifier(fromBase58: fields.contractId) else {
            throw DescriptionDocumentInputError.notBase58("contract id")
        }
        return DescriptionDocumentParams(id: id, contractId: contractId, documentType: fields.documentType)
    }
}

enum DescriptionDocumentInputError: LocalizedError {
    case notBase58(String)

    var errorDescription: String? {
        switch self {
        case .notBase58(let field):
            return "The description document's \(field) is not a base58 identifier."
        }
    }
}

/// The text fields of a ``DescriptionDocumentInput``.
private struct DescriptionDocumentFields: View {
    @Binding var input: DescriptionDocumentInput
    let identifierPrefix: String

    var body: some View {
        TextField("Document ID (base58)", text: $input.id)
            .font(.system(.caption, design: .monospaced))
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            .accessibilityIdentifier("\(identifierPrefix).descriptionDocumentId")
        TextField("Contract ID (base58)", text: $input.contractId)
            .font(.system(.caption, design: .monospaced))
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            .accessibilityIdentifier("\(identifierPrefix).descriptionDocumentContractId")
        TextField("Document type", text: $input.documentType)
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            .accessibilityIdentifier("\(identifierPrefix).descriptionDocumentType")
    }
}

// MARK: - Create

/// Files a new multiple choice proposal for an identity held by a loaded
/// wallet.
///
/// The platform wallet creates it
/// (`ManagedPlatformWallet.createMultipleChoiceProposal`): rs-sdk builds the
/// proposal and the wallet chooses and uses the signing key on the Rust side
/// through the `KeychainSigner`. Platform judges the title, the description
/// document and the url, and charges the processing fee shown here (read
/// from the SDK); this form only collects the input.
struct CreateMultipleChoiceProposalView: View {
    /// Called with the new proposal's id (base58) once Platform confirmed it.
    let onCreated: (String) -> Void

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss
    @Query private var identities: [PersistentIdentity]

    /// Base58 id of the proposer.
    @State private var selectedOwnerId = ""
    @State private var title = ""
    @State private var url = ""
    @State private var descriptionDocument = DescriptionDocumentInput()
    @State private var open = true

    /// The processing fee in credits, once read.
    @State private var processingFee: UInt64?
    @State private var processingFeeError: String?

    @State private var isSubmitting = false
    @State private var submitError: String?
    @State private var createdProposalId: String?

    /// Identities on the active network held by a loaded wallet, so a
    /// `KeychainSigner` can sign for them.
    private var ownerIdentities: [PersistentIdentity] {
        identities.filter { $0.network == appState.currentNetwork && $0.wallet != nil }
    }

    private var selectedOwner: PersistentIdentity? {
        ownerIdentities.first { $0.identityIdBase58 == selectedOwnerId }
    }

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = selectedOwner?.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    var body: some View {
        NavigationStack {
            Form {
                if let createdProposalId {
                    successSection(proposalId: createdProposalId)
                } else {
                    ownerSection
                    questionSection
                    descriptionDocumentSection
                    submitSection
                }
            }
            .navigationTitle("New Proposal")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button(createdProposalId == nil ? "Cancel" : "Done") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            // The broadcast is not idempotent; keep the sheet up until it ends.
            .interactiveDismissDisabled(isSubmitting)
            .alert(
                "Create failed",
                isPresented: Binding(
                    get: { submitError != nil },
                    set: { if !$0 { submitError = nil } }
                )
            ) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(submitError ?? "")
            }
            .onAppear {
                if selectedOwnerId.isEmpty, let first = ownerIdentities.first {
                    selectedOwnerId = first.identityIdBase58
                }
            }
            .task {
                await loadProcessingFee()
            }
        }
    }

    // MARK: - Sections

    private var ownerSection: some View {
        Section {
            Picker("Proposer", selection: $selectedOwnerId) {
                Text("Select identity").tag("")
                ForEach(ownerIdentities) { identity in
                    Text(identity.alias ?? identity.identityIdBase58)
                        .tag(identity.identityIdBase58)
                }
            }
            .accessibleFormPicker("createMultipleChoiceProposal.ownerPicker")
            .disabled(isSubmitting)

            HStack {
                Text("Processing fee")
                Spacer()
                if let processingFee {
                    Text(multipleChoiceProposalDash(processingFee))
                        .foregroundColor(.secondary)
                        .accessibilityIdentifier("createMultipleChoiceProposal.processingFee")
                } else if processingFeeError != nil {
                    Text("Unavailable")
                        .foregroundColor(.orange)
                } else {
                    ProgressView().controlSize(.small)
                }
            }
        } header: {
            Text("Proposer")
        } footer: {
            if let processingFeeError {
                Text("The processing fee could not be read: \(processingFeeError)")
            } else {
                Text("Platform charges this identity the processing fee on top of the fees when the proposal is created.")
            }
        }
    }

    private var questionSection: some View {
        Section {
            TextField("Title: the question, in one line", text: $title)
                .accessibilityIdentifier("createMultipleChoiceProposal.title")
            TextField("URL where it is described or discussed", text: $url)
                .keyboardType(.URL)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .accessibilityIdentifier("createMultipleChoiceProposal.url")
            Toggle("Anyone may enter choices", isOn: $open)
                .accessibilityIdentifier("createMultipleChoiceProposal.open")
        } header: {
            Text("Question")
        } footer: {
            Text("When it is off, only the proposer may enter choices. This never changes after the proposal is created; the title, url and description document can be edited for two hours.")
        }
        .disabled(isSubmitting)
    }

    private var descriptionDocumentSection: some View {
        Section {
            DescriptionDocumentFields(
                input: $descriptionDocument,
                identifierPrefix: "createMultipleChoiceProposal"
            )
        } header: {
            Text("Description Document (optional)")
        } footer: {
            Text("A document saying what is being decided. Platform requires a description document, a url, or both, and accepts only a document its contract's elected team may delete.")
        }
        .disabled(isSubmitting)
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView()
                            .controlSize(.small)
                        Text("Broadcasting…")
                    } else {
                        Text("Create / Broadcast")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("createMultipleChoiceProposal.submitButton")
            .disabled(isSubmitting || managedWallet == nil)
        }
    }

    private func successSection(proposalId: String) -> some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Proposal created", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                HStack(alignment: .top) {
                    Text("ID:")
                        .foregroundColor(.secondary)
                    Text(proposalId)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(2)
                        .truncationMode(.middle)
                }
                Button {
                    dismiss()
                } label: {
                    Text("Done")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("createMultipleChoiceProposal.doneButton")
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Actions

    private func loadProcessingFee() async {
        guard let sdk = appState.sdk else {
            processingFeeError = "SDK not initialized"
            return
        }
        do {
            processingFee = try await sdk.multipleChoiceProposalProcessingFee()
            processingFeeError = nil
        } catch {
            processingFeeError = error.localizedDescription
        }
    }

    private func submit() {
        guard let owner = selectedOwner, let wallet = managedWallet else {
            submitError = DocumentActionError.noWallet.localizedDescription
            return
        }
        let document: DescriptionDocumentParams?
        do {
            document = try descriptionDocument.params()
        } catch {
            submitError = error.localizedDescription
            return
        }

        let trimmedURL = url.trimmingCharacters(in: .whitespacesAndNewlines)
        let params = MultipleChoiceProposalParams(
            title: title,
            descriptionDocument: document,
            url: trimmedURL.isEmpty ? nil : trimmedURL,
            open: open
        )
        let ownerId = owner.identityId
        // Fresh `KeychainSigner` per submit, as `CreateDocumentView` does.
        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            do {
                let (proposalId, _) = try await wallet.createMultipleChoiceProposal(
                    ownerIdentityId: ownerId,
                    params: params,
                    signer: signer
                )
                let proposalIdBase58 = proposalId.toBase58String()
                createdProposalId = proposalIdBase58
                onCreated(proposalIdBase58)
            } catch {
                submitError = error.localizedDescription
            }
            isSubmitting = false
        }
    }
}

// MARK: - Edit

/// Changes the title, url or description document of a proposal a local
/// identity filed.
///
/// The platform wallet edits it
/// (`ManagedPlatformWallet.editMultipleChoiceProposal`): it reads the
/// proposal, rs-sdk applies the edit (refusing one that changes nothing) and
/// the wallet chooses the signing key and puts the proposal at the revision
/// after the one it read. Platform refuses the edit outside the edit window,
/// one that leaves the proposal with neither a description document nor a
/// url, and one made after another edit landed after that read.
///
/// After any failure the form reads the proposal again. When Platform's copy
/// differs from the one the form was opened on, the form shows that copy in
/// place of the fields, so the next try starts from it.
struct EditMultipleChoiceProposalView: View {
    let owner: PersistentIdentity
    /// Called with a summary once Platform confirmed the edit.
    let onEdited: (String) -> Void

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// The proposal as last read, which the fields were filled from.
    @State private var proposal: MultipleChoiceProposal
    @State private var title: String
    @State private var url: String
    @State private var descriptionDocument: DescriptionDocumentInput
    @State private var isSubmitting = false
    @State private var submitError: String?

    init(proposal: MultipleChoiceProposal, owner: PersistentIdentity, onEdited: @escaping (String) -> Void) {
        self.owner = owner
        self.onEdited = onEdited
        _proposal = State(initialValue: proposal)
        _title = State(initialValue: proposal.title)
        _url = State(initialValue: proposal.url ?? "")
        _descriptionDocument = State(initialValue: DescriptionDocumentInput(proposal.descriptionDocument))
    }

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = owner.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Title", text: $title)
                        .accessibilityIdentifier("editMultipleChoiceProposal.title")
                    TextField("URL (empty for none)", text: $url)
                        .keyboardType(.URL)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .accessibilityIdentifier("editMultipleChoiceProposal.url")
                } header: {
                    Text("Question")
                }
                .disabled(isSubmitting)

                Section {
                    DescriptionDocumentFields(
                        input: $descriptionDocument,
                        identifierPrefix: "editMultipleChoiceProposal"
                    )
                } header: {
                    Text("Description Document")
                } footer: {
                    Text("Empty every field to remove it. Only the title, url and description document can change, and only for two hours after the proposal was created.")
                }
                .disabled(isSubmitting)

                Section {
                    Button {
                        submit()
                    } label: {
                        HStack {
                            if isSubmitting {
                                ProgressView()
                                    .controlSize(.small)
                                Text("Broadcasting…")
                            } else {
                                Text("Replace / Broadcast")
                            }
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("editMultipleChoiceProposal.submitButton")
                    .disabled(isSubmitting || managedWallet == nil)
                }
            }
            .navigationTitle("Edit Proposal")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .interactiveDismissDisabled(isSubmitting)
            .alert(
                "Edit failed",
                isPresented: Binding(
                    get: { submitError != nil },
                    set: { if !$0 { submitError = nil } }
                )
            ) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(submitError ?? "")
            }
        }
    }

    private func submit() {
        guard let wallet = managedWallet else {
            submitError = DocumentActionError.noWallet.localizedDescription
            return
        }
        guard let proposalId = Data.identifier(fromBase58: proposal.id), proposalId.count == 32 else {
            submitError = "The proposal id is not a 32-byte base58 identifier."
            return
        }

        // Only the fields the user changed from the proposal this form shows
        // are sent: an untouched field must not write back the value it was
        // opened with over a newer edit by another client. An emptied field
        // removes the value.
        let trimmedURL = url.trimmingCharacters(in: .whitespacesAndNewlines)
        let urlChange: MultipleChoiceProposalEdit.Change<String>
        if trimmedURL == (proposal.url ?? "") {
            urlChange = .keep
        } else if trimmedURL.isEmpty {
            urlChange = .remove
        } else {
            urlChange = .set(trimmedURL)
        }
        let documentChange: MultipleChoiceProposalEdit.Change<DescriptionDocumentParams>
        if descriptionDocument.matches(proposal.descriptionDocument) {
            documentChange = .keep
        } else {
            do {
                if let document = try descriptionDocument.params() {
                    documentChange = .set(document)
                } else {
                    documentChange = .remove
                }
            } catch {
                submitError = error.localizedDescription
                return
            }
        }
        let edit = MultipleChoiceProposalEdit(
            title: title == proposal.title ? nil : title,
            descriptionDocument: documentChange,
            url: urlChange
        )
        let ownerId = owner.identityId
        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            do {
                _ = try await wallet.editMultipleChoiceProposal(
                    ownerIdentityId: ownerId,
                    proposalId: proposalId,
                    edit: edit,
                    signer: signer
                )
                onEdited("Edit accepted by Platform.")
                isSubmitting = false
                dismiss()
            } catch {
                let reloadNote = await reloadAfterFailure()
                submitError = [error.localizedDescription, reloadNote]
                    .compactMap { $0 }
                    .joined(separator: "\n\n")
                isSubmitting = false
            }
        }
    }

    /// Read the proposal again after a failed edit. When Platform's copy
    /// differs from the one the fields were filled from, take it and refill
    /// the fields. Returns what the person should know about the reload, or
    /// `nil` when there is nothing to add.
    private func reloadAfterFailure() async -> String? {
        guard let sdk = appState.sdk else { return nil }
        do {
            guard let current = try await sdk.multipleChoiceProposal(id: proposal.id) else {
                return "Platform no longer has this proposal."
            }
            guard current != proposal else { return nil }
            proposal = current
            title = current.title
            url = current.url ?? ""
            descriptionDocument = DescriptionDocumentInput(current.descriptionDocument)
            let revision = current.revision.map { " (revision \($0))" } ?? ""
            return "Platform's copy of the proposal changed since this form was opened; the form now shows it\(revision)."
        } catch {
            return "The proposal could not be read again: \(error.localizedDescription)"
        }
    }
}

// MARK: - Enter a choice

/// Enters an answer for a multiple choice proposal, owned by an identity
/// held by a loaded wallet.
///
/// The platform wallet enters it
/// (`ManagedPlatformWallet.enterMultipleChoiceProposalChoice`): rs-sdk builds
/// the choice and its put states the contest fund shown here (read from the
/// SDK). Platform decides who may enter a choice and until when; this form
/// only collects the answer.
struct EnterProposalChoiceView: View {
    let proposal: MultipleChoiceProposal
    /// Called with a summary once Platform confirmed the choice.
    let onEntered: (String) -> Void

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss
    @Query private var identities: [PersistentIdentity]

    /// Base58 id of the identity entering the choice.
    @State private var selectedOwnerId = ""
    @State private var title = ""
    @State private var description = ""

    /// The contest fund in credits, once read.
    @State private var fund: UInt64?
    @State private var fundError: String?

    @State private var isSubmitting = false
    @State private var submitError: String?

    /// Identities on the active network held by a loaded wallet, so a
    /// `KeychainSigner` can sign for them.
    private var ownerIdentities: [PersistentIdentity] {
        identities.filter { $0.network == appState.currentNetwork && $0.wallet != nil }
    }

    private var selectedOwner: PersistentIdentity? {
        ownerIdentities.first { $0.identityIdBase58 == selectedOwnerId }
    }

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = selectedOwner?.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Answering") {
                    Text(proposal.title)
                        .font(.headline)
                }

                Section {
                    Picker("Entered by", selection: $selectedOwnerId) {
                        Text("Select identity").tag("")
                        ForEach(ownerIdentities) { identity in
                            Text(identity.alias ?? identity.identityIdBase58)
                                .tag(identity.identityIdBase58)
                        }
                    }
                    .accessibleFormPicker("enterProposalChoice.ownerPicker")

                    HStack {
                        Text("Contest fund")
                        Spacer()
                        if let fund {
                            Text(multipleChoiceProposalDash(fund))
                                .foregroundColor(.secondary)
                                .accessibilityIdentifier("enterProposalChoice.fund")
                        } else if fundError != nil {
                            Text("Unavailable")
                                .foregroundColor(.orange)
                        } else {
                            ProgressView().controlSize(.small)
                        }
                    }
                } header: {
                    Text("Identity")
                } footer: {
                    if let fundError {
                        Text("The contest fund could not be read: \(fundError)")
                    } else if proposal.open {
                        Text("Entering a choice pays the contest fund from this identity's credits, on top of the fees. The amount is read again when the choice is signed.")
                    } else {
                        Text("Only the proposer may enter choices for this proposal. Entering a choice pays the contest fund on top of the fees.")
                    }
                }
                .disabled(isSubmitting)

                Section {
                    TextField("Title: the answer, in one line", text: $title)
                        .accessibilityIdentifier("enterProposalChoice.title")
                    TextField("What voting for it means (optional)", text: $description, axis: .vertical)
                        .lineLimit(3...10)
                        .accessibilityIdentifier("enterProposalChoice.description")
                } header: {
                    Text("Choice")
                } footer: {
                    Text("A choice is never edited after it is entered.")
                }
                .disabled(isSubmitting)

                Section {
                    Button {
                        submit()
                    } label: {
                        HStack {
                            if isSubmitting {
                                ProgressView()
                                    .controlSize(.small)
                                Text("Broadcasting…")
                            } else {
                                Text("Enter / Broadcast")
                            }
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("enterProposalChoice.submitButton")
                    .disabled(isSubmitting || managedWallet == nil)
                }
            }
            .navigationTitle("Enter a Choice")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            // The broadcast is not idempotent; keep the sheet up until it ends.
            .interactiveDismissDisabled(isSubmitting)
            .alert(
                "Entering the choice failed",
                isPresented: Binding(
                    get: { submitError != nil },
                    set: { if !$0 { submitError = nil } }
                )
            ) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(submitError ?? "")
            }
            .onAppear {
                if selectedOwnerId.isEmpty {
                    // Start on the proposer when it is local, else the first
                    // local identity.
                    selectedOwnerId = ownerIdentities.first { $0.identityIdBase58 == proposal.ownerId }?
                        .identityIdBase58 ?? ownerIdentities.first?.identityIdBase58 ?? ""
                }
            }
            .task {
                await loadFund()
            }
        }
    }

    private func loadFund() async {
        guard let sdk = appState.sdk else {
            fundError = "SDK not initialized"
            return
        }
        do {
            fund = try await sdk.multipleChoiceProposalChoiceFundToJoin(proposalId: proposal.id)
            fundError = nil
        } catch {
            fundError = error.localizedDescription
        }
    }

    private func submit() {
        guard let owner = selectedOwner, let wallet = managedWallet else {
            submitError = DocumentActionError.noWallet.localizedDescription
            return
        }
        guard let proposalId = Data.identifier(fromBase58: proposal.id), proposalId.count == 32 else {
            submitError = "The proposal id is not a 32-byte base58 identifier."
            return
        }

        let trimmedDescription = description.trimmingCharacters(in: .whitespacesAndNewlines)
        let params = ProposalChoiceParams(
            title: title,
            description: trimmedDescription.isEmpty ? nil : description
        )
        let ownerId = owner.identityId
        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            do {
                let (choiceId, _) = try await wallet.enterMultipleChoiceProposalChoice(
                    ownerIdentityId: ownerId,
                    proposalId: proposalId,
                    params: params,
                    signer: signer
                )
                onEntered("Choice \(choiceId.toBase58String()) accepted by Platform.")
                isSubmitting = false
                dismiss()
            } catch {
                submitError = error.localizedDescription
                isSubmitting = false
            }
        }
    }
}
