import SwiftUI
import SwiftDashSDK
import DashSDKFFI
import SwiftData

struct TransitionDetailView: View {
  let transitionKey: String
  let transitionLabel: String
  /// Optional pre-filled form values applied once on first appear.
  /// Set by the contracts-register flows (Pasteboard / Quick Basic
  /// Token) so the user lands on a partially-configured form rather
  /// than a blank one. Empty for the regular "build a transition by
  /// hand" entry point.
  var initialInputs: [String: String] = [:]
  /// Optional pre-filled checkbox values applied alongside
  /// `initialInputs`. Same single-shot semantics — applied once on
  /// first appear, never re-applied.
  var initialCheckboxInputs: [String: Bool] = [:]

  @EnvironmentObject var appState: AppState
  @EnvironmentObject var walletManager: PlatformWalletManager
  @Environment(\.modelContext) private var modelContext
  @Query private var identities: [PersistentIdentity]
  @EnvironmentObject var transitionState: TransitionState
  @State private var selectedIdentityId: String = ""
  @State private var isExecuting = false
  @State private var showResult = false
  @State private var resultText = ""
  @State private var isError = false
  /// Gates the resume-from-tracked-lock confirmation. A tracked asset lock
  /// isn't bound to an identity — resume directs it at whatever identity is
  /// selected, and a stray lock landing on the wrong (self-owned) identity
  /// is not undoable — so this flow requires an explicit confirm.
  @State private var showResumeConfirm = false

  // Dynamic form inputs
  @State private var formInputs: [String: String] = [:]
  @State private var checkboxInputs: [String: Bool] = [:]
  @State private var selectedContractId: String = ""
  @State private var selectedDocumentType: String = ""
  @State private var documentFieldValues: [String: Any] = [:]
  /// Guards the one-time form setup in `.onAppear`. Contract / document-type
  /// selection now pushes a child list onto the navigation stack; popping it
  /// re-fires this view's `.onAppear`, and re-running `clearForm()` there
  /// would wipe the in-progress form (including the just-picked contract).
  /// Run setup only on the first appearance of each builder instance.
  @State private var didInitializeForm = false

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
      let canPurchase = transitionState.canPurchaseDocument

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
      // Initialize once per builder instance. Popping a pushed picker (the
      // contract / document-type selection lists) re-fires `.onAppear`, and
      // re-running clearForm() would wipe the in-progress form — including
      // the contract the user just picked. A fresh builder (navigated to
      // anew) is a new view instance with didInitializeForm == false, so it
      // still resets correctly.
      guard !didInitializeForm else { return }
      didInitializeForm = true

      clearForm()
      // Merge any caller-supplied pre-fills (Pasteboard / Quick-Token flows)
      // on top of the schema defaults clearForm() just set.
      for (k, v) in initialInputs {
        formInputs[k] = v
      }
      for (k, v) in initialCheckboxInputs {
        checkboxInputs[k] = v
      }
    }
  }

  private var identitySelector: some View {
    VStack(alignment: .leading, spacing: 12) {
      Text("Select Identity")
        .font(.headline)

      if identities.isEmpty {
        Text("No identities available. Create one first.")
          .font(.caption)
          .foregroundColor(.secondary)
          .padding()
          .frame(maxWidth: .infinity, alignment: .leading)
          .background(Color.orange.opacity(0.1))
          .cornerRadius(8)
      } else {
        Picker("Identity", selection: $selectedIdentityId) {
          // Placeholder option matching the `@State` initial
          // empty-string value. Without this SwiftUI emits
          // "Picker: the selection '' is invalid and does not
          // have an associated tag" until the user picks a real
          // identity.
          Text("Select an identity").tag("")
          ForEach(identities, id: \.identityIdBase58) { identity in
            Text(identity.displayName)
              .tag(identity.identityIdBase58)
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
    let enabled = isButtonEnabled
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
    .confirmationDialog(
      "Resume top-up?",
      isPresented: $showResumeConfirm,
      titleVisibility: .visible
    ) {
      Button("Top Up") {
        Task { await performTransition() }
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      let txid = (formInputs["outPointTxid"] ?? "").trimmingCharacters(in: .whitespaces)
      let vout = (formInputs["outPointVout"] ?? "0").trimmingCharacters(in: .whitespaces)
      Text(
        "Consume asset lock \(txid.isEmpty ? "?" : txid):\(vout) into identity "
          + "\(selectedIdentityId)? This credits the selected identity and cannot be undone."
      )
    }
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
    transitionState.reset()

    // Set default values
    if let transition = getTransitionDefinition(transitionKey) {
      for input in transition.inputs {
        if let defaultValue = input.defaultValue {
          formInputs[input.name] = defaultValue
        }
      }
    }

    // Set the first identity as default if we need identity selection
    if needsIdentitySelection && !identities.isEmpty {
      selectedIdentityId = identities.first?.identityIdBase58 ?? ""
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
      let canPurchase = transitionState.canPurchaseDocument
      print("DEBUG: Document purchase form validation - canPurchase: \(canPurchase), price: \(String(describing: transitionState.documentPrice))")
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
    // Resume directs a tracked asset lock at whatever identity is selected;
    // a stray lock landing on the wrong (self-owned) identity is not
    // undoable, so require explicit confirmation before firing.
    if transitionKey == "identityTopUpResume" {
      showResumeConfirm = true
      return
    }
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

    case "identityTopUpResume":
      return try await executeIdentityTopUpResume(sdk: sdk)

    case "identityUpdate":
      return try await executeIdentityUpdate(sdk: sdk)

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

    // Persist the newly-created identity directly to SwiftData.
    // This path is the legacy SDK-level create; wallet-backed
    // creates go through the Rust persister callback which writes
    // the same row as a side-effect of `IdentityChangeSet`.
    await MainActor.run {
      let row = PersistentIdentity(
        identityId: idData,
        balance: Int64(bitPattern: balance),
        revision: 0,
        isLocal: false,
        alias: formInputs["alias"],
        network: appState.currentNetwork
      )
      modelContext.insert(row)
      // Back-fill any locally-cached contracts that already name
      // this identity as their owner. The save below persists
      // the relationship.
      ContractIdentityLinker.linkIdentityToOwnedContracts(
        identity: row,
        modelContext: modelContext
      )
      try? modelContext.save()
    }

    return [
      "identityId": idString,
      "balance": balance,
      "message": "Identity created successfully"
    ]
  }

  /// Minimum Core-side funding for a managed top-up, in duffs. Mirrors the
  /// Rust `MIN_TOP_UP_DUFFS` guard so the UI blocks a sub-floor amount
  /// *before* any asset lock is broadcast — a lock below Platform's minimum
  /// required fee (active v1 calc: 500-duff base cost + 50_000-duff asset-lock
  /// floor = 50_500 duffs) is accepted by Core but rejected by Platform,
  /// stranding the funds in a lock that can't complete the top-up.
  private static let minTopUpDuffs: UInt64 = 50_500

  /// Top up the selected identity by building a new Core asset lock from
  /// the owning wallet's balance (managed path — the credit-output key
  /// stays behind the Keychain resolver and never crosses FFI as bytes).
  /// Mirrors `executeIdentityUpdate`'s wallet resolution and
  /// `executeIdentityCreditTransfer`'s local balance update.
  @MainActor
  private func executeIdentityTopUp(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
      throw SDKError.invalidParameter("No identity selected")
    }
    guard let walletId = ownerIdentity.wallet?.walletId,
          let wallet = walletManager.wallet(for: walletId) else {
      throw SDKError.invalidParameter(
        "Identity has no wallet linkage; cannot fund the top-up"
      )
    }
    guard let amountString = formInputs["amount"],
          let amountDuffs = UInt64(amountString.trimmingCharacters(in: .whitespaces)) else {
      throw SDKError.invalidParameter("Invalid amount (duffs)")
    }
    guard amountDuffs >= Self.minTopUpDuffs else {
      throw SDKError.invalidParameter(
        "Amount must be at least \(Self.minTopUpDuffs) duffs; a smaller top-up would be rejected by Platform and strand the funds"
      )
    }
    let accountIndex = UInt32(
      formInputs["accountIndex"]?.trimmingCharacters(in: .whitespaces) ?? "0"
    ) ?? 0

    let newBalance = try await wallet.topUpIdentityWithFunding(
      identityId: ownerIdentity.identityId,
      amountDuffs: amountDuffs,
      accountIndex: accountIndex
    )

    PersistentIdentity.updateBalance(
      in: modelContext, identityId: ownerIdentity.identityId, balance: newBalance
    )
    try? modelContext.save()

    return [
      "identityId": ownerIdentity.identityIdBase58,
      "newBalance": newBalance,
      "fundedDuffs": amountDuffs,
      "accountIndex": accountIndex,
      "message": "Identity topped up successfully",
    ]
  }

  /// Recover a stuck top-up by consuming an already-tracked Core asset lock
  /// by outpoint (crash-recovery path). Same managed signing as
  /// `executeIdentityTopUp`. The txid is entered in display order and
  /// reversed to raw wire order here, matching `OutPointFFI.txid` (same
  /// convention as `CreateIdentityView.parseOutPointHex`).
  @MainActor
  private func executeIdentityTopUpResume(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
      throw SDKError.invalidParameter("No identity selected")
    }
    guard let walletId = ownerIdentity.wallet?.walletId,
          let wallet = walletManager.wallet(for: walletId) else {
      throw SDKError.invalidParameter(
        "Identity has no wallet linkage; cannot resume the top-up"
      )
    }
    let txidHex = (formInputs["outPointTxid"] ?? "").trimmingCharacters(in: .whitespaces)
    guard txidHex.count == 64, let txidForward = Data(hexString: txidHex) else {
      throw SDKError.invalidParameter("Asset lock txid must be 64 hex characters (32 bytes)")
    }
    let txidRaw = Data(txidForward.reversed())
    let vout = UInt32(
      formInputs["outPointVout"]?.trimmingCharacters(in: .whitespaces) ?? "0"
    ) ?? 0

    do {
      let newBalance = try await wallet.resumeTopUpWithAssetLock(
        identityId: ownerIdentity.identityId,
        outPointTxid: txidRaw,
        outPointVout: vout
      )
      PersistentIdentity.updateBalance(
        in: modelContext, identityId: ownerIdentity.identityId, balance: newBalance
      )
      try? modelContext.save()
      return [
        "identityId": ownerIdentity.identityIdBase58,
        "newBalance": newBalance,
        "message": "Stuck top-up recovered successfully",
      ]
    } catch {
      // Classify the opaque "already consumed" consensus rejection into a
      // friendly message (mirrors the DIP-15 reclaim classifier) rather
      // than surfacing the raw SDK error.
      let desc = String(describing: error).lowercased()
      if (desc.contains("already") && desc.contains("consumed"))
        || desc.contains("already completely used") {
        throw SDKError.invalidParameter("Asset lock already consumed — nothing to resume")
      }
      throw error
    }
  }

  /// Generic-builder IdentityUpdate handler.
  ///
  /// Mirrors `AddIdentityKeyView`: the keys-to-add are *derived*
  /// against the owning wallet (the app never accepts raw key bytes
  /// from the form, because a key whose private scalar the app
  /// doesn't hold couldn't be signed with later), pre-persisted to
  /// the iOS Keychain, then submitted via the same
  /// `wallet.updateIdentity(...)` entry point — which also carries
  /// the key IDs to disable. The shared derive → validate → persist →
  /// build-`IdentityPubkey` plumbing lives in
  /// `IdentityKeyAddition.prepareKeys(...)`.
  ///
  /// Form inputs (`identityUpdate` in StateTransitionDefinitions):
  ///   - `addPublicKeys`: JSON array of `{ keyType, purpose,
  ///     securityLevel? }` rows (DPP token strings such as
  ///     `"ECDSA_HASH160"` / `"AUTHENTICATION"`). Any `data` field is
  ///     ignored — the keypair is derived Rust-side. `keyId` slots are
  ///     auto-assigned as `max(existing) + 1`.
  ///   - `disablePublicKeys`: comma-separated existing key IDs.
  ///
  /// `@MainActor` because the derive + Keychain-persist step
  /// (`IdentityKeyAddition.prepareKeys`) is main-actor-bound, matching
  /// `AddIdentityKeyView.submit()`.
  @MainActor
  private func executeIdentityUpdate(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
      throw SDKError.invalidParameter("No identity selected")
    }

    // Resolve the wallet that owns this identity — same lookup as
    // executeDataContractCreate. IdentityUpdate routes through
    // platform-wallet's updateIdentity(...) so derivation + signing
    // + broadcast all happen Rust-side.
    guard let walletId = ownerIdentity.wallet?.walletId,
          let wallet = walletManager.wallet(for: walletId) else {
      throw SDKError.invalidParameter(
        "Identity has no wallet linkage; cannot derive keys or sign the update"
      )
    }

    // Parse the keys-to-add JSON array (optional).
    var keySpecs: [IdentityKeyAddition.KeySpec] = []
    if let addJson = formInputs["addPublicKeys"],
       !addJson.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      guard let data = addJson.data(using: .utf8),
            let rows = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
        throw SDKError.serializationError(
          "Keys to add must be a JSON array of objects"
        )
      }
      keySpecs = try rows.map { row in
        guard let keyTypeStr = row["keyType"] as? String,
              let keyType = KeyType(dppToken: keyTypeStr) else {
          throw SDKError.invalidParameter(
            "Each key needs a valid `keyType` (e.g. ECDSA_SECP256K1, ECDSA_HASH160)"
          )
        }
        guard let purposeStr = row["purpose"] as? String,
              let purpose = KeyPurpose(dppToken: purposeStr) else {
          throw SDKError.invalidParameter(
            "Each key needs a valid `purpose` (e.g. AUTHENTICATION, TRANSFER)"
          )
        }
        // Fail fast — BEFORE any derivation or Keychain write — on key
        // types / purposes this generic path can't safely add. Mirrors
        // AddIdentityKeyView's gating so we never derive a secp256k1
        // pubkey for a BIP13/EdDSA hash payload, never orphan key
        // material in the Keychain for an ENCRYPTION/DECRYPTION key the
        // JSON parser can't attach contractBounds to, and never submit a
        // SYSTEM/VOTING/OWNER purpose DPP forbids on externally-added
        // keys. prepareKeys derives + writes the Keychain per spec, so
        // the guard has to sit here, ahead of it.
        switch keyType {
        case .ecdsaSecp256k1, .ecdsaHash160:
          break
        case .bls12_381, .bip13ScriptHash, .eddsa25519Hash160:
          throw SDKError.invalidParameter(
            "Key type \(keyTypeStr) is not supported by this flow; "
              + "use ECDSA_SECP256K1 or ECDSA_HASH160"
          )
        }
        switch purpose {
        case .authentication, .transfer:
          break
        case .encryption, .decryption:
          throw SDKError.invalidParameter(
            "Purpose \(purposeStr) requires contract bounds, which this "
              + "generic builder can't supply; use AUTHENTICATION or TRANSFER"
          )
        case .system, .voting, .owner:
          throw SDKError.invalidParameter(
            "Purpose \(purposeStr) cannot be added to an identity here; "
              + "use AUTHENTICATION or TRANSFER"
          )
        }
        // securityLevel is optional in the form JSON; derive a sane
        // protocol-locked default per purpose when absent, matching
        // AddIdentityKeyView's effectiveSecurityLevel.
        let securityLevel: SecurityLevel
        if let secStr = row["securityLevel"] as? String {
          guard let parsed = SecurityLevel(dppToken: secStr) else {
            throw SDKError.invalidParameter(
              "Invalid `securityLevel` (use MASTER, CRITICAL, HIGH, or MEDIUM)"
            )
          }
          securityLevel = parsed
        } else {
          switch purpose {
          case .transfer: securityLevel = .critical
          case .encryption, .decryption: securityLevel = .medium
          default: securityLevel = .high
          }
        }
        return IdentityKeyAddition.KeySpec(
          keyType: keyType,
          purpose: purpose,
          securityLevel: securityLevel
        )
      }
    }

    // Parse the comma-separated key IDs to disable (optional).
    var disableIds: [UInt32] = []
    if let disableStr = formInputs["disablePublicKeys"],
       !disableStr.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      disableIds = try disableStr
        .split(separator: ",")
        .map { $0.trimmingCharacters(in: .whitespaces) }
        .filter { !$0.isEmpty }
        .map { token in
          guard let id = UInt32(token) else {
            throw SDKError.invalidParameter(
              "Invalid key ID to disable: '\(token)'"
            )
          }
          return id
        }
    }

    guard !keySpecs.isEmpty || !disableIds.isEmpty else {
      throw SDKError.invalidParameter(
        "Provide at least one key to add or one key ID to disable"
      )
    }

    let network = appState.sdk?.network ?? appState.currentNetwork

    // Derive + validate + Keychain-persist the new keys, then build
    // their IdentityPubkey rows (no broadcast yet). Mirrors the exact
    // sequence AddIdentityKeyView runs. Runs inline on the main actor
    // (this handler is @MainActor).
    let addPublicKeys = try IdentityKeyAddition.prepareKeys(
      specs: keySpecs,
      identity: ownerIdentity,
      wallet: wallet,
      walletId: walletId,
      network: network
    )

    // Submit the IdentityUpdate. Rust signs with the identity's
    // MASTER auth key via the trampoline and broadcasts; the
    // persister callback writes the new PersistentPublicKey rows when
    // the transition lands on-chain.
    let signer = KeychainSigner(modelContainer: modelContext.container)
    try await wallet.updateIdentity(
      identityId: ownerIdentity.identityId,
      addPublicKeys: addPublicKeys,
      disablePublicKeyIds: disableIds,
      signer: signer
    )
    _ = signer  // keepalive: see KeychainSigner lifetime contract.

    return [
      "success": true,
      "identityId": ownerIdentity.identityIdBase58,
      "addedKeyIds": addPublicKeys.map { $0.keyId },
      "disabledKeyIds": disableIds,
      "message": "Identity update broadcast successfully",
    ]
  }

  private func executeIdentityCreditTransfer(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let fromIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: fromIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: fromIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: fromIdentity.balance),
      revision: 0
    )

    // Pick the transfer key (no bytes extraction); KeychainSigner
    // services the actual signature on demand.
    let transferKey = try await MainActor.run { () throws -> IdentityPublicKey in
      let km = KeyManager.withSharedKeychain()
      guard let k = km.findSigningKey(
        for: dppIdentity,
        purpose: .transfer,
        minimumSecurityLevel: nil,
        preferCritical: true
      ) else {
        throw KeyManagerError.noSuitableKey("No transfer key found for identity")
      }
      return k
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    print("🔑 Using transfer key #\(transferKey.id)")

    let (senderBalance, receiverBalance) = try await sdk.transferCredits(
      from: dppIdentity,
      toIdentityId: normalizedToIdentityId,
      amount: amount,
      signer: signer.handle
    )
    _ = signer  // keepalive

    // Update sender's balance in our local state
    await MainActor.run {
      PersistentIdentity.updateBalance(in: modelContext, identityId: fromIdentity.identityId, balance: senderBalance); try? modelContext.save()
    }

    return [
      "senderIdentityId": fromIdentity.identityIdBase58,
      "senderBalance": senderBalance,
      "receiverIdentityId": normalizedToIdentityId,
      "receiverBalance": receiverBalance,
      "transferAmount": amount,
      "message": "Credits transferred successfully"
    ]
  }

  private func executeIdentityCreditWithdrawal(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    // External-signer pattern. The trampoline picks the transfer
    // key off the identity at sign time.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let newBalance = try await sdk.withdrawFromIdentity(
      dppIdentity,
      amount: amount,
      toAddress: toAddress,
      coreFeePerByte: coreFeePerByte,
      signer: signer.handle
    )
    _ = signer  // keepalive

    // Update identity's balance in our local state
    await MainActor.run {
      PersistentIdentity.updateBalance(in: modelContext, identityId: identity.identityId, balance: newBalance); try? modelContext.save()
    }

    return [
      "identityId": identity.identityIdBase58,
      "withdrawnAmount": amount,
      "toAddress": toAddress,
      "coreFeePerByte": coreFeePerByte,
      "newBalance": newBalance,
      "message": "Credits withdrawn successfully"
    ]
  }

  private func executeDocumentCreate(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
    if let persistentContract = try? modelContext.fetch(descriptor).first,
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
      id: ownerIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: ownerIdentity.balance),
      revision: 0
    )

    // Pick an authentication key meeting the document-type's required
    // security level (no bytes extraction); the KeychainSigner
    // trampoline pulls private bytes from Keychain on demand at
    // sign time.
    let selectedKey = try await MainActor.run { () throws -> IdentityPublicKey in
      let km = KeyManager.withSharedKeychain()
      guard let k = km.findSigningKey(
        for: dppIdentity,
        purpose: .authentication,
        minimumSecurityLevel: requiredSecurityLevel,
        preferCritical: true
      ) else {
        throw KeyManagerError.noSuitableKey(
          "No authentication key with security \(requiredSecurityLevel.name) or higher and available private key"
        )
      }
      return k
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    print("🔑 Selected signing key: ID: \(selectedKey.id), Purpose: \(selectedKey.purpose.name), Security: \(selectedKey.securityLevel.name)")

    let result = try await sdk.documentCreate(
      contractId: contractId,
      documentType: documentType,
      ownerIdentity: dppIdentity,
      properties: properties,
      signer: signer.handle
    )
    _ = signer  // keepalive

    return result
  }

  private func executeDocumentDelete(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: ownerIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: ownerIdentity.balance),
      revision: 0
    )

    // External-signer pattern (see executeDataContractCreate for the
    // architectural rationale). Rust calls back through the
    // `KeychainSigner` trampoline when it needs a signature.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    // Call the document delete function
    try await sdk.documentDelete(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      ownerIdentity: dppIdentity,
      signer: signer.handle
    )
    _ = signer  // keepalive across the await — see KeychainSigner lifetime contract

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
    guard let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
      throw SDKError.invalidParameter("Selected identity not found")
    }

    // Use the DPPIdentity
    let fromIdentity = DPPIdentity(
      id: ownerIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: ownerIdentity.balance),
      revision: 0
    )

    // External-signer pattern.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    // Call the document transfer function
    let result = try await sdk.documentTransfer(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      fromIdentity: fromIdentity,
      toIdentityId: recipientId,
      signer: signer.handle
    )
    _ = signer  // keepalive

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
    guard let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
      throw SDKError.invalidParameter("Selected identity not found")
    }

    // Use the DPPIdentity
    let ownerDPPIdentity = DPPIdentity(
      id: ownerIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: ownerIdentity.balance),
      revision: 0
    )

    // External-signer pattern.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    // Call the document update price function
    let result = try await sdk.documentUpdatePrice(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      newPrice: newPrice,
      ownerIdentity: ownerDPPIdentity,
      signer: signer.handle
    )
    _ = signer  // keepalive

    return result
  }

  private func executeDocumentPurchase(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let purchaserIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
    if let error = transitionState.documentPurchaseError {
      throw SDKError.invalidParameter(error)
    }

    // Get the price that was fetched by DocumentWithPriceView
    guard let price = transitionState.documentPrice else {
      throw SDKError.invalidParameter("Document price not available. Please enter a valid document ID to fetch its price.")
    }

    // Validate that the document is actually for sale (price > 0)
    if price == 0 {
      throw SDKError.invalidParameter("This document is not for sale")
    }

    // Use the DPPIdentity
    let fromIdentity = DPPIdentity(
      id: purchaserIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: purchaserIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: purchaserIdentity.balance),
      revision: 0
    )

    // External-signer pattern.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    // Call the document purchase function
    let result = try await sdk.documentPurchase(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      purchaserIdentity: fromIdentity,
      price: price,
      signer: signer.handle
    )
    _ = signer  // keepalive

    return result
  }

  private func executeDocumentReplace(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
    if let persistentContract = try? modelContext.fetch(descriptor).first,
       let documentTypes = persistentContract.documentTypes,
       let docType = documentTypes.first(where: { $0.name == documentType }) {
      requiredSecurityLevel = SecurityLevel(rawValue: UInt8(docType.securityLevel)) ?? .high
      print("📋 Document type '\(documentType)' requires security level: \(requiredSecurityLevel.name)")
    } else {
      print("⚠️ Could not determine security level for document type '\(documentType)', using default: HIGH")
    }

    // Find a key for signing - must meet security requirements
    print("🔑 Available keys for identity:")
    for key in ownerIdentity.identityPublicKeys {
      print("  - ID: \(key.id), Purpose: \(key.purpose.name), Security: \(key.securityLevel.name), Disabled: \(key.isDisabled)")
    }

    // Use the DPPIdentity for document replacement
    let dppIdentity = DPPIdentity(
      id: ownerIdentity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: ownerIdentity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: ownerIdentity.balance),
      revision: 0
    )

    // Pick an authentication key meeting the required security
    // level (no bytes extraction). Sign-time bytes come through
    // the KeychainSigner trampoline.
    let selectedKey = try await MainActor.run { () throws -> IdentityPublicKey in
      let km = KeyManager.withSharedKeychain()
      guard let k = km.findSigningKey(
        for: dppIdentity,
        purpose: .authentication,
        minimumSecurityLevel: requiredSecurityLevel,
        preferCritical: true
      ) else {
        throw KeyManagerError.noSuitableKey(
          "No authentication key with security \(requiredSecurityLevel.name) or higher and available private key"
        )
      }
      return k
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    print("🔑 Selected signing key: ID: \(selectedKey.id), Purpose: \(selectedKey.purpose.name), Security: \(selectedKey.securityLevel.name)")

    let result = try await sdk.documentReplace(
      contractId: contractId,
      documentType: documentType,
      documentId: documentId,
      ownerIdentity: dppIdentity,
      properties: properties,
      signer: signer.handle
    )
    _ = signer  // keepalive

    return result
  }

  /// Resolve the selected persisted token and apply its protocol decimal
  /// scale. This keeps the generic transition forms aligned with the
  /// dedicated token screens and avoids Double rounding / hard-coded 8s.
  private func parseTransitionTokenAmount(
    _ text: String,
    selection: String
  ) throws -> UInt64 {
    let components = selection.split(separator: ":")
    guard components.count == 2,
          let position = Int(components[1]),
          let token = dataContracts
            .compactMap(\.tokens)
            .flatMap({ $0 })
            .first(where: {
              $0.contractIdBase58 == String(components[0]) && $0.position == position
            }),
          let amount = parseTokenAmount(text, decimals: token.decimals) else {
      throw SDKError.invalidParameter("Invalid token amount or token metadata")
    }
    return amount
  }

  private func executeTokenMint(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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

    let amount = try parseTransitionTokenAmount(amountString, selection: tokenSelection)

    // Find the minting key - for tokens, we need a critical security level key
    // Use the DPPIdentity for minting
    let dppIdentity = DPPIdentity(
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    // Pick a critical key (owner first, then authentication) without
    // extracting private bytes — `KeychainSigner` does the
    // bytes-out-of-keychain work later, only when Rust calls back
    // for a signature. The id from the picked key is still needed
    // up-front for `tokenMint`'s `keyId:` parameter.
    let mintingKey = try await MainActor.run { () throws -> IdentityPublicKey in
      let km = KeyManager.withSharedKeychain()
      if let owner = km.findSigningKey(
        for: dppIdentity,
        purpose: .owner,
        minimumSecurityLevel: .critical,
        preferCritical: true
      ) {
        return owner
      }
      guard let auth = km.findSigningKey(
        for: dppIdentity,
        purpose: .authentication,
        minimumSecurityLevel: .critical,
        preferCritical: true
      ) else {
        throw KeyManagerError.noSuitableKey(
          "No critical owner or authentication key with available private key"
        )
      }
      return auth
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    print("🔑 TOKEN MINT: Selected key \(mintingKey.id) with purpose \(mintingKey.purpose) and security level \(mintingKey.securityLevel)")

    let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil

    let result = try await sdk.tokenMint(
      contractId: contractId,
      recipientId: recipientIdString,
      amount: amount,
      ownerIdentity: dppIdentity,
      keyId: mintingKey.id,
      signer: signer.handle,
      note: note
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenBurn(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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

    let amount = try parseTransitionTokenAmount(amountString, selection: tokenSelection)

    // Use the DPPIdentity for burning
    let dppIdentity = DPPIdentity(
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    // Owner-then-authentication critical key, no bytes extraction.
    // The trampoline pulls private material on demand at sign time.
    let burningKey = try await MainActor.run { () throws -> IdentityPublicKey in
      let km = KeyManager.withSharedKeychain()
      if let owner = km.findSigningKey(
        for: dppIdentity,
        purpose: .owner,
        minimumSecurityLevel: .critical,
        preferCritical: true
      ) {
        return owner
      }
      guard let auth = km.findSigningKey(
        for: dppIdentity,
        purpose: .authentication,
        minimumSecurityLevel: .critical,
        preferCritical: true
      ) else {
        throw KeyManagerError.noSuitableKey(
          "No critical owner or authentication key with available private key"
        )
      }
      return auth
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil

    let result = try await sdk.tokenBurn(
      contractId: contractId,
      amount: amount,
      ownerIdentity: dppIdentity,
      keyId: burningKey.id,
      signer: signer.handle,
      note: note
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenFreeze(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    // Pick the freezing key (no bytes extraction); construct a
    // KeychainSigner that pulls private material from Keychain on
    // demand at sign time.
    let freezingKey = try await MainActor.run {
      try findTokenOperationKey(for: dppIdentity)
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil

    let result = try await sdk.tokenFreeze(
      contractId: contractId,
      targetIdentityId: targetIdentityId,
      ownerIdentity: dppIdentity,
      keyId: freezingKey.id,
      signer: signer.handle,
      note: note
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenUnfreeze(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    let unfreezingKey = try await MainActor.run {
      try findTokenOperationKey(for: dppIdentity)
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let result = try await sdk.tokenUnfreeze(
      contractId: contractId,
      targetIdentityId: targetIdentityId,
      ownerIdentity: dppIdentity,
      keyId: unfreezingKey.id,
      signer: signer.handle,
      note: formInputs["note"]
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenDestroyFrozenFunds(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    let destroyKey = try await MainActor.run {
      try findTokenOperationKey(for: dppIdentity)
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let result = try await sdk.tokenDestroyFrozenFunds(
      contractId: contractId,
      frozenIdentityId: frozenIdentityId,
      ownerIdentity: dppIdentity,
      keyId: destroyKey.id,
      signer: signer.handle,
      note: formInputs["note"]
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenClaim(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    let claimingKey = try await MainActor.run {
      try findTokenOperationKey(for: dppIdentity)
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil

    let result = try await sdk.tokenClaim(
      contractId: contractId,
      distributionType: distributionType,
      ownerIdentity: dppIdentity,
      keyId: claimingKey.id,
      signer: signer.handle,
      note: note
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenTransfer(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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

    let amount = try parseTransitionTokenAmount(amountString, selection: tokenSelection)

    // Use the DPPIdentity for transfer
    let dppIdentity = DPPIdentity(
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    let transferKey = try await MainActor.run {
      try findTokenOperationKey(for: dppIdentity)
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let note = formInputs["note"]?.isEmpty == false ? formInputs["note"] : nil

    let result = try await sdk.tokenTransfer(
      contractId: contractId,
      recipientId: recipientId,
      amount: amount,
      ownerIdentity: dppIdentity,
      keyId: transferKey.id,
      signer: signer.handle,
      note: note
    )
    _ = signer  // keepalive

    return result
  }

  private func executeTokenSetPrice(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let identity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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
      id: identity.identityId,
      publicKeys: Dictionary(uniqueKeysWithValues: identity.identityPublicKeys.map { ($0.id, $0) }),
      balance: UInt64(bitPattern: identity.balance),
      revision: 0
    )

    let pricingKey = try await MainActor.run {
      try findTokenOperationKey(for: dppIdentity)
    }
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let note = formInputs["publicNote"]?.isEmpty == false ? formInputs["publicNote"] : nil

    let result = try await sdk.tokenSetPrice(
      contractId: contractId,
      pricingType: priceType,
      priceData: priceData,
      ownerIdentity: dppIdentity,
      keyId: pricingKey.id,
      signer: signer.handle,
      note: note
    )
    _ = signer  // keepalive

    return result
  }

  private func executeDataContractCreate(sdk: SDK) async throws -> Any {
    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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

    // Parse groups if provided. `RegisterContractSourceView` flattens
    // the chain-shape `{ "<position>": { ... } }` map into an array
    // with an injected `id`, so the form carries an array. The Rust
    // V1 contract format wants a `BTreeMap<GroupContractPosition,
    // Group>` though, so re-key the array back into a position-keyed
    // map before crossing the FFI — otherwise the assembler rejects
    // it with "expected a map, got sequence".
    var groups: [String: Any]? = nil
    if let groupsJson = formInputs["groups"], !groupsJson.isEmpty {
      guard let data = groupsJson.data(using: .utf8) else {
        throw SDKError.serializationError("Invalid groups JSON")
      }
      // `RegisterContractSourceView` flattens the chain-shape map
      // into an array with an injected `id`. Other callers of this
      // transition builder (manual JSON paste, future refactors)
      // may pass the canonical position-keyed map directly. Accept
      // either, and let genuine `JSONSerialization` errors surface
      // instead of getting collapsed into the same "Invalid groups
      // JSON" as a shape mismatch.
      let parsed = try JSONSerialization.jsonObject(with: data)
      switch parsed {
      case let byPosition as [String: Any]:
        // Canonical chain shape — already what the FFI assembler
        // wants, no re-keying needed.
        groups = byPosition
      case let entries as [[String: Any]]:
        // Form-flattened shape from `RegisterContractSourceView`.
        // The injected `id` is Int when the source map's key
        // parsed as integer, String otherwise; entries derive from
        // a map so keys are unique by construction. The three
        // failure modes below are therefore unreachable from the
        // current upstream — but silently dropping or overwriting
        // entries here would corrupt the contract submission and
        // surface as a confusing chain rejection rather than a
        // clear registration error, so trip loudly with `SDKError`
        // on each.
        var byPosition: [String: Any] = [:]
        for var entry in entries {
          guard let rawId = entry.removeValue(forKey: "id") else {
            throw SDKError.serializationError("Group entry is missing an `id` field")
          }
          let key: String
          if let intId = rawId as? Int {
            key = String(intId)
          } else if let strId = rawId as? String {
            key = strId
          } else {
            throw SDKError.serializationError("Group entry has unsupported `id` type — expected Int or String")
          }
          guard byPosition[key] == nil else {
            throw SDKError.serializationError("Duplicate group position `\(key)` in groups payload")
          }
          byPosition[key] = entry
        }
        groups = byPosition
      default:
        throw SDKError.serializationError("Invalid groups JSON")
      }
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

    // Optional contract metadata. Both fields live at the top of
    // the V1 serialized contract format, NOT inside `config`, so we
    // pass them as siblings to `contractConfig` rather than nesting
    // them. The Rust FFI maps each to its own JSON parameter.
    let contractKeywords: [String]? = {
      guard let raw = formInputs["keywords"], !raw.isEmpty else { return nil }
      return raw.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
    }()
    let contractDescription: String? = {
      guard let raw = formInputs["description"], !raw.isEmpty else { return nil }
      return raw
    }()

    // Validate that at least one schema is provided
    if documentSchemas == nil && tokenSchemas == nil {
      throw SDKError.invalidParameter("At least one document schema or token schema must be provided")
    }

    // Resolve the wallet that owns this identity — contract create
    // now goes through `platform-wallet`'s
    // `createDataContract(...)` (instead of the old
    // `sdk.dataContractCreate(...)` rs-sdk-ffi path). The
    // platform-wallet runtime configures an 8 MB worker stack so
    // the post-broadcast GroveDB proof recursion doesn't blow the
    // iOS default 512 KB thread stack like it did under rs-sdk-ffi.
    guard let walletId = ownerIdentity.wallet?.walletId,
          let wallet = walletManager.wallet(for: walletId) else {
      throw SDKError.invalidParameter(
        "Identity has no wallet linkage; cannot sign contract create"
      )
    }

    // Re-serialize the parsed dicts back to JSON strings — the
    // wallet-side wrapper takes JSON strings (the V1 contract
    // format builder lives in `rs-platform-wallet` and assembles
    // a `serde_json::Map` internally). The form's raw inputs
    // would also work, but going through the parsed dicts catches
    // malformed JSON before it crosses the FFI.
    let documentSchemasJSON = try toJSONString(
      documentSchemas as Any?,
      fieldName: "documentSchemas",
      defaultIfNil: "{}"
    ) ?? "{}"
    let tokenSchemasJSON = try toJSONString(tokenSchemas as Any?, fieldName: "tokenSchemas")
    let groupsJSON = try toJSONString(groups as Any?, fieldName: "groups")
    let keywordsJSON: String? = try {
      guard let keywords = contractKeywords, !keywords.isEmpty else { return nil }
      return try toJSONString(keywords, fieldName: "keywords")
    }()
    let configJSON = try toJSONString(
      contractConfig.isEmpty ? nil : contractConfig,
      fieldName: "contractConfig"
    )

    // External-signer pattern (matches identity flows after #3541):
    // Rust calls back into Swift over the `KeychainSigner`
    // trampoline whenever it needs a signature.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let contractId = try await wallet.createDataContract(
      ownerIdentityId: ownerIdentity.identityId,
      documentSchemasJSON: documentSchemasJSON,
      tokenSchemasJSON: tokenSchemasJSON,
      groupsJSON: groupsJSON,
      keywordsJSON: keywordsJSON,
      description: contractDescription,
      contractConfigJSON: configJSON,
      signer: signer
    )
    // Keepalive: see KeychainSigner lifetime contract.
    _ = signer

    return [
      "success": true,
      "contractId": contractId.toBase58String(),
      "message": "Data contract created and broadcast successfully",
    ]
  }

  /// Helper: turn a Swift Foundation value into a JSON string the
  /// FFI accepts. `nil` input returns `defaultIfNil` (typically
  /// `nil` for optional FFI params). Throws
  /// `SDKError.serializationError` on encoding failure.
  private func toJSONString(
    _ value: Any?,
    fieldName: String,
    defaultIfNil: String? = nil
  ) throws -> String? {
    guard let value = value else { return defaultIfNil }
    guard JSONSerialization.isValidJSONObject(value) else {
      throw SDKError.serializationError("\(fieldName) is not JSON-serializable")
    }
    guard let data = try? JSONSerialization.data(withJSONObject: value),
          let str = String(data: data, encoding: .utf8) else {
      throw SDKError.serializationError("Failed to serialize \(fieldName)")
    }
    return str
  }

  private func executeDataContractUpdate(sdk: SDK) async throws -> Any {
    guard let contractId = formInputs["dataContractId"], !contractId.isEmpty else {
      throw SDKError.invalidParameter("Data contract ID is required")
    }

    guard !selectedIdentityId.isEmpty,
          let ownerIdentity = identities.first(where: { $0.identityIdBase58 == selectedIdentityId }) else {
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

    // Parse new groups if provided. As with contract create,
    // `RegisterContractSourceView` flattens the chain-shape
    // `{ "<position>": { ... } }` map into an array with an
    // injected `id`. The V1 contract format wants a position-keyed
    // map, so re-key the array back before crossing the FFI.
    var newGroups: [String: Any]? = nil
    if let groupsJson = formInputs["newGroups"], !groupsJson.isEmpty {
      guard let data = groupsJson.data(using: .utf8) else {
        throw SDKError.serializationError("Invalid groups JSON")
      }
      let parsed = try JSONSerialization.jsonObject(with: data)
      switch parsed {
      case let byPosition as [String: Any]:
        newGroups = byPosition
      case let entries as [[String: Any]]:
        var byPosition: [String: Any] = [:]
        for var entry in entries {
          guard let rawId = entry.removeValue(forKey: "id") else {
            throw SDKError.serializationError("Group entry is missing an `id` field")
          }
          let key: String
          if let intId = rawId as? Int {
            key = String(intId)
          } else if let strId = rawId as? String {
            key = strId
          } else {
            throw SDKError.serializationError("Group entry has unsupported `id` type — expected Int or String")
          }
          guard byPosition[key] == nil else {
            throw SDKError.serializationError("Duplicate group position `\(key)` in groups payload")
          }
          byPosition[key] = entry
        }
        newGroups = byPosition
      default:
        throw SDKError.serializationError("Invalid groups JSON")
      }
    }

    // Validate that at least one update is provided
    if newDocumentSchemas == nil && newTokenSchemas == nil && newGroups == nil {
      throw SDKError.invalidParameter("At least one update (document schemas, token schemas, or groups) must be provided")
    }

    // Resolve the wallet that owns this identity — contract update
    // mirrors contract create and goes through `platform-wallet`'s
    // `updateDataContract(...)`. The wallet fetches the live
    // contract, validates the owner, bumps its version, and *merges*
    // the supplied sections onto the fetched definition at the next
    // version (omitted sections — and any document/token/group entry
    // we don't pass — are preserved), then builds a
    // `DataContractUpdateTransition`, signs via the external
    // `KeychainSigner`, and broadcasts on the 8 MB-stack worker (same
    // rationale as create). Because the merge happens Rust-side, this
    // form only needs to forward the sections the user actually
    // changed; keywords / description / config are left to the
    // on-chain values.
    guard let walletId = ownerIdentity.wallet?.walletId,
          let wallet = walletManager.wallet(for: walletId) else {
      throw SDKError.invalidParameter(
        "Identity has no wallet linkage; cannot sign contract update"
      )
    }

    // Document schemas are merged onto the fetched contract Rust-side
    // (add or replace by type name; existing types are kept), so an
    // empty `{}` is a no-op overlay — the right default for a token- or
    // group-only update that touches no document types.
    let documentSchemasJSON = try toJSONString(
      newDocumentSchemas as Any?,
      fieldName: "newDocumentSchemas",
      defaultIfNil: "{}"
    ) ?? "{}"
    let tokenSchemasJSON = try toJSONString(newTokenSchemas as Any?, fieldName: "newTokenSchemas")
    let groupsJSON = try toJSONString(newGroups as Any?, fieldName: "newGroups")

    // Decode the base58 contract id the form provided.
    guard let contractIdData = Data.identifier(fromBase58: contractId),
          contractIdData.count == 32 else {
      throw SDKError.invalidParameter("Invalid data contract ID (expected 32-byte base58)")
    }

    // External-signer pattern (matches contract create): Rust calls
    // back into Swift over the `KeychainSigner` trampoline whenever
    // it needs a signature.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    let updatedContractId = try await wallet.updateDataContract(
      ownerIdentityId: ownerIdentity.identityId,
      contractId: contractIdData,
      documentSchemasJSON: documentSchemasJSON,
      tokenSchemasJSON: tokenSchemasJSON,
      groupsJSON: groupsJSON,
      signer: signer
    )
    // Keepalive: see KeychainSigner lifetime contract.
    _ = signer

    return [
      "success": true,
      "contractId": updatedContractId.toBase58String(),
      "message": "Data contract updated and broadcast successfully",
    ]
  }

  // MARK: - Helper Functions

  /// Pick a critical key on `identity` to drive a token operation.
  /// Tries owner purpose first, falls back to authentication —
  /// matches the legacy `createTokenOperationSigner` selection
  /// policy.
  ///
  /// Returns just the `IdentityPublicKey`, not a signer: the
  /// actual signing now happens via `KeychainSigner`'s callback
  /// trampoline at the call site, which fetches private bytes
  /// from Keychain on demand and zeroes them after each signature.
  /// Callers pass `key.id` to the SDK as `keyId:` and a freshly
  /// constructed `KeychainSigner.handle` as `signer:`.
  @MainActor
  private func findTokenOperationKey(for identity: DPPIdentity) throws -> IdentityPublicKey {
    let keyManager = KeyManager.withSharedKeychain()

    if let owner = keyManager.findSigningKey(
      for: identity,
      purpose: .owner,
      minimumSecurityLevel: .critical,
      preferCritical: true
    ) {
      return owner
    }
    guard let auth = keyManager.findSigningKey(
      for: identity,
      purpose: .authentication,
      minimumSecurityLevel: .critical,
      preferCritical: true
    ) else {
      throw KeyManagerError.noSuitableKey(
        "No critical owner or authentication key with available private key"
      )
    }
    return auth
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

// IdentityModel's `displayName` extension is gone — the same
// helper now lives on `PersistentIdentity` (see
// `Sources/SwiftDashSDK/Persistence/Models/PersistentIdentity.swift`).
