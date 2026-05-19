import SwiftUI
import SwiftData
import SwiftDashSDK

struct DocumentsView: View {
    @EnvironmentObject var appState: AppState
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

struct CreateDocumentView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @Query private var contracts: [PersistentDataContract]
    @Query private var identities: [PersistentIdentity]

    @State private var selectedContract: PersistentDataContract?
    @State private var selectedDocumentType = ""
    @State private var selectedOwnerId: String = ""
    @State private var dataKeyToAdd = ""
    @State private var dataValueToAdd = ""
    @State private var documentData: [String: String] = [:]
    @State private var isLoading = false

    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Document Configuration")) {
                    Picker("Contract", selection: $selectedContract) {
                        Text("Select a contract").tag(nil as PersistentDataContract?)
                        ForEach(contracts) { contract in
                            Text(contract.name).tag(contract as PersistentDataContract?)
                        }
                    }

                    if let contract = selectedContract {
                        Picker("Document Type", selection: $selectedDocumentType) {
                            Text("Select type").tag("")
                            ForEach(contract.documentTypesList, id: \.self) { type in
                                Text(type).tag(type)
                            }
                        }
                    }

                    Picker("Owner", selection: $selectedOwnerId) {
                        Text("Select owner").tag("")
                        ForEach(identities) { identity in
                            Text(identity.alias ?? identity.identityIdBase58)
                                .tag(identity.identityIdBase58)
                        }
                    }
                }

                Section("Document Data") {
                    ForEach(Array(documentData.keys), id: \.self) { key in
                        HStack {
                            Text(key)
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(documentData[key] ?? "")
                                .font(.subheadline)
                        }
                    }

                    HStack {
                        TextField("Key", text: $dataKeyToAdd)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                        TextField("Value", text: $dataValueToAdd)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                        Button("Add") {
                            if !dataKeyToAdd.isEmpty && !dataValueToAdd.isEmpty {
                                documentData[dataKeyToAdd] = dataValueToAdd
                                dataKeyToAdd = ""
                                dataValueToAdd = ""
                            }
                        }
                    }
                }
            }
            .navigationTitle("Create Document")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Create") {
                        Task {
                            await createDocument()
                            dismiss()
                        }
                    }
                    .disabled(selectedContract == nil ||
                              selectedDocumentType.isEmpty ||
                              selectedOwnerId.isEmpty ||
                              isLoading)
                }
            }
        }
    }

    private func createDocument() async {
        guard appState.sdk != nil,
              let contract = selectedContract,
              !selectedDocumentType.isEmpty else {
            appState.showError(message: "Please select a contract and document type")
            return
        }

        isLoading = true
        defer { isLoading = false }

        // Local-only create for demonstration. In the real flow we
        // would broadcast the document through the SDK and wait for
        // the platform acknowledgement.
        let dataBlob = (try? JSONSerialization.data(
            withJSONObject: documentData,
            options: []
        )) ?? Data()

        let document = PersistentDocument(
            documentId: UUID().uuidString,
            documentType: selectedDocumentType,
            revision: 1,
            data: dataBlob,
            contractId: contract.idBase58,
            ownerId: selectedOwnerId,
            network: appState.currentNetwork
        )
        // Link to the parent contract so cascading cleanup works.
        document.dataContract = contract
        modelContext.insert(document)
        try? modelContext.save()

        appState.showError(message: "Document created locally")
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
