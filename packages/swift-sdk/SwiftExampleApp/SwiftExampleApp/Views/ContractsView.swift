import SwiftUI
import SwiftData
import SwiftDashSDK

struct ContractsView: View {
    @EnvironmentObject var appState: AppState
    @Query(sort: \PersistentDataContract.createdAt, order: .reverse)
    private var contracts: [PersistentDataContract]

    @State private var showingFetchContract = false
    @State private var selectedContract: PersistentDataContract?

    var body: some View {
        NavigationView {
            List {
                if contracts.isEmpty {
                    EmptyStateView(
                        systemImage: "doc.plaintext",
                        title: "No Contracts",
                        message: "Fetch contracts from the network to see them here"
                    )
                    .listRowBackground(Color.clear)
                } else {
                    ForEach(contracts) { contract in
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
        }
    }
}

struct ContractRow: View {
    let contract: PersistentDataContract
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(contract.name)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    if let version = contract.version {
                        Text("v\(version)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Color.blue.opacity(0.2))
                            .cornerRadius(4)
                    }
                }

                Text(contract.idBase58)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)

                HStack {
                    Image(systemName: "doc.text")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("\(contract.documentTypesList.count) document types")
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
    let contract: PersistentDataContract
    @Environment(\.dismiss) var dismiss
    @State private var selectedDocumentType: String?

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            DetailRow(label: "Contract Name", value: contract.name)
                            DetailRow(label: "Contract ID", value: contract.idBase58)
                            if let version = contract.version {
                                DetailRow(label: "Version", value: "\(version)")
                            }
                            if let ownerId = contract.ownerIdBase58 {
                                DetailRow(label: "Owner ID", value: ownerId)
                            }
                        }
                        .padding()
                        .background(Color.gray.opacity(0.1))
                        .cornerRadius(10)
                    }

                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Document Types")
                                .font(.headline)

                            ForEach(contract.documentTypesList, id: \.self) { docType in
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

                            Text(formattedSchema(contract.schema))
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

    private func formattedSchema(_ schema: [String: Any]) -> String {
        guard let jsonData = try? JSONSerialization.data(withJSONObject: schema, options: .prettyPrinted),
              let jsonString = String(data: jsonData, encoding: .utf8) else {
            return "Invalid schema"
        }
        return jsonString
    }
}

struct FetchContractView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
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
                            let didFetch = await fetchContract()
                            if didFetch {
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
    private func fetchContract() async -> Bool {
        guard let sdk = appState.sdk else {
            appState.showError(message: "SDK not initialized")
            return false
        }

        do {
            isLoading = true
            defer { isLoading = false }

            let trimmedId = contractIdToFetch.trimmingCharacters(in: .whitespacesAndNewlines)
            let contractData = try await sdk.dataContractGet(id: trimmedId)
            try persistFetchedContract(contractData, requestedId: trimmedId)
            return true
        } catch {
            appState.showError(message: "Failed to fetch contract: \(error.localizedDescription)")
            return false
        }
    }

    private func persistFetchedContract(_ contractData: [String: Any], requestedId: String) throws {
        let serializedContract = try JSONSerialization.data(withJSONObject: contractData, options: [])
        let contractId = try resolveContractId(from: contractData, fallbackId: requestedId)

        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: #Predicate { $0.id == contractId }
        )
        if try modelContext.fetch(descriptor).first != nil {
            throw FetchContractError.contractAlreadySaved
        }

        let documents = contractDocuments(from: contractData)
        let tokens = contractData["tokens"] as? [String: Any] ?? [:]
        let ownerId = (contractData["ownerId"] as? String).flatMap {
            Data.identifier(fromBase58: $0) ?? Data(hexString: $0)
        }

        let persistentContract = PersistentDataContract(
            id: contractId,
            name: contractName(from: contractData, documents: documents, tokens: tokens, fallbackId: requestedId),
            serializedContract: serializedContract,
            version: contractData["version"] as? Int,
            ownerId: ownerId,
            schema: documents,
            documentTypesList: documents.keys.sorted(),
            keywords: contractData["keywords"] as? [String] ?? [],
            description: contractData["description"] as? String,
            hasTokens: !tokens.isEmpty,
            network: appState.currentNetwork
        )

        modelContext.insert(persistentContract)
        try modelContext.save()

        try DataContractParser.parseDataContract(
            contractData: contractData,
            contractId: contractId,
            modelContext: modelContext
        )
        try modelContext.save()
    }

    private func resolveContractId(from contractData: [String: Any], fallbackId: String) throws -> Data {
        let idString = (contractData["id"] as? String) ?? fallbackId
        if let id = Data.identifier(fromBase58: idString) ?? Data(hexString: idString) {
            return id
        }
        throw FetchContractError.invalidContractId
    }

    private func contractDocuments(from contractData: [String: Any]) -> [String: Any] {
        contractData["documents"] as? [String: Any]
            ?? contractData["documentSchemas"] as? [String: Any]
            ?? [:]
    }

    private func contractName(
        from contractData: [String: Any],
        documents: [String: Any],
        tokens: [String: Any],
        fallbackId: String
    ) -> String {
        if let name = contractData["name"] as? String,
           !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return name
        }

        if documents.isEmpty,
           tokens.count == 1,
           let tokenData = tokens.values.first as? [String: Any],
           let tokenName = tokenName(from: tokenData) {
            return "\(tokenName) Token Contract"
        }

        if let documentType = documents.keys.sorted().first {
            return "Contract with \(documentType)"
        }

        return "Contract \(fallbackId.prefix(8))..."
    }

    private func tokenName(from tokenData: [String: Any]) -> String? {
        if let conventions = tokenData["conventions"] as? [String: Any],
           let localizations = conventions["localizations"] as? [String: Any],
           let english = localizations["en"] as? [String: Any],
           let singularForm = english["singularForm"] as? String {
            return singularForm
        }

        return tokenData["description"] as? String ?? tokenData["name"] as? String
    }
}

private enum FetchContractError: LocalizedError {
    case invalidContractId
    case contractAlreadySaved

    var errorDescription: String? {
        switch self {
        case .invalidContractId:
            return "Could not extract contract ID from response"
        case .contractAlreadySaved:
            return "This contract is already saved locally"
        }
    }
}
