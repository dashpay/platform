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
        /// What Rust's outcome for the entered voting key says to record,
        /// with that key's address, written on the identity only when that
        /// key replaces the stored one; `nil` keeps what is recorded.
        let votingKeyRecord: MasternodeRecordedVotingKey?
        /// What the report says when the entered voting key could not be
        /// checked, shown only when that key is in the voting slot after the
        /// replacement.
        let uncheckedVotingKeyNote: String?
        /// What the load reported when the other voting keys stored for the
        /// masternode could not be marked to be checked again, repeated in
        /// the report after the replacement.
        let siblingNote: String?
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
                        .machineTextEntry()
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
                            .machineTextEntry()
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

                // The masternode key fields take hex or WIF. Replace any
                // testnet preset placeholders before loading. A key that is typed
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

                // A masternode or evonode is loaded through Rust's rule
                // (`SDK.loadMasternodeForVoting`): its owner identity, whose
                // id is the pro_tx_hash, or, for a masternode without an
                // owner identity (a shared masternode), its voter identity
                // when Platform proves that one holds the voting key. The row
                // is stored under the loaded identity's id, with the
                // pro_tx_hash kept beside it. Any other identity is fetched
                // at the typed id.
                let rowId: Data
                let identityData: [String: Any]
                var masternodeProTxHash: Data?
                var masternodeRole: MasternodeVotingLoadedIdentity.Role?
                var votingKeyCheck: MasternodeVotingKeyCheck?
                var votingKeyAddress: MasternodeVotingKeyAddress?
                if isMasternodeType {
                    // A typed id that names a row this device stores for a
                    // masternode (its detail screen shows the row's id, which
                    // for a shared masternode is the voter identity) loads
                    // that row's masternode again, by its recorded
                    // pro_tx_hash. Any other id is read as the pro_tx_hash.
                    let storedProTxHash: Data?
                    do {
                        storedProTxHash = try await MainActor.run {
                            try Self.storedIdentityRow(validIdData, on: network, in: modelContext)?.masternodeProTxHash
                        }
                    } catch {
                        await MainActor.run {
                            errorMessage = "The stored identity could not be read, so nothing was saved: \(error.localizedDescription)"
                            isLoading = false
                            loadStartTime = nil
                        }
                        return
                    }
                    let proTxHash = storedProTxHash
                        ?? MasternodeVoter.wireOrderProTxHash(displayOrder: validIdData)
                    if let votingKeyData {
                        votingKeyAddress = try SDK.masternodeVotingAddress(privateKey: votingKeyData)
                    }
                    let load = try await sdk.loadMasternodeForVoting(
                        proTxHash: proTxHash,
                        votingAddress: votingKeyAddress
                    )
                    let loaded: MasternodeVotingLoadedIdentity
                    switch load {
                    case .refused(let message, let votingKeyRefusedFor):
                        // A refused voting key is recorded on a row this
                        // device already stores for the voter identity it
                        // was refused for, holding the same key, as Rust's
                        // outcome says. Nothing is created or replaced.
                        if let votingKeyRefusedFor, let votingKeyData, let votingKeyAddress,
                           appState.sdk === sdk {
                            _ = await MainActor.run {
                                applyVotingKeyCheck(
                                    .invalid(message: message),
                                    of: votingKeyData,
                                    votingAddress: votingKeyAddress,
                                    identityId: votingKeyRefusedFor,
                                    network: network,
                                    refusal: message
                                )
                            }
                        } else {
                            await MainActor.run {
                                errorMessage = message
                                isLoading = false
                                loadStartTime = nil
                            }
                        }
                        return
                    case .loaded(let identity):
                        loaded = identity
                    }
                    rowId = loaded.identityId
                    identityData = try loaded.identityObject()
                    masternodeProTxHash = proTxHash
                    masternodeRole = loaded.role
                    votingKeyCheck = loaded.votingKeyCheck
                } else {
                    rowId = validIdData
                    identityData = try await sdk.identityGet(identityId: validIdData.toHexString())
                }

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

                // A masternode voting key was checked against Platform with
                // the load. Rust decides from the check and what this device
                // stores for the voting key whether the key is stored and
                // what is recorded for it
                // (`MasternodeVotingKeyCheck.outcome(votingAddress:slot:)`).
                // Every record is written with the address of the key it was
                // made of.
                if let votingKeyData, let check = votingKeyCheck, let votingAddress = votingKeyAddress {
                    let refusal = "The voting key does not belong to masternode \(validIdData.toHexString()), so the identity was not saved: \(Self.reason(of: check))"
                    let storeKey = await MainActor.run {
                        applyVotingKeyCheck(
                            check,
                            of: votingKeyData,
                            votingAddress: votingAddress,
                            identityId: rowId,
                            network: network,
                            refusal: refusal
                        )
                    }
                    guard storeKey else { return }
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

                // Persist the fetched identity directly via SwiftData.
                // Private keys are stored in the keychain and linked
                // to the matching `PersistentPublicKey`s below.
                let trimmedAlias = aliasInput.isEmpty ? nil : aliasInput
                let capturedPrivateKeys = privateKeyData
                let capturedVotingKey = votingKeyData
                let capturedOwnerKey = ownerKeyData
                let capturedPayoutKey = payoutKeyData
                let capturedVotingKeyCheck = votingKeyCheck
                let capturedVotingKeyAddress = votingKeyAddress
                let capturedMasternodeProTxHash = masternodeProTxHash
                let capturedMasternodeRole = masternodeRole

                await MainActor.run {
                    // Read the keys the network returned before writing
                    // anything. A key list that cannot be read completely
                    // is not applied: no identity row, no key rows and no
                    // Keychain writes.
                    guard let fetchedKeys = PersistentPublicKey.fromFetchedIdentity(
                        identityData,
                        identityId: rowId
                    ) else {
                        errorMessage = "This identity's keys could not be read from the response, so nothing was saved. Load it again."
                        isLoading = false
                        loadStartTime = nil
                        return
                    }

                    // Upsert the PersistentIdentity row. A failed read is
                    // not "no row": inserting then could add a second row
                    // for the identity, and whether a wallet owns it would be
                    // unknown, so stop before writing anything.
                    let existing: PersistentIdentity?
                    do {
                        let target = rowId
                        existing = try modelContext.fetch(
                            FetchDescriptor<PersistentIdentity>(
                                predicate: #Predicate { $0.identityId == target }
                            )
                        ).first
                    } catch {
                        errorMessage = "The stored identity could not be read, so nothing was saved: \(error.localizedDescription)"
                        isLoading = false
                        loadStartTime = nil
                        return
                    }
                    // A wallet that holds this identity is the only writer
                    // of its key rows and balance (the persister, through
                    // the wallet's proved refresh, which never lets an older
                    // answer undo a newer one). A plain fetch here could be
                    // a block behind, so it leaves both alone and only
                    // records what the user entered.
                    let walletHeld = existing?.wallet != nil
                    let row: PersistentIdentity
                    if let existing = existing {
                        if !walletHeld {
                            existing.balance = Int64(bitPattern: fetchedBalance)
                        }
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
                            identityId: rowId,
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

                    // A masternode or evonode row records the pro_tx_hash its
                    // votes name, apart from its identity id, and which of
                    // the masternode's identities Rust loaded.
                    if let capturedMasternodeProTxHash {
                        row.masternodeProTxHash = capturedMasternodeProTxHash
                        row.masternodeRole = capturedMasternodeRole
                    }

                    // Keep the fetched keys' DPP projection for matching the
                    // entered private keys, then (for an identity no wallet
                    // holds) bring the rows in line with them, keeping each
                    // unchanged key's local signing references.
                    let capturedPublicKeys = fetchedKeys.compactMap { $0.toIdentityPublicKey() }
                    if !walletHeld {
                        row.mergeFetchedPublicKeys(fetchedKeys)
                    }

                    // Match each user-provided private key to its
                    // public key and stash it in the keychain.
                    // A key that matches a fetched key but has no stored row
                    // with the same id, type and bytes (a wallet-held identity
                    // whose wallet has not refreshed since the key was added),
                    // or that the Keychain refused, is reported rather than
                    // dropped silently.
                    var unlinkedKeyIds: [UInt32] = []
                    for privateKey in capturedPrivateKeys {
                        guard let matched = KeyValidation.matchPrivateKeyToPublicKeys(
                            privateKeyData: privateKey,
                            publicKeys: capturedPublicKeys,
                            network: network
                        ) else {
                            continue
                        }
                        guard let persistentKey = row.publicKeys.first(where: {
                            $0.keyId == Int32(bitPattern: matched.id)
                                && $0.keyTypeEnum == matched.keyType
                                && $0.publicKeyData == matched.data
                        }), let keychainId = KeychainManager.shared.storePrivateKey(
                            privateKey,
                            identityId: rowId,
                            keyIndex: persistentKey.keyId
                        ) else {
                            unlinkedKeyIds.append(matched.id)
                            continue
                        }
                        persistentKey.privateKeyKeychainIdentifier = keychainId
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
                            identityId: rowId,
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
                    // Rust's outcome for the entered key, from what the
                    // store found in the slot (the read before the check
                    // only decided whether to store): an empty slot it
                    // filled, the same key already there, or a different
                    // key it kept.
                    var storedVotingKeyOutcome: MasternodeVotingKeyCheckOutcome?
                    if let identifier = storeSlot(capturedVotingKey, .voting, named: "voting") {
                        enteredVotingKeyStored = true
                        row.votingPrivateKeyIdentifier = identifier
                        if let check = capturedVotingKeyCheck, let address = capturedVotingKeyAddress {
                            storedVotingKeyOutcome = check.outcome(
                                votingAddress: address,
                                slot: storedKeys.contains("voting")
                                    ? .empty
                                    : .sameKey(recorded: row.recordedVotingKey)
                            )
                            // The slot holds the entered key: record what
                            // Rust says for it, or keep what is recorded.
                            if let record = storedVotingKeyOutcome?.record {
                                row.recordedVotingKey = record
                            }
                        }
                    } else if conflictingKeys.contains("voting"),
                              let check = capturedVotingKeyCheck, let address = capturedVotingKeyAddress {
                        storedVotingKeyOutcome = check.outcome(votingAddress: address, slot: .differentKey)
                    }
                    if let identifier = storeSlot(capturedOwnerKey, .owner, named: "owner") {
                        row.ownerPrivateKeyIdentifier = identifier
                    }
                    if let identifier = storeSlot(capturedPayoutKey, .payout, named: "payout") {
                        row.payoutPrivateKeyIdentifier = identifier
                    }

                    // Once the entered voting key is in its slot, Rust says
                    // what to record for the other voting keys this device
                    // stores for the same masternode (a rotation gives each
                    // voting address its own voter identity, so each has its
                    // own row). A list that cannot be read is reported.
                    var siblingNote: String?
                    if enteredVotingKeyStored, let check = capturedVotingKeyCheck,
                       let address = capturedVotingKeyAddress,
                       let proTxHash = capturedMasternodeProTxHash {
                        let target: Data? = proTxHash
                        do {
                            let siblings = try modelContext.fetch(
                                FetchDescriptor<PersistentIdentity>(
                                    predicate: #Predicate { $0.masternodeProTxHash == target }
                                )
                            )
                            for sibling in siblings where sibling.identityId != rowId && sibling.network == network {
                                if let record = check.siblingRecord(votingAddress: address, sibling: sibling.recordedVotingKey) {
                                    sibling.recordedVotingKey = record
                                }
                            }
                        } catch {
                            siblingNote = "Other voting keys stored for this masternode could not be read, so they were not marked to be checked again: \(error.localizedDescription). Load the identity again to retry."
                        }
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
                    let uncheckedNoteIfStored = Self.uncheckedVotingKeyNote(
                        capturedVotingKeyCheck,
                        outcome: storedVotingKeyOutcome
                    )
                    let uncheckedNote = enteredVotingKeyStored ? uncheckedNoteIfStored : nil
                    if unstoredKeys.isEmpty && conflictingKeys.isEmpty && unlinkedKeyIds.isEmpty && uncheckedNote == nil,
                       let siblingNote {
                        errorMessage = "The identity was loaded. " + siblingNote
                        isLoading = false
                        loadStartTime = nil
                    } else if unstoredKeys.isEmpty && conflictingKeys.isEmpty && unlinkedKeyIds.isEmpty && uncheckedNote == nil {
                        showSuccess = true
                    } else if unstoredKeys.isEmpty && conflictingKeys.isEmpty && unlinkedKeyIds.isEmpty, let uncheckedNote {
                        errorMessage = "The identity was loaded and its voting key was stored. " + uncheckedNote
                            + (siblingNote.map { " " + $0 } ?? "")
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
                        if !unlinkedKeyIds.isEmpty {
                            let ids = ListFormatter.localizedString(byJoining: unlinkedKeyIds.sorted().map(String.init))
                            let noun = unlinkedKeyIds.count == 1 ? "key" : "keys"
                            parts.append("the private \(noun) entered for key ID \(ids) could not be saved on this device")
                            let retry = walletHeld
                                ? "Refresh the identity from the wallet that holds it, then load it again."
                                : "Load the identity again."
                            let note = "The private \(noun) entered for key ID \(ids) could not be saved on this device. \(retry)"
                            unstoredNote = unstoredNote.map { $0 + " " + note } ?? note
                        }
                        if !conflictingKeys.isEmpty {
                            let names = ListFormatter.localizedString(byJoining: conflictingKeys)
                            let noun = conflictingKeys.count == 1 ? "private key is" : "private keys are"
                            parts.append("a different \(names) \(noun) already stored for this identity and was kept")
                        }
                        var text = "The identity was loaded, but " + parts.joined(separator: ", and ") + "."
                        if !unlinkedKeyIds.isEmpty {
                            text += walletHeld
                                ? " Refresh the identity from the wallet that holds it, then load it again."
                                : " Load the identity again."
                        }
                        if !conflictingKeys.isEmpty {
                            text += " You can replace the stored key with the one you entered when asked, or load the identity again later to be asked again."
                        }
                        if unstoredKeys.contains("voting") && row.votingPrivateKeyIdentifier == nil {
                            text += " The identity cannot vote until it is loaded again."
                        }
                        if let uncheckedNote {
                            text += " The voting key was stored. " + uncheckedNote
                        }
                        if let siblingNote {
                            text += " " + siblingNote
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
                                identityId: rowId,
                                keys: conflictingReplacements,
                                unstoredNote: unstoredNote,
                                enteredKeyDigests: enteredKeyDigests,
                                votingKeyRecord: storedVotingKeyOutcome?.record,
                                uncheckedVotingKeyNote: uncheckedNoteIfStored,
                                siblingNote: siblingNote
                            )
                        }
                    }

                    // Also fetch DPNS names for the identity
                    Task {
                        do {
                            let usernames = try await sdk.dpnsGetUsername(
                                identityId: rowId.toHexString(),
                                limit: 1
                            )

                            if let firstUsername = usernames.first,
                               let label = firstUsername["label"] as? String {
                                // Update the identity with DPNS name
                                PersistentIdentity.updateDpnsName(
                                    in: modelContext,
                                    identityId: rowId,
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

    /// Applies Rust's outcome for a checked voting key
    /// (`MasternodeVotingKeyCheck.outcome(votingAddress:slot:)`) to what this
    /// device stores for `identityId`, and returns whether the entered key is
    /// stored. When it is not, nothing entered is stored, what Rust says to
    /// record is written on the identity's existing row (for the same stored
    /// key only), and the load reports `refusal`.
    @MainActor
    private func applyVotingKeyCheck(
        _ check: MasternodeVotingKeyCheck,
        of votingKeyData: Data,
        votingAddress: MasternodeVotingKeyAddress,
        identityId: Data,
        network: Network,
        refusal: String
    ) -> Bool {
        // One read of the row, for the slot decision and the update. A row
        // that is absent changes nothing; one that cannot be read stops the
        // load, since what is recorded for its key would be unknown.
        let row: PersistentIdentity?
        do {
            row = try Self.storedIdentityRow(identityId, on: network, in: modelContext)
        } catch {
            var text = "The stored identity could not be read, so nothing was saved and what is recorded for its voting key was not updated: \(error.localizedDescription). Load the identity again."
            if case .invalid = check { text = refusal + " " + text }
            errorMessage = text
            isLoading = false
            loadStartTime = nil
            return false
        }
        let read = Self.votingKeySlot(
            holding: votingKeyData,
            identityId: identityId,
            recorded: row?.recordedVotingKey
        )
        let outcome = check.outcome(votingAddress: votingAddress, slot: read.slot)
        guard !outcome.storeKey else { return true }
        var text = refusal
        if let record = outcome.record, let row {
            row.recordedVotingKey = record
            do {
                try modelContext.save()
            } catch {
                text += " The verdict on the stored voting key could not be saved: \(error.localizedDescription). Load the identity again to retry."
            }
        }
        if read.storedKeyUnreadable {
            text += " The stored voting key could not be read, so what is recorded for it was not updated. Unlock the device and load the identity again."
        }
        errorMessage = text
        isLoading = false
        loadStartTime = nil
        return false
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
                    // The slot now holds the entered key: record what Rust's
                    // outcome at the load says for it, or keep what is
                    // recorded.
                    if let record = pending.votingKeyRecord {
                        row.recordedVotingKey = record
                    }
                case .owner: row.ownerPrivateKeyIdentifier = identifier
                case .payout: row.payoutPrivateKeyIdentifier = identifier
                }
            }
        }

        // Once the entered voting key replaced the stored one, Rust says what
        // to record for the other voting keys this device stores for the same
        // masternode, as a load does. A list that cannot be read is reported.
        var siblingNote = pending.siblingNote
        if replaced.contains(SpecialKeyType.voting.rawValue), let record = pending.votingKeyRecord,
           let proTxHash = row.masternodeProTxHash {
            let target: Data? = proTxHash
            let network = row.network
            do {
                let siblings = try modelContext.fetch(
                    FetchDescriptor<PersistentIdentity>(predicate: #Predicate { $0.masternodeProTxHash == target })
                )
                for sibling in siblings where sibling.identityId != row.identityId && sibling.network == network {
                    if let siblingRecord = record.check.siblingRecord(
                        votingAddress: record.votingAddress,
                        sibling: sibling.recordedVotingKey
                    ) {
                        sibling.recordedVotingKey = siblingRecord
                    }
                }
            } catch {
                siblingNote = "Other voting keys stored for this masternode could not be read, so they were not marked to be checked again: \(error.localizedDescription). Load the identity again to retry."
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
        // Whether the entered voting key is in its slot now, stored by the
        // load itself or by this replacement.
        var enteredVotingKeyInPlace = false
        for (keyType, digest) in pending.enteredKeyDigests.sorted(by: { $0.key.rawValue < $1.key.rawValue }) {
            let inPlace: Bool
            if case .success(let stored?) = keychain.readSpecialKey(identityId: pending.identityId, keyType: keyType) {
                inPlace = Data(SHA256.hash(data: stored)) == digest
            } else {
                inPlace = false
            }
            if keyType == .voting { enteredVotingKeyInPlace = inPlace }
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
        let uncheckedNote = enteredVotingKeyInPlace ? pending.uncheckedVotingKeyNote : nil
        if let uncheckedNote {
            parts.append(uncheckedNote)
        }
        if let siblingNote {
            parts.append(siblingNote)
        }
        if saved && replaced.count == pending.keys.count && pending.unstoredNote == nil && notInPlace.isEmpty
            && uncheckedNote == nil && siblingNote == nil {
            errorMessage = nil
            showSuccess = true
        } else {
            errorMessage = parts.joined(separator: " ")
            isLoading = false
            loadStartTime = nil
        }
    }

    /// What the load reports when the entered voting key could not be
    /// checked against Platform, worded from Rust's `outcome` for it: whether
    /// what was recorded is kept. `nil` when the key was checked. Shown only
    /// when the entered key is in the voting slot.
    private static func uncheckedVotingKeyNote(
        _ check: MasternodeVotingKeyCheck?,
        outcome: MasternodeVotingKeyCheckOutcome?
    ) -> String? {
        guard case .undetermined(let reason) = check, let outcome else { return nil }
        if outcome.record == nil {
            return reason + " The result of its earlier check is kept."
        }
        return reason + " It is checked again when you pick this identity to vote."
    }

    /// The message a voting key check carries, empty for a valid one.
    private static func reason(of check: MasternodeVotingKeyCheck) -> String {
        switch check {
        case .valid: return ""
        case .invalid(let message), .undetermined(let message): return message
        }
    }

    /// The identity's row on `network`, `nil` when there is none; a failed
    /// read throws, so it is never mistaken for an absent row.
    @MainActor
    private static func storedIdentityRow(
        _ identityId: Data,
        on network: Network,
        in modelContext: ModelContext
    ) throws -> PersistentIdentity? {
        let target = identityId
        let row = try modelContext.fetch(
            FetchDescriptor<PersistentIdentity>(predicate: #Predicate { $0.identityId == target })
        ).first
        guard let row, row.network == network else { return nil }
        return row
    }


    /// What this device's voting key slot for the identity holds compared
    /// with `enteredKey`, the fact Rust's rules take
    /// (`MasternodeVotingKeyCheck.outcome(votingAddress:slot:)`), and whether
    /// the stored key could not be read. The keys are compared by SHA-256
    /// digest, so no copy of the stored key outlives the read; the recorded
    /// check, with its key's address, comes from the identity's row on
    /// `network`, if any.
    @MainActor
    private static func votingKeySlot(
        holding enteredKey: Data,
        identityId: Data,
        recorded: MasternodeRecordedVotingKey?
    ) -> (slot: MasternodeVotingKeySlot, storedKeyUnreadable: Bool) {
        let storedDigest: Data
        switch KeychainManager.shared.readSpecialKey(identityId: identityId, keyType: .voting) {
        case .success(nil):
            return (.empty, false)
        case .success(let stored?):
            storedDigest = Data(SHA256.hash(data: stored))
        case .failure:
            // The stored key could not be read, so it is not known to be
            // the entered one: reported as a different key, the
            // conservative fact (Rust records nothing on a different key),
            // and the caller tells the user.
            return (.differentKey, true)
        }
        guard storedDigest == Data(SHA256.hash(data: enteredKey)) else {
            return (.differentKey, false)
        }
        return (.sameKey(recorded: recorded), false)
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
