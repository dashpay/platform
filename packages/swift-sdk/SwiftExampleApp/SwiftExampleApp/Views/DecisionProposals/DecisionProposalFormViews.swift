import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Create

/// Files a new decision proposal for an identity held by a loaded wallet.
///
/// rs-sdk builds the proposal (`SDK.buildDecisionProposal`) and the platform
/// wallet creates it (`ManagedPlatformWallet.createDecisionProposal`), the
/// same document-create path `CreateDocumentView` takes, so the signing key
/// is chosen and used on the Rust side through the wallet's
/// `KeychainSigner`. Platform judges the text, the rule and the end date;
/// this form only collects them.
struct CreateDecisionProposalView: View {
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
    @State private var description = ""
    @State private var url = ""
    @State private var passPercent = 60
    @State private var minimumTurnoutPercent = 10
    @State private var endDate = Date().addingTimeInterval(14 * 24 * 60 * 60)

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
                    ruleSection
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
            .accessibleFormPicker("createDecisionProposal.ownerPicker")
            .disabled(isSubmitting)
        } header: {
            Text("Proposer")
        } footer: {
            Text("The create pays the decision proposal fund (0.5 Dash) from this identity's credits, on top of the fees.")
        }
    }

    private var questionSection: some View {
        Section {
            TextField("Title: the question, in one line", text: $title)
                .accessibilityIdentifier("createDecisionProposal.title")
            TextField("Description: what a yes and a no mean", text: $description, axis: .vertical)
                .lineLimit(3...10)
                .accessibilityIdentifier("createDecisionProposal.description")
            TextField("URL where it is discussed (optional)", text: $url)
                .keyboardType(.URL)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .accessibilityIdentifier("createDecisionProposal.url")
        } header: {
            Text("Question")
        } footer: {
            Text("The title, description and url can be edited for a day after the proposal is created.")
        }
        .disabled(isSubmitting)
    }

    private var ruleSection: some View {
        Section {
            Stepper(value: $passPercent, in: 0...100) {
                LabeledContent("Passes when yes is more than", value: "\(passPercent)%")
            }
            .accessibilityIdentifier("createDecisionProposal.passPercent")
            Stepper(value: $minimumTurnoutPercent, in: 0...100) {
                LabeledContent("Minimum turnout", value: "\(minimumTurnoutPercent)%")
            }
            .accessibilityIdentifier("createDecisionProposal.turnoutPercent")
            DatePicker("Voting ends", selection: $endDate, in: Date()...)
                .accessibilityIdentifier("createDecisionProposal.endDate")
        } header: {
            Text("Rule")
        } footer: {
            Text("Yes must be more than the pass percent of yes plus no, and yes plus no must reach the turnout percent of the masternodes' total voting power. Platform accepts a pass percent of 50 to 80, a turnout of 0 to 50, and an end date one to four weeks after the block that creates the proposal. The rule never changes.")
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
            .accessibilityIdentifier("createDecisionProposal.submitButton")
            .disabled(isSubmitting || appState.sdk == nil || managedWallet == nil)
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
                .accessibilityIdentifier("createDecisionProposal.doneButton")
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Submit

    private func submit() {
        guard let sdk = appState.sdk else {
            submitError = "SDK not initialized"
            return
        }
        guard let owner = selectedOwner, let wallet = managedWallet else {
            submitError = DocumentActionError.noWallet.localizedDescription
            return
        }

        let trimmedURL = url.trimmingCharacters(in: .whitespacesAndNewlines)
        let params = DecisionProposalParams(
            title: title,
            description: description,
            url: trimmedURL.isEmpty ? nil : trimmedURL,
            passPercent: UInt8(clamping: passPercent),
            minimumTurnoutPercent: UInt8(clamping: minimumTurnoutPercent),
            endDate: endDate
        )
        let ownerIdBase58 = owner.identityIdBase58
        let ownerId = owner.identityId
        // Fresh `KeychainSigner` per submit, as `CreateDocumentView` does.
        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            do {
                let proposal = try await sdk.buildDecisionProposal(
                    ownerIdentityId: ownerIdBase58,
                    params: params
                )
                let (proposalId, _) = try await wallet.createDecisionProposal(
                    proposal,
                    ownerIdentityId: ownerId,
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

/// Gives a proposal a local identity filed new text.
///
/// rs-sdk reads the stored proposal and applies the edit
/// (`SDK.fetchDecisionProposalDocument`, `DecisionProposalDocument.edited`),
/// refusing one that changes nothing; the platform wallet replaces it
/// (`ManagedPlatformWallet.replaceDecisionProposal`), signing with the key
/// `DocumentActionRunner.resolveSigning` picks, as `ReplaceDocumentView`
/// does. Platform refuses the replace outside the edit window.
struct EditDecisionProposalView: View {
    let proposal: DecisionProposal
    let owner: PersistentIdentity
    /// Called with a summary once Platform confirmed the edit.
    let onEdited: (String) -> Void

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var title: String
    @State private var description: String
    @State private var url: String
    @State private var isSubmitting = false
    @State private var submitError: String?

    init(proposal: DecisionProposal, owner: PersistentIdentity, onEdited: @escaping (String) -> Void) {
        self.proposal = proposal
        self.owner = owner
        self.onEdited = onEdited
        _title = State(initialValue: proposal.title)
        _description = State(initialValue: proposal.description)
        _url = State(initialValue: proposal.url ?? "")
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Title", text: $title)
                        .accessibilityIdentifier("editDecisionProposal.title")
                    TextField("Description", text: $description, axis: .vertical)
                        .lineLimit(3...10)
                        .accessibilityIdentifier("editDecisionProposal.description")
                    TextField("URL (empty for none)", text: $url)
                        .keyboardType(.URL)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .accessibilityIdentifier("editDecisionProposal.url")
                } header: {
                    Text("Text")
                } footer: {
                    Text("Only the title, description and url can change, and only for a day after the proposal was created (or a fifth of its voting time, if that is shorter).")
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
                    .accessibilityIdentifier("editDecisionProposal.submitButton")
                    .disabled(isSubmitting || appState.sdk == nil)
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
        guard let sdk = appState.sdk else {
            submitError = "SDK not initialized"
            return
        }
        guard let proposalId = Data.identifier(fromBase58: proposal.id), proposalId.count == 32 else {
            submitError = "The proposal id is not a 32-byte base58 identifier."
            return
        }
        let wallet: ManagedPlatformWallet
        let signingKeyId: UInt32
        do {
            // The proposal type states no signature security level, so the
            // document type is left out and the HIGH default applies.
            (wallet, signingKeyId) = try DocumentActionRunner.resolveSigning(
                for: owner,
                documentType: nil,
                walletManager: walletManager
            )
        } catch {
            submitError = error.localizedDescription
            return
        }

        // An emptied url field removes the url; an untouched empty one keeps
        // having none.
        let trimmedURL = url.trimmingCharacters(in: .whitespacesAndNewlines)
        let urlChange: DecisionProposalEdit.URLChange
        if trimmedURL.isEmpty {
            urlChange = proposal.url == nil ? .keep : .remove
        } else {
            urlChange = .set(trimmedURL)
        }
        let edit = DecisionProposalEdit(title: title, description: description, url: urlChange)
        let ownerId = owner.identityId
        let proposalIdBase58 = proposal.id
        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            do {
                guard let stored = try await sdk.fetchDecisionProposalDocument(
                    proposalId: proposalIdBase58
                ) else {
                    throw SDKError.notFound("Platform has no proposal \(proposalIdBase58)")
                }
                let edited = try stored.edited(edit)
                _ = try await wallet.replaceDecisionProposal(
                    edited,
                    ownerIdentityId: ownerId,
                    proposalId: proposalId,
                    signingKeyId: signingKeyId,
                    signer: signer
                )
                onEdited("Edit accepted by Platform.")
                isSubmitting = false
                dismiss()
            } catch {
                submitError = error.localizedDescription
                isSubmitting = false
            }
        }
    }
}
