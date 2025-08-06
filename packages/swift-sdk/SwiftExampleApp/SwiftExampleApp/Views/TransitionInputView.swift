import SwiftUI
import SwiftData

struct TransitionInputView: View {
    let input: TransitionInput
    @Binding var value: String
    @Binding var checkboxValue: Bool
    let onSpecialAction: (String) -> Void
    
    @Query private var dataContracts: [PersistentDataContract]
    @Query private var contracts: [PersistentContract]
    @EnvironmentObject var appState: UnifiedAppState
    
    // State for dynamic selections
    @State private var selectedContractId: String = ""
    @State private var selectedDocumentType: String = ""
    
    // Computed property to get mintable tokens
    var mintableTokens: [(token: PersistentToken, contract: PersistentDataContract)] {
        var results: [(token: PersistentToken, contract: PersistentDataContract)] = []
        
        for contract in dataContracts {
            if let tokens = contract.tokens {
                for token in tokens {
                    if token.manualMintingRules != nil {
                        results.append((token: token, contract: contract))
                    }
                }
            }
        }
        
        return results.sorted(by: { $0.token.displayName < $1.token.displayName })
    }
    
    // Computed property to get burnable tokens
    var burnableTokens: [(token: PersistentToken, contract: PersistentDataContract)] {
        var results: [(token: PersistentToken, contract: PersistentDataContract)] = []
        
        for contract in dataContracts {
            if let tokens = contract.tokens {
                for token in tokens {
                    if token.manualBurningRules != nil {
                        results.append((token: token, contract: contract))
                    }
                }
            }
        }
        
        return results.sorted(by: { $0.token.displayName < $1.token.displayName })
    }
    
    // Computed property to get freezable tokens
    var freezableTokens: [(token: PersistentToken, contract: PersistentDataContract)] {
        var results: [(token: PersistentToken, contract: PersistentDataContract)] = []
        
        for contract in dataContracts {
            if let tokens = contract.tokens {
                for token in tokens {
                    if token.freezeRules != nil {
                        results.append((token: token, contract: contract))
                    }
                }
            }
        }
        
        return results.sorted(by: { $0.token.displayName < $1.token.displayName })
    }
    
    // Computed property to get all tokens (for operations that work on any token)
    var allTokens: [(token: PersistentToken, contract: PersistentDataContract)] {
        var results: [(token: PersistentToken, contract: PersistentDataContract)] = []
        
        for contract in dataContracts {
            if let tokens = contract.tokens {
                for token in tokens {
                    results.append((token: token, contract: contract))
                }
            }
        }
        
        return results.sorted(by: { $0.token.displayName < $1.token.displayName })
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if input.type != "button" && input.type != "checkbox" {
                HStack {
                    Text(input.label)
                        .font(.subheadline)
                        .fontWeight(.medium)
                    if input.required {
                        Text("*")
                            .foregroundColor(.red)
                    }
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
                
            case "mintableToken":
                tokenSelector(tokens: mintableTokens, emptyMessage: "No mintable tokens available")
                
            case "burnableToken":
                tokenSelector(tokens: burnableTokens, emptyMessage: "No burnable tokens available")
                
            case "freezableToken":
                tokenSelector(tokens: freezableTokens, emptyMessage: "No freezable tokens available")
                
            case "anyToken":
                tokenSelector(tokens: allTokens, emptyMessage: "No tokens available")
                
            case "contractPicker":
                contractPicker()
                
            case "documentTypePicker":
                documentTypePicker()
                
            case "identityPicker":
                identityPicker()
                
            case "documentPicker":
                documentPicker()
                
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
        .padding(.vertical, 4)
    }
    
    @ViewBuilder
    private func tokenSelector(tokens: [(token: PersistentToken, contract: PersistentDataContract)], emptyMessage: String) -> some View {
        if tokens.isEmpty {
            Text(emptyMessage)
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else {
            Picker("Select Token", selection: $value) {
                Text("Select a token...").tag("")
                ForEach(tokens, id: \.token.id) { tokenData in
                    let displayName = tokenData.token.getSingularForm(languageCode: "en") ?? tokenData.token.displayName
                    let contractName = getContractDisplayName(tokenData.contract)
                    Text("\(displayName) (from \(contractName))")
                        .tag("\(tokenData.contract.idBase58):\(tokenData.token.position)")
                }
            }
            .pickerStyle(MenuPickerStyle())
            .padding()
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
        }
    }
    
    private func getContractDisplayName(_ contract: PersistentDataContract) -> String {
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
    
    // MARK: - New Picker Components
    
    @ViewBuilder
    private func contractPicker() -> some View {
        if contracts.isEmpty {
            Text("No contracts available")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else {
            Picker("Select Contract", selection: $value) {
                Text("Select a contract...").tag("")
                ForEach(contracts, id: \.contractId) { contract in
                    Text(contract.name)
                        .tag(contract.contractId)
                }
            }
            .pickerStyle(MenuPickerStyle())
            .padding()
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
            .onChange(of: value) { newValue in
                selectedContractId = newValue
                // Notify parent to update related fields
                onSpecialAction("contractSelected:\(newValue)")
            }
        }
    }
    
    @ViewBuilder
    private func documentTypePicker() -> some View {
        // Get the selected contract from parent's form data
        let contractId = input.placeholder ?? selectedContractId
        
        if contractId.isEmpty {
            Text("Please select a contract first")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else if let contract = contracts.first(where: { $0.contractId == contractId }) {
            let docTypes = contract.documentTypes
            if docTypes.isEmpty {
                Text("No document types in selected contract")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(8)
            } else {
                Picker("Select Document Type", selection: $value) {
                    Text("Select a type...").tag("")
                    ForEach(docTypes, id: \.self) { docType in
                        Text(docType).tag(docType)
                    }
                }
                .pickerStyle(MenuPickerStyle())
                .padding()
                .background(Color.gray.opacity(0.1))
                .cornerRadius(8)
                .onChange(of: value) { newValue in
                    selectedDocumentType = newValue
                    // Notify parent to update schema
                    onSpecialAction("documentTypeSelected:\(newValue)")
                }
            }
        } else {
            Text("Invalid contract selected")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.red.opacity(0.1))
                .cornerRadius(8)
        }
    }
    
    @ViewBuilder
    private func identityPicker() -> some View {
        let identities = appState.platformState.identities
        
        if identities.isEmpty {
            Text("No identities available")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else {
            Picker("Select Identity", selection: $value) {
                Text("Select an identity...").tag("")
                ForEach(identities, id: \.idString) { identity in
                    Text(identity.displayName)
                        .tag(identity.idString)
                }
            }
            .pickerStyle(MenuPickerStyle())
            .padding()
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
        }
    }
    
    @ViewBuilder
    private func documentPicker() -> some View {
        // This would need contract and document type context
        // For now, just show a text field with placeholder
        VStack(alignment: .leading, spacing: 4) {
            TextField(input.placeholder ?? "Enter document ID", text: $value)
                .textFieldStyle(RoundedBorderTextFieldStyle())
            Text("Document search coming soon")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }
}