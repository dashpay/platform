import CryptoKit
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

    /// A masternode key the user typed for a slot that already holds a
    /// different key.
    private struct PendingKeyReplacement {
        let keyType: SpecialKeyType
        let keyData: Data
        /// Digest of the stored key the user is asked about; the
        /// replacement refuses if the slot holds anything else by then.
        let storedKeyDigest: Data
    }

    /// The conflicting keys of the last load, offered for replacement.
    private struct PendingKeyReplacements {
        let identityId: Data
        let keys: [PendingKeyReplacement]
        /// What the load reported about keys that could not be stored,
        /// repeated in the report after the replacement.
        let unstoredNote: String?
        /// SHA-256 of every masternode key the user entered in the load,
        /// by role. Success requires each slot to still hold its key, so a
        /// key changed elsewhere while the dialog was open is reported.
        let enteredKeyDigests: [SpecialKeyType: Data]
        /// What Platform said about the entered voting key, recorded on the
        /// identity only when that key replaces the stored one.
        let votingKeyCheck: MasternodeVotingKeyCheck?
    }

    /// Held only until the replacement dialog is answered, then cleared.
    @State private var pendingKeyReplacements: PendingKeyReplacements?

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
        .confirmationDialog(
            "Replace the stored key?",
            isPresented: Binding(
                get: { pendingKeyReplacements != nil },
                set: { newValue in
                    // Every answer, cancel included, drops the typed bytes;
                    // the buttons act on the value they were given.
                    if !newValue { pendingKeyReplacements = nil }
                }
            ),
            titleVisibility: .visible,
            presenting: pendingKeyReplacements
        ) { pending in
            ForEach(pending.keys, id: \.keyType) { key in
                Button("Replace stored \(key.keyType.rawValue) key", role: .destructive) {
                    replaceStoredKeys([key], of: pending)
                }
                .accessibilityIdentifier("loadIdentity.replaceKey.\(key.keyType.rawValue)")
            }
            if pending.keys.count > 1 {
                Button("Replace all stored keys", role: .destructive) {
                    replaceStoredKeys(pending.keys, of: pending)
                }
                .accessibilityIdentifier("loadIdentity.replaceKey.all")
            }
            Button("Keep stored keys", role: .cancel) {
                pendingKeyReplacements = nil
            }
            .accessibilityIdentifier("loadIdentity.replaceKey.cancel")
        } message: { _ in
            Text(
                "This identity already has a different key stored. Replacing it deletes the stored key from this device and stores the one you entered. Keeping it leaves everything as it is."
            )
        }
    }

    private var masternodeKeyInputs: some View {
        Section("Masternode Keys") {
            VStack(alignment: .leading) {
                Text("Voting Private Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                SecureField("Hex or WIF", text: $votingPrivateKeyInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
            }

            VStack(alignment: .leading) {
                Text("Owner Private Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                SecureField("Hex or WIF", text: $ownerPrivateKeyInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
            }

            if selectedIdentityType == .evonode {
                VStack(alignment: .leading) {
                    Text("Payout Address Private Key")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    SecureField("Hex or WIF", text: $payoutPrivateKeyInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
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

                        TextField("Hex (64 characters)", text: $privateKeys[index])
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

                let identityType = selectedIdentityType
                let isMasternodeType = identityType == .masternode || identityType == .evonode

                // Only the fields the selected identity type shows are
                // read: the "Private Keys" list for a type that is neither
                // masternode nor evonode, voting and owner for a masternode
                // or evonode, payout for an evonode only. Keys typed before
                // switching the type are not stored.
                //
                // The "Private Keys" list is hex, matched to a public key by
                // its type below (a key of any type, Ed25519 included).
                let privateKeyData: [Data] = isMasternodeType ? [] : privateKeys.compactMap { keyString in
                    let trimmed = keyString.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !trimmed.isEmpty else { return nil }
                    return Data(hexString: trimmed)
                }

                // The masternode key fields take hex or WIF (the testnet
                // presets fill WIF); Rust reads either. A key that is typed
                // but refused stops the load with Rust's reason instead of
                // being dropped, since the vote sheets sign with the stored
                // voting key. The refusal never repeats the typed text.
                var refusals: [String] = []
                func readKey(_ input: String, named name: String) -> Data? {
                    guard !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                        return nil
                    }
                    do {
                        return try KeyValidation.privateKeyBytes(fromText: input)
                    } catch SDKError.invalidParameter(let reason) {
                        refusals.append("The \(name) was refused: \(reason).")
                    } catch {
                        refusals.append("The \(name) was refused: \(error.localizedDescription)")
                    }
                    return nil
                }
                let votingKeyData = isMasternodeType
                    ? readKey(votingPrivateKeyInput, named: "voting private key") : nil
                let ownerKeyData = isMasternodeType
                    ? readKey(ownerPrivateKeyInput, named: "owner private key") : nil
                let payoutKeyData = identityType == .evonode
                    ? readKey(payoutPrivateKeyInput, named: "payout private key") : nil
                guard refusals.isEmpty else {
                    let text = refusals.joined(separator: " ")
                    await MainActor.run {
                        errorMessage = text
                        isLoading = false
                        loadStartTime = nil
                    }
                    return
                }

                // Fetch the identity from the network to verify it exists
                guard let sdk = appState.sdk else {
                    await MainActor.run {
                        errorMessage = "SDK not initialized"
                        isLoading = false
                        loadStartTime = nil
                    }
                    return
                }

                // The network the identity is fetched from, captured with
                // the SDK: the rows are stored under it.
                let network = appState.currentNetwork

                // Try to fetch the identity
                let identityData = try await sdk.identityGet(identityId: validIdData.toHexString())

                // The network was switched while the fetch was in flight:
                // the answer is from the previous network, so nothing is
                // written.
                guard appState.sdk === sdk else {
                    await MainActor.run {
                        errorMessage = "The network changed during the load, so the identity was not saved. Load it again."
                        isLoading = false
                        loadStartTime = nil
                    }
                    return
                }

                // A masternode voting key is checked against Platform before
                // anything is stored: a key Platform proves cannot vote for
                // this masternode stops the load, one that could not be
                // checked is stored and marked unchecked.
                var votingKeyCheck: MasternodeVotingKeyCheck?
                if let votingKeyData {
                    let check = try await sdk.checkMasternodeVotingKey(
                        proTxHash: MasternodeVoter.wireOrderProTxHash(identityId: validIdData),
                        votingPrivateKey: votingKeyData
                    )
                    guard appState.sdk === sdk else {
                        await MainActor.run {
                            errorMessage = "The network changed during the load, so the identity was not saved. Load it again."
                            isLoading = false
                            loadStartTime = nil
                        }
                        return
                    }
                    if case .invalid(let reason) = check {
                        let text = "The voting key does not belong to masternode \(validIdData.toHexString()), so the identity was not saved: \(reason)"
                        await MainActor.run {
                            errorMessage = text
                            isLoading = false
                            loadStartTime = nil
                        }
                        return
                    }
                    votingKeyCheck = check
                }

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
                        // `disabledAt`, `totalBudget` and `expiresAt` are all protocol
                        // `u64`s, which DPP writes as a JSON number up to 2^53 - 1 and
                        // as a decimal string above it. `UInt64(jsonValue:)` reads both
                        // shapes; a plain `as? UInt64` would drop the large ones.
                        let disabledAt = UInt64(jsonValue: keyData["disabledAt"])
                        // Usage limits (protocol version 14). Present only on a
                        // version 1 key: `totalBudget` is credits for the key's
                        // whole lifetime, `expiresAt` is block time in
                        // milliseconds. Dropping them here would persist a
                        // limited key as unlimited.
                        let totalBudget = UInt64(jsonValue: keyData["totalBudget"])
                        let expiresAt = UInt64(jsonValue: keyData["expiresAt"])

                        return IdentityPublicKey(
                            id: UInt32(id),
                            purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                            securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                            contractBounds: nil,
                            keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                            readOnly: readOnly,
                            data: data,
                            disabledAt: disabledAt,
                            totalBudget: totalBudget,
                            expiresAt: expiresAt
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
                        // `disabledAt`, `totalBudget` and `expiresAt` are all protocol
                        // `u64`s, which DPP writes as a JSON number up to 2^53 - 1 and
                        // as a decimal string above it. `UInt64(jsonValue:)` reads both
                        // shapes; a plain `as? UInt64` would drop the large ones.
                        let disabledAt = UInt64(jsonValue: keyData["disabledAt"])
                        // Usage limits (protocol version 14). Present only on a
                        // version 1 key: `totalBudget` is credits for the key's
                        // whole lifetime, `expiresAt` is block time in
                        // milliseconds. Dropping them here would persist a
                        // limited key as unlimited.
                        let totalBudget = UInt64(jsonValue: keyData["totalBudget"])
                        let expiresAt = UInt64(jsonValue: keyData["expiresAt"])

                        return IdentityPublicKey(
                            id: UInt32(id),
                            purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                            securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                            contractBounds: nil,
                            keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                            readOnly: readOnly,
                            data: data,
                            disabledAt: disabledAt,
                            totalBudget: totalBudget,
                            expiresAt: expiresAt
                        )
                    }
                } else {
                    print("❌ Public keys not found in identity data")
                }

                // Persist the fetched identity directly via SwiftData.
                // Private keys are stored in the keychain and linked
                // to the matching `PersistentPublicKey`s below.
                let trimmedAlias = aliasInput.isEmpty ? nil : aliasInput
                let capturedPublicKeys = parsedPublicKeys
                let capturedPrivateKeys = privateKeyData
                let capturedVotingKey = votingKeyData
                let capturedOwnerKey = ownerKeyData
                let capturedPayoutKey = payoutKeyData
                let capturedVotingKeyCheck = votingKeyCheck

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
                        // The user just added/reloaded this identity
                        // by hand — mark the provenance. (This flow
                        // previously wrote `false`, which erased the
                        // very thing the flag exists to record.)
                        existing.isLocal = true
                        existing.identityType = identityType.rawValue
                        existing.network = network
                        existing.lastUpdated = Date()
                        row = existing
                    } else {
                        row = PersistentIdentity(
                            identityId: validIdData,
                            balance: Int64(bitPattern: fetchedBalance),
                            revision: 0,
                            isLocal: true,
                            alias: trimmedAlias,
                            dpnsName: nil,
                            mainDpnsName: nil,
                            identityType: identityType,
                            network: network
                        )
                        modelContext.insert(row)
                        // Back-fill any locally-cached contracts that
                        // name this identity as their owner. The
                        // `modelContext.save()` later in this block
                        // persists the link.
                        ContractIdentityLinker.linkIdentityToOwnedContracts(
                            identity: row,
                            modelContext: modelContext
                        )
                    }

                    // Bring the rows in line with the keys the network
                    // returned, keeping each unchanged key's local signing
                    // references.
                    Self.mergeFetchedPublicKeys(capturedPublicKeys, into: row)

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
                    // keychain under their dedicated slots, only into an
                    // empty slot: a stored key is never replaced or
                    // deleted here, so a failed or refused write keeps the
                    // previous key and the row's identifier. Reported below;
                    // a conflicting key is replaced only once the user
                    // confirms it.
                    var unstoredKeys: [String] = []
                    var conflictingKeys: [String] = []
                    var conflictingReplacements: [PendingKeyReplacement] = []
                    var storedKeys: [String] = []
                    @MainActor
                    func storeSlot(_ key: Data?, _ keyType: SpecialKeyType, named name: String) -> String? {
                        guard let key else { return nil }
                        switch KeychainManager.shared.storeSpecialKeyIfAbsent(
                            key,
                            identityId: validIdData,
                            keyType: keyType
                        ) {
                        case .stored(let identifier):
                            storedKeys.append(name)
                            return identifier
                        case .alreadyStored(let identifier):
                            return identifier
                        case .conflicting(let digest):
                            conflictingKeys.append(name)
                            // The digest names the stored key that caused
                            // the conflict, so a confirmation always refers
                            // to that key.
                            conflictingReplacements.append(
                                PendingKeyReplacement(keyType: keyType, keyData: key, storedKeyDigest: digest)
                            )
                            return nil
                        case .failed, .changedSinceConfirmation:
                            unstoredKeys.append(name)
                            return nil
                        }
                    }
                    var enteredVotingKeyStored = false
                    if let identifier = storeSlot(capturedVotingKey, .voting, named: "voting") {
                        enteredVotingKeyStored = true
                        row.votingPrivateKeyIdentifier = identifier
                        // The slot holds the entered key, so the check made
                        // of that key is the row's. A key kept in its place
                        // keeps its own check.
                        row.votingKeyCheck = capturedVotingKeyCheck
                    }
                    if let identifier = storeSlot(capturedOwnerKey, .owner, named: "owner") {
                        row.ownerPrivateKeyIdentifier = identifier
                    }
                    if let identifier = storeSlot(capturedPayoutKey, .payout, named: "payout") {
                        row.payoutPrivateKeyIdentifier = identifier
                    }

                    // The identity itself loaded, so its row is saved
                    // even when a key could not be stored.
                    do {
                        try modelContext.save()
                    } catch {
                        var text = "The identity could not be saved: \(error.localizedDescription)"
                        if !storedKeys.isEmpty {
                            // The Keychain writes are not undone, so say
                            // which keys already changed.
                            let names = ListFormatter.localizedString(byJoining: storedKeys)
                            let noun = storedKeys.count == 1 ? "private key was" : "private keys were"
                            text += " The \(names) \(noun) already stored in the Keychain. Load the identity again to finish."
                        }
                        errorMessage = text
                        isLoading = false
                        loadStartTime = nil
                        return
                    }

                    // The entered voting key was stored but Platform could not
                    // be asked about it: said in place of the plain success.
                    let uncheckedNote = Self.uncheckedVotingKeyNote(
                        capturedVotingKeyCheck,
                        stored: enteredVotingKeyStored
                    )
                    if unstoredKeys.isEmpty && conflictingKeys.isEmpty && uncheckedNote == nil {
                        showSuccess = true
                    } else if unstoredKeys.isEmpty && conflictingKeys.isEmpty, let uncheckedNote {
                        errorMessage = "The identity was loaded and its voting key was stored. " + uncheckedNote
                        isLoading = false
                        loadStartTime = nil
                    } else {
                        var parts: [String] = []
                        // Kept apart so the report after a replacement can
                        // still say which keys were not stored.
                        var unstoredNote: String?
                        if !unstoredKeys.isEmpty {
                            let names = ListFormatter.localizedString(byJoining: unstoredKeys)
                            let noun = unstoredKeys.count == 1 ? "private key" : "private keys"
                            parts.append("the \(names) \(noun) could not be stored in the Keychain")
                            var note = "The \(names) \(noun) could not be stored in the Keychain."
                            if unstoredKeys.contains("voting") && row.votingPrivateKeyIdentifier == nil {
                                note += " The identity cannot vote until it is loaded again."
                            }
                            unstoredNote = note
                        }
                        if !conflictingKeys.isEmpty {
                            let names = ListFormatter.localizedString(byJoining: conflictingKeys)
                            let noun = conflictingKeys.count == 1 ? "private key is" : "private keys are"
                            parts.append("a different \(names) \(noun) already stored for this identity and was kept")
                        }
                        var text = "The identity was loaded, but " + parts.joined(separator: ", and ") + "."
                        if !conflictingKeys.isEmpty {
                            text += " You can replace the stored key with the one you entered when asked, or load the identity again later to be asked again."
                        }
                        if unstoredKeys.contains("voting") && row.votingPrivateKeyIdentifier == nil {
                            text += " The identity cannot vote until it is loaded again."
                        }
                        if let uncheckedNote {
                            text += " The voting key was stored. " + uncheckedNote
                        }
                        errorMessage = text
                        isLoading = false
                        loadStartTime = nil
                        if !conflictingReplacements.isEmpty {
                            var enteredKeyDigests: [SpecialKeyType: Data] = [:]
                            for (keyType, key) in [
                                (SpecialKeyType.voting, capturedVotingKey),
                                (.owner, capturedOwnerKey),
                                (.payout, capturedPayoutKey),
                            ] {
                                if let key {
                                    enteredKeyDigests[keyType] = Data(SHA256.hash(data: key))
                                }
                            }
                            pendingKeyReplacements = PendingKeyReplacements(
                                identityId: validIdData,
                                keys: conflictingReplacements,
                                unstoredNote: unstoredNote,
                                enteredKeyDigests: enteredKeyDigests,
                                votingKeyCheck: capturedVotingKeyCheck
                            )
                        }
                    }

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

    /// Replace the stored masternode keys in `keys` after the user confirmed
    /// it, leaving the identity and its other keys as they are.
    private func replaceStoredKeys(_ keys: [PendingKeyReplacement], of pending: PendingKeyReplacements) {
        pendingKeyReplacements = nil
        let keychain = KeychainManager.shared
        guard let row = PersistentIdentity.fetch(in: modelContext, identityId: pending.identityId) else {
            errorMessage = "The identity could not be found or read on this device, so no key was replaced. Load it again."
            return
        }

        var replaced: [String] = []
        var kept: [String] = []
        var removed: [String] = []
        var unknown: [String] = []
        var changed: [String] = []
        for key in keys {
            let name = key.keyType.rawValue
            let identifier: String?
            switch keychain.replaceSpecialKey(
                key.keyData,
                identityId: pending.identityId,
                keyType: key.keyType,
                replacing: key.storedKeyDigest
            ) {
            case .stored(let stored), .alreadyStored(let stored):
                identifier = stored
                replaced.append(name)
            case .conflicting:
                // Another writer filled the emptied slot; its key is kept.
                identifier = nil
                kept.append(name)
            case .changedSinceConfirmation:
                // The slot no longer holds the key the user agreed to
                // replace (another window replaced or removed it); untouched.
                identifier = nil
                changed.append(name)
            case .failed:
                // A refused delete keeps the old key; a failed add after
                // the delete leaves the slot empty.
                switch keychain.readSpecialKey(identityId: pending.identityId, keyType: key.keyType) {
                case .success(nil):
                    removed.append(name)
                    switch key.keyType {
                    case .voting:
                        row.votingPrivateKeyIdentifier = nil
                        row.votingKeyCheck = nil
                    case .owner: row.ownerPrivateKeyIdentifier = nil
                    case .payout: row.payoutPrivateKeyIdentifier = nil
                    }
                case .success:
                    kept.append(name)
                case .failure:
                    unknown.append(name)
                }
                identifier = nil
            }
            if let identifier {
                switch key.keyType {
                case .voting:
                    row.votingPrivateKeyIdentifier = identifier
                    // The slot now holds the entered key: record what
                    // Platform said about it at the load.
                    row.votingKeyCheck = pending.votingKeyCheck
                case .owner: row.ownerPrivateKeyIdentifier = identifier
                case .payout: row.payoutPrivateKeyIdentifier = identifier
                }
            }
        }

        var parts: [String] = []
        var saved = true
        do {
            try modelContext.save()
        } catch {
            saved = false
            parts.append("The identity could not be saved: \(error.localizedDescription). Load it again to finish.")
        }
        if !replaced.isEmpty {
            let names = ListFormatter.localizedString(byJoining: replaced)
            let noun = replaced.count == 1 ? "key was" : "keys were"
            parts.append("The stored \(names) \(noun) replaced.")
        }
        if !kept.isEmpty {
            let names = ListFormatter.localizedString(byJoining: kept)
            let noun = kept.count == 1 ? "key" : "keys"
            parts.append("The \(names) \(noun) could not be replaced; the stored one was kept.")
        }
        if !changed.isEmpty {
            let names = ListFormatter.localizedString(byJoining: changed)
            let noun = changed.count == 1
                ? "key is no longer the one"
                : "keys are no longer the ones"
            parts.append("The stored \(names) \(noun) you confirmed replacing, so nothing was changed. Load the identity again.")
        }
        if !unknown.isEmpty {
            let names = ListFormatter.localizedString(byJoining: unknown)
            let noun = unknown.count == 1 ? "key" : "keys"
            parts.append("The \(names) \(noun) could not be replaced, and the Keychain could not be read to check it; the stored one may still be there.")
        }
        if !removed.isEmpty {
            let names = ListFormatter.localizedString(byJoining: removed)
            let noun = removed.count == 1 ? "key was" : "keys were"
            parts.append("The stored \(names) \(noun) removed, but the new one could not be stored. Load the identity again with it.")
            if removed.contains("voting") {
                parts.append("The identity cannot vote until it is loaded again.")
            }
        }
        let notChosen = pending.keys.map(\.keyType.rawValue).filter { name in
            !keys.contains { $0.keyType.rawValue == name }
        }
        if !notChosen.isEmpty {
            let names = ListFormatter.localizedString(byJoining: notChosen)
            let noun = notChosen.count == 1 ? "key was" : "keys were"
            parts.append("The stored \(names) \(noun) kept.")
        }
        if let unstoredNote = pending.unstoredNote {
            parts.append(unstoredNote)
        }

        // `pending.keys` holds every conflicting key of the load, so this is
        // a plain success only when the identity saved and every key the
        // user entered is now stored. Any other outcome keeps the report.
        // Every key the user entered must still be the one in its slot,
        // including roles stored by the load itself, so a key changed in
        // another window while the dialog was open is not reported as done.
        var notInPlace: [String] = []
        for (keyType, digest) in pending.enteredKeyDigests.sorted(by: { $0.key.rawValue < $1.key.rawValue }) {
            let inPlace: Bool
            if case .success(let stored?) = keychain.readSpecialKey(identityId: pending.identityId, keyType: keyType) {
                inPlace = Data(SHA256.hash(data: stored)) == digest
            } else {
                inPlace = false
            }
            let handled = changed.contains(keyType.rawValue) || kept.contains(keyType.rawValue)
                || removed.contains(keyType.rawValue) || unknown.contains(keyType.rawValue)
                || notChosen.contains(keyType.rawValue)
            if !inPlace && !handled {
                notInPlace.append(keyType.rawValue)
            }
        }
        if !notInPlace.isEmpty {
            let names = ListFormatter.localizedString(byJoining: notInPlace)
            let noun = notInPlace.count == 1 ? "key is" : "keys are"
            parts.append("The \(names) \(noun) no longer the one you entered; it was changed on this device since the load. Load the identity again.")
        }
        let uncheckedNote = Self.uncheckedVotingKeyNote(
            pending.votingKeyCheck,
            stored: replaced.contains(SpecialKeyType.voting.rawValue)
        )
        if let uncheckedNote {
            parts.append(uncheckedNote)
        }
        if saved && replaced.count == pending.keys.count && pending.unstoredNote == nil && notInPlace.isEmpty
            && uncheckedNote == nil {
            errorMessage = nil
            showSuccess = true
        } else {
            errorMessage = parts.joined(separator: " ")
            isLoading = false
            loadStartTime = nil
        }
    }

    /// What the load reports when the voting key it stored could not be
    /// checked against Platform; `nil` when it was checked, or when the
    /// entered key did not end up in the voting slot.
    private static func uncheckedVotingKeyNote(_ check: MasternodeVotingKeyCheck?, stored: Bool) -> String? {
        guard stored, case .undetermined(let reason) = check else { return nil }
        return reason + " It is checked again when you pick this identity to vote."
    }

    /// Bring `identity`'s public-key rows in line with the keys fetched from
    /// Platform. A row whose key id, key type and bytes are unchanged is
    /// updated in place, so its local signing references (Keychain
    /// identifier, wallet id, derivation path) survive a reload; any other
    /// fetched key gets a new row. Rows for keys no longer fetched, or whose
    /// bytes changed, are taken off the identity as before, and their
    /// Keychain items are left alone.
    @MainActor
    private static func mergeFetchedPublicKeys(_ fetched: [IdentityPublicKey], into identity: PersistentIdentity) {
        let identityHex = identity.identityId.toHexString()
        var matched = Set<ObjectIdentifier>()
        var additions: [PersistentPublicKey] = []
        for publicKey in fetched {
            // Built the same way as a new row, so the in-place update
            // writes exactly the columns a new row would have.
            guard let fresh = PersistentPublicKey.from(publicKey, identityId: identityHex) else {
                continue
            }
            guard let current = identity.publicKeys.first(where: {
                !matched.contains(ObjectIdentifier($0))
                    && $0.keyId == fresh.keyId
                    && $0.keyType == fresh.keyType
                    && $0.publicKeyData == fresh.publicKeyData
            }) else {
                additions.append(fresh)
                continue
            }
            matched.insert(ObjectIdentifier(current))
            current.purpose = fresh.purpose
            current.securityLevel = fresh.securityLevel
            current.readOnly = fresh.readOnly
            current.disabledAt = fresh.disabledAt
            current.totalBudget = fresh.totalBudget
            current.expiresAt = fresh.expiresAt
            // Contract bounds are left as stored: this form's parsers do not
            // read them from the fetch (`contractBounds: nil`), so copying
            // would turn a contract-, document-type- or group-bound key into
            // an unbounded one.
            current.identityId = fresh.identityId
        }
        identity.publicKeys.removeAll { !matched.contains(ObjectIdentifier($0)) }
        for addition in additions {
            identity.addPublicKey(addition)
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
