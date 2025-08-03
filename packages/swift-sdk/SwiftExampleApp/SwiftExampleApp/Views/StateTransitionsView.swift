import SwiftUI
import SwiftDashSDK
import DashSDKFFI

struct StateTransitionsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var selectedCategory: TransitionCategory = .identity
    @State private var selectedTransition: String = ""
    @State private var selectedIdentityId: String = ""
    @State private var isExecuting = false
    @State private var showResult = false
    @State private var resultText = ""
    @State private var isError = false
    
    // Dynamic form inputs
    @State private var formInputs: [String: String] = [:]
    @State private var checkboxInputs: [String: Bool] = [:]
    
    enum TransitionCategory: String, CaseIterable {
        case identity = "Identity"
        case dataContract = "Data Contract"
        case document = "Document"
        case token = "Token"
        case voting = "Voting"
        
        var icon: String {
            switch self {
            case .identity: return "person.fill"
            case .dataContract: return "doc.text.fill"
            case .document: return "doc.fill"
            case .token: return "bitcoinsign.circle.fill"
            case .voting: return "hand.raised.fill"
            }
        }
    }
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Category Selection
                categorySelector
                
                // Transition Type Selection
                transitionTypeSelector
                
                // Identity Selector (for all transitions except Identity Create)
                if !selectedTransition.isEmpty && selectedTransition != "identityCreate" {
                    identitySelector
                }
                
                // Dynamic Form Inputs
                if !selectedTransition.isEmpty {
                    transitionForm
                }
                
                // Execute Button
                if !selectedTransition.isEmpty && (!selectedIdentityId.isEmpty || selectedTransition == "identityCreate") {
                    executeButton
                }
                
                // Result Display
                if showResult {
                    resultView
                }
            }
            .padding()
        }
        .navigationTitle("State Transitions")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private var categorySelector: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Select Category")
                .font(.headline)
            
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 12) {
                    ForEach(TransitionCategory.allCases, id: \.self) { category in
                        CategoryButton(
                            category: category,
                            isSelected: selectedCategory == category,
                            action: {
                                selectedCategory = category
                                selectedTransition = ""
                                clearForm()
                            }
                        )
                    }
                }
            }
        }
    }
    
    private var transitionTypeSelector: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Select Transition Type")
                .font(.headline)
            
            Picker("Transition Type", selection: $selectedTransition) {
                Text("Select...").tag("")
                ForEach(transitionsForCategory(selectedCategory), id: \.key) { transition in
                    Text(transition.label).tag(transition.key)
                }
            }
            .pickerStyle(MenuPickerStyle())
            .onChange(of: selectedTransition) { oldValue, newValue in
                clearForm()
            }
            
            if !selectedTransition.isEmpty,
               let transition = getTransitionDefinition(selectedTransition) {
                Text(transition.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.top, 4)
            }
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
            } else {
                Picker("Identity", selection: $selectedIdentityId) {
                    Text("Select...").tag("")
                    ForEach(appState.platformState.identities, id: \.idString) { identity in
                        Text(identity.alias ?? identity.idString)
                            .tag(identity.idString)
                    }
                }
                .pickerStyle(MenuPickerStyle())
            }
        }
    }
    
    private var transitionForm: some View {
        VStack(alignment: .leading, spacing: 16) {
            if let transition = getTransitionDefinition(selectedTransition) {
                ForEach(transition.inputs, id: \.name) { input in
                    TransitionInputView(
                        input: input,
                        value: binding(for: input),
                        checkboxValue: checkboxBinding(for: input),
                        onSpecialAction: handleSpecialAction
                    )
                }
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
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
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
            get: { checkboxInputs[input.name] ?? (input.defaultValue == "true") },
            set: { checkboxInputs[input.name] = $0 }
        )
    }
    
    private func clearForm() {
        formInputs = [:]
        checkboxInputs = [:]
        showResult = false
        resultText = ""
        isError = false
    }
    
    private func isFormValid() -> Bool {
        guard let transition = getTransitionDefinition(selectedTransition) else { return false }
        
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
        switch action {
        case "generateTestSeed":
            // Generate a test seed phrase
            formInputs["seedPhrase"] = generateTestSeedPhrase()
        case "fetchDocumentSchema":
            // TODO: Fetch document schema
            break
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
    
    private func generateTestSeedPhrase() -> String {
        // This is a placeholder - in production, use proper BIP39 generation
        return "test seed phrase for development only do not use in production ever please"
    }
    
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
            resultText = formatResult(result)
            isError = false
            showResult = true
        } catch {
            resultText = "Error: \(error.localizedDescription)"
            isError = true
            showResult = true
        }
    }
    
    private func executeStateTransition() async throws -> Any {
        guard let sdk = appState.platformState.sdk else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        switch selectedTransition {
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
            
        default:
            throw SDKError.notImplemented("State transition '\(selectedTransition)' not yet implemented")
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
        
        // For demo purposes, create mock instant lock and transaction
        // In production, these would come from the Core wallet
        let mockInstantLock = Data(repeating: 0, count: 165) // Typical IS lock size
        let mockTransaction = Data(repeating: 0, count: 250) // Typical tx size
        let mockPrivateKey = Data(repeating: 1, count: 32) // Mock private key
        let outputIndex: UInt32 = 0
        
        // Create Identity object from handle
        let identityHandle = try await sdk.identityGet(identityId: identity.idString)
        // Note: We need a way to convert the dictionary to an Identity object with handle
        
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
        
        let (senderBalance, receiverBalance) = try await sdk.identityTransferCredits(
            fromIdentityId: fromIdentity.idString,
            toIdentityId: normalizedToIdentityId,
            amount: amount
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
        
        let newBalance = try await sdk.identityWithdraw(
            identityId: identity.idString,
            amount: amount,
            toAddress: toAddress,
            coreFeePerByte: coreFeePerByte
        )
        
        // Update identity balance in our local state
        await MainActor.run {
            appState.platformState.updateIdentityBalance(id: identity.id, newBalance: newBalance)
        }
        
        return [
            "identityId": identity.idString,
            "newBalance": newBalance,
            "withdrawnAmount": amount,
            "toAddress": toAddress,
            "message": "Credits withdrawn successfully"
        ]
    }
    
    private func executeDocumentCreate(sdk: SDK) async throws -> Any {
        guard !selectedIdentityId.isEmpty,
              let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
            throw SDKError.invalidParameter("No identity selected")
        }
        
        guard let contractId = formInputs["contractId"], !contractId.isEmpty else {
            throw SDKError.invalidParameter("Contract ID is required")
        }
        
        guard let documentType = formInputs["documentType"], !documentType.isEmpty else {
            throw SDKError.invalidParameter("Document type is required")
        }
        
        guard let propertiesJson = formInputs["properties"],
              let propertiesData = propertiesJson.data(using: .utf8),
              let properties = try? JSONSerialization.jsonObject(with: propertiesData) as? [String: Any] else {
            throw SDKError.invalidParameter("Invalid document properties JSON")
        }
        
        // For demo purposes, we need to fetch the contract and create proper handles
        throw SDKError.notImplemented("Document creation requires proper contract and identity handle conversion")
    }
    
    private func formatResult(_ result: Any) -> String {
        if let dict = result as? [String: Any] {
            if let data = try? JSONSerialization.data(withJSONObject: dict, options: .prettyPrinted),
               let string = String(data: data, encoding: .utf8) {
                return string
            }
            return "Invalid JSON"
        }
        return String(describing: result)
    }
    
    // MARK: - Helper Methods
    
    private func normalizeIdentityId(_ identityId: String) -> String {
        let trimmed = identityId.trimmingCharacters(in: .whitespacesAndNewlines)
        
        // Check if it looks like hex (64 characters, only hex chars)
        if trimmed.count == 64 && trimmed.allSatisfy({ $0.isHexDigit }) {
            // Convert hex to base58 using FFI
            let result = trimmed.withCString { hexCStr in
                dash_sdk_utils_hex_to_base58(hexCStr)
            }
            
            // Check for errors
            if result.error != nil {
                let error = result.error!.pointee
                let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
                print("Failed to convert hex to base58: \(errorMessage)")
                dash_sdk_error_free(result.error)
                return trimmed
            }
            
            guard result.data != nil else {
                print("No data returned from hex to base58 conversion")
                return trimmed
            }
            
            // Get the base58 string
            let base58CStr = result.data.assumingMemoryBound(to: CChar.self)
            let base58String = String(cString: base58CStr)
            dash_sdk_string_free(base58CStr)
            
            print("Converted hex \(trimmed) to base58: \(base58String)")
            return base58String
        }
        
        // Check if it's valid base58 using FFI
        let isValid = trimmed.withCString { cStr in
            dash_sdk_utils_is_valid_base58(cStr)
        }
        
        if isValid == 1 {
            return trimmed
        } else {
            print("Invalid base58 string: '\(trimmed)'")
            // Still return it and let the SDK handle the error
            return trimmed
        }
    }
    
    // MARK: - Transition Definitions
    
    private func transitionsForCategory(_ category: TransitionCategory) -> [(key: String, label: String)] {
        switch category {
        case .identity:
            return [
                ("identityCreate", "Identity Create"),
                ("identityTopUp", "Identity Top Up"),
                ("identityUpdate", "Identity Update"),
                ("identityCreditTransfer", "Identity Credit Transfer"),
                ("identityCreditWithdrawal", "Identity Credit Withdrawal")
            ]
        case .dataContract:
            return [
                ("dataContractCreate", "Data Contract Create"),
                ("dataContractUpdate", "Data Contract Update")
            ]
        case .document:
            return [
                ("documentCreate", "Document Create"),
                ("documentReplace", "Document Replace"),
                ("documentDelete", "Document Delete"),
                ("documentTransfer", "Document Transfer"),
                ("documentPurchase", "Document Purchase")
            ]
        case .token:
            return [
                ("tokenBurn", "Token Burn"),
                ("tokenMint", "Token Mint"),
                ("tokenClaim", "Token Claim"),
                ("tokenSetPrice", "Token Set Price")
            ]
        case .voting:
            return [
                ("dpnsUsername", "DPNS Username Vote"),
                ("masternodeVote", "Masternode Vote")
            ]
        }
    }
    
    private func getTransitionDefinition(_ key: String) -> TransitionDefinition? {
        return TransitionDefinitions.all[key]
    }
}

// MARK: - Supporting Views

struct CategoryButton: View {
    let category: StateTransitionsView.TransitionCategory
    let isSelected: Bool
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: category.icon)
                    .font(.title2)
                Text(category.rawValue)
                    .font(.caption)
            }
            .frame(width: 80, height: 80)
            .background(isSelected ? Color.blue : Color.gray.opacity(0.2))
            .foregroundColor(isSelected ? .white : .primary)
            .cornerRadius(12)
        }
    }
}

struct TransitionInputView: View {
    let input: TransitionInput
    @Binding var value: String
    @Binding var checkboxValue: Bool
    let onSpecialAction: (String) -> Void
    
    var body: some View {
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
            
            switch input.type {
            case "text":
                TextField(input.placeholder ?? "", text: $value)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                
            case "textarea":
                TextEditor(text: $value)
                    .frame(minHeight: 100)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                    )
                
            case "number":
                TextField(input.placeholder ?? "", text: $value)
                    .keyboardType(.numberPad)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                
            case "checkbox":
                Toggle(isOn: $checkboxValue) {
                    Text(input.label)
                }
                
            case "select":
                Picker(input.label, selection: $value) {
                    Text("Select...").tag("")
                    ForEach(input.options ?? [], id: \.value) { option in
                        Text(option.label).tag(option.value)
                    }
                }
                .pickerStyle(MenuPickerStyle())
                
            case "button":
                Button(action: { onSpecialAction(input.action ?? "") }) {
                    Text(input.label)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(8)
                }
                
            case "json":
                TextEditor(text: $value)
                    .font(.system(.caption, design: .monospaced))
                    .frame(minHeight: 150)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                    )
                
            default:
                TextField(input.placeholder ?? "", text: $value)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            
            if let help = input.help {
                Text(help)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }
}

// MARK: - Data Models

struct TransitionDefinition {
    let key: String
    let label: String
    let description: String
    let inputs: [TransitionInput]
}

struct TransitionInput {
    let name: String
    let type: String
    let label: String
    let required: Bool
    let placeholder: String?
    let help: String?
    let defaultValue: String?
    let options: [SelectOption]?
    let action: String?
    let min: Int?
    let max: Int?
    
    init(
        name: String,
        type: String,
        label: String,
        required: Bool,
        placeholder: String? = nil,
        help: String? = nil,
        defaultValue: String? = nil,
        options: [SelectOption]? = nil,
        action: String? = nil,
        min: Int? = nil,
        max: Int? = nil
    ) {
        self.name = name
        self.type = type
        self.label = label
        self.required = required
        self.placeholder = placeholder
        self.help = help
        self.defaultValue = defaultValue
        self.options = options
        self.action = action
        self.min = min
        self.max = max
    }
}

struct SelectOption {
    let value: String
    let label: String
}

struct StateTransitionsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            StateTransitionsView()
                .environmentObject(UnifiedAppState())
        }
    }
}