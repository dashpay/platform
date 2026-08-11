import SwiftUI
import SwiftData
import SwiftDashSDK

struct LoadIdentityView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    // Form inputs
    @State private var identityIdInput = ""
    @State private var selectedIdentityType: IdentityType = .user
    @State private var aliasInput = ""

    // Masternode/Evonode specific keys
    @State private var votingPrivateKeyInput = ""
    @State private var ownerPrivateKeyInput = ""
    @State private var payoutPrivateKeyInput = ""

    // User identity keys
    @State private var privateKeys: [String] = ["", "", ""]

    // Loading state
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showSuccess = false
    @State private var loadStartTime: Date?

    // Testnet nodes
    private let testnetNodes = TestnetNodesLoader.loadFromYAML()

    // Info popups
    @State private var showInfoPopup = false
    @State private var infoPopupMessage = ""

    var body: some View {
        NavigationView {
            if showSuccess {
                successView
            } else {
                formView
            }
        }
    }

    private var formView: some View {
        Form {
            if appState.sdk?.network == .testnet && testnetNodes != nil {
                Section {
                    HStack {
                        Button("Fill Random HPMN") {
                            fillRandomHPMN()
                        }
                        .buttonStyle(.bordered)

                        Button("Fill Random Masternode") {
                            fillRandomMasternode()
                        }
                        .buttonStyle(.bordered)
                    }
                }
            }

            Section("Identity Information") {
                VStack(alignment: .leading) {
                    Text("Identity ID / ProTxHash")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    TextField("Hex or Base58", text: $identityIdInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }

                Picker("Identity Type", selection: $selectedIdentityType) {
                    ForEach(IdentityType.allCases, id: \.self) { type in
                        Text(type.rawValue).tag(type)
                    }
                }
                .pickerStyle(SegmentedPickerStyle())

                HStack {
                    VStack(alignment: .leading) {
                        Text("Alias (optional)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        TextField("Display name", text: $aliasInput)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }

                    Button(action: {
                        infoPopupMessage = "Alias is optional. It is only used to help identify the identity in the app. It isn't saved to Dash Platform."
                        showInfoPopup = true
                    }) {
                        Image(systemName: "info.circle")
                            .foregroundColor(.blue)
                    }
                }
            }

            // Show appropriate key inputs based on identity type
            if selectedIdentityType == .masternode || selectedIdentityType == .evonode {
                masternodeKeyInputs
            } else {
                userKeyInputs
            }

            if let errorMessage = errorMessage {
                Section {
                    Text(errorMessage)
                        .foregroundColor(.red)
                }
            }

            Section {
                loadIdentityButton
            }
        }
        .navigationTitle("Load Identity")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarLeading) {
                Button("Cancel") {
                    dismiss()
                }
            }
        }
        .disabled(isLoading)
        .sheet(isPresented: $showInfoPopup) {
            InfoPopupView(message: infoPopupMessage)
        }
    }

    private var masternodeKeyInputs: some View {
        Section("Masternode Keys") {
            VStack(alignment: .leading) {
                Text("Voting Private Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                TextField("Hex or WIF", text: $votingPrivateKeyInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }

            VStack(alignment: .leading) {
                Text("Owner Private Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                TextField("Hex or WIF", text: $ownerPrivateKeyInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }

            if selectedIdentityType == .evonode {
                VStack(alignment: .leading) {
                    Text("Payout Address Private Key")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    TextField("Hex or WIF", text: $payoutPrivateKeyInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
            }
        }
    }

    private var userKeyInputs: some View {
        Section("Private Keys") {
            ForEach(privateKeys.indices, id: \.self) { index in
                HStack {
                    VStack(alignment: .leading) {
                        HStack {
                            Text("Private Key \(index + 1)")
                                .font(.caption)
                                .foregroundColor(.secondary)

                            Button(action: {
                                infoPopupMessage = "You don't need to add all or even any private keys here. Private keys can be added later. However, without private keys, you won't be able to sign any transactions."
                                showInfoPopup = true
                            }) {
                                Image(systemName: "info.circle")
                                    .font(.caption)
                                    .foregroundColor(.blue)
                            }
                        }

                        TextField("Hex or WIF", text: $privateKeys[index])
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }

                    if privateKeys.count > 1 {
                        Button(action: {
                            privateKeys.remove(at: index)
                        }) {
                            Image(systemName: "minus.circle.fill")
                                .foregroundColor(.red)
                        }
                    }
                }
            }

            Button(action: {
                privateKeys.append("")
            }) {
                Label("Add Key", systemImage: "plus.circle.fill")
            }
        }
    }

    private var loadIdentityButton: some View {
        Button(action: loadIdentity) {
            HStack {
                if isLoading {
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle())
                        .scaleEffect(0.8)
                } else {
                    Text("Load Identity")
                }

                if let startTime = loadStartTime {
                    let elapsed = Date().timeIntervalSince(startTime)
                    Text(formatElapsedTime(elapsed))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
        .disabled(identityIdInput.isEmpty || isLoading)
    }

    private var successView: some View {
        VStack(spacing: 20) {
            Spacer()

            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 80))
                .foregroundColor(.green)

            Text("Successfully loaded identity!")
                .font(.title2)
                .fontWeight(.semibold)

            VStack(spacing: 10) {
                Button("Load Another") {
                    resetForm()
                    showSuccess = false
                }
                .buttonStyle(.borderedProminent)

                Button("Back to Identities") {
                    dismiss()
                }
                .buttonStyle(.bordered)
            }

            Spacer()
        }
        .padding()
        .navigationTitle("Success")
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: - Actions

    private func loadIdentity() {
        errorMessage = nil
        isLoading = true
        loadStartTime = Date()

        Task {
            do {
                // Validate and convert identity ID to Data
                let trimmedId = identityIdInput.trimmingCharacters(in: .whitespacesAndNewlines)

                // Try hex first, then Base58
                var idData: Data?
                if let hexData = Data(hexString: trimmedId), hexData.count == 32 {
                    idData = hexData
                } else if let base58Data = Data.identifier(fromBase58: trimmedId), base58Data.count == 32 {
                    idData = base58Data
                }

                guard let validIdData = idData else {
                    await MainActor.run {
                        errorMessage = "Invalid identity ID. Must be a 64-character hex string or valid Base58 string."
                        isLoading = false
                        loadStartTime = nil
                    }
                    return
                }

                // Convert private key strings to Data
                let privateKeyData = privateKeys.compactMap { keyString -> Data? in
                    let trimmed = keyString.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !trimmed.isEmpty else { return nil }
                    return Data(hexString: trimmed)
                }

                let votingKeyData = votingPrivateKeyInput.isEmpty ? nil : Data(hexString: votingPrivateKeyInput.trimmingCharacters(in: .whitespacesAndNewlines))
                let ownerKeyData = ownerPrivateKeyInput.isEmpty ? nil : Data(hexString: ownerPrivateKeyInput.trimmingCharacters(in: .whitespacesAndNewlines))
                let payoutKeyData = payoutPrivateKeyInput.isEmpty ? nil : Data(hexString: payoutPrivateKeyInput.trimmingCharacters(in: .whitespacesAndNewlines))

                // Fetch the identity from the network to verify it exists
                guard let sdk = appState.sdk else {
                    await MainActor.run {
                        errorMessage = "SDK not initialized"
                        isLoading = false
                        loadStartTime = nil
                    }
                    return
                }

                // Try to fetch the identity
                let identityData = try await sdk.identityGet(identityId: validIdData.toHexString())

                // Debug: Print the entire identity data to see its structure
                print("🔵 Fetched identity data: \(identityData)")

                // Extract balance
                var fetchedBalance: UInt64 = 0
                if let balanceValue = identityData["balance"] {
                    if let balanceNum = balanceValue as? NSNumber {
                        fetchedBalance = balanceNum.uint64Value
                    } else if let balanceString = balanceValue as? String,
                              let balanceUInt = UInt64(balanceString) {
                        fetchedBalance = balanceUInt
                    }
                }

                // Extract public keys if available
                var parsedPublicKeys: [IdentityPublicKey] = []

                // Try different possible key names for public keys in the JSON
                // The publicKeys might be a dictionary with key IDs as keys
                if let publicKeysDict = identityData["publicKeys"] as? [String: Any] {
                    print("🔵 Public keys are in dictionary format")
                    parsedPublicKeys = publicKeysDict.compactMap { (keyIdStr, keyData) -> IdentityPublicKey? in
                        guard let keyData = keyData as? [String: Any],
                              let id = Int(keyIdStr) ?? keyData["id"] as? Int,
                              let purpose = keyData["purpose"] as? Int,
                              let securityLevel = keyData["securityLevel"] as? Int,
                              let keyType = keyData["type"] as? Int,
                              let dataStr = keyData["data"] as? String else {
                            print("❌ Failed to parse key with ID: \(keyIdStr), data: \(keyData)")
                            return nil
                        }

                        // Data is in Base64 format, not hex
                        guard let data = Data(base64Encoded: dataStr) else {
                            print("❌ Failed to decode Base64 data for key \(id)")
                            return nil
                        }

                        let readOnly = keyData["readOnly"] as? Bool ?? false
                        let disabledAt = keyData["disabledAt"] as? UInt64

                        return IdentityPublicKey(
                            id: UInt32(id),
                            purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                            securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                            contractBounds: nil,
                            keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                            readOnly: readOnly,
                            data: data,
                            disabledAt: disabledAt
                        )
                    }
                } else if let publicKeysArray = identityData["publicKeys"] as? [[String: Any]] {
                    print("🔵 Public keys are in array format")
                    parsedPublicKeys = publicKeysArray.compactMap { keyData -> IdentityPublicKey? in
                        guard let id = keyData["id"] as? Int,
                              let purpose = keyData["purpose"] as? Int,
                              let securityLevel = keyData["securityLevel"] as? Int,
                              let keyType = keyData["type"] as? Int,
                              let dataStr = keyData["data"] as? String else {
                            print("❌ Failed to parse key data: \(keyData)")
                            return nil
                        }

                        // Data is in Base64 format, not hex
                        guard let data = Data(base64Encoded: dataStr) else {
                            print("❌ Failed to decode Base64 data for key \(id)")
                            return nil
                        }

                        let readOnly = keyData["readOnly"] as? Bool ?? false
                        let disabledAt = keyData["disabledAt"] as? UInt64

                        return IdentityPublicKey(
                            id: UInt32(id),
                            purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                            securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                            contractBounds: nil,
                            keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                            readOnly: readOnly,
                            data: data,
                            disabledAt: disabledAt
                        )
                    }
                } else {
                    print("❌ Public keys not found in identity data")
                }

                // Persist the fetched identity directly via SwiftData.
                // Private keys are stored in the keychain and linked
                // to the matching `PersistentPublicKey`s below.
                let trimmedAlias = aliasInput.isEmpty ? nil : aliasInput
                let identityType = selectedIdentityType
                let network = appState.currentNetwork
                let capturedPublicKeys = parsedPublicKeys
                let capturedPrivateKeys = privateKeyData
                let capturedVotingKey = votingKeyData
                let capturedOwnerKey = ownerKeyData
                let capturedPayoutKey = payoutKeyData

                await MainActor.run {
                    // Upsert the PersistentIdentity row.
                    let existing = PersistentIdentity.fetch(
                        in: modelContext,
                        identityId: validIdData
                    )
                    let row: PersistentIdentity
                    if let existing = existing {
                        existing.balance = Int64(bitPattern: fetchedBalance)
                        existing.alias = trimmedAlias
                        // Ownership travels on the `wallet`
                        // relationship, which this by-id load never
                        // touches — an existing row may be
                        // wallet-owned (the user can paste their own
                        // identity id) and keeps that linkage.
                        existing.identityType = identityType.rawValue
                        existing.network = network
                        existing.lastUpdated = Date()
                        // Replace public keys wholesale with the
                        // freshly fetched set.
                        existing.publicKeys.removeAll()
                        row = existing
                    } else {
                        row = PersistentIdentity(
                            identityId: validIdData,
                            balance: Int64(bitPattern: fetchedBalance),
                            revision: 0,
                            alias: trimmedAlias,
                            dpnsName: nil,
                            mainDpnsName: nil,
                            identityType: identityType,
                            network: network
                        )
                        modelContext.insert(row)
                        // Back-fill any locally-cached contracts that
                        // name this identity as their owner. The
                        // existing `try? modelContext.save()` later
                        // in this block persists the link.
                        ContractIdentityLinker.linkIdentityToOwnedContracts(
                            identity: row,
                            modelContext: modelContext
                        )
                    }

                    // Insert PersistentPublicKey rows for every key
                    // the network returned.
                    for publicKey in capturedPublicKeys {
                        if let persistentKey = PersistentPublicKey.from(
                            publicKey,
                            identityId: validIdData.toHexString()
                        ) {
                            row.addPublicKey(persistentKey)
                        }
                    }

                    // Match each user-provided private key to its
                    // public key and stash it in the keychain.
                    for privateKey in capturedPrivateKeys {
                        if let matched = KeyValidation.matchPrivateKeyToPublicKeys(
                            privateKeyData: privateKey,
                            publicKeys: capturedPublicKeys,
                            network: network
                        ), let persistentKey = row.publicKeys.first(
                            where: { $0.keyId == Int32(matched.id) }
                        ), let keychainId = KeychainManager.shared.storePrivateKey(
                            privateKey,
                            identityId: validIdData,
                            keyIndex: persistentKey.keyId
                        ) {
                            persistentKey.privateKeyKeychainIdentifier = keychainId
                        }
                    }

                    // Masternode-/evonode-specific keys go to the
                    // keychain under their dedicated slots.
                    if let votingKey = capturedVotingKey {
                        row.votingPrivateKeyIdentifier = KeychainManager.shared.storeSpecialKey(
                            votingKey,
                            identityId: validIdData,
                            keyType: .voting
                        )
                    }
                    if let ownerKey = capturedOwnerKey {
                        row.ownerPrivateKeyIdentifier = KeychainManager.shared.storeSpecialKey(
                            ownerKey,
                            identityId: validIdData,
                            keyType: .owner
                        )
                    }
                    if let payoutKey = capturedPayoutKey {
                        row.payoutPrivateKeyIdentifier = KeychainManager.shared.storeSpecialKey(
                            payoutKey,
                            identityId: validIdData,
                            keyType: .payout
                        )
                    }

                    try? modelContext.save()

                    showSuccess = true

                    // Also fetch DPNS names for the identity
                    Task {
                        do {
                            let usernames = try await sdk.dpnsGetUsername(
                                identityId: validIdData.toHexString(),
                                limit: 1
                            )

                            if let firstUsername = usernames.first,
                               let label = firstUsername["label"] as? String {
                                // Update the identity with DPNS name
                                PersistentIdentity.updateDpnsName(
                                    in: modelContext,
                                    identityId: validIdData,
                                    dpnsName: label
                                )
                                try? modelContext.save()
                            }
                        } catch {
                            // Silently fail - not all identities have DPNS names
                            print("No DPNS name found for identity: \(error)")
                        }
                    }
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                }
            }

            await MainActor.run {
                isLoading = false
                loadStartTime = nil
            }
        }
    }

    private func fillRandomHPMN() {
        guard let nodes = testnetNodes?.hpMasternodes.randomElement() else { return }

        let (name, hpmn) = nodes
        identityIdInput = hpmn.protxTxHash
        selectedIdentityType = .evonode
        aliasInput = name
        votingPrivateKeyInput = hpmn.voter.privateKey
        ownerPrivateKeyInput = hpmn.owner.privateKey
        payoutPrivateKeyInput = hpmn.payout.privateKey
    }

    private func fillRandomMasternode() {
        guard let nodes = testnetNodes?.masternodes.randomElement() else { return }

        let (name, masternode) = nodes
        identityIdInput = masternode.proTxHash
        selectedIdentityType = .masternode
        aliasInput = name
        votingPrivateKeyInput = masternode.voter.privateKey
        ownerPrivateKeyInput = masternode.owner.privateKey
        payoutPrivateKeyInput = ""
    }

    private func resetForm() {
        identityIdInput = ""
        selectedIdentityType = .user
        aliasInput = ""
        votingPrivateKeyInput = ""
        ownerPrivateKeyInput = ""
        payoutPrivateKeyInput = ""
        privateKeys = ["", "", ""]
        errorMessage = nil
    }

    private func formatElapsedTime(_ seconds: TimeInterval) -> String {
        let intSeconds = Int(seconds)
        if intSeconds < 60 {
            return "\(intSeconds)s"
        } else {
            let minutes = intSeconds / 60
            let remainingSeconds = intSeconds % 60
            return "\(minutes)m \(remainingSeconds)s"
        }
    }
}

struct InfoPopupView: View {
    let message: String
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                Text(message)
                    .padding()

                Button("Close") {
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
            }
            .padding()
            .navigationTitle("Information")
            .navigationBarTitleDisplayMode(.inline)
        }
    }
}
