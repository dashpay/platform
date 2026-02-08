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

  // Computed property that properly observes state changes
  var isButtonEnabled: Bool {
    if transitionKey == "documentPurchase" {
      // For document purchase, enable if all fields are filled AND canPurchaseDocument is true
      let hasContractId = !formInputs["contractId", default: ""].isEmpty
      let hasDocumentType = !formInputs["documentType", default: ""].isEmpty
      let hasDocumentId = !formInputs["documentId", default: ""].isEmpty
      let canPurchase = appState.transitionState.canPurchaseDocument

      print("DEBUG: Button enabled check - contract: \(hasContractId), type: \(hasDocumentType), id: \(hasDocumentId), canPurchase: \(canPurchase), executing: \(isExecuting)")

      // Enable if all fields are filled and document can be purchased
      return hasContractId && hasDocumentType && hasDocumentId && canPurchase && !isExecuting
    } else {
      return isFormValid() && !isExecuting
    }
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

  @ViewBuilder
  private var executeButton: some View {
    // Explicitly read the state to ensure SwiftUI tracks the dependency
    let canPurchase = transitionKey == "documentPurchase" ? appState.transitionState.canPurchaseDocument : true
    let enabled = isButtonEnabled
    let _ = print("DEBUG: executeButton render - isButtonEnabled: \(enabled), canPurchase: \(canPurchase), background: \(enabled ? "blue" : "gray")")

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
    .background(enabled ? Color.blue : Color.gray)
    .foregroundColor(.white)
    .cornerRadius(10)
    .disabled(!enabled)
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

    // Reset transition state
    appState.transitionState.reset()

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

    // Special validation for document purchase
    if transitionKey == "documentPurchase" {
      // Debug: Show all form inputs
      print("DEBUG: Current formInputs: \(formInputs)")
      print("DEBUG: selectedContractId: \(selectedContractId)")
      print("DEBUG: selectedDocumentType: \(selectedDocumentType)")

      // Check if all required fields are filled
      for input in transition.inputs {
        if input.required {
          var value = formInputs[input.name] ?? ""

          // Special handling for contract and document type - check both formInputs and selected* variables
          if input.name == "contractId" && value.isEmpty {
            value = selectedContractId
            if !value.isEmpty {
              formInputs["contractId"] = value  // Update formInputs
            }
          }
          if input.name == "documentType" && value.isEmpty {
            value = selectedDocumentType
            if !value.isEmpty {
              formInputs["documentType"] = value  // Update formInputs
            }
          }

          if value.isEmpty {
            print("DEBUG: Form invalid - missing required field: \(input.name), value: '\(value)'")
            return false
          }
        }
      }
      // Also check if the document can be purchased
      // Force re-evaluation of the published property
      let canPurchase = appState.transitionState.canPurchaseDocument
      print("DEBUG: Document purchase form validation - canPurchase: \(canPurchase), price: \(String(describing: appState.transitionState.documentPrice))")
      return canPurchase
    }

    // Standard validation for other transitions
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

    case "dataContractCreate":
      return try await executeDataContractCreate(sdk: sdk)

    case "dataContractUpdate":
      return try await executeDataContractUpdate(sdk: sdk)

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
          appState.platformState.identities.contains(where: { $0.idString == selectedIdentityId }) else {
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

    // Use the convenience method with DPPIdentity
    let dppIdentity = DPPIdentity(
      id: fromIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: fromIdentity.publicKeys.map { ($0.id, $0) }),
      balance: fromIdentity.balance,
      revision: 0
    )

    // Use KeyManager to create transfer signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (transferKey, signer) = try await MainActor.run {
      try keyManager.createTransferSigner(for: dppIdentity)
    }
    defer {
      keyManager.destroySigner(signer)
    }

    print("🔑 Using transfer key #\(transferKey.id)")

    let (senderBalance, receiverBalance) = try await sdk.transferCredits(
      from: dppIdentity,
      toIdentityId: normalizedToIdentityId,
      amount: amount,
      signer: signer
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

    // Use the DPPIdentity for withdrawal
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create transfer signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createTransferSigner(for: dppIdentity)
    }
    defer {
      keyManager.destroySigner(signer)
    }

    let newBalance = try await sdk.withdrawFromIdentity(
      dppIdentity,
      amount: amount,
      toAddress: toAddress,
      coreFeePerByte: coreFeePerByte,
      signer: signer
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

    // Use the DPPIdentity for document creation
    let dppIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to find authentication key with required security level and create signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (selectedKey, signer) = try await MainActor.run {
      try keyManager.createDocumentSigner(
        for: dppIdentity,
        minimumSecurityLevel: requiredSecurityLevel
      )
    }
    defer {
      keyManager.destroySigner(signer)
    }

    print("🔑 Selected signing key: ID: \(selectedKey.id), Purpose: \(selectedKey.purpose.name), Security: \(selectedKey.securityLevel.name)")

    let result = try await sdk.documentCreate(
      contractId: contractId,
      documentType: documentType,
      ownerIdentity: dppIdentity,
      properties: properties,
      signer: signer
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
    let dppIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to find authentication key with private key and create signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createSignerForKey(
        for: dppIdentity,
        purpose: .authentication,
        minimumSecurityLevel: nil,
        preferCritical: true
      )
    }
    defer {
      keyManager.destroySigner(signer)
    }

    // Call the document delete function
    try await sdk.documentDelete(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      ownerIdentity: dppIdentity,
      signer: signer
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
    let fromIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to find authentication key with private key and create signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createSignerForKey(
        for: fromIdentity,
        purpose: .authentication,
        minimumSecurityLevel: nil,
        preferCritical: true
      )
    }
    defer {
      keyManager.destroySigner(signer)
    }

    // Call the document transfer function
    let result = try await sdk.documentTransfer(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      fromIdentity: fromIdentity,
      toIdentityId: recipientId,
      signer: signer
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
    let ownerDPPIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to find authentication key with private key and create signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createSignerForKey(
        for: ownerDPPIdentity,
        purpose: .authentication,
        minimumSecurityLevel: nil,
        preferCritical: true
      )
    }
    defer {
      keyManager.destroySigner(signer)
    }

    // Call the document update price function
    let result = try await sdk.documentUpdatePrice(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      newPrice: newPrice,
      ownerIdentity: ownerDPPIdentity,
      signer: signer
    )

    return result
  }

  private func executeDocumentPurchase(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let purchaserIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
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

    // Check if we can purchase (this should already be validated by the button state)
    if let error = appState.transitionState.documentPurchaseError {
      throw SDKError.invalidParameter(error)
    }

    // Get the price that was fetched by DocumentWithPriceView
    guard let price = appState.transitionState.documentPrice else {
      throw SDKError.invalidParameter("Document price not available. Please enter a valid document ID to fetch its price.")
    }

    // Validate that the document is actually for sale (price > 0)
    if price == 0 {
      throw SDKError.invalidParameter("This document is not for sale")
    }

    // Use the DPPIdentity
    let fromIdentity = DPPIdentity(
      id: purchaserIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: purchaserIdentity.publicKeys.map { ($0.id, $0) }),
      balance: purchaserIdentity.balance,
      revision: 0
    )

    // Use KeyManager to find any key with private key and create signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createSignerForKey(
        for: fromIdentity,
        purpose: nil,
        minimumSecurityLevel: nil,
        preferCritical: true
      )
    }
    defer {
      keyManager.destroySigner(signer)
    }

    // Call the document purchase function
    let result = try await sdk.documentPurchase(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      purchaserIdentity: fromIdentity,
      price: price,
      signer: signer
    )

    return result
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

    // Use the DPPIdentity for document replacement
    let dppIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to find authentication key with required security level and create signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (selectedKey, signer) = try await MainActor.run {
      try keyManager.createDocumentSigner(
        for: dppIdentity,
        minimumSecurityLevel: requiredSecurityLevel
      )
    }
    defer {
      keyManager.destroySigner(signer)
    }

    print("🔑 Selected signing key: ID: \(selectedKey.id), Purpose: \(selectedKey.purpose.name), Security: \(selectedKey.securityLevel.name)")

    let result = try await sdk.documentReplace(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      ownerIdentity: dppIdentity,
      properties: properties,
      signer: signer
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
    // Use the DPPIdentity for minting
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to find critical key with owner or authentication purpose
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let mintingKey: IdentityPublicKey
    let signer: OpaquePointer

    // Try owner purpose first
    let ownerResult = try? await MainActor.run {
      try keyManager.createSignerForKey(
        for: dppIdentity,
        purpose: .owner,
        minimumSecurityLevel: .critical,
        preferCritical: true
      )
    }

    if let (key, sig) = ownerResult {
      mintingKey = key
      signer = sig
    } else {
      // Fall back to authentication
      let (key, sig) = try await MainActor.run {
        try keyManager.createSignerForKey(
          for: dppIdentity,
          purpose: .authentication,
          minimumSecurityLevel: .critical,
          preferCritical: true
        )
      }
      mintingKey = key
      signer = sig
    }
    defer {
      keyManager.destroySigner(signer)
    }

    print("🔑 TOKEN MINT: Selected key \(mintingKey.id) with purpose \(mintingKey.purpose) and security level \(mintingKey.securityLevel)")

    let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil

    let result = try await sdk.tokenMint(
      contractId: contractId,
      recipientId: recipientIdString,
      amount: amount,
      ownerIdentity: dppIdentity,
      keyId: mintingKey.id,
      signer: signer,
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

    // Use the DPPIdentity for burning
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to find critical key with owner or authentication purpose
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let burningKey: IdentityPublicKey
    let signer: OpaquePointer

    // Try owner purpose first
    let ownerResult = try? await MainActor.run {
      try keyManager.createSignerForKey(
        for: dppIdentity,
        purpose: .owner,
        minimumSecurityLevel: .critical,
        preferCritical: true
      )
    }

    if let (key, sig) = ownerResult {
      burningKey = key
      signer = sig
    } else {
      // Fall back to authentication
      let (key, sig) = try await MainActor.run {
        try keyManager.createSignerForKey(
          for: dppIdentity,
          purpose: .authentication,
          minimumSecurityLevel: .critical,
          preferCritical: true
        )
      }
      burningKey = key
      signer = sig
    }
    defer {
      keyManager.destroySigner(signer)
    }

    let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil

    let result = try await sdk.tokenBurn(
      contractId: contractId,
      amount: amount,
      ownerIdentity: dppIdentity,
      keyId: burningKey.id,
      signer: signer,
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

    // Use the DPPIdentity for freezing
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create token operation signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (freezingKey, signer) = try createTokenOperationSigner(for: dppIdentity)
    defer {
      keyManager.destroySigner(signer)
    }

    let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil

    let result = try await sdk.tokenFreeze(
      contractId: contractId,
      targetIdentityId: targetIdentityId,
      ownerIdentity: dppIdentity,
      keyId: freezingKey.id,
      signer: signer,
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

    // Use the DPPIdentity for unfreezing
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create token operation signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (unfreezingKey, signer) = try createTokenOperationSigner(for: dppIdentity)
    defer {
      keyManager.destroySigner(signer)
    }

    let result = try await sdk.tokenUnfreeze(
      contractId: contractId,
      targetIdentityId: targetIdentityId,
      ownerIdentity: dppIdentity,
      keyId: unfreezingKey.id,
      signer: signer,
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

    // Use the DPPIdentity for destroying frozen funds
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create token operation signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (destroyKey, signer) = try createTokenOperationSigner(for: dppIdentity)
    defer {
      keyManager.destroySigner(signer)
    }

    let result = try await sdk.tokenDestroyFrozenFunds(
      contractId: contractId,
      frozenIdentityId: frozenIdentityId,
      ownerIdentity: dppIdentity,
      keyId: destroyKey.id,
      signer: signer,
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

    // Use the DPPIdentity for claiming
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create token operation signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (claimingKey, signer) = try createTokenOperationSigner(for: dppIdentity)
    defer {
      keyManager.destroySigner(signer)
    }

    let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil

    let result = try await sdk.tokenClaim(
      contractId: contractId,
      distributionType: distributionType,
      ownerIdentity: dppIdentity,
      keyId: claimingKey.id,
      signer: signer,
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

    // Use the DPPIdentity for transfer
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create token operation signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (transferKey, signer) = try createTokenOperationSigner(for: dppIdentity)
    defer {
      keyManager.destroySigner(signer)
    }

    let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil

    let result = try await sdk.tokenTransfer(
      contractId: contractId,
      recipientId: recipientId,
      amount: amount,
      ownerIdentity: dppIdentity,
      keyId: transferKey.id,
      signer: signer,
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

    // Use the DPPIdentity for setting price
    let dppIdentity = DPPIdentity(
      id: identity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.publicKeys.map { ($0.id, $0) }),
      balance: identity.balance,
      revision: 0
    )

    // Use KeyManager to create token operation signer
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (pricingKey, signer) = try createTokenOperationSigner(for: dppIdentity)
    defer {
      keyManager.destroySigner(signer)
    }

    let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil

    let result = try await sdk.tokenSetPrice(
      contractId: contractId,
      pricingType: priceType,
      priceData: priceData,
      ownerIdentity: dppIdentity,
      keyId: pricingKey.id,
      signer: signer,
      note: note
    )

    return result
  }

  private func executeDataContractCreate(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
      throw SDKError.invalidParameter("No identity selected")
    }

    // Parse document schemas if provided
    var documentSchemas: [String: Any]? = nil
    if let schemasJson = formInputs["documentSchemas"], !schemasJson.isEmpty {
      guard let data = schemasJson.data(using: .utf8),
            let parsed = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw SDKError.serializationError("Invalid document schemas JSON")
      }
      documentSchemas = parsed
    }

    // Parse token schemas if provided
    var tokenSchemas: [String: Any]? = nil
    if let tokensJson = formInputs["tokenSchemas"], !tokensJson.isEmpty {
      guard let data = tokensJson.data(using: .utf8),
            let parsed = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw SDKError.serializationError("Invalid token schemas JSON")
      }
      tokenSchemas = parsed
    }

    // Parse groups if provided
    var groups: [[String: Any]]? = nil
    if let groupsJson = formInputs["groups"], !groupsJson.isEmpty {
      guard let data = groupsJson.data(using: .utf8),
            let parsed = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
        throw SDKError.serializationError("Invalid groups JSON")
      }
      groups = parsed
    }

    // Build contract configuration
    var contractConfig: [String: Any] = [:]

    // Add boolean configurations
    if formInputs["canBeDeleted"] == "true" {
      contractConfig["canBeDeleted"] = true
    }
    if formInputs["readonly"] == "true" {
      contractConfig["readonly"] = true
    }
    if formInputs["keepsHistory"] == "true" {
      contractConfig["keepsHistory"] = true
    }
    if formInputs["documentsKeepHistoryContractDefault"] == "true" {
      contractConfig["documentsKeepHistoryContractDefault"] = true
    }
    if formInputs["documentsMutableContractDefault"] == "true" {
      contractConfig["documentsMutableContractDefault"] = true
    }
    if formInputs["documentsCanBeDeletedContractDefault"] == "true" {
      contractConfig["documentsCanBeDeletedContractDefault"] = true
    }
    if formInputs["requiresIdentityEncryptionBoundedKey"] == "true" {
      contractConfig["requiresIdentityEncryptionBoundedKey"] = true
    }
    if formInputs["requiresIdentityDecryptionBoundedKey"] == "true" {
      contractConfig["requiresIdentityDecryptionBoundedKey"] = true
    }

    // Add optional text fields
    if let keywords = formInputs["keywords"], !keywords.isEmpty {
      contractConfig["keywords"] = keywords.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
    }
    if let description = formInputs["description"], !description.isEmpty {
      contractConfig["description"] = description
    }

    // Validate that at least one schema is provided
    if documentSchemas == nil && tokenSchemas == nil {
      throw SDKError.invalidParameter("At least one document schema or token schema must be provided")
    }

    // Use the DPPIdentity for contract creation
    let dppIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to create contract signer (requires CRITICAL + AUTHENTICATION)
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createContractSigner(for: dppIdentity)
    }
    defer {
      keyManager.destroySigner(signer)
    }

    let result = try await sdk.dataContractCreate(
      identity: dppIdentity,
      documentSchemas: documentSchemas,
      tokenSchemas: tokenSchemas,
      groups: groups,
      contractConfig: contractConfig,
      signer: signer
    )

    return result
  }

  private func executeDataContractUpdate(sdk: SDK) async throws -> Any {
    guard let contractId = formInputs["dataContractId"], !contractId.isEmpty else {
      throw SDKError.invalidParameter("Data contract ID is required")
    }

    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = appState.platformState.identities.first(where: { $0.idString == selectedIdentityId }) else {
      throw SDKError.invalidParameter("No identity selected")
    }

    // Parse new document schemas if provided
    var newDocumentSchemas: [String: Any]? = nil
    if let schemasJson = formInputs["newDocumentSchemas"], !schemasJson.isEmpty {
      guard let data = schemasJson.data(using: .utf8),
            let parsed = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw SDKError.serializationError("Invalid document schemas JSON")
      }
      newDocumentSchemas = parsed
    }

    // Parse new token schemas if provided
    var newTokenSchemas: [String: Any]? = nil
    if let tokensJson = formInputs["newTokenSchemas"], !tokensJson.isEmpty {
      guard let data = tokensJson.data(using: .utf8),
            let parsed = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw SDKError.serializationError("Invalid token schemas JSON")
      }
      newTokenSchemas = parsed
    }

    // Parse new groups if provided
    var newGroups: [[String: Any]]? = nil
    if let groupsJson = formInputs["newGroups"], !groupsJson.isEmpty {
      guard let data = groupsJson.data(using: .utf8),
            let parsed = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
        throw SDKError.serializationError("Invalid groups JSON")
      }
      newGroups = parsed
    }

    // Validate that at least one update is provided
    if newDocumentSchemas == nil && newTokenSchemas == nil && newGroups == nil {
      throw SDKError.invalidParameter("At least one update (document schemas, token schemas, or groups) must be provided")
    }

    // Use the DPPIdentity for contract update
    let dppIdentity = DPPIdentity(
      id: ownerIdentity.id,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.publicKeys.map { ($0.id, $0) }),
      balance: ownerIdentity.balance,
      revision: 0
    )

    // Use KeyManager to create contract signer (requires CRITICAL + AUTHENTICATION)
    let keyManager = await MainActor.run { KeyManager.withSharedKeychain() }
    let (_, signer) = try await MainActor.run {
      try keyManager.createContractSigner(for: dppIdentity)
    }
    defer {
      keyManager.destroySigner(signer)
    }

    let result = try await sdk.dataContractUpdate(
      contractId: contractId,
      identity: dppIdentity,
      newDocumentSchemas: newDocumentSchemas,
      newTokenSchemas: newTokenSchemas,
      newGroups: newGroups,
      signer: signer
    )

    return result
  }

  // MARK: - Helper Functions

  /// Helper function to create a signer for token operations (requires critical owner or authentication key)
  @MainActor
  private func createTokenOperationSigner(for identity: DPPIdentity) throws -> (key: IdentityPublicKey, signer: OpaquePointer) {
    let keyManager = KeyManager.withSharedKeychain()

    // Try owner purpose first
    if let (key, sig) = try? keyManager.createSignerForKey(
      for: identity,
      purpose: .owner,
      minimumSecurityLevel: .critical,
      preferCritical: true
    ) {
      return (key, sig)
    }

    // Fall back to authentication
    return try keyManager.createSignerForKey(
      for: identity,
      purpose: .authentication,
      minimumSecurityLevel: .critical,
      preferCritical: true
    )
  }

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

    // For documentWithPrice picker, pass contract, document type, and identity ID in action field
    if input.type == "documentWithPrice" {
      let contractId = formInputs["contractId"] ?? ""
      let documentType = formInputs["documentType"] ?? ""
      let identityId = selectedIdentityId
      return TransitionInput(
        name: input.name,
        type: input.type,
        label: input.label,
        required: input.required,
        placeholder: input.placeholder,
        help: input.help,
        defaultValue: input.defaultValue,
        options: input.options,
        action: "\(contractId)|\(documentType)|\(identityId)",  // Pass all values separated by |
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

    // If it's hex (64 chars = 32 bytes), convert to base58
    if AddressValidator.isHexIdentityId(cleanId), let data = Data(hexString: cleanId) {
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
    } else if let mainDpnsName = mainDpnsName, !mainDpnsName.isEmpty {
      return mainDpnsName
    } else if let dpnsName = dpnsName, !dpnsName.isEmpty {
      return dpnsName
    } else {
      return String(idHexString.prefix(12)) + "..."
    }
  }
}
