import SwiftUI
import SwiftData
import SwiftDashSDK
import UniformTypeIdentifiers

/// Update a data contract a local identity owns (TEST_PLAN DC-04): the iOS
/// counterpart of the Kotlin app's `UpdateContractScreen`, with a review step
/// in front of the broadcast.
///
/// The five inputs are the wallet's update arguments. The wallet fetches the
/// live contract and merges them onto it: document types, tokens and groups
/// are added or replaced key by key, omitted ones keep their on-chain
/// definition, and keywords and description replace the live ones when
/// given. "Review Changes" asks the wallet what the update would change
/// (`previewDataContractUpdate`); "Update Contract" is offered only when that
/// review reports a change, so a paid update that changes nothing is not
/// sent. Whether Platform accepts the update is decided by the network when
/// it is broadcast; this screen checks no contract update rule.
struct UpdateContractView: View {
    let contract: PersistentDataContract

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var documentsJSON = "{}"
    @State private var tokensJSON = ""
    @State private var groupsJSON = ""
    @State private var keywords = ""
    @State private var descriptionText = ""

    /// The review on screen and the wallet arguments it was made with. The
    /// update is sent with exactly these arguments.
    @State private var review: Review?
    @State private var isReviewing = false
    @State private var isSubmitting = false
    @State private var showingFileImporter = false
    /// One line under the source buttons: what a source filled in, or why
    /// it filled nothing.
    @State private var sourceMessage: String?
    @State private var errorMessage: String?
    @State private var outcome: Outcome?
    /// Set once an update was sent and its outcome could not be confirmed.
    /// Update Contract then stays off for the rest of this screen's life, so
    /// a second update is not sent while the first may still land. Held by
    /// this screen only, not persisted.
    @State private var unconfirmedUpdateSent = false

    var body: some View {
        Form {
            contractSection
            sourceSection
            documentsSection
            tokensSection
            groupsSection
            metadataSection
            reviewSection
            submitSection
        }
        .navigationTitle("Update Contract")
        .navigationBarTitleDisplayMode(.inline)
        .navigationBarBackButtonHidden(isBusy)
        .onChange(of: inputs) { _, _ in
            review = nil
        }
        .fileImporter(
            isPresented: $showingFileImporter,
            allowedContentTypes: [.json, .plainText]
        ) { result in
            importFile(result)
        }
        .alert(
            "Update Failed",
            isPresented: Binding(
                get: { errorMessage != nil },
                set: { if !$0 { errorMessage = nil } }
            )
        ) {
            Button("OK", role: .cancel) { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "")
        }
        .alert(
            outcome?.title ?? "",
            isPresented: Binding(
                get: { outcome != nil },
                set: { if !$0 { outcome = nil } }
            ),
            presenting: outcome
        ) { presented in
            Button("OK") {
                outcome = nil
                if presented.dismissesScreen {
                    dismiss()
                }
            }
        } message: { presented in
            Text(presented.message)
                .accessibilityIdentifier(presented.dismissesScreen ? "updateContract.success" : "updateContract.unconfirmed")
        }
    }

    // MARK: - Sections

    @ViewBuilder
    private var contractSection: some View {
        Section("Contract") {
            LabeledContent("Name", value: contract.name)
            LabeledContent("ID") {
                Text(contract.idBase58)
                    .font(.caption.monospaced())
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            LabeledContent("Owner", value: contract.ownerIdentity?.displayName ?? "Not held locally")
            LabeledContent("Stored Version", value: contract.version.map { String($0) } ?? "Unknown")
            if ownerWallet == nil {
                Text("The owner identity has no wallet linkage, so this device cannot sign the update.")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
    }

    @ViewBuilder
    private var sourceSection: some View {
        Section {
            Button {
                pasteFromClipboard()
            } label: {
                Label("Paste from Clipboard", systemImage: "doc.on.clipboard")
            }
            .disabled(isBusy)
            .accessibilityIdentifier("updateContract.pasteButton")

            Button {
                showingFileImporter = true
            } label: {
                Label("Import from File", systemImage: "folder")
            }
            .disabled(isBusy)
            .accessibilityIdentifier("updateContract.importButton")

            if let sourceMessage {
                Text(sourceMessage)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("updateContract.sourceMessage")
            }
        } header: {
            Text("Source")
        } footer: {
            Text("Fills the fields below from contract JSON. Nothing is sent until you review the changes and tap Update Contract.")
        }
    }

    @ViewBuilder
    private var documentsSection: some View {
        Section {
            jsonEditor(text: $documentsJSON, identifier: "updateContract.documentsJson")
        } header: {
            Text("Document Schemas (JSON)")
        } footer: {
            Text("Merged onto the live contract: new document types are added, existing ones replaced key by key, and omitted ones keep their on-chain definition. Use {} to leave document types unchanged.")
        }
    }

    @ViewBuilder
    private var tokensSection: some View {
        Section {
            jsonEditor(text: $tokensJSON, identifier: "updateContract.tokensJson")
        } header: {
            Text("Token Schemas (JSON, optional)")
        } footer: {
            Text("Keyed by token position. This update can add new tokens; Platform refuses a change to an existing token. Omitted tokens keep their on-chain definition.")
        }
    }

    @ViewBuilder
    private var groupsSection: some View {
        Section {
            jsonEditor(text: $groupsJSON, identifier: "updateContract.groupsJson")
        } header: {
            Text("Groups (JSON, optional)")
        } footer: {
            Text("Keyed by group position, or a list of groups each with an `id`. This update can add new groups; Platform refuses a change to an existing group. Omitted groups keep their on-chain definition.")
        }
    }

    @ViewBuilder
    private var metadataSection: some View {
        Section {
            TextField("Keywords (comma separated or a JSON array)", text: $keywords)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .disabled(isBusy)
                .accessibilityIdentifier("updateContract.keywords")
            TextField("Description", text: $descriptionText)
                .disabled(isBusy)
                .accessibilityIdentifier("updateContract.description")
        } header: {
            Text("Metadata (Optional)")
        } footer: {
            Text("Replace the live keywords or description when given; left empty, the on-chain values are kept. Keywords are comma separated, or a JSON array such as [\"alpha,beta\", \"gamma\"] for keywords that contain a comma. The description is sent as typed.")
        }
    }

    @ViewBuilder
    private var reviewSection: some View {
        Section {
            Button {
                Task { await reviewChanges() }
            } label: {
                HStack {
                    if isReviewing {
                        ProgressView().controlSize(.small)
                    }
                    Label("Review Changes", systemImage: "list.bullet.clipboard")
                }
            }
            .disabled(isBusy || ownerWallet == nil)
            .accessibilityIdentifier("updateContract.reviewButton")
        } footer: {
            Text("The review lists what the update would change. Platform decides whether the update is accepted when it is broadcast.")
        }

        if let review {
            UpdateContractPreviewSection(preview: review.preview)
        }
    }

    @ViewBuilder
    private var submitSection: some View {
        Section {
            Button {
                Task { await submitUpdate() }
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                    }
                    Label("Update Contract", systemImage: "arrow.up.circle.fill")
                        .frame(maxWidth: .infinity)
                }
            }
            .disabled(!canSubmit)
            .accessibilityIdentifier("updateContract.submitButton")

            if unconfirmedUpdateSent {
                Text("An update was sent and its outcome is unknown: it may still land. Do not send another update now. Check the contract's version on the contract screen later, then try again if it did not change.")
                    .font(.caption)
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("updateContract.unconfirmedHold")
            }
        } footer: {
            if unconfirmedUpdateSent {
                EmptyView()
            } else if review?.preview.hasChanges == false {
                Text("The update would change nothing, so it is not offered.")
            } else if review == nil {
                Text("Review the changes first.")
            }
        }
    }

    private func jsonEditor(text: Binding<String>, identifier: String) -> some View {
        TextEditor(text: text)
            .font(.system(.footnote, design: .monospaced))
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .frame(minHeight: 110)
            .disabled(isBusy)
            .accessibilityIdentifier(identifier)
    }

    // MARK: - State

    private var isBusy: Bool {
        isReviewing || isSubmitting
    }

    /// The raw text of the five inputs. Any change clears the review.
    private var inputs: UpdateContractImport.Fields {
        UpdateContractImport.Fields(
            documents: documentsJSON,
            tokens: tokensJSON,
            groups: groupsJSON,
            keywords: keywords,
            description: descriptionText
        )
    }

    /// The owner's wallet, resolved as the transition builder resolves it.
    private var ownerWallet: ManagedPlatformWallet? {
        guard let walletId = contract.ownerIdentity?.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    /// The paid-no-op guard: the update is offered only for a review of the
    /// inputs on screen that reports a change. `hasChanges` comes from Rust.
    /// Never after an update whose outcome is unknown.
    private var canSubmit: Bool {
        guard !isBusy, !unconfirmedUpdateSent, ownerWallet != nil, let review else { return false }
        return review.inputs == inputs && review.preview.hasChanges
    }

    // MARK: - Sources

    private func pasteFromClipboard() {
        // Read only on the tap, so the system paste banner shows only when
        // the user asked to paste.
        fill(from: UIPasteboard.general.string ?? "", source: "clipboard")
    }

    private func importFile(_ result: Result<URL, Error>) {
        switch result {
        case .failure(let error):
            sourceMessage = "Could not open the file: \(error.localizedDescription)"
        case .success(let url):
            let scoped = url.startAccessingSecurityScopedResource()
            defer {
                if scoped { url.stopAccessingSecurityScopedResource() }
            }
            do {
                let data = try Data(contentsOf: url)
                guard let text = String(data: data, encoding: .utf8) else {
                    sourceMessage = "\(url.lastPathComponent) is not UTF-8 text."
                    return
                }
                fill(from: text, source: url.lastPathComponent)
            } catch {
                sourceMessage = "Could not read \(url.lastPathComponent): \(error.localizedDescription)"
            }
        }
    }

    /// Fill all five fields from `text` with `UpdateContractImport`, which
    /// writes each supplied section as given; a field the source does not
    /// carry is reset.
    private func fill(from text: String, source: String) {
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            sourceMessage = "The \(source) holds no contract JSON."
            return
        }
        let fields: UpdateContractImport.Fields
        do {
            fields = try UpdateContractImport.fields(from: text)
        } catch {
            sourceMessage = "Nothing filled from \(source): \(Self.describe(error))"
            return
        }
        documentsJSON = fields.documents
        tokensJSON = fields.tokens
        groupsJSON = fields.groups
        keywords = fields.keywords
        descriptionText = fields.description
        sourceMessage = "Filled from \(source)."
    }

    // MARK: - Actions

    private func reviewChanges() async {
        guard let wallet = ownerWallet, let owner = contract.ownerIdentity else {
            errorMessage = "The owner identity has no wallet linkage."
            return
        }
        let reviewedInputs = inputs
        let arguments: UpdateContractImport.WalletArguments
        do {
            arguments = try UpdateContractImport.WalletArguments(reviewedInputs)
        } catch {
            errorMessage = Self.describe(error)
            return
        }

        isReviewing = true
        defer { isReviewing = false }
        do {
            let preview = try await wallet.previewDataContractUpdate(
                ownerIdentityId: owner.identityId,
                contractId: contract.id,
                documentSchemasJSON: arguments.documents,
                tokenSchemasJSON: arguments.tokens,
                groupsJSON: arguments.groups,
                keywordsJSON: arguments.keywords,
                description: arguments.description
            )
            // An edit made while the review ran is not what was reviewed.
            if inputs == reviewedInputs {
                review = Review(inputs: reviewedInputs, arguments: arguments, preview: preview)
            }
        } catch {
            errorMessage = Self.describe(error)
        }
    }

    private func submitUpdate() async {
        guard canSubmit, let review, let wallet = ownerWallet, let owner = contract.ownerIdentity else {
            return
        }
        let arguments = review.arguments

        isSubmitting = true
        defer { isSubmitting = false }

        let signer = KeychainSigner(modelContainer: modelContext.container)
        do {
            let contractId = try await wallet.updateDataContract(
                ownerIdentityId: owner.identityId,
                contractId: contract.id,
                documentSchemasJSON: arguments.documents,
                tokenSchemasJSON: arguments.tokens,
                groupsJSON: arguments.groups,
                keywordsJSON: arguments.keywords,
                description: arguments.description,
                // Bind the update to the reviewed contract: if the contract
                // changed since the review, the wallet refuses the update.
                expectedCurrentVersion: review.preview.currentVersion,
                signer: signer
            )
            // Keepalive: see the KeychainSigner lifetime contract on
            // `updateDataContract`.
            _ = signer

            // Platform holds at least the reviewed new version now, so an
            // older copy from a lagging node is not stored.
            let refreshFailure = await refreshStoredContract(
                minimumVersion: Int(review.preview.newVersion)
            )
            // The confirmed update gave the contract the reviewed new
            // version. Read it from the review, not the stored row, which
            // the refresh writes through another context.
            var message = "Contract id:\n\(contractId.toBase58String())\n\nVersion: \(review.preview.newVersion)"
            if let refreshFailure {
                message += "\n\nThe contract was updated, but the local copy could not be refreshed: \(refreshFailure)"
            }
            outcome = Outcome(title: "Contract Updated", message: message, dismissesScreen: true)
        } catch PlatformWalletError.transactionBroadcastUnconfirmed(let detail) {
            _ = signer
            // The update may have executed, or may still. It is not retried,
            // and no other update is offered from this screen. The refresh
            // has only the stored version as its floor: the update's new
            // version is not known to be on chain.
            self.review = nil
            unconfirmedUpdateSent = true
            let refreshFailure = await refreshStoredContract(minimumVersion: nil)
            var message = "The update was sent, but its outcome is unknown: it may have executed, or may still. It was not retried. Do not send another update now: check the contract's version on the contract screen later before trying again.\n\n\(detail)"
            if let refreshFailure {
                message += "\n\nThe local copy could not be refreshed: \(refreshFailure)"
            }
            outcome = Outcome(title: "Update Unconfirmed", message: message, dismissesScreen: false)
        } catch {
            _ = signer
            // A failed submit may leave the reviewed contract behind (it may
            // have changed, or this update may have reached Platform), so
            // the review is cleared and Update Contract stays off until the
            // changes are reviewed again.
            self.review = nil
            errorMessage = Self.describe(error)
        }
    }

    /// Bring the stored contract up to date with Platform, refusing a
    /// fetched version below `minimumVersion` or the stored one. Returns why
    /// it failed, or nil.
    private func refreshStoredContract(minimumVersion: Int?) async -> String? {
        guard let sdk = appState.sdk else {
            return "the SDK is not initialized"
        }
        do {
            try await ContractDownloader.refreshPersistedContract(
                contract,
                sdk: sdk,
                modelContext: modelContext,
                minimumVersion: minimumVersion
            )
            return nil
        } catch {
            return Self.describe(error)
        }
    }

    /// The message to show for `error`: Rust's rendered message, and the
    /// consensus code when Platform refused the transition.
    private static func describe(_ error: Error) -> String {
        if let walletError = error as? PlatformWalletError {
            let message = walletError.errorDescription ?? String(describing: walletError)
            if let consensus = walletError.consensusError {
                return "\(message)\n\nConsensus error code: \(consensus.code)"
            }
            return message
        }
        if let localized = error as? LocalizedError, let description = localized.errorDescription {
            return description
        }
        return error.localizedDescription
    }
}

// MARK: - Supporting types

private extension UpdateContractView {
    struct Review {
        let inputs: UpdateContractImport.Fields
        let arguments: UpdateContractImport.WalletArguments
        let preview: DataContractUpdatePreview
    }

    struct Outcome {
        let title: String
        let message: String
        /// Whether OK goes back to the contract: after a confirmed update.
        let dismissesScreen: Bool
    }
}

/// What a reviewed update would change, as the wallet reported it.
private struct UpdateContractPreviewSection: View {
    let preview: DataContractUpdatePreview

    var body: some View {
        Section("Changes") {
            LabeledContent("Version", value: "\(preview.currentVersion) -> \(preview.newVersion)")
            if preview.hasChanges {
                sectionRows(title: "Document types", changes: preview.documentTypes)
                sectionRows(title: "Tokens", changes: preview.tokens)
                sectionRows(title: "Groups", changes: preview.groups)
                flagRow("Keywords", changed: preview.keywordsChanged)
                flagRow("Description", changed: preview.descriptionChanged)
            } else {
                Text("No changes")
                    .foregroundColor(.secondary)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("updateContract.preview")
    }

    @ViewBuilder
    private func sectionRows(
        title: String,
        changes: DataContractUpdatePreview.SectionChanges
    ) -> some View {
        if !changes.added.isEmpty {
            LabeledContent("\(title) added", value: changes.added.joined(separator: ", "))
        }
        if !changes.changed.isEmpty {
            LabeledContent("\(title) changed", value: changes.changed.joined(separator: ", "))
        }
        if changes.added.isEmpty && changes.changed.isEmpty {
            LabeledContent(title, value: "Unchanged")
        }
    }

    private func flagRow(_ title: String, changed: Bool) -> some View {
        LabeledContent(title, value: changed ? "Changes" : "Unchanged")
    }
}
