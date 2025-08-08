import SwiftUI
import SwiftDashSDK
import DashSDKFFI
import SwiftData

struct TransitionDetailView: View {
    let transitionKey: String
    let transitionLabel: String
    
    @EnvironmentObject var appState: UnifiedAppState
    @State private var selectedIdentityId: String = ""
    @State private var isExecuting = false
    @State private var showResult = false
    @State private var resultText = ""
    @State private var isError = false
    
    // Dynamic form inputs
    @State private var formInputs: [String: String] = [:]
    @State private var checkboxInputs: [String: Bool] = [:]
    @State private var selectedContractId: String = ""
    @State private var selectedDocumentType: String = ""
    @State private var documentFieldValues: [String: Any] = [:]
    
    // Query for data contracts
    @Query private var dataContracts: [PersistentDataContract]
    
    var needsIdentitySelection: Bool {
        transitionKey != "identityCreate"
    }
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                // Description
                if let transition = getTransitionDefinition(transitionKey) {
                    Text(transition.description)
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal)
                        .padding(.top)
                }
                
                // Identity Selector (for all transitions except Identity Create)
                if needsIdentitySelection {
                    identitySelector
                        .padding(.horizontal)
                }
                
                // Dynamic Form Inputs
                if let transition = getTransitionDefinition(transitionKey) {
                    VStack(alignment: .leading, spacing: 16) {
                        ForEach(transition.inputs, id: \.name) { input in
                            // Special handling for document fields
                            if input.name == "documentFields" && input.type == "json" {
                                documentFieldsInput(for: input)
                            } else {
                                TransitionInputView(
                                    input: enrichedInput(for: input),
                                    value: binding(for: input),
                                    checkboxValue: checkboxBinding(for: input),
                                    onSpecialAction: handleSpecialAction
                                )
                                .environmentObject(appState)
                            }
                        }
                    }
                    .padding(.horizontal)
                }
                
                // Execute Button
                if !needsIdentitySelection || !selectedIdentityId.isEmpty {
                    executeButton
                        .padding(.horizontal)
                        .padding(.top)
                }
                
                // Result Display
                if showResult {
                    resultView
                        .padding(.horizontal)
                }
                
                Spacer(minLength: 20)
            }
        }
        .navigationTitle(transitionLabel)
        .navigationBarTitleDisplayMode(.inline)
        .onAppear {
            clearForm()
        }
    }
    
    private var identitySelector: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Select Identity")
                .font(.headline)
            
            if appState.platformState.identities.isEmpty {
                Text("No identities available. Create one first.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(8)
            } else {
                Picker("Identity", selection: $selectedIdentityId) {
                    ForEach(appState.platformState.identities, id: \.idString) { identity in
                        Text(identity.displayName)
                            .tag(identity.idString)
                    }
                }
                .pickerStyle(MenuPickerStyle())
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.gray.opacity(0.1))
                .cornerRadius(8)
            }
        }
    }
    
    private var executeButton: some View {
        Button(action: executeTransition) {
            if isExecuting {
                ProgressView()
                    .progressViewStyle(CircularProgressViewStyle(tint: .white))
                    .scaleEffect(0.8)
            } else {
                Text("Execute Transition")
                    .fontWeight(.semibold)
            }
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(isExecuting ? Color.gray : Color.blue)
        .foregroundColor(.white)
        .cornerRadius(10)
        .disabled(isExecuting || !isFormValid())
    }
    
    private var resultView: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: isError ? "xmark.circle.fill" : "checkmark.circle.fill")
                    .foregroundColor(isError ? .red : .green)
                Text(isError ? "Error" : "Success")
                    .font(.headline)
                Spacer()
                Button("Copy") {
                    UIPasteboard.general.string = resultText
                }
                .font(.caption)
                .padding(.trailing, 8)
                Button("Dismiss") {
                    showResult = false
                    resultText = ""
                }
                .font(.caption)
            }
            
            ScrollView {
                Text(resultText)
                    .font(.system(.caption, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(maxHeight: 200)
            .padding(8)
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
        }
        .padding()
        .background(isError ? Color.red.opacity(0.1) : Color.green.opacity(0.1))
        .cornerRadius(10)
    }
    
    // MARK: - Document Fields Input
    
    @ViewBuilder
    private func documentFieldsInput(for input: TransitionInput) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(input.label)
                    .font(.subheadline)
                    .fontWeight(.medium)
                if input.required {
                    Text("*")
                        .foregroundColor(.red)
                }
            }
            
            let contractId = formInputs["contractId"] ?? selectedContractId
            let documentTypeName = formInputs["documentType"] ?? selectedDocumentType
            
            if contractId.isEmpty || documentTypeName.isEmpty {
                Text("Please select a contract and document type first")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(8)
            } else if let contract = dataContracts.first(where: { $0.idBase58 == contractId }),
                      let documentTypes = contract.documentTypes {
                if let documentType = documentTypes.first(where: { $0.name == documentTypeName }) {
                    DocumentFieldsView(
                        documentType: documentType,
                        fieldValues: Binding(
                            get: { documentFieldValues },
                            set: { newValues in
                                documentFieldValues = newValues
                                // Convert to JSON string for the form
                                if let jsonData = try? JSONSerialization.data(withJSONObject: newValues, options: [.prettyPrinted]),
                                   let jsonString = String(data: jsonData, encoding: .utf8) {
                                    formInputs["documentFields"] = jsonString
                                }
                            }
                        )
                    )
                } else {
                    Text("Document type '\(documentTypeName)' not found in contract")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding()
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.orange.opacity(0.1))
                        .cornerRadius(8)
                }
            } else {
                Text("Invalid contract or document type selected")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.red.opacity(0.1))
                    .cornerRadius(8)
            }
            
            if let help = input.help {
                Text(help)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    // MARK: - Helper Methods
    
    private func binding(for input: TransitionInput) -> Binding<String> {
        Binding(
            get: { formInputs[input.name] ?? input.defaultValue ?? "" },
            set: { formInputs[input.name] = $0 }
        )
    }
    
    private func checkboxBinding(for input: TransitionInput) -> Binding<Bool> {
        Binding(
            get: { checkboxInputs[input.name] ?? false },
            set: { checkboxInputs[input.name] = $0 }
        )
    }
    
    private func clearForm() {
        formInputs.removeAll()
        checkboxInputs.removeAll()
        
        // Set default values
        if let transition = getTransitionDefinition(transitionKey) {
            for input in transition.inputs {
                if let defaultValue = input.defaultValue {
                    formInputs[input.name] = defaultValue
                }
            }
        }
        
        // Set the first identity as default if we need identity selection
        if needsIdentitySelection && !appState.platformState.identities.isEmpty {
            selectedIdentityId = appState.platformState.identities.first?.idString ?? ""
        }
        
        showResult = false
        resultText = ""
        isError = false
    }
    
    private func isFormValid() -> Bool {
        guard let transition = getTransitionDefinition(transitionKey) else { return false }
        
        for input in transition.inputs {
            if input.required {
                if input.type == "checkbox" {
                    // Checkboxes are always valid
                    continue
                } else {
                    let value = formInputs[input.name] ?? ""
                    if value.isEmpty {
                        return false
                    }
                }
            }
        }
        
        return true
    }
    
    private func handleSpecialAction(_ action: String) {
        if action.starts(with: "contractSelected:") {
            let contractId = String(action.dropFirst("contractSelected:".count))
            selectedContractId = contractId
            formInputs["contractId"] = contractId
            // Clear document type when contract changes
            selectedDocumentType = ""
            formInputs["documentType"] = ""
        } else if action.starts(with: "documentTypeSelected:") {
            let docType = String(action.dropFirst("documentTypeSelected:".count))
            selectedDocumentType = docType
            formInputs["documentType"] = docType
            // Fetch schema for the selected document type
            fetchDocumentSchema(contractId: selectedContractId, documentType: docType)
        } else {
            switch action {
            case "generateTestSeed":
                // Generate a test seed phrase
                formInputs["seedPhrase"] = generateTestSeedPhrase()
            case "fetchDocumentSchema":
                if !selectedContractId.isEmpty && !selectedDocumentType.isEmpty {
                    fetchDocumentSchema(contractId: selectedContractId, documentType: selectedDocumentType)
                }
            case "loadExistingDocument":
                // TODO: Load existing document
                break
            case "fetchContestedResources":
                // TODO: Fetch contested resources
                break
            default:
                break
            }
        }
    }
    
    private func generateTestSeedPhrase() -> String {
        // This is a placeholder - in production, use proper BIP39 generation
        return "test seed phrase for development only do not use in production ever please"
    }
    
    private func getTransitionDefinition(_ key: String) -> TransitionDefinition? {
        return TransitionDefinitions.all[key]
    }
    
    // MARK: - Transition Execution
    
    private func executeTransition() {
        Task {
            await performTransition()
        }
    }
    
    @MainActor
    private func performTransition() async {
        isExecuting = true
        defer { isExecuting = false }
        
        do {
            let result = try await executeStateTransition()
            
            // Format the result as JSON
            let data = try JSONSerialization.data(withJSONObject: result, options: .prettyPrinted)
            resultText = String(data: data, encoding: .utf8) ?? "Success"
            isError = false
            showResult = true
        } catch {
            resultText = error.localizedDescription
            isError = true
            showResult = true
        }
    }
    
    private func executeStateTransition() async throws -> Any {
        guard let sdk = appState.sdk else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        switch transitionKey {
        case "identityCreate":
            return try await executeIdentityCreate(sdk: sdk)
            
        case "identityTopUp":
            return try await executeIdentityTopUp(sdk: sdk)
            
        case "identityCreditTransfer":
            return try await executeIdentityCreditTransfer(sdk: sdk)
            
        case "identityCreditWithdrawal":
            return try await executeIdentityCreditWithdrawal(sdk: sdk)
            
        case "documentCreate":
            return try await executeDocumentCreate(sdk: sdk)
            
        case "documentReplace":
            return try await executeDocumentReplace(sdk: sdk)
            
        case "documentDelete":
            return try await executeDocumentDelete(sdk: sdk)
            
        case "documentTransfer":
            return try await executeDocumentTransfer(sdk: sdk)
            
        case "documentUpdatePrice":
            return try await executeDocumentUpdatePrice(sdk: sdk)
            
        case "documentPurchase":
            return try await executeDocumentPurchase(sdk: sdk)
            
        case "tokenMint":
            return try await executeTokenMint(sdk: sdk)
            
        case "tokenBurn":
            return try await executeTokenBurn(sdk: sdk)
            
        case "tokenFreeze":
            return try await executeTokenFreeze(sdk: sdk)
            
        case "tokenUnfreeze":
            return try await executeTokenUnfreeze(sdk: sdk)
            
        case "tokenDestroyFrozenFunds":
            return try await executeTokenDestroyFrozenFunds(sdk: sdk)
            
        case "tokenClaim":
            return try await executeTokenClaim(sdk: sdk)
            
        case "tokenTransfer":
            return try await executeTokenTransfer(sdk: sdk)
            
        case "tokenSetPrice":
            return try await executeTokenSetPrice(sdk: sdk)
            
        default:
            throw SDKError.notImplemented("State transition '\(transitionKey)' not yet implemented")
        }
    }
    
    // MARK: - Individual State Transition Implementations
    
    private func executeIdentityCreate(sdk: SDK) async throws -> Any {
        let identityData = try await sdk.identityCreate()
        
        // Extract identity ID from the response
        guard let idString = identityData["id"] as? String,
              let idData = Data(hexString: idString), idData.count == 32 else {
            throw SDKError.invalidParameter("Invalid identity ID in response")
        }
        
        // Extract balance
        var balance: UInt64 = 0
        if let balanceValue = identityData["balance"] {
            if let balanceNum = balanceValue as? NSNumber {
                balance = balanceNum.uint64Value
            } else if let balanceString = balanceValue as? String,
                      let balanceUInt = UInt64(balanceString) {
                balance = balanceUInt
            }
        }
        
        // Add the new identity to our list
        let identityModel = IdentityModel(
            id: idData,
            balance: balance,
            isLocal: false,
            alias: formInputs["alias"],
            dpnsName: nil
        )
        
        await MainActor.run {
            appState.platformState.addIdentity(identityModel)
        }
        
        return [
            "identityId": idString,
            "balance": balance,
            "message": "Identity created successfully"
        ]
    }
    
    private func executeIdentityTopUp(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        throw SDKError.notImplemented("Identity top-up requires proper Identity handle conversion")
    }
    
    private func executeIdentityCreditTransfer(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let fromIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let toIdentityId = formInputs["toIdentityId"], !toIdentityId.isEmpty else {
            throw SDKError.invalidParameter("Recipient identity ID is required")
        }
        
        guard let amountString = formInputs["amount"],
              let amount = UInt64(amountString) else {
            throw SDKError.invalidParameter("Invalid amount")
        }
        
        // Normalize the recipient identity ID to base58
        let normalizedToIdentityId = normalizeIdentityId(toIdentityId)
        
        // Find the transfer key from the identity's public keys
        let transferKey = fromIdentity.publicKeys.first { key in
            key.purpose == .transfer
        }
        
        guard let transferKey = transferKey else {
            throw SDKError.invalidParameter("No transfer key found for this identity")
        }
        
        // Get the actual private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: fromIdentity.id,
            keyIndex: Int32(transferKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for transfer key #\(transferKey.id). Please add the private key first.")
        }
        
        print("🔑 Using private key for key #\(transferKey.id): \(privateKeyData.toHexString())")
        
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the convenience method with DPPIdentity
        let dppIdentity = fromIdentity.dppIdentity ?? DPPIdentity(
            id: fromIdentity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: fromIdentity.publicKeys.map { ($0.id, $0) }),
            balance: fromIdentity.balance,
            revision: 0
        )
        
        let (senderBalance, receiverBalance) = try await sdk.transferCredits(
            from: dppIdentity,
            toIdentityId: normalizedToIdentityId,
            amount: amount,
            signer: OpaquePointer(signer)!
        )
        
        // Update sender's balance in our local state
        await MainActor.run {
            appState.platformState.updateIdentityBalance(id: fromIdentity.id, newBalance: senderBalance)
        }
        
        return [
            "senderIdentityId": fromIdentity.idString,
            "senderBalance": senderBalance,
            "receiverIdentityId": normalizedToIdentityId,
            "receiverBalance": receiverBalance,
            "transferAmount": amount,
            "message": "Credits transferred successfully"
        ]
    }
    
    private func executeIdentityCreditWithdrawal(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let toAddress = formInputs["toAddress"], !toAddress.isEmpty else {
            throw SDKError.invalidParameter("Recipient address is required")
        }
        
        guard let amountString = formInputs["amount"],
              let amount = UInt64(amountString) else {
            throw SDKError.invalidParameter("Invalid amount")
        }
        
        let coreFeePerByteString = formInputs["coreFeePerByte"] ?? "0"
        let coreFeePerByte = UInt32(coreFeePerByteString) ?? 0
        
        // Find the transfer key for withdrawal
        let transferKey = identity.publicKeys.first { key in
            key.purpose == .transfer
        }
        
        guard let transferKey = transferKey else {
            throw SDKError.invalidParameter("No transfer key found for this identity")
        }
        
        // Get the actual private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(transferKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for transfer key #\(transferKey.id). Please add the private key first.")
        }
        
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for withdrawal
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let newBalance = try await sdk.withdrawFromIdentity(
            dppIdentity,
            amount: amount,
            toAddress: toAddress,
            coreFeePerByte: coreFeePerByte,
            signer: OpaquePointer(signer)!
        )
        
        // Update identity's balance in our local state
        await MainActor.run {
            appState.platformState.updateIdentityBalance(id: identity.id, newBalance: newBalance)
        }
        
        return [
            "identityId": identity.idString,
            "withdrawnAmount": amount,
            "toAddress": toAddress,
            "coreFeePerByte": coreFeePerByte,
            "newBalance": newBalance,
            "message": "Credits withdrawn successfully"
        ]
    }
    
    private func executeDocumentCreate(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Data contract ID is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let propertiesJson = formInputs["documentFields"], !propertiesJson.isEmpty else {
            throw SDKError.invalidParameter("Document properties are required")
        }
        
        // Parse the JSON properties
        guard let propertiesData = propertiesJson.data(using: .utf8),
              let properties = try? JSONSerialization.jsonObject(with: propertiesData) as? [String: Any] else {
            throw SDKError.invalidParameter("Invalid JSON in properties field")
        }
        
        // Determine the required security level for this document type
        var requiredSecurityLevel: SecurityLevel = .high // Default to HIGH as per DPP
        
        // Try to get the document type's security requirement from persistent storage
        // Convert contractId (base58 string) to Data for comparison
        let contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: #Predicate { $0.id == contractIdData }
        )
        if let persistentContract = try? appState.modelContainer.mainContext.fetch(descriptor).first,
           let documentTypes = persistentContract.documentTypes,
           let docType = documentTypes.first(where: { $0.name == documentType }) {
            // Security level in storage: 0=MASTER, 1=CRITICAL, 2=HIGH, 3=MEDIUM
            requiredSecurityLevel = SecurityLevel(rawValue: UInt8(docType.securityLevel)) ?? .high
            print("📋 Document type '\(documentType)' requires security level: \(requiredSecurityLevel.name)")
        } else {
            print("⚠️ Could not determine security level for document type '\(documentType)', using default: HIGH")
        }
        
        // Find a key for signing - must meet security requirements
        print("🔑 Available keys for identity:")
        for key in ownerIdentity.publicKeys {
            print("  - ID: \(key.id), Purpose: \(key.purpose.name), Security: \(key.securityLevel.name), Disabled: \(key.isDisabled)")
        }
        
        // For document operations, we need AUTHENTICATION purpose keys
        // The key's security level must be equal to or stronger than the document's requirement
        let suitableKeys = ownerIdentity.publicKeys.filter { key in
            // Never use disabled keys
            guard !key.isDisabled else { return false }
            
            // Must be AUTHENTICATION purpose for document operations
            guard key.purpose == .authentication else { return false }
            
            // Security level must meet or exceed requirement (lower rawValue = higher security)
            guard key.securityLevel.rawValue <= requiredSecurityLevel.rawValue else { return false }
            
            return true
        }.sorted { k1, k2 in
            // Sort by security level preference:
            // 1. Exact match (e.g., MEDIUM for MEDIUM requirement)
            // 2. Next level up (e.g., HIGH for MEDIUM requirement)
            // 3. Higher levels (e.g., CRITICAL for MEDIUM requirement)
            
            // If one matches exactly and the other doesn't, prefer exact match
            if k1.securityLevel == requiredSecurityLevel && k2.securityLevel != requiredSecurityLevel {
                return true
            }
            if k1.securityLevel != requiredSecurityLevel && k2.securityLevel == requiredSecurityLevel {
                return false
            }
            
            // If neither matches exactly, prefer the one closer to the requirement
            // (higher rawValue = lower security, so we want the highest rawValue that still meets the requirement)
            if k1.securityLevel != requiredSecurityLevel && k2.securityLevel != requiredSecurityLevel {
                // Both are stronger than required, prefer the weaker (closer to requirement)
                if k1.securityLevel.rawValue > k2.securityLevel.rawValue {
                    return true
                } else if k1.securityLevel.rawValue < k2.securityLevel.rawValue {
                    return false
                }
            }
            
            // If same security level, prefer lower ID (non-master keys)
            return k1.id < k2.id
        }
        
        // Try to find a key with its private key available
        var finalSigningKey: IdentityPublicKey? = nil
        var privateKeyData: Data? = nil
        
        for key in suitableKeys {
            print("🔑 Trying key: ID: \(key.id), Purpose: \(key.purpose.name), Security: \(key.securityLevel.name)")
            
            // Try to get the private key from keychain
            if let keyData = KeychainManager.shared.retrievePrivateKey(
                identityId: ownerIdentity.id,
                keyIndex: Int32(key.id)
            ) {
                print("✅ Found private key for key #\(key.id)")
                finalSigningKey = key
                privateKeyData = keyData
                break
            } else {
                print("⚠️ Private key not found for key #\(key.id), trying next suitable key...")
            }
        }
        
        guard let selectedKey = finalSigningKey, let keyData = privateKeyData else {
            let availableKeys = ownerIdentity.publicKeys.map { 
                "ID: \($0.id), Purpose: \($0.purpose.name), Security: \($0.securityLevel.name)" 
            }.joined(separator: "\n  ")
            
            let triedKeys = suitableKeys.map { 
                "ID: \($0.id) (\($0.securityLevel.name))" 
            }.joined(separator: ", ")
            
            throw SDKError.invalidParameter(
                "No suitable key with available private key found for signing document type '\(documentType)' (requires \(requiredSecurityLevel.name) security with AUTHENTICATION purpose).\n\nTried keys: \(triedKeys)\n\nAll available keys:\n  \(availableKeys)\n\nPlease add the private key for one of the suitable keys."
            )
        }
        
        print("🔑 Selected signing key: ID: \(selectedKey.id), Purpose: \(selectedKey.purpose.name), Security: \(selectedKey.securityLevel.name)")
        
        // Create signer using the already retrieved private key data
        let signerResult = keyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(keyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for document creation
        let dppIdentity = ownerIdentity.dppIdentity ?? DPPIdentity(
            id: ownerIdentity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
            balance: ownerIdentity.balance,
            revision: 0
        )
        
        let result = try await sdk.documentCreate(
            contractId: contractId,
            documentType: documentType,
            ownerIdentity: dppIdentity,
            properties: properties,
            signer: OpaquePointer(signer)!
        )
        
        return result
    }
    
    private func executeDocumentDelete(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Data contract is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let documentId = formInputs["documentId"], !documentId.isEmpty else {
            throw SDKError.invalidParameter("Document ID is required")
        }
        
        // Use the DPPIdentity
        let dppIdentity = ownerIdentity.dppIdentity ?? DPPIdentity(
            id: ownerIdentity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
            balance: ownerIdentity.balance,
            revision: 0
        )
        
        // Find a suitable signing key with private key available
        // For delete, we typically use the critical key (ID 1)
        var privateKeyData: Data?
        var selectedKey: IdentityPublicKey?
        
        // First try to find the critical key (ID 1)
        if let criticalKey = ownerIdentity.publicKeys.first(where: { $0.id == 1 && $0.securityLevel == .critical }) {
            if let keyData = KeychainManager.shared.retrievePrivateKey(
                identityId: ownerIdentity.id,
                keyIndex: Int32(criticalKey.id)
            ) {
                selectedKey = criticalKey
                privateKeyData = keyData
            }
        }
        
        // If critical key not found or no private key, try any authentication key
        if selectedKey == nil {
            for key in ownerIdentity.publicKeys.filter({ $0.purpose == .authentication }) {
                if let keyData = KeychainManager.shared.retrievePrivateKey(
                    identityId: ownerIdentity.id,
                    keyIndex: Int32(key.id)
                ) {
                    selectedKey = key
                    privateKeyData = keyData
                    break
                }
            }
        }
        
        guard let signingKey = selectedKey, let keyData = privateKeyData else {
            throw SDKError.invalidParameter("No suitable key with available private key found for signing")
        }
        
        // Create signer using the private key
        let signerResult = keyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(keyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Call the document delete function
        try await sdk.documentDelete(
            contractId: contractId,
            documentType: documentType,
            documentId: documentId,
            ownerIdentity: dppIdentity,
            signer: OpaquePointer(signer)!
        )
        
        return ["message": "Document deleted successfully"]
    }
    
    private func executeDocumentTransfer(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Data contract is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let documentId = formInputs["documentId"], !documentId.isEmpty else {
            throw SDKError.invalidParameter("Document ID is required")
        }
        
        guard let recipientId = formInputs["recipientId"], !recipientId.isEmpty else {
            throw SDKError.invalidParameter("Recipient identity is required")
        }
        
        // Validate that recipient is not the same as sender
        if recipientId == selectedIdentityId {
            throw SDKError.invalidParameter("Cannot transfer document to yourself")
        }
        
        // Get the owner identity from persistent storage
        guard let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("Selected identity not found")
        }
        
        // Use the DPPIdentity
        let fromIdentity = ownerIdentity.dppIdentity ?? DPPIdentity(
            id: ownerIdentity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
            balance: ownerIdentity.balance,
            revision: 0
        )
        
        // Find a suitable signing key with private key available
        var privateKeyData: Data?
        var selectedKey: IdentityPublicKey?
        
        // For transfer, try to find the critical key (ID 1) first
        if let criticalKey = ownerIdentity.publicKeys.first(where: { $0.id == 1 && $0.securityLevel == .critical }) {
            if let keyData = KeychainManager.shared.retrievePrivateKey(
                identityId: ownerIdentity.id,
                keyIndex: Int32(criticalKey.id)
            ) {
                selectedKey = criticalKey
                privateKeyData = keyData
            }
        }
        
        // If critical key not found or no private key, try any authentication key
        if selectedKey == nil {
            for key in ownerIdentity.publicKeys.filter({ $0.purpose == .authentication }) {
                if let keyData = KeychainManager.shared.retrievePrivateKey(
                    identityId: ownerIdentity.id,
                    keyIndex: Int32(key.id)
                ) {
                    selectedKey = key
                    privateKeyData = keyData
                    break
                }
            }
        }
        
        guard let keyData = privateKeyData else {
            throw SDKError.invalidParameter("No suitable key with available private key found for signing")
        }
        
        // Create signer using the private key
        let signerResult = keyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(keyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Call the document transfer function
        let result = try await sdk.documentTransfer(
            contractId: contractId,
            documentType: documentType,
            documentId: documentId,
            fromIdentity: fromIdentity,
            toIdentityId: recipientId,
            signer: OpaquePointer(signer)!
        )
        
        return result
    }
    
    private func executeDocumentUpdatePrice(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Data contract is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let documentId = formInputs["documentId"], !documentId.isEmpty else {
            throw SDKError.invalidParameter("Document ID is required")
        }
        
        guard let newPriceStr = formInputs["newPrice"], !newPriceStr.isEmpty else {
            throw SDKError.invalidParameter("New price is required")
        }
        
        guard let newPrice = UInt64(newPriceStr) else {
            throw SDKError.invalidParameter("Invalid price format")
        }
        
        // Get the owner identity from persistent storage
        guard let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("Selected identity not found")
        }
        
        // Use the DPPIdentity
        let ownerDPPIdentity = ownerIdentity.dppIdentity ?? DPPIdentity(
            id: ownerIdentity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
            balance: ownerIdentity.balance,
            revision: 0
        )
        
        // Find a suitable signing key with private key available
        var privateKeyData: Data?
        var selectedKey: IdentityPublicKey?
        
        // For update price, try to find the critical key (ID 1) first
        if let criticalKey = ownerIdentity.publicKeys.first(where: { $0.id == 1 && $0.securityLevel == .critical }) {
            if let keyData = KeychainManager.shared.retrievePrivateKey(
                identityId: ownerIdentity.id,
                keyIndex: Int32(criticalKey.id)
            ) {
                selectedKey = criticalKey
                privateKeyData = keyData
            }
        }
        
        // If critical key not found or no private key, try any authentication key
        if selectedKey == nil {
            for key in ownerIdentity.publicKeys.filter({ $0.purpose == .authentication }) {
                if let keyData = KeychainManager.shared.retrievePrivateKey(
                    identityId: ownerIdentity.id,
                    keyIndex: Int32(key.id)
                ) {
                    selectedKey = key
                    privateKeyData = keyData
                    break
                }
            }
        }
        
        guard let keyData = privateKeyData else {
            throw SDKError.invalidParameter("No suitable key with available private key found for signing")
        }
        
        // Create signer using the private key
        let signerResult = keyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(keyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Call the document update price function
        let result = try await sdk.documentUpdatePrice(
            contractId: contractId,
            documentType: documentType,
            documentId: documentId,
            newPrice: newPrice,
            ownerIdentity: ownerDPPIdentity,
            signer: OpaquePointer(signer)!
        )
        
        return result
    }
    
    private func executeDocumentPurchase(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Data contract is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let documentId = formInputs["documentId"], !documentId.isEmpty else {
            throw SDKError.invalidParameter("Document ID is required")
        }
        
        guard let priceString = formInputs["price"],
              let _ = UInt64(priceString) else {
            throw SDKError.invalidParameter("Invalid price")
        }
        
        throw SDKError.notImplemented("Document purchase is prepared but FFI bindings not yet exposed to Swift")
    }
    
    private func executeDocumentReplace(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Data contract ID is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let documentId = formInputs["documentId"], !documentId.isEmpty else {
            throw SDKError.invalidParameter("Document ID is required")
        }
        
        guard let propertiesJson = formInputs["documentFields"], !propertiesJson.isEmpty else {
            throw SDKError.invalidParameter("Document properties are required")
        }
        
        // Parse the JSON properties
        guard let propertiesData = propertiesJson.data(using: .utf8),
              let properties = try? JSONSerialization.jsonObject(with: propertiesData) as? [String: Any] else {
            throw SDKError.invalidParameter("Invalid JSON in properties field")
        }
        
        // Determine the required security level for this document type (similar to create)
        var requiredSecurityLevel: SecurityLevel = .high // Default to HIGH as per DPP
        
        // Try to get the document type's security requirement from persistent storage
        let contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: #Predicate { $0.id == contractIdData }
        )
        if let persistentContract = try? appState.modelContainer.mainContext.fetch(descriptor).first,
           let documentTypes = persistentContract.documentTypes,
           let docType = documentTypes.first(where: { $0.name == documentType }) {
            requiredSecurityLevel = SecurityLevel(rawValue: UInt8(docType.securityLevel)) ?? .high
            print("📋 Document type '\(documentType)' requires security level: \(requiredSecurityLevel.name)")
        } else {
            print("⚠️ Could not determine security level for document type '\(documentType)', using default: HIGH")
        }
        
        // Find a key for signing - must meet security requirements
        print("🔑 Available keys for identity:")
        for key in ownerIdentity.publicKeys {
            print("  - ID: \(key.id), Purpose: \(key.purpose.name), Security: \(key.securityLevel.name), Disabled: \(key.isDisabled)")
        }
        
        // For document operations, we need AUTHENTICATION purpose keys
        let suitableKeys = ownerIdentity.publicKeys.filter { key in
            guard !key.isDisabled else { return false }
            guard key.purpose == .authentication else { return false }
            guard key.securityLevel.rawValue <= requiredSecurityLevel.rawValue else { return false }
            return true
        }.sorted { k1, k2 in
            // Prefer exact match, then closer to requirement
            if k1.securityLevel == requiredSecurityLevel && k2.securityLevel != requiredSecurityLevel {
                return true
            }
            if k1.securityLevel != requiredSecurityLevel && k2.securityLevel == requiredSecurityLevel {
                return false
            }
            if k1.securityLevel != requiredSecurityLevel && k2.securityLevel != requiredSecurityLevel {
                if k1.securityLevel.rawValue > k2.securityLevel.rawValue {
                    return true
                }
            }
            return k1.id < k2.id
        }
        
        guard !suitableKeys.isEmpty else {
            print("❌ No suitable keys found for document type '\(documentType)' (requires \(requiredSecurityLevel.name) security)")
            throw SDKError.invalidParameter(
                "No suitable keys found for signing document type '\(documentType)' (requires \(requiredSecurityLevel.name) security with AUTHENTICATION purpose)"
            )
        }
        
        // Find a key with a private key available
        var selectedKey: IdentityPublicKey?
        var keyData: Data?
        
        for candidateKey in suitableKeys {
            print("🔍 Checking key ID \(candidateKey.id) for private key...")
            
            // Get private key from keychain
            if let privateKeyData = KeychainManager.shared.retrievePrivateKey(
                identityId: ownerIdentity.id,
                keyIndex: Int32(candidateKey.id)
            ) {
                selectedKey = candidateKey
                keyData = privateKeyData
                print("✅ Found private key for key ID \(candidateKey.id)")
                break
            } else {
                print("⚠️ No private key found for key ID \(candidateKey.id)")
            }
        }
        
        guard let selectedKey = selectedKey, let keyData = keyData else {
            let availableKeys = ownerIdentity.publicKeys.map { 
                "ID: \($0.id), Purpose: \($0.purpose.name), Security: \($0.securityLevel.name)" 
            }.joined(separator: "\n  ")
            
            let triedKeys = suitableKeys.map { 
                "ID: \($0.id) (\($0.securityLevel.name))" 
            }.joined(separator: ", ")
            
            throw SDKError.invalidParameter(
                "No suitable key with available private key found for signing document type '\(documentType)' (requires \(requiredSecurityLevel.name) security with AUTHENTICATION purpose).\n\nTried keys: \(triedKeys)\n\nAll available keys:\n  \(availableKeys)\n\nPlease add the private key for one of the suitable keys."
            )
        }
        
        print("🔑 Selected signing key: ID: \(selectedKey.id), Purpose: \(selectedKey.purpose.name), Security: \(selectedKey.securityLevel.name)")
        
        // Create signer using the already retrieved private key data
        let signerResult = keyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(keyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for document replacement
        let dppIdentity = ownerIdentity.dppIdentity ?? DPPIdentity(
            id: ownerIdentity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
            balance: ownerIdentity.balance,
            revision: 0
        )
        
        let result = try await sdk.documentReplace(
            contractId: contractId,
            documentType: documentType,
            documentId: documentId,
            ownerIdentity: dppIdentity,
            properties: properties,
            signer: OpaquePointer(signer)!
        )
        
        return result
    }
    
    private func executeTokenMint(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let amountString = formInputs["amount"], !amountString.isEmpty else {
            throw SDKError.invalidParameter("Amount is required")
        }
        
        // The issuedToIdentityId is optional - if not provided, tokens go to the contract owner
        let recipientIdString = formInputs["issuedToIdentityId"]?.isEmpty == false ? formInputs["issuedToIdentityId"] : nil
        
        // Parse amount based on whether it contains a decimal
        let amount: UInt64
        if amountString.contains(".") {
            // Handle decimal input (e.g., "1.5" tokens)
            guard let doubleValue = Double(amountString) else {
                throw SDKError.invalidParameter("Invalid amount format")
            }
            // Convert to smallest unit (assuming 8 decimal places like Dash)
            amount = UInt64(doubleValue * 100_000_000)
        } else {
            // Handle integer input
            guard let intValue = UInt64(amountString) else {
                throw SDKError.invalidParameter("Invalid amount format")
            }
            amount = intValue
        }
        
        // Find the minting key - for tokens, we need a critical security level key
        // First try to find a critical key with OWNER purpose, then fall back to critical AUTHENTICATION
        
        // Debug: log all available keys
        print("🔑 TOKEN MINT: Available keys for identity:")
        for key in identity.publicKeys {
            print("  - Key \(key.id): purpose=\(key.purpose), securityLevel=\(key.securityLevel)")
        }
        
        let mintingKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let mintingKey = mintingKey else {
            throw SDKError.invalidParameter("No suitable key found for minting. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        print("🔑 TOKEN MINT: Selected key \(mintingKey.id) with purpose \(mintingKey.purpose) and security level \(mintingKey.securityLevel)")
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(mintingKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for minting key #\(mintingKey.id). Please add the private key first.")
        }
        
        // Create signer
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for minting
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil
        
        let result = try await sdk.tokenMint(
            contractId: contractId,
            recipientId: recipientIdString,
            amount: amount,
            ownerIdentity: dppIdentity,
            keyId: mintingKey.id,
            signer: OpaquePointer(signer)!,
            note: note
        )
        
        return result
    }
    
    private func executeTokenBurn(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let amountString = formInputs["amount"], !amountString.isEmpty else {
            throw SDKError.invalidParameter("Amount is required")
        }
        
        // Parse amount based on whether it contains a decimal
        let amount: UInt64
        if amountString.contains(".") {
            // Handle decimal input (e.g., "1.5" tokens)
            guard let doubleValue = Double(amountString) else {
                throw SDKError.invalidParameter("Invalid amount format")
            }
            // Convert to smallest unit (assuming 8 decimal places like Dash)
            amount = UInt64(doubleValue * 100_000_000)
        } else {
            // Handle integer input
            guard let intValue = UInt64(amountString) else {
                throw SDKError.invalidParameter("Invalid amount format")
            }
            amount = intValue
        }
        
        // Find the burning key - for tokens, we need a critical security level key
        // First try to find a critical key with OWNER purpose, then fall back to critical AUTHENTICATION
        let burningKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let burningKey = burningKey else {
            throw SDKError.invalidParameter("No suitable key found for burning. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(burningKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for burning key #\(burningKey.id). Please add the private key first.")
        }
        
        // Create signer
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for burning
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil
        
        let result = try await sdk.tokenBurn(
            contractId: contractId,
            amount: amount,
            ownerIdentity: dppIdentity,
            keyId: burningKey.id,
            signer: OpaquePointer(signer)!,
            note: note
        )
        
        return result
    }
    
    private func executeTokenFreeze(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let targetIdentityId = formInputs["targetIdentityId"], !targetIdentityId.isEmpty else {
            throw SDKError.invalidParameter("Target identity ID is required")
        }
        
        // Find the freezing key - for tokens, we need a critical security level key
        // First try to find a critical key with OWNER purpose, then fall back to critical AUTHENTICATION
        let freezingKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let freezingKey = freezingKey else {
            throw SDKError.invalidParameter("No suitable key found for freezing. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(freezingKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for freezing key #\(freezingKey.id). Please add the private key first.")
        }
        
        // Create signer
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for freezing
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil
        
        let result = try await sdk.tokenFreeze(
            contractId: contractId,
            targetIdentityId: targetIdentityId,
            ownerIdentity: dppIdentity,
            keyId: freezingKey.id,
            signer: OpaquePointer(signer)!,
            note: note
        )
        
        return result
    }
    
    private func executeTokenUnfreeze(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let targetIdentityId = formInputs["targetIdentityId"], !targetIdentityId.isEmpty else {
            throw SDKError.invalidParameter("Target identity ID is required")
        }
        
        // Find the unfreezing key - for tokens, we need a critical security level key
        // First try to find a critical key with OWNER purpose, then fall back to critical AUTHENTICATION
        let unfreezingKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let unfreezingKey = unfreezingKey else {
            throw SDKError.invalidParameter("No suitable key found for unfreezing. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(unfreezingKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for unfreezing key #\(unfreezingKey.id). Please add the private key first.")
        }
        
        // Create signer
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signerHandle = signerResult.data else {
            let errorString = signerResult.error?.pointee.message != nil ?
                String(cString: signerResult.error!.pointee.message) : "Failed to create signer"
            dash_sdk_error_free(signerResult.error)
            throw SDKError.internalError(errorString)
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signerHandle))
        }
        
        // Use the DPPIdentity for unfreezing
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let result = try await sdk.tokenUnfreeze(
            contractId: contractId,
            targetIdentityId: targetIdentityId,
            ownerIdentity: dppIdentity,
            keyId: unfreezingKey.id,
            signer: OpaquePointer(signerHandle)!,
            note: formInputs["note"]
        )
        
        return result
    }
    
    private func executeTokenDestroyFrozenFunds(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let frozenIdentityId = formInputs["frozenIdentityId"], !frozenIdentityId.isEmpty else {
            throw SDKError.invalidParameter("Frozen identity ID is required")
        }
        
        // Find the destroy frozen funds key - for tokens, we need a critical security level key
        // First try to find a critical key with OWNER purpose, then fall back to critical AUTHENTICATION
        let destroyKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let destroyKey = destroyKey else {
            throw SDKError.invalidParameter("No suitable key found for destroying frozen funds. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(destroyKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for destroy key #\(destroyKey.id). Please add the private key first.")
        }
        
        // Create signer
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signerHandle = signerResult.data else {
            let errorString = signerResult.error?.pointee.message != nil ?
                String(cString: signerResult.error!.pointee.message) : "Failed to create signer"
            dash_sdk_error_free(signerResult.error)
            throw SDKError.internalError(errorString)
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signerHandle))
        }
        
        // Use the DPPIdentity for destroying frozen funds
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let result = try await sdk.tokenDestroyFrozenFunds(
            contractId: contractId,
            frozenIdentityId: frozenIdentityId,
            ownerIdentity: dppIdentity,
            keyId: destroyKey.id,
            signer: OpaquePointer(signerHandle)!,
            note: formInputs["note"]
        )
        
        return result
    }
    
    private func executeTokenClaim(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let distributionType = formInputs["distributionType"], !distributionType.isEmpty else {
            throw SDKError.invalidParameter("Distribution type is required")
        }
        
        // Find the claiming key - for tokens, we need a critical security level key
        let claimingKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let claimingKey = claimingKey else {
            throw SDKError.invalidParameter("No suitable key found for claiming. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(claimingKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for claiming key #\(claimingKey.id). Please add the private key first.")
        }
        
        // Create signer using the same pattern as other token operations
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for claiming
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil
        
        let result = try await sdk.tokenClaim(
            contractId: contractId,
            distributionType: distributionType,
            ownerIdentity: dppIdentity,
            keyId: claimingKey.id,
            signer: OpaquePointer(signer)!,
            note: note
        )
        
        return result
    }
    
    private func executeTokenTransfer(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let recipientId = formInputs["recipientId"], !recipientId.isEmpty else {
            throw SDKError.invalidParameter("Recipient identity ID is required")
        }
        
        guard let amountString = formInputs["amount"], !amountString.isEmpty else {
            throw SDKError.invalidParameter("Amount is required")
        }
        
        // Parse amount based on whether it contains a decimal
        let amount: UInt64
        if amountString.contains(".") {
            // Handle decimal input (e.g., "1.5" tokens)
            guard let doubleValue = Double(amountString) else {
                throw SDKError.invalidParameter("Invalid amount format")
            }
            // Convert to smallest unit (assuming 8 decimal places like Dash)
            amount = UInt64(doubleValue * 100_000_000)
        } else {
            // Handle integer input
            guard let intValue = UInt64(amountString) else {
                throw SDKError.invalidParameter("Invalid amount format")
            }
            amount = intValue
        }
        
        // Find the transfer key - for tokens, we need a critical security level key
        let transferKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let transferKey = transferKey else {
            throw SDKError.invalidParameter("No suitable key found for transfer. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(transferKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for transfer key #\(transferKey.id). Please add the private key first.")
        }
        
        // Create signer using the same pattern as other token operations
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for transfer
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil
        
        let result = try await sdk.tokenTransfer(
            contractId: contractId,
            recipientId: recipientId,
            amount: amount,
            ownerIdentity: dppIdentity,
            keyId: transferKey.id,
            signer: OpaquePointer(signer)!,
            note: note
        )
        
        return result
    }
    
    private func executeTokenSetPrice(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let identity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        // Parse the token selection (format: "contractId:position")
        guard let tokenSelection = formInputs["token"], !tokenSelection.isEmpty else {
            throw SDKError.invalidParameter("No token selected")
        }
        
        let components = tokenSelection.split(separator: ":")
        guard components.count == 2 else {
            throw SDKError.invalidParameter("Invalid token selection format")
        }
        
        let contractId = String(components[0])
        
        guard let priceType = formInputs["priceType"], !priceType.isEmpty else {
            throw SDKError.invalidParameter("Price type is required")
        }
        
        // Price data is optional - empty means remove pricing
        let priceData = formInputs["priceData"]?.isEmpty == false ? formInputs["priceData"] : nil
        
        // Find the pricing key - for tokens, we need a critical security level key
        let pricingKey = identity.publicKeys.first { key in
            key.securityLevel == .critical && (key.purpose == .owner || key.purpose == .authentication)
        }
        
        guard let pricingKey = pricingKey else {
            throw SDKError.invalidParameter("No suitable key found for setting price. Need a CRITICAL security level key with OWNER or AUTHENTICATION purpose.")
        }
        
        // Get the private key from keychain
        guard let privateKeyData = KeychainManager.shared.retrievePrivateKey(
            identityId: identity.id,
            keyIndex: Int32(pricingKey.id)
        ) else {
            throw SDKError.invalidParameter("Private key not found for pricing key #\(pricingKey.id). Please add the private key first.")
        }
        
        // Create signer using the same pattern as other token operations
        let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(privateKeyData.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        // Use the DPPIdentity for setting price
        let dppIdentity = identity.dppIdentity ?? DPPIdentity(
            id: identity.id,
            publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
            balance: identity.balance,
            revision: 0
        )
        
        let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil
        
        let result = try await sdk.tokenSetPrice(
            contractId: contractId,
            pricingType: priceType,
            priceData: priceData,
            ownerIdentity: dppIdentity,
            keyId: pricingKey.id,
            signer: OpaquePointer(signer)!,
            note: note
        )
        
        return result
    }
    
    // MARK: - Helper Functions
    
    private func enrichedInput(for input: TransitionInput) -> TransitionInput {
        // For document type picker, pass the selected contract ID in placeholder
        if input.name == "documentType" && input.type == "documentTypePicker" {
            return TransitionInput(
                name: input.name,
                type: input.type,
                label: input.label,
                required: input.required,
                placeholder: selectedContractId.isEmpty ? formInputs["contractId"] : selectedContractId,
                help: input.help,
                defaultValue: input.defaultValue,
                options: input.options,
                action: "transition:\(transitionKey)",  // Pass the transition context
                min: input.min,
                max: input.max
            )
        }
        
        // For contract picker, pass the transition context
        if input.name == "contractId" && input.type == "contractPicker" {
            return TransitionInput(
                name: input.name,
                type: input.type,
                label: input.label,
                required: input.required,
                placeholder: input.placeholder,
                help: input.help,
                defaultValue: input.defaultValue,
                options: input.options,
                action: "transition:\(transitionKey)",  // Pass the transition context
                min: input.min,
                max: input.max
            )
        }
        
        // For recipient identity picker in credit transfer, pass the sender identity ID
        // Pass sender identity ID to exclude it from recipients for transfers
        if (input.name == "toIdentityId" && input.type == "identityPicker" && transitionKey == "identityCreditTransfer") ||
           (input.name == "recipientId" && input.type == "identityPicker" && transitionKey == "documentTransfer") {
            return TransitionInput(
                name: input.name,
                type: input.type,
                label: input.label,
                required: input.required,
                placeholder: selectedIdentityId,  // Pass sender identity ID to exclude it from recipients
                help: input.help,
                defaultValue: input.defaultValue,
                options: input.options,
                action: input.action,
                min: input.min,
                max: input.max
            )
        }
        
        return input
    }
    
    private func fetchDocumentSchema(contractId: String, documentType: String) {
        // TODO: Implement fetching schema and generating dynamic form
        // For now, provide a template based on common patterns
        var schemaTemplate = "{\n"
        
        // Common document type templates
        switch documentType.lowercased() {
        case "note", "message":
            schemaTemplate += "  \"message\": \"Enter your message here\"\n"
        case "profile", "user":
            schemaTemplate += "  \"displayName\": \"John Doe\",\n"
            schemaTemplate += "  \"bio\": \"About me...\"\n"
        case "post":
            schemaTemplate += "  \"title\": \"Post title\",\n"
            schemaTemplate += "  \"content\": \"Post content...\"\n"
        default:
            schemaTemplate += "  // Add document fields here\n"
        }
        
        schemaTemplate += "}"
        formInputs["documentFields"] = schemaTemplate
    }
    
    private func normalizeIdentityId(_ identityId: String) -> String {
        // Remove any prefix
        let cleanId = identityId
            .replacingOccurrences(of: "id:", with: "")
            .replacingOccurrences(of: "0x", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        
        // If it's hex (64 chars), convert to base58
        if cleanId.count == 64, let data = Data(hexString: cleanId) {
            return data.toBase58String()
        }
        
        // Otherwise assume it's already base58
        return cleanId
    }
}

// Extension for IdentityModel display name
extension IdentityModel {
    var displayName: String {
        if let alias = alias, !alias.isEmpty {
            return alias
        } else if let dpnsName = dpnsName, !dpnsName.isEmpty {
            return dpnsName
        } else {
            return String(idHexString.prefix(12)) + "..."
        }
    }
}
