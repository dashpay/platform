import SwiftUI
import SwiftData
import SwiftDashSDK

struct DocumentsView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Query(sort: \PersistentDocument.createdAt, order: .reverse)
    private var documents: [PersistentDocument]

    @State private var showingCreateDocument = false
    @State private var selectedDocument: PersistentDocument?

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
                DocumentDetailView(document: document)
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
    @Environment(\.dismiss) var dismiss

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
            }
        }
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
                ForEach(contracts) { contract in
                    Text(contract.name).tag(contract as PersistentDataContract?)
                }
            }
            .accessibleFormPicker("createDocument.contractPicker")
            .disabled(isSubmitting)

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
            propertiesJSON = try Self.propertiesJSON(from: fieldValues)
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
                let documentId = try await wallet.createDocument(
                    ownerIdentityId: ownerId,
                    contractId: contractId,
                    documentType: typeName,
                    propertiesJSON: propertiesJSON,
                    signer: signer
                )
                _ = signer
                await MainActor.run {
                    persistConfirmedDocument(
                        documentId: documentId,
                        documentType: typeName,
                        contractId: contractId,
                        ownerId: ownerId,
                        propertiesJSON: propertiesJSON,
                        network: network,
                        parentContract: parentContract
                    )
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
        propertiesJSON: String,
        network: Network,
        parentContract: PersistentDataContract?
    ) {
        let dataBlob = propertiesJSON.data(using: .utf8) ?? Data()
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
        try? modelContext.save()
    }

    // MARK: - Properties JSON

    /// Convert the form's `[String: Any]` into a JSON object string the
    /// Rust side can parse. `Data` values (byte arrays + identifiers)
    /// are encoded as hex strings — the schema-driven sanitize step in
    /// `create_document_with_signer` decodes hex/base64 byte arrays and
    /// hex/base58 identifiers back to native values. Other values are
    /// JSON-native and pass through unchanged.
    static func propertiesJSON(from fieldValues: [String: Any]) throws -> String {
        var jsonObject: [String: Any] = [:]
        for (key, value) in fieldValues {
            if let data = value as? Data {
                jsonObject[key] = data.toHexString()
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
