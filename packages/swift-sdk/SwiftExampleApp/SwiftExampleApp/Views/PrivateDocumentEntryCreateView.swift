// PrivateDocumentEntryCreateView.swift
// SwiftExampleApp
//
// Creates one entry of a private document type (protocol version 15): an
// encrypted entry sealed by one of a wallet's identities to a key of any
// identity, appended by an anonymous transition (type 29) paid from the
// wallet's shielded account 0. Everything that decides something is Rust's:
// which recipient keys qualify (`privateDocumentRecipientKeys`), which of
// the sender's keys seals, the schema check, the fee
// (`privateDocumentCreateFee`) and the outcome
// (`createPrivateDocumentEntry`). This screen gathers the choices, shows
// what Rust answers, and keeps nothing.
//
// Like the entries screen it reads the device's wallets and identities, so
// the caller hands it the device's model context. The document type is read
// only for its schema, to build the property form.

import SwiftUI
import SwiftData
import SwiftDashSDK

struct PrivateDocumentEntryCreateView: View {
    /// The private type, read for its schema only.
    let documentType: PersistentDocumentType
    let contractId: Data
    let documentTypeName: String

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Every persisted wallet; filtered to the active network by `wallets`.
    @Query(sort: \PersistentWallet.createdAt) private var allWallets: [PersistentWallet]
    @Query private var identities: [PersistentIdentity]

    @State private var selectedWalletId: Data?
    /// The sender: one of the selected wallet's identities.
    @State private var selectedSenderId: Data?
    /// The recipient identity id as typed (base58 or hex).
    @State private var recipientText = ""
    /// The recipient's keys an entry may be sealed to, as Rust read them
    /// for `recipientKeysFor`; nil before they are loaded.
    @State private var recipientKeys: [PrivateDocumentRecipientKey]?
    @State private var recipientKeysFor: Data?
    @State private var recipientKeysError: String?
    @State private var isLoadingKeys = false
    @State private var selectedRecipientKeyId: UInt32?
    @State private var auth: PrivateDocumentSenderAuth = .anonymous
    /// Values from `DocumentFieldsView`, encoded as `CreateDocumentView`
    /// encodes a document's.
    @State private var fieldValues: [String: Any] = [:]

    @State private var fee: UInt64?
    @State private var feeError: String?

    @State private var isSubmitting = false
    @State private var outcome: Outcome?

    private enum Outcome {
        case created(PrivateDocumentEntryCreated)
        /// The entry may already be in the store.
        case unconfirmed(String)
        /// The network does not serve private stores yet.
        case notActive
        /// Nothing was created; shown as Rust gave it.
        case failed(String)
    }

    var body: some View {
        Form {
            walletSection
            senderSection
            recipientSection
            authSection
            fieldsSection
            costSection
            submitSection
            if let outcome {
                outcomeSection(outcome)
            }
        }
        .navigationTitle("New \(documentTypeName) Entry")
        .navigationBarTitleDisplayMode(.inline)
        .interactiveDismissDisabled(isSubmitting)
        .navigationBarBackButtonHidden(isSubmitting)
        .onAppear(perform: ensureDefaultSelection)
        .onChange(of: wallets) {
            ensureDefaultSelection()
        }
        .task {
            await loadFee()
        }
    }

    // MARK: - Sections

    @ViewBuilder
    private var walletSection: some View {
        Section {
            if wallets.isEmpty {
                Text("No wallets on this network. Create or import a wallet first.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker(
                    "Wallet",
                    selection: Binding<Data?>(
                        get: { selectedWalletId },
                        set: { newId in
                            selectedWalletId = newId
                            selectedSenderId = senders(of: newId).first?.identityId
                            outcome = nil
                        }
                    )
                ) {
                    ForEach(wallets, id: \.walletId) { wallet in
                        Text("\(wallet.label) \(fingerprint(wallet.walletId))")
                            .tag(Optional(wallet.walletId))
                    }
                }
                .pickerStyle(.menu)
                .disabled(isSubmitting)
                .accessibilityIdentifier("privateEntryCreate.walletPicker")
                if let walletId = selectedWalletId, walletManager.wallet(for: walletId) == nil {
                    Text("This wallet is not loaded yet.")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                if let balance = shieldedBalance {
                    InfoRow(label: "Shielded balance (account 0):", value: "\(balance) credits")
                        .font(.caption)
                        .accessibilityIdentifier("privateEntryCreate.shieldedBalance")
                }
            }
        } header: {
            Text("Pay From")
        } footer: {
            Text("The entry is appended by an anonymous transition paid from this wallet's shielded account 0: nothing on chain names the sender or the recipient.")
        }
    }

    @ViewBuilder
    private var senderSection: some View {
        Section {
            let senders = senders(of: selectedWalletId)
            if senders.isEmpty {
                Text("This wallet holds no identity on this network.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker("Sender", selection: $selectedSenderId) {
                    ForEach(senders, id: \.identityId) { identity in
                        Text(identity.displayName).tag(Optional(identity.identityId))
                    }
                }
                .pickerStyle(.menu)
                .disabled(isSubmitting)
                .accessibilityIdentifier("privateEntryCreate.senderPicker")
            }
        } header: {
            Text("Sender")
        } footer: {
            Text("The entry is sealed with one of the sender's encryption keys, so this wallet's Read Entries recovers it as sent. An identity without an encryption key cannot send entries.")
        }
    }

    @ViewBuilder
    private var recipientSection: some View {
        Section {
            HStack {
                TextField("Recipient identity ID (base58)", text: $recipientText)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                    .disabled(isSubmitting)
                    .accessibilityIdentifier("privateEntryCreate.recipientId")
                if !localIdentities.isEmpty {
                    Menu {
                        ForEach(localIdentities, id: \.identityId) { identity in
                            Button(identity.displayName) {
                                recipientText = identity.identityIdBase58
                            }
                        }
                    } label: {
                        Image(systemName: "person.crop.circle")
                    }
                    .disabled(isSubmitting)
                    .accessibilityIdentifier("privateEntryCreate.localRecipientMenu")
                }
            }
            .onChange(of: recipientText) {
                // Keys read for another identity do not apply to this one.
                if recipientKeysFor != recipientId {
                    recipientKeys = nil
                    recipientKeysError = nil
                    selectedRecipientKeyId = nil
                }
            }

            Button {
                loadRecipientKeys()
            } label: {
                HStack {
                    if isLoadingKeys {
                        ProgressView().controlSize(.small)
                    }
                    Text("Load Keys")
                }
            }
            .disabled(recipientId == nil || isLoadingKeys || isSubmitting)
            .accessibilityIdentifier("privateEntryCreate.loadKeysButton")

            if let recipientKeysError {
                Text(recipientKeysError)
                    .font(.caption)
                    .foregroundColor(.red)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("privateEntryCreate.recipientKeysError")
            } else if let recipientKeys {
                if recipientKeys.isEmpty {
                    Text("This identity has no enabled encryption or decryption key, so no entry can be sealed to it.")
                        .font(.caption)
                        .foregroundColor(.orange)
                        .accessibilityIdentifier("privateEntryCreate.noRecipientKeys")
                } else {
                    Picker("Key", selection: $selectedRecipientKeyId) {
                        ForEach(recipientKeys, id: \.keyId) { key in
                            Text("Key \(key.keyId) (\(key.purposeName))").tag(Optional(key.keyId))
                        }
                    }
                    .pickerStyle(.menu)
                    .disabled(isSubmitting)
                    .accessibilityIdentifier("privateEntryCreate.recipientKeyPicker")
                }
            }
        } header: {
            Text("Recipient")
        } footer: {
            Text("The entry is sealed to one key of the recipient. Its owner reads the entry with that key.")
        }
    }

    private var authSection: some View {
        Section {
            Picker("Sender shown", selection: $auth) {
                Text("Anonymous").tag(PrivateDocumentSenderAuth.anonymous)
                Text("Deniable MAC").tag(PrivateDocumentSenderAuth.deniableMac)
            }
            .pickerStyle(.segmented)
            .disabled(isSubmitting)
            .accessibilityIdentifier("privateEntryCreate.authPicker")
        } header: {
            Text("Authentication")
        } footer: {
            switch auth {
            case .anonymous:
                Text("The recipient learns nothing of who wrote the entry.")
            case .deniableMac:
                Text("The entry names the sender and its encryption key. The recipient can tell who wrote it but cannot prove that to anyone else.")
            }
        }
    }

    private var fieldsSection: some View {
        Section {
            DocumentFieldsView(documentType: documentType, fieldValues: $fieldValues)
                .disabled(isSubmitting)
                .accessibilityIdentifier("privateEntryCreate.fields")
        } header: {
            Text("Properties")
        } footer: {
            if let required = documentType.requiredFields, !required.isEmpty {
                Text("Required: \(required.joined(separator: ", "))")
            }
        }
    }

    @ViewBuilder
    private var costSection: some View {
        Section("Cost") {
            if let fee {
                LabeledContent("Fee (at least)", value: "\(fee) credits")
                    .accessibilityIdentifier("privateEntryCreate.fee")
            } else if let feeError {
                Text(feeError)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("privateEntryCreate.feeError")
            } else {
                ProgressView()
            }
            Text("Paid from the shielded notes when one or two notes are spent; each further note adds to it.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small)
                        Text("Proving and broadcasting…")
                    } else {
                        Text("Create Entry")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isSubmitting)
            .accessibilityIdentifier("privateEntryCreate.createButton")
        }
    }

    @ViewBuilder
    private func outcomeSection(_ outcome: Outcome) -> some View {
        Section {
            switch outcome {
            case .created(let created):
                VStack(alignment: .leading, spacing: 8) {
                    Label("Entry created", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    InfoRow(label: "Commitment:", value: created.commitment.toHexString())
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                    InfoRow(label: "Sealed with sender key:", value: "\(created.senderKeyId)")
                        .font(.caption)
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("privateEntryCreate.success")
                NavigationLink {
                    PrivateDocumentEntriesView(
                        contractId: contractId,
                        documentTypeName: documentTypeName
                    )
                    .environmentObject(appState)
                    .environmentObject(walletManager)
                } label: {
                    Label("Read Entries", systemImage: "envelope.open")
                }
                .accessibilityIdentifier("privateEntryCreate.readEntriesLink")
            case .unconfirmed(let message):
                Label(message, systemImage: "hourglass")
                    .font(.caption)
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("privateEntryCreate.unconfirmed")
            case .notActive:
                Label(
                    "This network does not serve private document stores yet: they arrive with protocol version 15.",
                    systemImage: "clock.badge.exclamationmark"
                )
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("privateEntryCreate.notActive")
            case .failed(let message):
                Text(message)
                    .font(.caption)
                    .foregroundColor(.red)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("privateEntryCreate.error")
            }
        }
    }

    // MARK: - Actions

    private func loadFee() async {
        do {
            fee = try await walletManager.privateDocumentCreateFee(
                contractId: contractId,
                documentTypeName: documentTypeName,
                entryCount: 1
            )
            feeError = nil
        } catch {
            fee = nil
            feeError = "Fee not known: \(error.localizedDescription)"
        }
    }

    private func loadRecipientKeys() {
        guard let recipient = recipientId else { return }
        isLoadingKeys = true
        recipientKeysError = nil
        Task {
            defer { isLoadingKeys = false }
            do {
                let keys = try await walletManager.privateDocumentRecipientKeys(identityId: recipient)
                // The user typed another id while this read ran
                guard recipient == recipientId else { return }
                recipientKeys = keys
                recipientKeysFor = recipient
                selectedRecipientKeyId = keys.first?.keyId
            } catch {
                guard recipient == recipientId else { return }
                recipientKeys = nil
                recipientKeysFor = nil
                selectedRecipientKeyId = nil
                recipientKeysError = error.localizedDescription
            }
        }
    }

    private func submit() {
        guard
            let walletId = selectedWalletId,
            let senderId = selectedSenderId,
            let recipient = recipientKeysFor,
            let recipientKeyId = selectedRecipientKeyId
        else { return }
        let propertiesJSON: String
        do {
            propertiesJSON = try CreateDocumentView.propertiesJSON(
                from: fieldValues,
                documentType: documentType
            )
        } catch {
            outcome = .failed("Could not encode the properties: \(error.localizedDescription)")
            return
        }
        let auth = self.auth
        let contractId = self.contractId
        let typeName = documentTypeName
        isSubmitting = true
        outcome = nil
        Task {
            defer { isSubmitting = false }
            do {
                // Per-operation resolver: it supplies the sender's
                // encryption key and the spend authority for this call only.
                let created = try await walletManager.createPrivateDocumentEntry(
                    walletId: walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    contractId: contractId,
                    documentTypeName: typeName,
                    senderIdentityId: senderId,
                    recipientIdentityId: recipient,
                    recipientKeyId: recipientKeyId,
                    auth: auth,
                    propertiesJSON: propertiesJSON
                )
                outcome = .created(created)
            } catch let walletError as PlatformWalletError
                where SendViewModel.mayHaveGoneThrough(walletError) {
                // Rust keeps the notes reserved until a shielded sync sees
                // the transition: another tap would pay for a second entry.
                outcome = .unconfirmed("The entry may have gone through; a shielded sync settles it. Do not create it again. Read Entries shows it once it is in the store.")
            } catch let walletError as PlatformWalletError {
                switch walletError {
                case .protocolVersionNotActive:
                    outcome = .notActive
                default:
                    outcome = .failed(walletError.localizedDescription)
                }
            } catch {
                outcome = .failed(error.localizedDescription)
            }
        }
    }

    // MARK: - Derived state

    private var wallets: [PersistentWallet] {
        let raw = appState.currentNetwork.rawValue
        return allWallets.filter { $0.networkRaw == raw }
    }

    /// The identities of the wallet `walletId` on the active network.
    private func senders(of walletId: Data?) -> [PersistentIdentity] {
        guard let walletId else { return [] }
        let raw = appState.currentNetwork.rawValue
        return identities.filter { $0.wallet?.walletId == walletId && $0.networkRaw == raw }
    }

    /// The device's identities on the active network, to fill the recipient.
    private var localIdentities: [PersistentIdentity] {
        let raw = appState.currentNetwork.rawValue
        return identities.filter { $0.networkRaw == raw }
    }

    /// The typed recipient as a 32-byte identifier, or nil.
    private var recipientId: Data? {
        let trimmed = recipientText.trimmingCharacters(in: .whitespacesAndNewlines)
        if let data = Data.identifier(fromBase58: trimmed), data.count == 32 {
            return data
        }
        if let data = Data(hexString: trimmed), data.count == 32 {
            return data
        }
        return nil
    }

    private var shieldedBalance: UInt64? {
        guard let walletId = selectedWalletId else { return nil }
        return walletManager.spendableShieldedCredits(walletId: walletId, account: 0)
    }

    private var canSubmit: Bool {
        guard let walletId = selectedWalletId else { return false }
        return walletManager.wallet(for: walletId) != nil
            && selectedSenderId != nil
            && recipientKeysFor != nil
            && recipientKeysFor == recipientId
            && selectedRecipientKeyId != nil
    }

    private func ensureDefaultSelection() {
        if let current = selectedWalletId, wallets.contains(where: { $0.walletId == current }) {
            return
        }
        selectedWalletId = wallets.first?.walletId
        selectedSenderId = senders(of: selectedWalletId).first?.identityId
    }

    private func fingerprint(_ walletId: Data) -> String {
        let hex = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return hex.isEmpty ? "" : "(\(hex))"
    }
}
