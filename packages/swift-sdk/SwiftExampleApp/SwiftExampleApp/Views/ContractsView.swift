import SwiftUI

struct ContractsView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingFetchContract = false
    @State private var selectedContract: ContractModel?
    
    var body: some View {
        NavigationView {
            List {
                if appState.contracts.isEmpty {
                    EmptyStateView(
                        systemImage: "doc.plaintext",
                        title: "No Contracts",
                        message: "Fetch contracts from the network to see them here"
                    )
                    .listRowBackground(Color.clear)
                } else {
                    ForEach(appState.contracts) { contract in
                        ContractRow(contract: contract) {
                            selectedContract = contract
                        }
                    }
                }
            }
            .navigationTitle("Contracts")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingFetchContract = true }) {
                        Image(systemName: "arrow.down.circle")
                    }
                }
            }
            .sheet(isPresented: $showingFetchContract) {
                FetchContractView()
                    .environmentObject(appState)
            }
            .sheet(item: $selectedContract) { contract in
                ContractDetailView(contract: contract)
            }
            .onAppear {
                if appState.contracts.isEmpty {
                    loadSampleContracts()
                }
            }
        }
    }
    
    private func loadSampleContracts() {
        // Add sample contracts for demonstration
        appState.contracts = [
            ContractModel(
                id: "dpns-contract",
                name: "DPNS",
                version: 1,
                ownerId: Data(repeating: 0, count: 32),
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
                ownerId: Data(repeating: 0, count: 32),
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
            ),
            ContractModel(
                id: "masternode-reward-shares-contract",
                name: "Masternode Reward Shares",
                version: 1,
                ownerId: Data(repeating: 0, count: 32),
                documentTypes: ["rewardShare"],
                schema: [
                    "rewardShare": [
                        "type": "object",
                        "properties": [
                            "payToId": ["type": "string"],
                            "percentage": ["type": "number"]
                        ]
                    ]
                ]
            )
        ]
    }
}

struct ContractRow: View {
    let contract: ContractModel
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(contract.name)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    Text("v\(contract.version)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Color.blue.opacity(0.2))
                        .cornerRadius(4)
                }
                
                Text(contract.id)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                
                HStack {
                    Image(systemName: "doc.text")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("\(contract.documentTypes.count) document types")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct ContractDetailView: View {
    let contract: ContractModel
    @Environment(\.dismiss) var dismiss
    @State private var selectedDocumentType: String?
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            DetailRow(label: "Contract Name", value: contract.name)
                            DetailRow(label: "Contract ID", value: contract.id)
                            DetailRow(label: "Version", value: "\(contract.version)")
                            DetailRow(label: "Owner ID", value: contract.ownerIdString)
                        }
                        .padding()
                        .background(Color.gray.opacity(0.1))
                        .cornerRadius(10)
                    }
                    
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Document Types")
                                .font(.headline)
                            
                            ForEach(contract.documentTypes, id: \.self) { docType in
                                Button(action: {
                                    selectedDocumentType = selectedDocumentType == docType ? nil : docType
                                }) {
                                    HStack {
                                        Image(systemName: "doc.text")
                                            .foregroundColor(.blue)
                                        Text(docType)
                                            .foregroundColor(.primary)
                                        Spacer()
                                        Image(systemName: selectedDocumentType == docType ? "chevron.up" : "chevron.down")
                                            .foregroundColor(.secondary)
                                    }
                                    .padding(.vertical, 8)
                                    .padding(.horizontal, 12)
                                    .background(Color.gray.opacity(0.1))
                                    .cornerRadius(8)
                                }
                                
                                if selectedDocumentType == docType {
                                    Text(getSchemaForDocumentType(docType))
                                        .font(.system(.caption, design: .monospaced))
                                        .padding()
                                        .background(Color.gray.opacity(0.05))
                                        .cornerRadius(8)
                                        .padding(.horizontal)
                                }
                            }
                        }
                        .padding()
                    }
                    
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Full Schema")
                                .font(.headline)
                            
                            Text(contract.formattedSchema)
                                .font(.system(.caption, design: .monospaced))
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                        }
                        .padding()
                    }
                }
            }
            .navigationTitle("Contract Details")
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
    
    private func getSchemaForDocumentType(_ docType: String) -> String {
        if let typeSchema = contract.schema[docType] {
            guard let jsonData = try? JSONSerialization.data(withJSONObject: typeSchema, options: .prettyPrinted),
                  let jsonString = String(data: jsonData, encoding: .utf8) else {
                return "Invalid schema"
            }
            return jsonString
        }
        return "Schema not available"
    }
}

struct FetchContractView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var contractIdToFetch = ""
    @State private var isLoading = false
    
    var body: some View {
        NavigationView {
            Form {
                Section("Fetch Contract from Network") {
                    TextField("Contract ID", text: $contractIdToFetch)
                        .textContentType(.none)
                        .autocapitalization(.none)
                }
                
                if isLoading {
                    Section {
                        HStack {
                            ProgressView()
                            Text("Fetching contract...")
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
            .navigationTitle("Fetch Contract")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Fetch") {
                        Task {
                            await fetchContract()
                            if !isLoading {
                                dismiss()
                            }
                        }
                    }
                    .disabled(contractIdToFetch.isEmpty || isLoading)
                }
            }
        }
    }
    
    @MainActor
    private func fetchContract() async {
        guard let sdk = appState.sdk else {
            appState.showError(message: "SDK not initialized")
            return
        }
        
        do {
            isLoading = true
            
            // In a real app, we would use the SDK's contract fetching functionality
            if (try await sdk.getDataContract(id: contractIdToFetch)) != nil {
                // Convert SDK contract to our model
                // For now, we'll show a success message
                appState.showError(message: "Contract fetched successfully")
            } else {
                appState.showError(message: "Contract not found")
            }
            
            isLoading = false
        } catch {
            appState.showError(message: "Failed to fetch contract: \(error.localizedDescription)")
            isLoading = false
        }
    }
}
