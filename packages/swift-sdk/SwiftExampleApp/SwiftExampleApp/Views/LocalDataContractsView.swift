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
    
    // Known testnet contracts - these are the common system contracts
    let exampleContracts = [
        ("DPNS Contract", "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"),
        ("DashPay Contract", "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7"),
        ("Withdrawals Contract", "4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6"),
        ("Wallet Utils", "7CSFGeF4WNzgDmx94zwvHkYaG3Dx4XEe5LFsFgJswLbm"),
        ("Token History", "43gujrzZgXqcKBiScLa4T8XTDnRhenR9BLx8GWVHjPxF"),
        ("Keyword Search", "BsjE6tQxG47wffZCRQCovFx5rYrAYYC3rTVRWKro27LA")
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
                        Section(header: Text("Common System Contracts (\(unifiedState.platformState.currentNetwork.rawValue))")) {
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
            
            // Fetch the contract with both JSON and binary serialization
            guard let handle = sdk.handle else {
                throw SDKError.invalidState("SDK not initialized")
            }
            
            let result = trimmedId.withCString { idCStr in
                dash_sdk_data_contract_fetch_with_serialization(handle, idCStr, true, true)
            }
            
            // Check for error
            if let error = result.error {
                let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
                dash_sdk_error_free(error)
                throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
            }
            
            // Get the JSON string
            guard result.json_string != nil else {
                throw SDKError.internalError("No JSON data returned from contract fetch")
            }
            
            let jsonString = String(cString: result.json_string!)
            
            // Get the binary serialization
            var binaryData: Data? = nil
            if result.serialized_data != nil && result.serialized_data_len > 0 {
                binaryData = Data(bytes: result.serialized_data, count: Int(result.serialized_data_len))
            }
            
            // Clean up the contract handle if it was returned
            defer {
                if result.contract_handle != nil {
                    dash_sdk_data_contract_destroy(result.contract_handle)
                }
            }
            
            // Parse the JSON
            guard let jsonData = jsonString.data(using: String.Encoding.utf8),
                  let contractData = try? JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any] else {
                throw SDKError.serializationError("Failed to parse contract JSON")
            }
            
            print("✅ Contract fetched successfully")
            if let binaryData = binaryData {
                print("📦 Binary serialization size: \(binaryData.count) bytes")
            }
            
            // Add the contract to the trusted context if we have binary data
            if let binaryData = binaryData,
               let contractId = contractData["id"] as? String {
                do {
                    try sdk.addContractToContext(contractId: contractId, binaryData: binaryData)
                    print("✅ Added contract to trusted context provider")
                } catch {
                    print("⚠️ Failed to add contract to trusted context: \(error)")
                    // Continue even if adding to context fails
                }
            } else {
                print("⚠️ No binary data available to add contract to trusted context")
            }
            
            await MainActor.run {
                fetchedContract = contractData
            }
            
            // Store the JSON for the contract
            let serializedContract = jsonData
            
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
            
            // Add the binary serialization if available
            persistentContract.binarySerialization = binaryData
            
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