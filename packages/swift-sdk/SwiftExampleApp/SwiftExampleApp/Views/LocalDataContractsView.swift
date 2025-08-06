import SwiftUI
import SwiftData
import SwiftDashSDK

struct LocalDataContractsView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    @Query(sort: \PersistentDataContract.lastAccessedAt, order: .reverse)
    private var dataContracts: [PersistentDataContract]
    
    @State private var showingLoadContract = false
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showError = false
    
    @Environment(\.modelContext) private var modelContext
    
    var body: some View {
        List {
            if dataContracts.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "doc.text")
                        .font(.system(size: 60))
                        .foregroundColor(.secondary)
                    
                    Text("No Local Contracts")
                        .font(.title2)
                        .fontWeight(.semibold)
                    
                    Text("Load data contracts from the network to use them offline")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .listRowBackground(Color.clear)
                .listRowInsets(EdgeInsets())
            } else {
                ForEach(dataContracts) { contract in
                    DataContractRow(contract: contract)
                }
                .onDelete(perform: deleteContracts)
            }
        }
        .navigationTitle("Local Data Contracts")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button(action: { showingLoadContract = true }) {
                    Label("Load Contract", systemImage: "arrow.down.circle")
                }
                .disabled(isLoading)
            }
        }
        .sheet(isPresented: $showingLoadContract) {
            LoadDataContractView(isLoading: $isLoading)
                .environmentObject(unifiedState)
                .environment(\.modelContext, modelContext)
        }
        .alert("Error", isPresented: $showError) {
            Button("OK") { }
        } message: {
            Text(errorMessage ?? "Unknown error occurred")
        }
    }
    
    private func deleteContracts(at offsets: IndexSet) {
        for index in offsets {
            modelContext.delete(dataContracts[index])
        }
        
        do {
            try modelContext.save()
        } catch {
            errorMessage = "Failed to delete contract: \(error.localizedDescription)"
            showError = true
        }
    }
}

struct DataContractRow: View {
    let contract: PersistentDataContract
    @State private var showingDetails = false
    
    var displayName: String {
        // Check if this is a token-only contract
        if let tokens = contract.tokens,
           tokens.count == 1,
           let documentTypes = contract.documentTypes,
           documentTypes.isEmpty,
           let token = tokens.first {
            // Use the token's singular form for display
            if let singularName = token.getSingularForm(languageCode: "en") {
                return "\(singularName) Token Contract"
            } else {
                return "Token Contract"
            }
        }
        
        // Otherwise use the stored name
        return contract.name
    }
    
    var body: some View {
        Button(action: { showingDetails = true }) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(displayName)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Text(contract.idBase58)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                
                HStack {
                    Text("Size: \(ByteCountFormatter.string(fromByteCount: Int64(contract.serializedContract.count), countStyle: .binary))")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Text("Last used: \(contract.lastAccessedAt, style: .relative)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
        .sheet(isPresented: $showingDetails) {
            DataContractDetailsView(contract: contract)
        }
    }
}

struct LoadDataContractView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @Binding var isLoading: Bool
    
    @Query private var existingContracts: [PersistentDataContract]
    
    @State private var contractId = ""
    @State private var contractName = ""
    @State private var errorMessage: String?
    @State private var showError = false
    @State private var fetchedContract: [String: Any]?
    @State private var showExampleContracts = false
    @State private var currentNetwork: String = "Unknown"
    
    // Known testnet contracts - these are the system contracts that should always exist
    let exampleContracts = [
        ("DPNS Contract", "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"),
        ("DashPay Contract", "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7"),
        ("Feature Flags", "HY1jjghgVz6aERbyDPjTk7CZqjKQCKK8AGzJndVwwRCN"),
        ("Masternode Rewards", "rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX"),
        ("Withdrawals Contract", "5G5kBnF3z8Y6cmiJHvNJkSjJc26cX7vb1CiEtRKqfKaD")
    ]
    
    var body: some View {
        NavigationView {
            Form {
                Section(footer: Text("Connected to: \(unifiedState.platformState.currentNetwork.rawValue)")) {
                    EmptyView()
                }
                
                Section("Contract Details") {
                    HStack {
                        TextField("Contract ID (Base58)", text: $contractId)
                            .textContentType(.none)
                            .autocapitalization(.none)
                            .disabled(isLoading)
                        
                        Button(action: { showExampleContracts.toggle() }) {
                            Image(systemName: "list.bullet")
                                .foregroundColor(.blue)
                        }
                        .disabled(isLoading)
                    }
                    
                    TextField("Name (Optional)", text: $contractName)
                        .textContentType(.none)
                        .disabled(isLoading)
                    
                    if showExampleContracts {
                        Section(header: Text("System Contracts (\(unifiedState.platformState.currentNetwork.rawValue))")) {
                            ForEach(exampleContracts, id: \.1) { example in
                                Button(action: {
                                    contractId = example.1
                                    contractName = example.0
                                    showExampleContracts = false
                                }) {
                                    HStack {
                                        VStack(alignment: .leading) {
                                            Text(example.0)
                                                .font(.subheadline)
                                            Text(example.1)
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                                .lineLimit(1)
                                                .truncationMode(.middle)
                                        }
                                        Spacer()
                                        Image(systemName: "arrow.right.circle")
                                            .foregroundColor(.secondary)
                                    }
                                }
                                .disabled(isLoading)
                            }
                        }
                    }
                }
                
                if isLoading {
                    Section {
                        HStack {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle())
                            Text("Loading contract from network...")
                                .foregroundColor(.secondary)
                        }
                    }
                }
                
                if let contract = fetchedContract {
                    Section("Fetched Contract") {
                        if let id = contract["id"] as? String {
                            HStack {
                                Text("ID")
                                    .foregroundColor(.secondary)
                                Spacer()
                                Text(id)
                                    .font(.caption)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                        }
                        
                        if let schema = contract["schema"] as? [String: Any],
                           let documentTypes = schema["documents"] as? [String: Any] {
                            HStack {
                                Text("Document Types")
                                    .foregroundColor(.secondary)
                                Spacer()
                                Text("\(documentTypes.count)")
                            }
                        }
                    }
                }
            }
            .navigationTitle("Load Data Contract")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                    .disabled(isLoading)
                }
                
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Load") {
                        Task {
                            await loadContract()
                        }
                    }
                    .disabled(contractId.isEmpty || isLoading)
                }
            }
            .alert("Error", isPresented: $showError) {
                Button("OK") { }
            } message: {
                Text(errorMessage ?? "Unknown error occurred")
            }
        }
    }
    
    private func loadContract() async {
        guard let sdk = unifiedState.sdk else {
            errorMessage = "SDK not initialized"
            showError = true
            return
        }
        
        await MainActor.run {
            isLoading = true
        }
        
        do {
            // Validate contract ID
            let trimmedId = contractId.trimmingCharacters(in: .whitespacesAndNewlines)
            
            print("🔵 Attempting to load contract with ID: \(trimmedId)")
            
            // Basic validation - just check it's not empty
            guard !trimmedId.isEmpty else {
                await MainActor.run {
                    errorMessage = "Please enter a contract ID"
                    showError = true
                    isLoading = false
                }
                return
            }
            
            // Fetch the contract - SDK expects a Base58 string
            let contractData = try await sdk.dataContractGet(id: trimmedId)
            print("✅ Contract fetched successfully")
            
            await MainActor.run {
                fetchedContract = contractData
            }
            
            // Extract contract details - the response contains the serialized contract data
            // We need to serialize the entire contract response for storage
            let serializedContract = try JSONSerialization.data(withJSONObject: contractData, options: [])
            
            // Get the contract ID from the response or convert from the input
            let contractIdData: Data
            if let idString = contractData["id"] as? String,
               let idData = Data.identifier(fromBase58: idString) ?? Data(hexString: idString) {
                contractIdData = idData
            } else {
                // Fall back to converting the input ID
                guard let idData = Data.identifier(fromBase58: trimmedId) else {
                    await MainActor.run {
                        errorMessage = "Could not extract contract ID from response"
                        showError = true
                        isLoading = false
                    }
                    return
                }
                contractIdData = idData
            }
            
            // Check if contract already exists
            if existingContracts.contains(where: { $0.id == contractIdData }) {
                await MainActor.run {
                    errorMessage = "This contract is already saved locally"
                    showError = true
                    isLoading = false
                }
                return
            }
            
            // Determine name
            var finalName = contractName.trimmingCharacters(in: .whitespacesAndNewlines)
            if finalName.isEmpty {
                // Check if it's a token-only contract
                let documents = contractData["documents"] as? [String: Any] ?? contractData["documentSchemas"] as? [String: Any] ?? [:]
                let tokens = contractData["tokens"] as? [String: Any] ?? [:]
                
                if documents.isEmpty && tokens.count == 1,
                   let tokenData = tokens.values.first as? [String: Any] {
                    // Extract token name
                    var tokenName: String? = nil
                    
                    // Try to get localized name first
                    if let conventions = tokenData["conventions"] as? [String: Any],
                       let localizations = conventions["localizations"] as? [String: Any],
                       let enLocalization = localizations["en"] as? [String: Any],
                       let singularForm = enLocalization["singularForm"] as? String {
                        tokenName = singularForm
                    }
                    
                    // Fallback to description or generic name
                    if tokenName == nil {
                        tokenName = tokenData["description"] as? String ?? tokenData["name"] as? String
                    }
                    
                    if let tokenName = tokenName {
                        finalName = "\(tokenName) Token Contract"
                    } else {
                        finalName = "Token Contract"
                    }
                } else if let firstDocType = documents.keys.first {
                    // Has documents
                    finalName = "Contract with \(firstDocType)"
                } else {
                    // Fallback
                    finalName = "Contract \(trimmedId.prefix(8))..."
                }
            }
            
            // Save to persistent storage
            let persistentContract = PersistentDataContract(
                id: contractIdData,
                name: finalName,
                serializedContract: serializedContract
            )
            
            modelContext.insert(persistentContract)
            try modelContext.save()
            
            // Parse tokens and document types from the contract
            try DataContractParser.parseDataContract(
                contractData: contractData,
                contractId: contractIdData,
                modelContext: modelContext
            )
            
            // Save again to persist relationships
            try modelContext.save()
            
            await MainActor.run {
                isLoading = false
                dismiss()
            }
            
        } catch {
            print("❌ Failed to load contract: \(error)")
            await MainActor.run {
                // Provide more helpful error messages
                if error.localizedDescription.contains("Data contract not found") {
                    errorMessage = "Contract not found on \(unifiedState.platformState.currentNetwork.rawValue). This contract may exist on a different network or the ID may be incorrect."
                } else {
                    errorMessage = "Failed to load contract: \(error.localizedDescription)"
                }
                showError = true
                isLoading = false
            }
        }
    }
}

#Preview {
    NavigationStack {
        LocalDataContractsView()
            .environmentObject(UnifiedAppState())
    }
}