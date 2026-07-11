import SwiftUI
import SwiftData
import SwiftDashSDK

struct DocumentsView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var transitionState: TransitionState
    @Environment(\.modelContext) private var modelContext
    @Query(sort: \PersistentDocument.createdAt, order: .reverse)
    private var documents: [PersistentDocument]

    @State private var showingCreateDocument = false
    @State private var selectedDocument: PersistentDocument?
    /// Hoisted to the stable `DocumentsView` container (rather than the
    /// detail sheet) so a background-sync `@Query` re-render of the
    /// detail view can't tear down the action sheet mid-flow — the
    /// known nav-churn self-dismiss. See
    /// `reference_swiftexampleapp_nav_churn`.
    @State private var documentActionMode: DocumentActionMode?

    var body: some View {
        NavigationView {
            List {
                if documents.isEmpty {
                    EmptyStateView(
                        systemImage: "doc.text",
                        title: "No Documents",
                        message: "Create documents to see them here"
                    )
                    .listRowBackground(Color.clear)
                } else {
                    ForEach(documents) { document in
                        DocumentRow(document: document) {
                            selectedDocument = document
                        }
                    }
                    .onDelete { indexSet in
                        deleteDocuments(at: indexSet)
                    }
                }
            }
            .navigationTitle("Documents")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingCreateDocument = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
            .sheet(isPresented: $showingCreateDocument) {
                CreateDocumentView()
                    .environmentObject(appState)
                    .environmentObject(walletManager)
            }
            .sheet(item: $selectedDocument) { document in
                DocumentDetailView(document: document) { mode in
                    // Close the detail sheet, then present the action
                    // sheet from the stable container on the next runloop
                    // tick (two sheets can't be presented from the same
                    // anchor simultaneously).
                    selectedDocument = nil
                    DispatchQueue.main.async {
                        documentActionMode = mode
                    }
                }
                .environmentObject(appState)
                .environmentObject(walletManager)
            }
            // Action sheets hoisted here so they outlive detail-view churn.
            .sheet(item: $documentActionMode) { mode in
                DocumentActionSheet(mode: mode)
                    .environmentObject(appState)
                    .environmentObject(walletManager)
                    .environmentObject(transitionState)
            }
        }
    }

    private func deleteDocuments(at offsets: IndexSet) {
        for index in offsets where index < documents.count {
            let document = documents[index]
            document.markAsDeleted()
        }
        try? modelContext.save()
    }
}

/// The five ownership-gated document state-transition actions, carrying
/// the document they operate on. `Identifiable` so it can drive a single
/// hoisted `sheet(item:)` on `DocumentsView`.
enum DocumentActionMode: Identifiable {
    case replace(PersistentDocument)
    case delete(PersistentDocument)
    case transfer(PersistentDocument)
    case setPrice(PersistentDocument)
    case purchase(PersistentDocument)

    var id: String {
        switch self {
        case .replace(let d): return "replace-\(d.documentId)"
        case .delete(let d): return "delete-\(d.documentId)"
        case .transfer(let d): return "transfer-\(d.documentId)"
        case .setPrice(let d): return "setPrice-\(d.documentId)"
        case .purchase(let d): return "purchase-\(d.documentId)"
        }
    }

    var document: PersistentDocument {
        switch self {
        case .replace(let d), .delete(let d), .transfer(let d),
             .setPrice(let d), .purchase(let d):
            return d
        }
    }
}

struct DocumentRow: View {
    let document: PersistentDocument
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(document.documentType)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    Text(document.contractIdBase58)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .frame(maxWidth: 100)
                }

                Text("Owner: \(document.ownerIdBase58)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)

                Text("Created: \(document.createdAt, formatter: dateFormatter)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct DocumentDetailView: View {
    let document: PersistentDocument
    /// Invoked when the user picks one of the gated actions; the parent
    /// (`DocumentsView`) dismisses this detail sheet and presents the
    /// action sheet from its stable container.
    var onAction: (DocumentActionMode) -> Void = { _ in }

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    /// Identities the wallet controls on the active network (own a
    /// `KeychainSigner` for signing). Used to gate the owner-only and
    /// purchaser actions.
    @Query private var identities: [PersistentIdentity]

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            DetailRow(label: "Document Type", value: document.documentType)
                            DetailRow(label: "Document ID", value: document.documentId)
                            DetailRow(label: "Contract ID", value: document.contractIdBase58)
                            DetailRow(label: "Owner ID", value: document.ownerIdBase58)
                            DetailRow(label: "Revision", value: "\(document.revision)")
                            DetailRow(label: "Created", value: AppDate.formatted(document.createdAt))
                            DetailRow(label: "Updated", value: AppDate.formatted(document.updatedAt))
                        }
                        .padding()
                        .background(Color.gray.opacity(0.1))
                        .cornerRadius(10)
                    }

                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Document Data")
                                .font(.headline)

                            Text(formattedProperties(document.properties))
                                .font(.system(.caption, design: .monospaced))
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                        }
                        .padding()
                    }
                }
            }
            .navigationTitle("Document Details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
                if !availableActions.isEmpty {
                    ToolbarItem(placement: .navigationBarLeading) {
                        actionMenu
                    }
                }
            }
        }
    }

    // MARK: - Action menu + gating

    @ViewBuilder
    private var actionMenu: some View {
        Menu {
            ForEach(availableActions) { action in
                Button {
                    onAction(action.mode(for: document))
                } label: {
                    Label(action.title, systemImage: action.systemImage)
                }
                .accessibilityIdentifier("documentAction.\(action.rawValue)")
            }
        } label: {
            Image(systemName: "ellipsis.circle")
        }
        .accessibilityIdentifier("documentAction.menu")
    }

    /// The `PersistentDocumentType` backing this document — preferred via
    /// the persisted relationship, falling back to a contract+name lookup
    /// (older rows may predate the relationship being linked). Drives the
    /// `transferable` / `tradeMode` gating.
    private var documentTypeRow: PersistentDocumentType? {
        if let linked = document.documentType_relation {
            return linked
        }
        return document.dataContract?.documentTypes?
            .first { $0.name == document.documentType }
    }

    /// True when the document's owner is an identity this wallet controls
    /// (held locally with a wallet, so a `KeychainSigner` can sign).
    private var ownerIsControlled: Bool {
        controlledIdentities.contains { $0.identityIdBase58 == document.ownerIdBase58 }
    }

    /// Controlled identities on the active network that differ from the
    /// current owner — the eligible purchasers (buyer ≠ owner).
    private var nonOwnerControlledIdentities: [PersistentIdentity] {
        controlledIdentities.filter { $0.identityIdBase58 != document.ownerIdBase58 }
    }

    private var controlledIdentities: [PersistentIdentity] {
        identities.filter { $0.network == appState.currentNetwork && $0.wallet != nil }
    }

    /// Whether the on-chain document carries a `$price` is fetched
    /// asynchronously inside the Purchase sheet (reusing the
    /// `DocumentWithPriceView` read). For the menu we surface Purchase
    /// whenever a different controlled identity exists and the doc type
    /// supports a trade mode — the sheet then resolves the actual price /
    /// for-sale state and disables the button if it isn't for sale.
    private var availableActions: [DocumentAction] {
        var actions: [DocumentAction] = []
        let docType = documentTypeRow
        // `tradeMode == 1` is DirectPurchase — the only mode that supports
        // listing/buying — matching the marketplace gating in
        // TransitionInputView (`$0.tradeMode == 1`).
        let tradeable = (docType?.tradeMode ?? 0) == 1

        if ownerIsControlled {
            actions.append(.replace)
            actions.append(.delete)
            if docType?.documentsTransferable == true {
                actions.append(.transfer)
            }
            if tradeable {
                actions.append(.setPrice)
            }
        }
        // Surface Purchase whenever the doc type is tradeable and the wallet
        // holds a controlled identity that isn't the owner (the buyer ≠
        // owner, and the buyer signs). This intentionally also covers a doc
        // owned by another *controlled* identity — the two-identities-in-one
        // -app flow — not just externally-owned docs, matching the doc
        // comment above. The Purchase sheet resolves the real on-chain price
        // / for-sale state and disables the button when it isn't for sale.
        if tradeable && !nonOwnerControlledIdentities.isEmpty {
            actions.append(.purchase)
        }
        return actions
    }

    private func formattedProperties(_ properties: [String: Any]?) -> String {
        guard let properties = properties else { return "No data" }
        guard let jsonData = try? JSONSerialization.data(
            withJSONObject: properties,
            options: .prettyPrinted
        ), let text = String(data: jsonData, encoding: .utf8) else {
            return properties.map { "\($0.key): \($0.value)" }.joined(separator: "\n")
        }
        return text
    }
}

// MARK: - Document actions (replace / delete / transfer / set-price / purchase)

/// The menu entries the gated action menu surfaces. Maps to a
/// `DocumentActionMode` once a concrete document is in hand.
enum DocumentAction: String, Identifiable, CaseIterable {
    case replace
    case delete
    case transfer
    case setPrice
    case purchase

    var id: String { rawValue }

    var title: String {
        switch self {
        case .replace: return "Replace…"
        case .delete: return "Delete…"
        case .transfer: return "Transfer…"
        case .setPrice: return "Set Price…"
        case .purchase: return "Purchase…"
        }
    }

    var systemImage: String {
        switch self {
        case .replace: return "pencil"
        case .delete: return "trash"
        case .transfer: return "arrow.left.arrow.right"
        case .setPrice: return "tag"
        case .purchase: return "cart"
        }
    }

    func mode(for document: PersistentDocument) -> DocumentActionMode {
        switch self {
        case .replace: return .replace(document)
        case .delete: return .delete(document)
        case .transfer: return .transfer(document)
        case .setPrice: return .setPrice(document)
        case .purchase: return .purchase(document)
        }
    }
}

/// Dispatches the hoisted `sheet(item:)` to the per-action editor view.
/// A thin router so `DocumentsView` only presents one sheet type.
struct DocumentActionSheet: View {
    let mode: DocumentActionMode

    var body: some View {
        switch mode {
        case .replace(let doc):
            ReplaceDocumentView(document: doc)
        case .delete(let doc):
            DeleteDocumentView(document: doc)
        case .transfer(let doc):
            TransferDocumentView(document: doc)
        case .setPrice(let doc):
            SetDocumentPriceView(document: doc)
        case .purchase(let doc):
            PurchaseDocumentView(document: doc)
        }
    }
}

/// Shared select-key → sign → call-wrapper → persist pipeline behind all
/// five document mutate actions.
///
/// Per `project_document_signing_key_purpose_bug` the signing key MUST be
/// an AUTHENTICATION key (a TRANSFER/CRITICAL key is rejected by consensus
/// with "requires AUTHENTICATION"); we resolve it via
/// `KeyManager.findSigningKey(purpose: .authentication, ...)` and pass its
/// id to the `ManagedPlatformWallet` wrapper, which re-validates it
/// Rust-side. The actual signature is produced on demand by the
/// `KeychainSigner` trampoline — no private bytes ever cross into Swift
/// logic here.
@MainActor
enum DocumentActionRunner {
    /// Resolve the controlled identity + its `ManagedPlatformWallet` and
    /// the AUTHENTICATION signing-key id for `identity`, satisfying the
    /// document type's security-level requirement. Throws a descriptive
    /// error if any precondition is missing.
    static func resolveSigning(
        for identity: PersistentIdentity,
        documentType: PersistentDocumentType?,
        walletManager: PlatformWalletManager
    ) throws -> (wallet: ManagedPlatformWallet, signingKeyId: UInt32) {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            throw DocumentActionError.noWallet
        }

        // The document type's security level bounds which AUTHENTICATION
        // keys may sign (mirrors the Rust-side requirement). Fall back to
        // HIGH when unknown — the same default the builder path uses.
        let requiredLevel = SecurityLevel(rawValue: UInt8(documentType?.securityLevel ?? 2)) ?? .high

        let dppIdentity = DPPIdentity(
            id: identity.identityId,
            publicKeys: Dictionary(
                uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }
            ),
            balance: UInt64(bitPattern: identity.balance),
            revision: 0
        )

        let km = KeyManager.withSharedKeychain()
        guard let key = km.findSigningKey(
            for: dppIdentity,
            purpose: .authentication,
            minimumSecurityLevel: requiredLevel,
            preferCritical: true
        ) else {
            throw DocumentActionError.noSigningKey(requiredLevel.name)
        }
        return (wallet, key.id)
    }
}

enum DocumentActionError: LocalizedError {
    case noWallet
    case noSigningKey(String)
    case identityNotFound

    var errorDescription: String? {
        switch self {
        case .noWallet:
            return "The owner identity is not held by a loaded wallet on this device."
        case .noSigningKey(let level):
            return "No AUTHENTICATION key with security \(level) or higher (and an available private key) found on the signing identity."
        case .identityNotFound:
            return "Could not resolve the controlled identity for this operation."
        }
    }
}

/// Small reusable success/error footer for the action sheets.
private struct ActionStatusView: View {
    let didComplete: Bool
    let confirmedId: String?
    let persistWarning: String?
    let onDone: () -> Void

    var body: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Broadcast confirmed", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let id = confirmedId {
                    HStack(alignment: .top) {
                        Text("Doc ID:").foregroundColor(.secondary)
                        Text(id)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(2)
                            .truncationMode(.middle)
                    }
                }
                if let warning = persistWarning {
                    Label(warning, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                Button {
                    onDone()
                } label: {
                    Text("Done").frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("documentAction.doneButton")
                .padding(.top, 4)
            }
        }
    }
}

private struct DocumentActionErrorBox: Identifiable {
    let id = UUID()
    let message: String
}

// MARK: Replace

/// Replace the document's properties. The JSON editor is seeded from the
/// document's current properties; the full object is sent as the
/// replacement (the Rust side schema-sanitizes + bumps the revision).
struct ReplaceDocumentView: View {
    let document: PersistentDocument

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var identities: [PersistentIdentity]

    @State private var propertiesText: String = ""
    @State private var isSubmitting = false
    @State private var didComplete = false
    @State private var confirmedId: String?
    @State private var persistWarning: String?
    @State private var actionError: DocumentActionErrorBox?

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    ActionStatusView(
                        didComplete: didComplete,
                        confirmedId: confirmedId,
                        persistWarning: persistWarning,
                        onDone: { dismiss() }
                    )
                } else {
                    Section {
                        DetailRow(label: "Document ID", value: document.documentId)
                        DetailRow(label: "Type", value: document.documentType)
                    }
                    Section {
                        TextEditor(text: $propertiesText)
                            .font(.system(.body, design: .monospaced))
                            .frame(minHeight: 200)
                            .disabled(isSubmitting)
                            .accessibilityIdentifier("documentReplace.jsonEditor")
                    } header: {
                        Text("Properties (JSON)")
                    } footer: {
                        Text("Full replacement property object. Byte-array fields as hex, identifier fields as base58.")
                    }
                    submitSection
                }
            }
            .navigationTitle("Replace Document")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }.disabled(isSubmitting)
                }
            }
            .interactiveDismissDisabled(isSubmitting)
            .alert(item: $actionError) { err in
                Alert(title: Text("Replace failed"), message: Text(err.message), dismissButton: .default(Text("OK")))
            }
            .onAppear { seedProperties() }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                        Text("Broadcasting…")
                    } else {
                        Text("Replace / Broadcast")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("documentReplace.submitButton")
            .disabled(isSubmitting || propertiesText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
    }

    private func seedProperties() {
        // Seed from the persisted document data (its canonical JSON),
        // stripping the system fields so only mutable properties remain.
        guard let props = document.properties else {
            propertiesText = "{}"
            return
        }
        let mutableProps = props.filter { !$0.key.hasPrefix("$") }
        if let data = try? JSONSerialization.data(withJSONObject: mutableProps, options: [.prettyPrinted, .sortedKeys]),
           let text = String(data: data, encoding: .utf8) {
            propertiesText = text
        } else {
            propertiesText = "{}"
        }
    }

    private func ownerIdentity() -> PersistentIdentity? {
        identities.first {
            $0.network == appState.currentNetwork
                && $0.wallet != nil
                && $0.identityIdBase58 == document.ownerIdBase58
        }
    }

    private func submit() {
        guard let owner = ownerIdentity() else {
            actionError = .init(message: DocumentActionError.identityNotFound.localizedDescription)
            return
        }
        // Validate the JSON up front.
        let trimmed = propertiesText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let jsonData = trimmed.data(using: .utf8),
              (try? JSONSerialization.jsonObject(with: jsonData)) is [String: Any] else {
            actionError = .init(message: "Properties must be a valid JSON object.")
            return
        }

        let docType = document.documentType_relation
        let wallet: ManagedPlatformWallet
        let signingKeyId: UInt32
        do {
            (wallet, signingKeyId) = try DocumentActionRunner.resolveSigning(
                for: owner, documentType: docType, walletManager: walletManager
            )
        } catch {
            actionError = .init(message: error.localizedDescription)
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let ownerId = owner.identityId
        let contractId = document.contractIdData
        let typeName = document.documentType
        let docId = document.id

        Task {
            do {
                let (confirmedDocId, canonicalJSON) = try await wallet.replaceDocument(
                    ownerIdentityId: ownerId,
                    contractId: contractId,
                    documentType: typeName,
                    documentId: docId,
                    propertiesJSON: trimmed,
                    signingKeyId: signingKeyId,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    persistWarning = DocumentPersistence.applyUpdate(
                        document: document,
                        canonicalJSON: canonicalJSON,
                        modelContext: modelContext
                    )
                    confirmedId = confirmedDocId.toBase58String()
                    isSubmitting = false
                    didComplete = true
                }
            } catch {
                await MainActor.run {
                    actionError = .init(message: error.localizedDescription)
                    isSubmitting = false
                }
            }
        }
    }
}

// MARK: Delete

struct DeleteDocumentView: View {
    let document: PersistentDocument

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var identities: [PersistentIdentity]

    @State private var isSubmitting = false
    @State private var didComplete = false
    @State private var confirmedId: String?
    @State private var persistWarning: String?
    @State private var actionError: DocumentActionErrorBox?

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    ActionStatusView(
                        didComplete: didComplete,
                        confirmedId: confirmedId,
                        persistWarning: persistWarning,
                        onDone: { dismiss() }
                    )
                } else {
                    Section {
                        DetailRow(label: "Document ID", value: document.documentId)
                        DetailRow(label: "Type", value: document.documentType)
                    } footer: {
                        Text("This permanently deletes the document on Platform. This cannot be undone.")
                    }
                    Section {
                        Button(role: .destructive) {
                            submit()
                        } label: {
                            HStack {
                                if isSubmitting {
                                    ProgressView().controlSize(.small)
                                    Text("Broadcasting…")
                                } else {
                                    Text("Delete / Broadcast")
                                }
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("documentDelete.submitButton")
                        .disabled(isSubmitting)
                    }
                }
            }
            .navigationTitle("Delete Document")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }.disabled(isSubmitting)
                }
            }
            .interactiveDismissDisabled(isSubmitting)
            .alert(item: $actionError) { err in
                Alert(title: Text("Delete failed"), message: Text(err.message), dismissButton: .default(Text("OK")))
            }
        }
    }

    private func ownerIdentity() -> PersistentIdentity? {
        identities.first {
            $0.network == appState.currentNetwork
                && $0.wallet != nil
                && $0.identityIdBase58 == document.ownerIdBase58
        }
    }

    private func submit() {
        guard let owner = ownerIdentity() else {
            actionError = .init(message: DocumentActionError.identityNotFound.localizedDescription)
            return
        }
        let docType = document.documentType_relation
        let wallet: ManagedPlatformWallet
        let signingKeyId: UInt32
        do {
            (wallet, signingKeyId) = try DocumentActionRunner.resolveSigning(
                for: owner, documentType: docType, walletManager: walletManager
            )
        } catch {
            actionError = .init(message: error.localizedDescription)
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let ownerId = owner.identityId
        let contractId = document.contractIdData
        let typeName = document.documentType
        let docId = document.id

        Task {
            do {
                let deletedId = try await wallet.deleteDocument(
                    ownerIdentityId: ownerId,
                    contractId: contractId,
                    documentType: typeName,
                    documentId: docId,
                    signingKeyId: signingKeyId,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    persistWarning = DocumentPersistence.applyDelete(
                        document: document,
                        modelContext: modelContext
                    )
                    confirmedId = deletedId.toBase58String()
                    isSubmitting = false
                    didComplete = true
                }
            } catch {
                await MainActor.run {
                    actionError = .init(message: error.localizedDescription)
                    isSubmitting = false
                }
            }
        }
    }
}

// MARK: Transfer

struct TransferDocumentView: View {
    let document: PersistentDocument

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var identities: [PersistentIdentity]

    @State private var recipientId: String = ""
    @State private var isSubmitting = false
    @State private var didComplete = false
    @State private var confirmedId: String?
    @State private var persistWarning: String?
    @State private var actionError: DocumentActionErrorBox?

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    ActionStatusView(
                        didComplete: didComplete,
                        confirmedId: confirmedId,
                        persistWarning: persistWarning,
                        onDone: { dismiss() }
                    )
                } else {
                    Section {
                        DetailRow(label: "Document ID", value: document.documentId)
                        DetailRow(label: "Type", value: document.documentType)
                    }
                    Section {
                        TextField("Recipient identity ID (base58)", text: $recipientId)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .disabled(isSubmitting)
                            .accessibilityIdentifier("documentTransfer.recipientField")
                    } header: {
                        Text("Recipient Identity")
                    } footer: {
                        Text("The identity that will become the new owner.")
                    }
                    submitSection
                }
            }
            .navigationTitle("Transfer Document")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }.disabled(isSubmitting)
                }
            }
            .interactiveDismissDisabled(isSubmitting)
            .alert(item: $actionError) { err in
                Alert(title: Text("Transfer failed"), message: Text(err.message), dismissButton: .default(Text("OK")))
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                        Text("Broadcasting…")
                    } else {
                        Text("Transfer / Broadcast")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("documentTransfer.submitButton")
            .disabled(isSubmitting || normalizedRecipient == nil)
        }
    }

    /// Recipient normalized to a 32-byte `Identifier`, or nil if the
    /// entered string isn't a valid base58/hex identifier.
    private var normalizedRecipient: Identifier? {
        let trimmed = recipientId.trimmingCharacters(in: .whitespacesAndNewlines)
        if let data = Data.identifier(fromBase58: trimmed), data.count == 32 {
            return data
        }
        if let data = Data(hexString: trimmed), data.count == 32 {
            return data
        }
        return nil
    }

    private func ownerIdentity() -> PersistentIdentity? {
        identities.first {
            $0.network == appState.currentNetwork
                && $0.wallet != nil
                && $0.identityIdBase58 == document.ownerIdBase58
        }
    }

    private func submit() {
        guard let owner = ownerIdentity() else {
            actionError = .init(message: DocumentActionError.identityNotFound.localizedDescription)
            return
        }
        guard let recipient = normalizedRecipient else {
            actionError = .init(message: "Enter a valid recipient identity ID.")
            return
        }
        let docType = document.documentType_relation
        let wallet: ManagedPlatformWallet
        let signingKeyId: UInt32
        do {
            (wallet, signingKeyId) = try DocumentActionRunner.resolveSigning(
                for: owner, documentType: docType, walletManager: walletManager
            )
        } catch {
            actionError = .init(message: error.localizedDescription)
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let ownerId = owner.identityId
        let contractId = document.contractIdData
        let typeName = document.documentType
        let docId = document.id

        Task {
            do {
                let (confirmedDocId, canonicalJSON) = try await wallet.transferDocument(
                    ownerIdentityId: ownerId,
                    contractId: contractId,
                    documentType: typeName,
                    documentId: docId,
                    recipientId: recipient,
                    signingKeyId: signingKeyId,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    persistWarning = DocumentPersistence.applyOwnerChange(
                        document: document,
                        newOwnerId: recipient,
                        canonicalJSON: canonicalJSON,
                        modelContext: modelContext
                    )
                    confirmedId = confirmedDocId.toBase58String()
                    isSubmitting = false
                    didComplete = true
                }
            } catch {
                await MainActor.run {
                    actionError = .init(message: error.localizedDescription)
                    isSubmitting = false
                }
            }
        }
    }
}

// MARK: Set price

struct SetDocumentPriceView: View {
    let document: PersistentDocument

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var identities: [PersistentIdentity]

    @State private var priceText: String = ""
    @State private var isSubmitting = false
    @State private var didComplete = false
    @State private var confirmedId: String?
    @State private var persistWarning: String?
    @State private var actionError: DocumentActionErrorBox?

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    ActionStatusView(
                        didComplete: didComplete,
                        confirmedId: confirmedId,
                        persistWarning: persistWarning,
                        onDone: { dismiss() }
                    )
                } else {
                    Section {
                        DetailRow(label: "Document ID", value: document.documentId)
                        DetailRow(label: "Type", value: document.documentType)
                    }
                    Section {
                        TextField("Price (credits)", text: $priceText)
                            .keyboardType(.numberPad)
                            .disabled(isSubmitting)
                            .accessibilityIdentifier("documentSetPrice.priceField")
                    } header: {
                        Text("Price in credits")
                    } footer: {
                        Text("Listing price for the document. 1 DASH = 100,000,000,000 credits.")
                    }
                    submitSection
                }
            }
            .navigationTitle("Set Document Price")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }.disabled(isSubmitting)
                }
            }
            .interactiveDismissDisabled(isSubmitting)
            .alert(item: $actionError) { err in
                Alert(title: Text("Set price failed"), message: Text(err.message), dismissButton: .default(Text("OK")))
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                        Text("Broadcasting…")
                    } else {
                        Text("Set Price / Broadcast")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("documentSetPrice.submitButton")
            .disabled(isSubmitting || UInt64(priceText.trimmingCharacters(in: .whitespaces)) == nil)
        }
    }

    private func ownerIdentity() -> PersistentIdentity? {
        identities.first {
            $0.network == appState.currentNetwork
                && $0.wallet != nil
                && $0.identityIdBase58 == document.ownerIdBase58
        }
    }

    private func submit() {
        guard let owner = ownerIdentity() else {
            actionError = .init(message: DocumentActionError.identityNotFound.localizedDescription)
            return
        }
        guard let price = UInt64(priceText.trimmingCharacters(in: .whitespaces)) else {
            actionError = .init(message: "Enter a valid price in credits.")
            return
        }
        let docType = document.documentType_relation
        let wallet: ManagedPlatformWallet
        let signingKeyId: UInt32
        do {
            (wallet, signingKeyId) = try DocumentActionRunner.resolveSigning(
                for: owner, documentType: docType, walletManager: walletManager
            )
        } catch {
            actionError = .init(message: error.localizedDescription)
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let ownerId = owner.identityId
        let contractId = document.contractIdData
        let typeName = document.documentType
        let docId = document.id

        Task {
            do {
                let (confirmedDocId, canonicalJSON) = try await wallet.setDocumentPrice(
                    ownerIdentityId: ownerId,
                    contractId: contractId,
                    documentType: typeName,
                    documentId: docId,
                    price: price,
                    signingKeyId: signingKeyId,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    persistWarning = DocumentPersistence.applyUpdate(
                        document: document,
                        canonicalJSON: canonicalJSON,
                        modelContext: modelContext
                    )
                    confirmedId = confirmedDocId.toBase58String()
                    isSubmitting = false
                    didComplete = true
                }
            } catch {
                await MainActor.run {
                    actionError = .init(message: error.localizedDescription)
                    isSubmitting = false
                }
            }
        }
    }
}

// MARK: Purchase

/// Purchase a for-sale document with one of the wallet's controlled
/// identities that is NOT the current owner. Reuses
/// `DocumentWithPriceView`'s read to auto-fetch the on-chain price and
/// for-sale state, then signs the purchase with the chosen purchaser.
struct PurchaseDocumentView: View {
    let document: PersistentDocument

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var transitionState: TransitionState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var identities: [PersistentIdentity]

    /// Bound to `DocumentWithPriceView`, which fetches + publishes the
    /// price into `transitionState`. Seeded with the document id so the
    /// read fires immediately.
    @State private var documentIdField: String = ""
    @State private var selectedPurchaserId: String = ""
    @State private var isSubmitting = false
    @State private var didComplete = false
    @State private var confirmedId: String?
    @State private var persistWarning: String?
    @State private var actionError: DocumentActionErrorBox?

    /// Controlled identities on the active network that are not the
    /// current owner — eligible purchasers (buyer ≠ owner).
    private var eligiblePurchasers: [PersistentIdentity] {
        identities.filter {
            $0.network == appState.currentNetwork
                && $0.wallet != nil
                && $0.identityIdBase58 != document.ownerIdBase58
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    ActionStatusView(
                        didComplete: didComplete,
                        confirmedId: confirmedId,
                        persistWarning: persistWarning,
                        onDone: { dismiss() }
                    )
                } else {
                    Section {
                        DetailRow(label: "Document ID", value: document.documentId)
                        DetailRow(label: "Type", value: document.documentType)
                        DetailRow(label: "Current Owner", value: document.ownerIdBase58)
                    }
                    Section {
                        // Read-only price probe; user does not edit the id.
                        DocumentWithPriceView(
                            documentId: $documentIdField,
                            contractId: document.contractIdBase58,
                            documentType: document.documentType,
                            currentIdentityId: selectedPurchaserId.isEmpty ? nil : selectedPurchaserId
                        )
                        .disabled(true)
                    } header: {
                        Text("Price")
                    }
                    Section {
                        Picker("Purchaser", selection: $selectedPurchaserId) {
                            Text("Select purchaser").tag("")
                            ForEach(eligiblePurchasers) { identity in
                                Text(identity.alias ?? identity.identityIdBase58)
                                    .tag(identity.identityIdBase58)
                                    .accessibilityIdentifier("documentPurchase.buyer.\(identity.identityIdBase58)")
                            }
                        }
                        .accessibleFormPicker("documentPurchase.buyerPicker")
                        .disabled(isSubmitting)
                    } header: {
                        Text("Purchaser Identity")
                    } footer: {
                        Text("The identity that buys and becomes the new owner. Must differ from the current owner.")
                    }
                    submitSection
                }
            }
            .navigationTitle("Purchase Document")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }.disabled(isSubmitting)
                }
            }
            .interactiveDismissDisabled(isSubmitting)
            .alert(item: $actionError) { err in
                Alert(title: Text("Purchase failed"), message: Text(err.message), dismissButton: .default(Text("OK")))
            }
            .onAppear {
                documentIdField = document.documentId
                // Clear the shared price state on entry so a stale value
                // from a prior probe (this `transitionState` is app-wide)
                // can't enable Purchase before the disabled
                // `DocumentWithPriceView` above republishes *this*
                // document's price. submit() also re-reads it, so a stale
                // price could otherwise be broadcast.
                transitionState.documentPrice = nil
            }
            .onDisappear { transitionState.documentPrice = nil }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                        Text("Broadcasting…")
                    } else {
                        Text("Purchase / Broadcast")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("documentPurchase.submitButton")
            .disabled(isSubmitting || selectedPurchaserId.isEmpty || (transitionState.documentPrice ?? 0) == 0)
        } footer: {
            if (transitionState.documentPrice ?? 0) == 0 {
                Text("This document is not currently for sale.")
                    .foregroundColor(.secondary)
            }
        }
    }

    private func submit() {
        guard let purchaser = eligiblePurchasers.first(where: { $0.identityIdBase58 == selectedPurchaserId }) else {
            actionError = .init(message: "Select a purchaser identity held by a loaded wallet.")
            return
        }
        guard let price = transitionState.documentPrice, price > 0 else {
            actionError = .init(message: "This document is not for sale (no price found).")
            return
        }
        let docType = document.documentType_relation
        let wallet: ManagedPlatformWallet
        let signingKeyId: UInt32
        do {
            // The purchaser signs (and becomes the new owner), so resolve
            // the AUTHENTICATION key on the purchaser, not the owner.
            (wallet, signingKeyId) = try DocumentActionRunner.resolveSigning(
                for: purchaser, documentType: docType, walletManager: walletManager
            )
        } catch {
            actionError = .init(message: error.localizedDescription)
            return
        }

        isSubmitting = true
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let purchaserId = purchaser.identityId
        let contractId = document.contractIdData
        let typeName = document.documentType
        let docId = document.id

        Task {
            do {
                let (confirmedDocId, canonicalJSON) = try await wallet.purchaseDocument(
                    purchaserId: purchaserId,
                    contractId: contractId,
                    documentType: typeName,
                    documentId: docId,
                    price: price,
                    signingKeyId: signingKeyId,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    persistWarning = DocumentPersistence.applyOwnerChange(
                        document: document,
                        newOwnerId: purchaserId,
                        canonicalJSON: canonicalJSON,
                        modelContext: modelContext
                    )
                    confirmedId = confirmedDocId.toBase58String()
                    isSubmitting = false
                    didComplete = true
                }
            } catch {
                await MainActor.run {
                    actionError = .init(message: error.localizedDescription)
                    isSubmitting = false
                }
            }
        }
    }
}

// MARK: - Local persistence for confirmed mutations

/// Apply a confirmed document mutation to the local `PersistentDocument`
/// cache. Persistence stays in Swift per `swift-sdk/CLAUDE.md`; the
/// broadcast already happened in Rust. Each helper returns a non-nil
/// warning string if the local save failed (the broadcast is on-chain
/// regardless), or nil on success.
@MainActor
enum DocumentPersistence {
    /// Replace / set-price: refresh `data` from the confirmed canonical
    /// JSON and bump the revision.
    static func applyUpdate(
        document: PersistentDocument,
        canonicalJSON: String,
        modelContext: ModelContext
    ) -> String? {
        let blob = canonicalJSON.data(using: .utf8) ?? document.data
        document.updateProperties(blob)
        document.revision = nextRevision(from: canonicalJSON, fallback: document.revision)
        return save(modelContext)
    }

    /// Transfer / purchase: change the owner and refresh from canonical
    /// JSON (which now reflects the new owner + bumped revision).
    static func applyOwnerChange(
        document: PersistentDocument,
        newOwnerId: Identifier,
        canonicalJSON: String,
        modelContext: ModelContext
    ) -> String? {
        let blob = canonicalJSON.data(using: .utf8) ?? document.data
        document.updateProperties(blob)
        document.ownerId = newOwnerId.toBase58String()
        document.ownerIdData = newOwnerId
        document.revision = nextRevision(from: canonicalJSON, fallback: document.revision)
        // Re-link the owner relationship if the new owner is local.
        document.ownerIdentity = nil
        document.linkToLocalIdentityIfNeeded(in: modelContext)
        return save(modelContext)
    }

    /// Delete: remove the row.
    static func applyDelete(
        document: PersistentDocument,
        modelContext: ModelContext
    ) -> String? {
        modelContext.delete(document)
        return save(modelContext)
    }

    /// Extract `$revision` from the canonical JSON, falling back to the
    /// existing revision when absent / unparseable.
    private static func nextRevision(from canonicalJSON: String, fallback: Int32) -> Int32 {
        guard let data = canonicalJSON.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return fallback
        }
        if let rev = obj["$revision"] as? NSNumber {
            return rev.int32Value
        }
        if let revStr = obj["$revision"] as? String, let rev = Int32(revStr) {
            return rev
        }
        return fallback
    }

    private static func save(_ modelContext: ModelContext) -> String? {
        do {
            try modelContext.save()
            return nil
        } catch {
            return "Broadcast confirmed, but updating the local copy failed: \(error.localizedDescription). The change is on-chain."
        }
    }
}

/// Production "Create Document" flow.
///
/// Renders the document type's schema fields (via `DocumentFieldsView`),
/// picks an owner identity, and broadcasts a real document state
/// transition through `ManagedPlatformWallet.createDocument(...)` — which
/// routes to `platform_wallet_create_document_with_signer` and the
/// `platform-wallet` library's `create_document_with_signer`. The
/// signing key is selected and used entirely on the Rust side via the
/// wallet's keychain-backed `KeychainSigner`; this view only collects
/// values, marshals them to a properties JSON string, calls the wrapper,
/// and persists the confirmed `PersistentDocument`.
///
/// This is distinct from the Settings builder/test-signer path
/// (`documentCreate(...)` in `StateTransitionExtensions`).
///
/// Launchable two ways:
///   - From `DocumentTypeDetailsView` with `presetDocumentType` set
///     (contract + type fixed, schema already in scope).
///   - From the Documents tab "+" with no preset (the user picks a
///     contract + document type first).
struct CreateDocumentView: View {
    /// When set, the contract + document type are fixed to this row and
    /// the pickers are hidden. When nil, the user selects them.
    let presetDocumentType: PersistentDocumentType?

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var contracts: [PersistentDataContract]
    @Query private var identities: [PersistentIdentity]

    @State private var selectedContract: PersistentDataContract?
    @State private var selectedDocumentTypeName = ""
    /// Owner identity id (base58). Drives the AccessiblePicker selection.
    @State private var selectedOwnerId: String = ""

    /// Field values produced by `DocumentFieldsView`. Byte-array fields
    /// arrive as `Data`, identifier fields as `Data`, scalars as
    /// `Int`/`Double`/`Bool`/`String`, arrays as `[String]`.
    @State private var fieldValues: [String: Any] = [:]

    @State private var isSubmitting = false
    @State private var submitError: SubmitError?
    @State private var didComplete = false
    @State private var createdDocumentId: String?
    /// Set when the broadcast succeeded but writing the local SwiftData
    /// row failed — the document is on-chain, just not cached locally yet.
    @State private var persistWarning: String?

    init(presetDocumentType: PersistentDocumentType? = nil) {
        self.presetDocumentType = presetDocumentType
    }

    private struct SubmitError: Identifiable {
        let id = UUID()
        let message: String
    }

    var body: some View {
        NavigationStack {
            Form {
                if didComplete {
                    successSection
                } else {
                    if presetDocumentType == nil {
                        selectionSection
                    } else {
                        presetSection
                    }
                    ownerSection
                    if let docType = resolvedDocumentType {
                        schemaSection(for: docType)
                    }
                    submitSection
                }
            }
            .navigationTitle("New Document")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            // Prevent swipe-to-dismiss while the (non-idempotent) broadcast
            // is in flight, so the user can't lose the result/warning.
            .interactiveDismissDisabled(isSubmitting)
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Create failed"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear {
                if let preset = presetDocumentType {
                    selectedContract = preset.dataContract
                    selectedDocumentTypeName = preset.name
                }
            }
        }
    }

    // MARK: - Sections

    private var selectionSection: some View {
        Section("Document") {
            Picker("Contract", selection: $selectedContract) {
                Text("Select a contract").tag(nil as PersistentDataContract?)
                ForEach(activeContracts) { contract in
                    Text(contract.name).tag(contract as PersistentDataContract?)
                }
            }
            .accessibleFormPicker("createDocument.contractPicker")
            .disabled(isSubmitting)
            .onChange(of: selectedContract) { _, _ in
                // A new contract may not have the previously-selected type
                // (or could share a name) — clear so the picker isn't stale.
                selectedDocumentTypeName = ""
            }

            if let contract = selectedContract {
                Picker("Document Type", selection: $selectedDocumentTypeName) {
                    Text("Select type").tag("")
                    ForEach(documentTypeNames(for: contract), id: \.self) { type in
                        Text(type)
                            .tag(type)
                            .accessibilityIdentifier("createDocument.docType.\(type)")
                    }
                }
                .accessibleFormPicker("createDocument.docTypePicker")
                .disabled(isSubmitting)
            }
        }
    }

    private var presetSection: some View {
        Section("Document") {
            if let docType = presetDocumentType {
                HStack {
                    Label("Contract", systemImage: "doc.plaintext")
                    Spacer()
                    Text(docType.dataContract?.name ?? docType.contractIdBase58)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                HStack {
                    Label("Document Type", systemImage: "doc.text")
                    Spacer()
                    Text(docType.name)
                        .foregroundColor(.secondary)
                }
            }
        }
    }

    private var ownerSection: some View {
        Section {
            Picker("Owner", selection: $selectedOwnerId) {
                Text("Select owner").tag("")
                ForEach(ownerIdentities) { identity in
                    Text(identity.alias ?? identity.identityIdBase58)
                        .tag(identity.identityIdBase58)
                        .accessibilityIdentifier("createDocument.owner.\(identity.identityIdBase58)")
                }
            }
            .accessibleFormPicker("createDocument.ownerPicker")
            .disabled(isSubmitting)
        } header: {
            Text("Owner Identity")
        } footer: {
            Text("The identity that owns and signs for this document. Signing uses this wallet's keychain-backed signer.")
        }
    }

    @ViewBuilder
    private func schemaSection(for docType: PersistentDocumentType) -> some View {
        Section {
            DocumentFieldsView(documentType: docType, fieldValues: $fieldValues)
                // Re-identify per document type so the field editors reset,
                // and clear the parent values — otherwise switching type in
                // the no-preset flow could submit the previous schema's values.
                .id(docType.id)
                .onChange(of: docType.id) { _, _ in
                    fieldValues = [:]
                }
        } header: {
            Text("Fields")
        } footer: {
            if let required = docType.requiredFields, !required.isEmpty {
                Text("Required: \(required.joined(separator: ", "))")
            }
        }
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
            .accessibilityIdentifier("createDocument.submitButton")
            .disabled(!canSubmit || isSubmitting)
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Document created", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let id = createdDocumentId {
                    HStack(alignment: .top) {
                        Text("ID:")
                            .foregroundColor(.secondary)
                        Text(id)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(2)
                            .truncationMode(.middle)
                    }
                }
                if let warning = persistWarning {
                    Label(warning, systemImage: "exclamationmark.triangle.fill")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                Button {
                    dismiss()
                } label: {
                    Text("Done")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("createDocument.doneButton")
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Derived state

    /// The `PersistentDocumentType` row backing the schema form — either
    /// the preset, or the one matching the selected contract + type name.
    private var resolvedDocumentType: PersistentDocumentType? {
        if let preset = presetDocumentType { return preset }
        guard let contract = selectedContract, !selectedDocumentTypeName.isEmpty else {
            return nil
        }
        return contract.documentTypes?.first { $0.name == selectedDocumentTypeName }
    }

    /// Owner identities limited to the active network and to wallets the
    /// app actually holds (so a `KeychainSigner` exists for signing).
    private var ownerIdentities: [PersistentIdentity] {
        identities.filter { $0.network == appState.currentNetwork && $0.wallet != nil }
    }

    /// Contracts limited to the active network — pairing a current-network
    /// owner with a contract from another network would fetch/broadcast
    /// against the wrong SDK network.
    private var activeContracts: [PersistentDataContract] {
        contracts.filter { $0.network == appState.currentNetwork }
    }

    private var selectedOwnerIdentity: PersistentIdentity? {
        ownerIdentities.first { $0.identityIdBase58 == selectedOwnerId }
    }

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = selectedOwnerIdentity?.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    private var canSubmit: Bool {
        resolvedDocumentType != nil
            && selectedOwnerIdentity != nil
            && managedWallet != nil
    }

    private func documentTypeNames(for contract: PersistentDataContract) -> [String] {
        // Prefer the parsed PersistentDocumentType rows (they carry the
        // schema the form needs); fall back to the stored name list.
        if let types = contract.documentTypes, !types.isEmpty {
            return types.map { $0.name }.sorted()
        }
        return contract.documentTypesList.sorted()
    }

    // MARK: - Submit

    private func submit() {
        guard
            let docType = resolvedDocumentType,
            let ownerIdentity = selectedOwnerIdentity,
            let wallet = managedWallet
        else {
            submitError = .init(message: "Select a document type and an owner identity held by a loaded wallet.")
            return
        }

        let propertiesJSON: String
        do {
            propertiesJSON = try Self.propertiesJSON(from: fieldValues, documentType: docType)
        } catch {
            submitError = .init(message: "Could not encode document fields: \(error.localizedDescription)")
            return
        }

        isSubmitting = true
        // Fresh `KeychainSigner` per submit pass, same as
        // `TransferCreditsView` / `RegisterNameView`: the trampoline
        // derives the signing key on demand — no bytes leave Rust.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let ownerId = ownerIdentity.identityId
        let contractId = docType.contractId
        let typeName = docType.name
        let network = appState.currentNetwork
        let parentContract = docType.dataContract

        Task {
            do {
                let (documentId, canonicalJSON) = try await wallet.createDocument(
                    ownerIdentityId: ownerId,
                    contractId: contractId,
                    documentType: typeName,
                    propertiesJSON: propertiesJSON,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    // The broadcast is confirmed on-chain at this point.
                    // Persisting the local cache row is best-effort: if it
                    // fails we still report success (the document exists and
                    // is queryable) but flag the local-save failure rather
                    // than swallowing it.
                    do {
                        try persistConfirmedDocument(
                            documentId: documentId,
                            documentType: typeName,
                            contractId: contractId,
                            ownerId: ownerId,
                            canonicalJSON: canonicalJSON,
                            network: network,
                            parentContract: parentContract
                        )
                    } catch {
                        self.persistWarning = "Broadcast confirmed, but saving the local copy failed: \(error.localizedDescription). The document is on-chain and queryable."
                    }
                    self.createdDocumentId = documentId.toBase58String()
                    self.isSubmitting = false
                    self.didComplete = true
                }
            } catch {
                await MainActor.run {
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }

    /// Persist the confirmed document so it shows up in the Documents
    /// list (DOC-01). Persistence stays in Swift per
    /// `swift-sdk/CLAUDE.md`; the broadcast itself happened in Rust.
    private func persistConfirmedDocument(
        documentId: Identifier,
        documentType: String,
        contractId: Data,
        ownerId: Identifier,
        canonicalJSON: String,
        network: Network,
        parentContract: PersistentDataContract?
    ) throws {
        // Persist the confirmed document's canonical query-side JSON
        // (system fields + DPP-normalized properties as returned by the
        // Rust side), not the user's raw form input, so the local cache
        // matches what a DOC-01 query would return.
        let dataBlob = canonicalJSON.data(using: .utf8) ?? Data()
        let document = PersistentDocument(
            documentId: documentId.toBase58String(),
            documentType: documentType,
            revision: 1,
            data: dataBlob,
            contractId: contractId.toBase58String(),
            ownerId: ownerId.toBase58String(),
            network: network
        )
        // Link to the parent contract so cascading cleanup works and the
        // contract-scoped document list picks it up.
        document.dataContract = parentContract
        modelContext.insert(document)
        document.linkToLocalIdentityIfNeeded(in: modelContext)
        do {
            try modelContext.save()
        } catch {
            // Save failed — detach the row we just inserted so a later
            // save from elsewhere can't silently flush it, which would
            // contradict the "not saved locally" warning we surface.
            modelContext.delete(document)
            throw error
        }
    }

    // MARK: - Properties JSON

    /// Convert the form's `[String: Any]` into a JSON object string the
    /// Rust side can parse. `Data` values (byte arrays + identifiers)
    /// are encoded as hex strings — the schema-driven sanitize step in
    /// `create_document_with_signer` decodes hex/base64 byte arrays and
    /// hex/base58 identifiers back to native values. `object`-typed
    /// fields arrive as the editor's raw JSON `String`; they are parsed
    /// back into a nested object so they serialize as objects, not as a
    /// JSON string. Other values are JSON-native and pass through.
    static func propertiesJSON(
        from fieldValues: [String: Any],
        documentType: PersistentDocumentType
    ) throws -> String {
        let objectFields = Set(
            documentType.propertiesList?
                .filter { $0.type == "object" }
                .map(\.name) ?? []
        )
        var jsonObject: [String: Any] = [:]
        for (key, value) in fieldValues {
            if let data = value as? Data {
                jsonObject[key] = data.toHexString()
            } else if objectFields.contains(key), let text = value as? String {
                let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !trimmed.isEmpty, let objData = trimmed.data(using: .utf8) else { continue }
                // Throws on invalid JSON → surfaced as an encode error.
                jsonObject[key] = try JSONSerialization.jsonObject(with: objData)
            } else {
                jsonObject[key] = value
            }
        }
        let data = try JSONSerialization.data(withJSONObject: jsonObject, options: [])
        return String(data: data, encoding: .utf8) ?? "{}"
    }
}

struct DetailRow: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.subheadline)
                .lineLimit(nil)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

private let dateFormatter: DateFormatter = {
    let formatter = DateFormatter.gregorian()
    formatter.dateStyle = .medium
    formatter.timeStyle = .short
    return formatter
}()
