import SwiftUI
import SwiftDashSDK
import SwiftData

struct TransitionInputView: View {
    let input: TransitionInput
    @Binding var value: String
    @Binding var checkboxValue: Bool
    let onSpecialAction: (String) -> Void

    @Query private var dataContracts: [PersistentDataContract]
    @Query private var persistedIdentities: [PersistentIdentity]
    @EnvironmentObject var appState: AppState

    // State for dynamic selections
    @State private var selectedContractId: String = ""
    @State private var selectedDocumentType: String = ""
    @State private var useManualEntry: Bool = false

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
                    Text("Select...")
                        .tag("")
                        .accessibilityIdentifier("transition.\(input.name).select.none")
                    ForEach(input.options ?? [], id: \.value) { option in
                        Text(option.label)
                            .tag(option.value)
                            .accessibilityIdentifier("transition.\(input.name).select.\(option.value)")
                    }
                }
                .accessibleInlinePicker("transition.\(input.name).selectPicker")

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
                if input.name == "toIdentityId" || input.name == "recipientId" {
                    recipientIdentityPicker()
                } else {
                    identityPicker()
                }

            case "documentPicker":
                documentPicker()

            case "documentWithPrice":
                documentWithPricePicker()

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
                Text("Select a token...")
                    .tag("")
                    .accessibilityIdentifier("transition.\(input.name).token.none")
                ForEach(tokens, id: \.token.id) { tokenData in
                    let displayName = tokenData.token.getSingularForm(languageCode: "en") ?? tokenData.token.displayName
                    let contractName = getContractDisplayName(tokenData.contract)
                    Text("\(displayName) (from \(contractName))")
                        .tag("\(tokenData.contract.idBase58):\(tokenData.token.position)")
                        .accessibilityIdentifier("transition.\(input.name).token.\(tokenData.contract.idBase58).\(tokenData.token.position)")
                }
            }
            .accessibleInlinePicker("transition.\(input.name).tokenPicker")
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
        // Check operation types from the action field
        let isTransferOperation = input.action?.contains("documentTransfer") == true
        let isPurchaseOperation = input.action?.contains("documentPurchase") == true
        let isSetPriceOperation = input.action?.contains("documentUpdatePrice") == true
        let isCreateOperation = input.action?.contains("documentCreate") == true
        let isReplaceOperation = input.action?.contains("documentReplace") == true
        let isDeleteOperation = input.action?.contains("documentDelete") == true
        let isMarketplaceOperation = isPurchaseOperation || isSetPriceOperation

        // Filter contracts based on operation type
        let availableContracts: [PersistentDataContract] = {
            if isTransferOperation {
                // Only show contracts that have transferable document types
                return dataContracts.filter { contract in
                    if let docTypes = contract.documentTypes {
                        return docTypes.contains { $0.documentsTransferable }
                    }
                    return false
                }
            } else if isMarketplaceOperation {
                // Only show contracts that have tradeable document types (tradeMode = 1)
                return dataContracts.filter { contract in
                    if let docTypes = contract.documentTypes {
                        return docTypes.contains { $0.tradeMode == 1 }
                    }
                    return false
                }
            } else if isCreateOperation {
                // For document creation, only show contracts with creationRestrictionMode 0 or 1 (not 2)
                return dataContracts.filter { contract in
                    if let docTypes = contract.documentTypes {
                        return docTypes.contains { docType in
                            docType.creationRestrictionMode <= 1  // 0 = anyone, 1 = owner only
                        }
                    }
                    return false
                }
            } else if isReplaceOperation {
                // For document replace, only show contracts with mutable document types
                return dataContracts.filter { contract in
                    if let docTypes = contract.documentTypes {
                        return docTypes.contains { $0.documentsMutable }
                    }
                    return false
                }
            } else if isDeleteOperation {
                // For document delete, only show contracts with deletable document types
                return dataContracts.filter { contract in
                    if let docTypes = contract.documentTypes {
                        return docTypes.contains { $0.documentsCanBeDeleted }
                    }
                    return false
                }
            } else {
                return dataContracts
            }
        }()

        let emptyMessage: String = {
            if isTransferOperation {
                return "No contracts with transferable documents"
            } else if isMarketplaceOperation {
                return "No contracts with tradeable documents (marketplace)"
            } else if isCreateOperation {
                return "No contracts allow document creation"
            } else if isReplaceOperation {
                return "No contracts with mutable documents"
            } else if isDeleteOperation {
                return "No contracts with deletable documents"
            } else {
                return "No contracts available"
            }
        }()

        if availableContracts.isEmpty {
            Text(emptyMessage)
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else {
            // Push a real selectable list (rather than a menu / inline
            // Picker) so UI automation (idb) can tap a specific contract by a
            // stable per-row `…contractPicker.row.<idBase58>` identifier.
            //
            // We deliberately do NOT use `.pickerStyle(.navigationLink)` /
            // `.accessibleFormPicker` here: that style only commits its
            // selection binding when it has a `List`/`Form` ancestor, and
            // TransitionDetailView hosts these inputs in a `ScrollView`/
            // `VStack`. In that host the navigationLink picker renders and
            // navigates but silently drops the selection. An explicit
            // `NavigationLink` + selection list writes the binding directly,
            // so it commits in any host. Shared by every document-transition
            // builder (Purchase / Transfer / Update Price / Create / Replace
            // / Delete), so all of them become idb-drivable through this.
            NavigationLink {
                ContractSelectionList(
                    inputName: input.name,
                    contracts: availableContracts,
                    displayName: getContractDisplayName,
                    selection: $value,
                    onSelect: { newValue in
                        selectedContractId = newValue
                        // Notify parent to update related fields
                        onSpecialAction("contractSelected:\(newValue)")
                    }
                )
            } label: {
                HStack {
                    Text("Select Contract")
                        .foregroundColor(.primary)
                    Spacer()
                    Text(availableContracts.first(where: { $0.idBase58 == value })
                        .map(getContractDisplayName) ?? "Select a contract...")
                        .foregroundColor(.secondary)
                }
            }
            .accessibilityIdentifier("transition.\(input.name).contractPicker")
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
        }
    }

    @ViewBuilder
    private func documentTypePicker() -> some View {
        // Get the selected contract from parent's form data
        let contractId = input.placeholder ?? selectedContractId

        // Check operation types
        let isTransferOperation = input.action?.contains("documentTransfer") == true
        let isPurchaseOperation = input.action?.contains("documentPurchase") == true
        let isSetPriceOperation = input.action?.contains("documentUpdatePrice") == true
        let isCreateOperation = input.action?.contains("documentCreate") == true
        let isReplaceOperation = input.action?.contains("documentReplace") == true
        let isDeleteOperation = input.action?.contains("documentDelete") == true
        let isMarketplaceOperation = isPurchaseOperation || isSetPriceOperation

        if contractId.isEmpty {
            Text("Please select a contract first")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else if let contract = dataContracts.first(where: { $0.idBase58 == contractId }) {
            if let docTypes = contract.documentTypes, !docTypes.isEmpty {
                // Filter document types based on operation type
                let availableDocTypes: [PersistentDocumentType] = {
                    if isTransferOperation {
                        return docTypes.filter { $0.documentsTransferable }
                    } else if isMarketplaceOperation {
                        // For marketplace operations, only show document types with tradeMode = 1
                        return docTypes.filter { $0.tradeMode == 1 }
                    } else if isCreateOperation {
                        // For document creation, exclude types with creationRestrictionMode = 2 (system only)
                        return docTypes.filter { $0.creationRestrictionMode <= 1 }
                    } else if isReplaceOperation {
                        // For document replace, only show mutable document types
                        return docTypes.filter { $0.documentsMutable }
                    } else if isDeleteOperation {
                        // For document delete, only show deletable document types
                        return docTypes.filter { $0.documentsCanBeDeleted }
                    } else {
                        return Array(docTypes)
                    }
                }()

                let emptyMessage: String = {
                    if isTransferOperation {
                        return "No transferable document types in selected contract"
                    } else if isMarketplaceOperation {
                        return "No tradeable document types (marketplace) in selected contract"
                    } else if isCreateOperation {
                        return "No document types allow creation in selected contract"
                    } else if isReplaceOperation {
                        return "No mutable document types in selected contract"
                    } else if isDeleteOperation {
                        return "No deletable document types in selected contract"
                    } else {
                        return "No document types in selected contract"
                    }
                }()

                if availableDocTypes.isEmpty {
                    Text(emptyMessage)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding()
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.orange.opacity(0.1))
                        .cornerRadius(8)
                } else {
                    // Same rationale as contractPicker(): push a real
                    // selectable list with stable per-row identifiers so idb
                    // can tap a document type. The inline Picker renders as an
                    // opaque control here (idb sees only a "Slider"), and a
                    // navigationLink Picker wouldn't commit its selection
                    // outside a List/Form ancestor.
                    NavigationLink {
                        DocumentTypeSelectionList(
                            inputName: input.name,
                            docTypeNames: availableDocTypes.map { $0.name },
                            selection: $value,
                            onSelect: { newValue in
                                selectedDocumentType = newValue
                                // Notify parent to update schema
                                onSpecialAction("documentTypeSelected:\(newValue)")
                            }
                        )
                    } label: {
                        HStack {
                            Text("Select Document Type")
                                .foregroundColor(.primary)
                            Spacer()
                            Text(value.isEmpty ? "Select a type..." : value)
                                .foregroundColor(.secondary)
                        }
                    }
                    .accessibilityIdentifier("transition.\(input.name).documentTypePicker")
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.gray.opacity(0.1))
                    .cornerRadius(8)

                    // Show warning if document type has owner-only creation restriction
                    if isCreateOperation && !value.isEmpty,
                       let selectedDocType = availableDocTypes.first(where: { $0.name == value }),
                       selectedDocType.creationRestrictionMode == 1 {
                        // Get the currently selected identity from parent
                        // The parent passes the selected identity through the action field pattern
                        let selectedIdentities = persistedIdentities.filter { identity in
                            // Check if this identity owns the contract
                            return identity.identityId == contract.ownerId
                        }

                        if selectedIdentities.isEmpty {
                            Text("⚠️ Only the contract owner can create documents of this type. You don't have the owner identity.")
                                .font(.caption)
                                .foregroundColor(.orange)
                                .padding()
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(Color.orange.opacity(0.1))
                                .cornerRadius(8)
                        } else {
                            Text("ℹ️ This document type is restricted to contract owner only. Make sure to select the owner identity: \(selectedIdentities.first?.displayName ?? "Unknown")")
                                .font(.caption)
                                .foregroundColor(.blue)
                                .padding()
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(Color.blue.opacity(0.1))
                                .cornerRadius(8)
                        }
                    }
                }
            } else {
                Text("No document types in selected contract")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(8)
            }
        } else {
            Text("Invalid contract selected")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.red.opacity(0.1))
                .cornerRadius(8)
        }
    }

    @ViewBuilder
    private func identityPicker() -> some View {
        let identities = persistedIdentities

        if identities.isEmpty {
            Text("No identities available")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
        } else {
            Picker("Select Identity", selection: $value) {
                Text("Select an identity...")
                    .tag("")
                    .accessibilityIdentifier("transition.\(input.name).identity.none")
                ForEach(identities, id: \.identityIdBase58) { identity in
                    Text(identity.displayName)
                        .tag(identity.identityIdBase58)
                        .accessibilityIdentifier("transition.\(input.name).identity.\(identity.identityIdBase58)")
                }
            }
            .accessibleInlinePicker("transition.\(input.name).identityPicker")
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
        }
    }

    @ViewBuilder
    private func recipientIdentityPicker() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            // Get the sender identity from the parent's selectedIdentityId
            let senderIdentityId = input.placeholder ?? ""
            let identities = persistedIdentities.filter { $0.identityIdBase58 != senderIdentityId }

            if !useManualEntry {
                if identities.isEmpty {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("No other identities available")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .padding()
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(Color.orange.opacity(0.1))
                            .cornerRadius(8)

                        Button(action: {
                            useManualEntry = true
                        }) {
                            Text("💳 Manually Enter Recipient")
                                .frame(maxWidth: .infinity)
                                .padding()
                                .background(Color.blue)
                                .foregroundColor(.white)
                                .cornerRadius(8)
                        }
                    }
                } else {
                    Picker("Select Identity", selection: $value) {
                        Text("Select an identity...")
                            .tag("")
                            .accessibilityIdentifier("transition.\(input.name).recipientIdentity.none")
                        ForEach(identities, id: \.identityIdBase58) { identity in
                            Text(identity.displayName)
                                .tag(identity.identityIdBase58)
                                .accessibilityIdentifier("transition.\(input.name).recipientIdentity.\(identity.identityIdBase58)")
                        }
                        Text("💳 Manually Enter Recipient")
                            .tag("__manual__")
                            .accessibilityIdentifier("transition.\(input.name).recipientIdentity.manual")
                    }
                    .accessibleInlinePicker("transition.\(input.name).recipientIdentityPicker")
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.gray.opacity(0.1))
                    .cornerRadius(8)
                    .onChange(of: value) { _, newValue in
                        if newValue == "__manual__" {
                            value = ""
                            useManualEntry = true
                        }
                    }
                }
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    TextField("Enter recipient identity ID", text: $value)
                        .textFieldStyle(RoundedBorderTextFieldStyle())

                    if !identities.isEmpty {
                        Button(action: {
                            useManualEntry = false
                            value = ""
                        }) {
                            Text("← Back to identity list")
                                .font(.caption)
                                .foregroundColor(.blue)
                        }
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func documentPicker() -> some View {
        TextField(input.placeholder ?? "Enter document ID", text: $value)
            .textFieldStyle(RoundedBorderTextFieldStyle())
    }

    @ViewBuilder
    private func documentWithPricePicker() -> some View {
        // Extract contract ID, document type, and identity ID from action field (format: "contractId|documentType|identityId")
        let parts = (input.action ?? "").split(separator: "|").map(String.init)
        let contractId = parts.count > 0 ? parts[0] : ""
        let documentType = parts.count > 1 ? parts[1] : ""
        let identityId = parts.count > 2 ? parts[2] : nil

        DocumentWithPriceView(
            documentId: $value,
            contractId: contractId,
            documentType: documentType,
            currentIdentityId: identityId
        )
        .environmentObject(appState)
    }
}

/// Pushed contract-selection list backing `contractPicker()`.
///
/// Each contract is a `Button` row carrying a stable
/// `transition.<inputName>.contractPicker.row.<idBase58>` accessibility
/// identifier, so UI automation (idb) can tap a specific contract reliably.
/// The row's action writes the selection binding directly and dismisses —
/// unlike `.pickerStyle(.navigationLink)`, which only commits inside a
/// `List`/`Form` ancestor (TransitionDetailView hosts inputs in a
/// `ScrollView`/`VStack`, where that style drops the selection).
private struct ContractSelectionList: View {
    let inputName: String
    let contracts: [PersistentDataContract]
    let displayName: (PersistentDataContract) -> String
    @Binding var selection: String
    let onSelect: (String) -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        List {
            selectionRow(id: "", label: "Select a contract...")
            ForEach(contracts, id: \.idBase58) { contract in
                selectionRow(id: contract.idBase58, label: displayName(contract))
            }
        }
        .navigationTitle("Select Contract")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func selectionRow(id: String, label: String) -> some View {
        Button {
            selection = id
            onSelect(id)
            dismiss()
        } label: {
            HStack {
                Text(label)
                    .foregroundColor(.primary)
                Spacer()
                if selection == id {
                    Image(systemName: "checkmark")
                        .foregroundColor(.accentColor)
                }
            }
        }
        .accessibilityIdentifier(
            "transition.\(inputName).contractPicker.row.\(id.isEmpty ? "none" : id)"
        )
    }
}

/// Pushed document-type-selection list backing `documentTypePicker()`.
///
/// Mirrors `ContractSelectionList`: each type is a `Button` row carrying a
/// stable `transition.<inputName>.documentTypePicker.row.<name>`
/// accessibility identifier so UI automation can tap it, and the row writes
/// the selection binding directly + dismisses.
private struct DocumentTypeSelectionList: View {
    let inputName: String
    let docTypeNames: [String]
    @Binding var selection: String
    let onSelect: (String) -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        List {
            selectionRow(id: "", label: "Select a type...")
            ForEach(docTypeNames, id: \.self) { name in
                selectionRow(id: name, label: name)
            }
        }
        .navigationTitle("Select Document Type")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func selectionRow(id: String, label: String) -> some View {
        Button {
            selection = id
            onSelect(id)
            dismiss()
        } label: {
            HStack {
                Text(label)
                    .foregroundColor(.primary)
                Spacer()
                if selection == id {
                    Image(systemName: "checkmark")
                        .foregroundColor(.accentColor)
                }
            }
        }
        .accessibilityIdentifier(
            "transition.\(inputName).documentTypePicker.row.\(id.isEmpty ? "none" : id)"
        )
    }
}
