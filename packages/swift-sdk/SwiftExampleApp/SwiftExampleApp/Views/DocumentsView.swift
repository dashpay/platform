import SwiftUI

struct DocumentsView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingCreateDocument = false
    @State private var selectedDocument: DocumentModel?
    
    var body: some View {
        NavigationView {
            List {
                if appState.documents.isEmpty {
                    EmptyStateView(
                        systemImage: "doc.text",
                        title: "No Documents",
                        message: "Create documents to see them here"
                    )
                    .listRowBackground(Color.clear)
                } else {
                    ForEach(appState.documents) { document in
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
            .onAppear {
                if appState.documents.isEmpty {
                    loadSampleDocuments()
                }
            }
        }
    }
    
    private func loadSampleDocuments() {
        // Add sample documents for demonstration
        appState.documents = [
            DocumentModel(
                id: "doc1",
                contractId: "dpns-contract",
                documentType: "domain",
                ownerId: Data(hexString: "1111111111111111111111111111111111111111111111111111111111111111")!,
                data: [
                    "label": "alice",
                    "normalizedLabel": "alice",
                    "normalizedParentDomainName": "dash"
                ],
                createdAt: Date(),
                updatedAt: Date()
            ),
            DocumentModel(
                id: "doc2",
                contractId: "dashpay-contract",
                documentType: "profile",
                ownerId: Data(hexString: "2222222222222222222222222222222222222222222222222222222222222222")!,
                data: [
                    "displayName": "Bob",
                    "publicMessage": "Hello from Bob!"
                ],
                createdAt: Date(),
                updatedAt: Date()
            )
        ]
    }
    
    private func deleteDocuments(at offsets: IndexSet) {
        for index in offsets {
            if index < appState.documents.count {
                let document = appState.documents[index]
                // In a real app, we would delete the document
                appState.documents.removeAll { $0.id == document.id }
            }
        }
    }
}

struct DocumentRow: View {
    let document: DocumentModel
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(document.documentType)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    Text(document.contractId)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .frame(maxWidth: 100)
                }
                
                Text("Owner: \(document.ownerIdString)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                
                if let createdAt = document.createdAt {
                    Text("Created: \(createdAt, formatter: dateFormatter)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct DocumentDetailView: View {
    let document: DocumentModel
    @Environment(\.dismiss) var dismiss
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            DetailRow(label: "Document Type", value: document.documentType)
                            DetailRow(label: "Document ID", value: document.id)
                            DetailRow(label: "Contract ID", value: document.contractId)
                            DetailRow(label: "Owner ID", value: document.ownerIdString)
                            
                            if let createdAt = document.createdAt {
                                DetailRow(label: "Created", value: createdAt.formatted())
                            }
                            
                            if let updatedAt = document.updatedAt {
                                DetailRow(label: "Updated", value: updatedAt.formatted())
                            }
                        }
                        .padding()
                        .background(Color.gray.opacity(0.1))
                        .cornerRadius(10)
                    }
                    
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Document Data")
                                .font(.headline)
                            
                            Text(document.formattedData)
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
}

struct CreateDocumentView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var selectedContract: ContractModel?
    @State private var selectedDocumentType = ""
    @State private var selectedOwnerId: String = ""
    @State private var dataKeyToAdd = ""
    @State private var dataValueToAdd = ""
    @State private var documentData: [String: Any] = [:]
    @State private var isLoading = false
    
    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Document Configuration")) {
                    Picker("Contract", selection: $selectedContract) {
                        Text("Select a contract").tag(nil as ContractModel?)
                        ForEach(appState.contracts) { contract in
                            Text(contract.name).tag(contract as ContractModel?)
                        }
                    }
                    
                    if let contract = selectedContract {
                        Picker("Document Type", selection: $selectedDocumentType) {
                            Text("Select type").tag("")
                            ForEach(contract.documentTypes, id: \.self) { type in
                                Text(type).tag(type)
                            }
                        }
                    }
                    
                    Picker("Owner", selection: $selectedOwnerId) {
                        Text("Select owner").tag("")
                        ForEach(appState.identities) { identity in
                            Text(identity.alias ?? identity.idString)
                                .tag(identity.idString)
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
                            Text("\(String(describing: documentData[key] ?? ""))")
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
            .onAppear {
                if appState.contracts.isEmpty {
                    // Load sample contracts if needed
                    loadSampleContracts()
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
        
        // In a real app, we would use the SDK's document creation functionality
        let document = DocumentModel(
            id: UUID().uuidString,
            contractId: contract.id,
            documentType: selectedDocumentType,
            ownerId: Data(hexString: selectedOwnerId) ?? Data(),
            data: documentData,
            createdAt: Date(),
            updatedAt: Date()
        )
        
        appState.documents.append(document)
        appState.showError(message: "Document created successfully")
        
        isLoading = false
    }
    
    private func loadSampleContracts() {
        // Add sample contracts for demonstration
        appState.contracts = [
            ContractModel(
                id: "dpns-contract",
                name: "DPNS",
                version: 1,
                ownerId: Data(hexString: "0000000000000000000000000000000000000000000000000000000000000000") ?? Data(),
                documentTypes: ["domain", "preorder"],
                schema: [
                    "domain": [
                        "type": "object",
                        "properties": [
                            "label": ["type": "string"],
                            "normalizedLabel": ["type": "string"],
                            "normalizedParentDomainName": ["type": "string"]
                        ]
                    ]
                ]
            ),
            ContractModel(
                id: "dashpay-contract",
                name: "DashPay",
                version: 1,
                ownerId: Data(hexString: "0000000000000000000000000000000000000000000000000000000000000000") ?? Data(),
                documentTypes: ["profile", "contactRequest"],
                schema: [
                    "profile": [
                        "type": "object",
                        "properties": [
                            "displayName": ["type": "string"],
                            "publicMessage": ["type": "string"]
                        ]
                    ]
                ]
            )
        ]
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
    let formatter = DateFormatter()
    formatter.dateStyle = .medium
    formatter.timeStyle = .short
    return formatter
}()
