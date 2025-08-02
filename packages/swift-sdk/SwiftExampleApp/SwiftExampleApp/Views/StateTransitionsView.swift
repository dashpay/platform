import SwiftUI
import SwiftDashSDK

struct StateTransitionsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var selectedCategory: TransitionCategory = .identity
    @State private var selectedTransition: String = ""
    @State private var selectedIdentity: PersistentIdentity?
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
                if !selectedTransition.isEmpty && (selectedIdentity != nil || selectedTransition == "identityCreate") {
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
                Picker("Identity", selection: $selectedIdentity) {
                    Text("Select...").tag(nil as PersistentIdentity?)
                    ForEach(appState.platformState.identities) { identity in
                        Text(identity.displayName)
                            .tag(identity.persistentModel as PersistentIdentity?)
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
        guard appState.platformState.sdk != nil else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // TODO: Implement actual state transition execution
        // This will require FFI bindings for each transition type
        
        throw SDKError.notImplemented("State transitions not yet implemented")
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