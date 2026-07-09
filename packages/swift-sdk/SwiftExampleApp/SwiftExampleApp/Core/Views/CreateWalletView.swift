import SwiftUI
import SwiftData
import SwiftDashSDK

struct CreateWalletView: View {
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var walletManagerStore: WalletManagerStore
    @EnvironmentObject var platformState: AppState

    @State private var walletLabel: String = ""
    @State private var showImportOption: Bool = false
    @State private var importMnemonic: String = ""
    @State private var importBirthHeight: String = ""
    @State private var walletPin: String = ""
    @State private var confirmPin: String = ""
    @State private var isCreating: Bool = false
    @State private var error: Error? = nil
    @FocusState private var focusedField: Field?

    // Seed backup flow
    @State private var showBackupScreen: Bool = false
    @State private var generatedMnemonic: String = ""
    @State private var selectedWordCount: Int = 12

    // Network selection states
    @State private var createForMainnet: Bool = false
    @State private var createForTestnet: Bool = false
    @State private var createForRegtest: Bool = false
    @State private var createForDevnet: Bool = false

    enum Field: Hashable {
        case walletName
        case pin
        case confirmPin
        case mnemonic
    }

    var currentNetwork: Network {
        platformState.currentNetwork
    }

    // Only show devnet option if currently on devnet
    var shouldShowDevnet: Bool {
        currentNetwork == .devnet
    }

    // Mirror of the devnet rule: regtest is a developer-only
    // network and the toggle would clutter the create screen on
    // mainnet/testnet. Showing it only when the active network is
    // regtest also matches the wallet-info network picker pattern.
    var shouldShowRegtest: Bool {
        currentNetwork == .regtest
    }

    var body: some View {
        Form {
            Section {
                TextField("Wallet Name", text: $walletLabel)
                    .textInputAutocapitalization(.words)
                    .focused($focusedField, equals: .walletName)
                    .submitLabel(.next)
                    .accessibilityIdentifier("createWallet.walletNameField")
                    .onSubmit {
                        focusedField = .pin
                    }
            } header: {
                Text("Wallet Information")
            }

            Section {
                VStack(alignment: .leading, spacing: 12) {
                    Text("Create wallet for:")
                        .font(.subheadline)
                        .foregroundColor(.secondary)

                    // Always show Mainnet and Testnet
                    Toggle(isOn: $createForMainnet) {
                        HStack {
                            Image(systemName: "network")
                                .foregroundColor(.orange)
                            Text("Mainnet")
                                .font(.body)
                        }
                    }
                    .toggleStyle(CheckboxToggleStyle())

                    Toggle(isOn: $createForTestnet) {
                        HStack {
                            Image(systemName: "network")
                                .foregroundColor(.blue)
                            Text("Testnet")
                                .font(.body)
                        }
                    }
                    .toggleStyle(CheckboxToggleStyle())

                    // Only show Devnet if currently on Devnet
                    if shouldShowDevnet {
                        Toggle(isOn: $createForDevnet) {
                            HStack {
                                Image(systemName: "network")
                                    .foregroundColor(.green)
                                Text("Devnet")
                                    .font(.body)
                            }
                        }
                        .toggleStyle(CheckboxToggleStyle())
                    }

                    // Mirror Devnet's gating: only render the local
                    // (regtest) toggle when the user is actually on
                    // regtest. Without this row there's no path to
                    // create a regtest wallet from the active-network
                    // creation flow.
                    if shouldShowRegtest {
                        Toggle(isOn: $createForRegtest) {
                            HStack {
                                Image(systemName: "network")
                                    .foregroundColor(.purple)
                                Text("Local (Regtest)")
                                    .font(.body)
                            }
                        }
                        .toggleStyle(CheckboxToggleStyle())
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Networks")
            } footer: {
                Text("Select which networks to create wallets for. The same seed will be used for all selected networks.")
            }

            Section {
                HStack {
                    Text("PIN:")
                        .frame(width: 100, alignment: .leading)
                    SecureField("4-6 digits", text: $walletPin)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
                        .autocorrectionDisabled()
                        .focused($focusedField, equals: .pin)
                        .accessibilityIdentifier("createWallet.pinField")
                }

                HStack {
                    Text("Confirm PIN:")
                        .frame(width: 100, alignment: .leading)
                    SecureField("4-6 digits", text: $confirmPin)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
                        .autocorrectionDisabled()
                        .focused($focusedField, equals: .confirmPin)
                        .accessibilityIdentifier("createWallet.confirmPinField")
                }
            } header: {
                Text("Security")
            } footer: {
                Text("Choose a PIN to secure your wallet (4-6 digits)")
            }

            Section {
                Toggle("Import Existing Wallet", isOn: $showImportOption)
            } header: {
                Text("Options")
            }

            if !showImportOption {
                Section {
                    Picker("Word Count", selection: $selectedWordCount) {
                        Text("12 words").tag(12)
                        Text("15 words").tag(15)
                        Text("18 words").tag(18)
                        Text("21 words").tag(21)
                        Text("24 words").tag(24)
                    }
                    .pickerStyle(.menu)
                } header: {
                    Text("Seed Phrase Length")
                } footer: {
                    Text("Choose the number of words for the generated recovery phrase.")
                }
            }

            if showImportOption {
                Section {
                    TextField("Enter recovery phrase (12–24 words)", text: $importMnemonic, axis: .vertical)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .lineLimit(3...6)
                        .focused($focusedField, equals: .mnemonic)
                } header: {
                    Text("Recovery Phrase")
                } footer: {
                    Text("Enter your 12-word recovery phrase separated by spaces")
                }

                Section {
                    TextField("Birth height (optional)", text: $importBirthHeight)
                        .keyboardType(.numberPad)
                } header: {
                    Text("Birth Height")
                } footer: {
                    Text("Block height the wallet first received funds. Sync anchors at the nearest checkpoint at or before it, avoiding a full-chain scan. Leave empty to scan from genesis.")
                }
            }
        }
        .navigationTitle("Create Wallet")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarLeading) {
                Button("Cancel") {
                    dismiss()
                }
            }

            ToolbarItem(placement: .navigationBarTrailing) {
                Button("Create") {
                    onCreateTapped()
                }
                .disabled(!canCreateWallet)
                .accessibilityIdentifier("createWallet.createButton")
            }
        }
        .disabled(isCreating)
        .alert("Wallet Created", isPresented: .constant(false)) {
            Button("OK") { }
        } message: {
            Text("Wallet created successfully")
        }
        .alert("Error", isPresented: .constant(error != nil)) {
            Button("OK") {
                error = nil
            }
        } message: {
            if let error = error {
                Text(error.localizedDescription)
            }
        }
        .onAppear {
            setupInitialNetworkSelection()
        }
        // Navigate to backup screen when requested (iOS 16+ API)
        .navigationDestination(isPresented: $showBackupScreen) {
            SeedBackupView(
                mnemonic: generatedMnemonic,
                onConfirm: {
                    createWallet(using: generatedMnemonic)
                }
            )
        }
    }

    private var canCreateWallet: Bool {
        !walletLabel.isEmpty &&
        !walletPin.isEmpty &&
        walletPin == confirmPin &&
        !isCreating &&
        hasNetworkSelected
    }

    private var hasNetworkSelected: Bool {
        // Mirror the same visibility gates used when building
        // `selectedNetworks` below — without this, a stale
        // `createForRegtest`/`createForDevnet` flag set by
        // `setupInitialNetworkSelection()` could leave the Create
        // button enabled while `selectedNetworks` ends up empty,
        // surfacing a `"No network selected"` error after the tap.
        createForMainnet ||
        createForTestnet ||
        (createForDevnet && shouldShowDevnet) ||
        (createForRegtest && shouldShowRegtest)
    }

    private func setupInitialNetworkSelection() {
        // Set the current network as selected by default
        switch currentNetwork {
        case .mainnet:
            createForMainnet = true
        case .testnet:
            createForTestnet = true
        case .regtest:
            createForRegtest = true
        case .devnet:
            createForDevnet = true
        }
    }

    private func onCreateTapped() {
        // If importing, go straight to creation with provided mnemonic
        if showImportOption {
            createWallet(using: importMnemonic)
            return
        }
        // Otherwise, generate and show backup/confirmation screen
        do {
            generatedMnemonic = try SwiftDashSDK.Mnemonic.generate(wordCount: UInt32(selectedWordCount))
            showBackupScreen = true
        } catch {
            self.error = error
        }
    }

    private func createWallet(using mnemonic: String) {
        guard !walletLabel.isEmpty,
              walletPin == confirmPin,
              walletPin.count >= 4 && walletPin.count <= 6 else {
            print("=== WALLET CREATION VALIDATION FAILED ===")
            print("Label empty: \(walletLabel.isEmpty)")
            print("PINs match: \(walletPin == confirmPin)")
            print("PIN length valid: \(walletPin.count >= 4 && walletPin.count <= 6)")
            return
        }

        isCreating = true

        Task {
            do {
                print("=== STARTING WALLET CREATION ===")

                let mnemonicPhrase = (showImportOption ? importMnemonic : mnemonic)
                print("PIN length: \(walletPin.count)")
                print("Import option enabled: \(showImportOption)")

                let selectedNetworks: [Network] = [
                    createForMainnet ? Network.mainnet : nil,
                    createForTestnet ? Network.testnet : nil,
                    (createForDevnet && shouldShowDevnet) ? Network.devnet : nil,
                    (createForRegtest && shouldShowRegtest) ? Network.regtest : nil,
                ].compactMap { $0 }

                guard !selectedNetworks.isEmpty else {
                    struct MissingNetwork: LocalizedError {
                        var errorDescription: String? { "No network selected" }
                    }
                    throw MissingNetwork()
                }

                // Create the wallet in EVERY ticked network. Each
                // network has its own `PlatformWalletManager` (the
                // Rust manager is network-locked at construction and
                // stamps its own network onto the wallet), so routing
                // through the active manager alone would ignore the
                // passed `network`. `backgroundManager(for:)` returns
                // the warm cached manager for the active network and
                // builds one on demand for the others. The `walletId`
                // is now network-scoped — the same mnemonic produces a
                // DIFFERENT id per network — so the Keychain mnemonic +
                // metadata must be written under EACH freshly-created
                // network's id, and `isImported` stamped on each id's
                // row.
                try await MainActor.run {
                    // Per-network results for networks the wallet was
                    // FRESHLY created on this pass — each carries the
                    // scoped `walletId` Rust returned, which is the
                    // Keychain key its mnemonic / metadata / `isImported`
                    // writes hang off of.
                    var createdWallets: [(network: Network, walletId: Data)] = []
                    // Real (non-"already exists") failures, surfaced to
                    // the user so a partial create isn't reported as
                    // success.
                    var failures: [(network: Network, message: String)] = []
                    // For an imported wallet, honour a user-entered birth height:
                    // sync anchors at the nearest checkpoint at or before it,
                    // avoiding a full-chain scan. Left empty it falls back to
                    // genesis (0) so a wallet of unknown age still sees all its
                    // history. A freshly generated wallet passes nil (tip).
                    let trimmedImportBirthHeight = importBirthHeight.trimmingCharacters(in: .whitespaces)
                    let importBirth = UInt32(trimmedImportBirthHeight) ?? 0
                    // Birth height is chain-local: a single value can't be
                    // correct for more than one network, so reject a non-empty
                    // height when importing to multiple networks. Left blank the
                    // safe genesis (0) fallback still applies to each network.
                    if showImportOption, !trimmedImportBirthHeight.isEmpty, selectedNetworks.count > 1 {
                        struct MultiNetworkBirthHeightUnsupported: LocalizedError {
                            var errorDescription: String? {
                                "Birth height is network-specific. Select one network or leave birth height empty when importing to multiple networks."
                            }
                        }
                        throw MultiNetworkBirthHeightUnsupported()
                    }
                    for net in selectedNetworks {
                        do {
                            let mgr = try walletManagerStore.backgroundManager(for: net)
                            let managed = try mgr.createWallet(
                                mnemonic: mnemonicPhrase,
                                network: net,
                                name: walletLabel,
                                birthHeight: showImportOption ? importBirth : nil
                            )
                            createdWallets.append((net, managed.walletId))
                        } catch {
                            // A typed `walletAlreadyExists` throw means the
                            // wallet is already on this network — benign. We
                            // do NOT resolve the existing scoped walletId to
                            // re-store the mnemonic: a wallet that already
                            // exists on this network had its mnemonic +
                            // metadata stored under that scoped id at its
                            // original creation, so there is nothing to
                            // write. It is also not counted as a freshly-
                            // created wallet. Any other error is a genuine
                            // failure.
                            if case PlatformWalletError.walletAlreadyExists = error {
                                SDKLogger.error(
                                    "Wallet already present on \(net.displayName); continuing"
                                )
                            } else {
                                let message = error.localizedDescription
                                failures.append((net, message))
                                SDKLogger.error(
                                    "Wallet creation failed for \(net.displayName): \(message)"
                                )
                            }
                        }
                    }

                    guard !createdWallets.isEmpty else {
                        // No wallet was freshly created. Two cases:
                        if failures.isEmpty {
                            // Every selected network reported "already
                            // exists" — the wallet is present on all of
                            // them and its per-network mnemonic/metadata
                            // were stored at the original creation.
                            // Re-importing is a benign no-op; dismiss
                            // without a misleading "could not be created"
                            // error.
                            dismiss()
                            return
                        }
                        // At least one network had a real failure and
                        // none succeeded — surface the failure detail.
                        struct AllNetworksFailed: LocalizedError {
                            let detail: String
                            var errorDescription: String? {
                                "Wallet could not be created on any selected network.\n\(detail)"
                            }
                        }
                        let detail = failures
                            .map { "\($0.network.displayName): \($0.message)" }
                            .joined(separator: "\n")
                        throw AllNetworksFailed(detail: detail)
                    }

                    // For EACH freshly-created network, persist that
                    // network's scoped walletId independently: store the
                    // mnemonic in the iOS Keychain keyed by that id (so
                    // the recovery flow can enumerate it on launch), stamp
                    // `isImported` on its row, and mirror the wallet
                    // metadata under that id. Each scoped wallet is
                    // independently recoverable, so its metadata records
                    // just THAT network. All writes are best-effort —
                    // failures are logged, not fatal.
                    let storage = WalletStorage()
                    for created in createdWallets {
                        let walletId = created.walletId

                        do {
                            try storage.storeMnemonic(mnemonicPhrase, for: walletId)
                        } catch {
                            SDKLogger.error(
                                "Failed to persist mnemonic to keychain for \(created.network.displayName): \(error.localizedDescription)"
                            )
                        }

                        // Stamp `isImported` on the per-network row for
                        // this scoped walletId. The persister callbacks
                        // run synchronously from `createWallet` via the
                        // background contexts; autosave propagates the
                        // rows into the main context before this fetch.
                        let descriptor = FetchDescriptor<PersistentWallet>(
                            predicate: PersistentWallet.predicate(walletId: walletId)
                        )
                        let rows = (try? modelContext.fetch(descriptor)) ?? []
                        for row in rows {
                            row.isImported = showImportOption
                        }
                        if !rows.isEmpty {
                            try? modelContext.save()
                        }

                        // Mirror name + birth height + just THIS network
                        // into the keychain alongside the mnemonic so an
                        // orphan-recovery after a wipe restores the
                        // original label / network / birth height for this
                        // scoped wallet.
                        do {
                            let metadata = WalletKeychainMetadata(
                                name: walletLabel,
                                walletDescription: nil,
                                networks: [created.network.networkName],
                                birthHeight: rows.first?.birthHeight
                            )
                            try storage.setMetadata(metadata, for: walletId)
                        } catch {
                            SDKLogger.error(
                                "Failed to persist wallet metadata to keychain for \(created.network.displayName): \(error.localizedDescription)"
                            )
                        }
                    }

                    // If some (but not all) networks failed, the wallet
                    // exists — but the user must know it wasn't added
                    // everywhere they ticked. Surface the partial
                    // failure instead of silently dismissing as success.
                    if !failures.isEmpty {
                        struct PartialCreate: LocalizedError {
                            let detail: String
                            var errorDescription: String? {
                                "Wallet created, but not on every selected network:\n\(detail)"
                            }
                        }
                        let detail = failures
                            .map { "\($0.network.displayName): \($0.message)" }
                            .joined(separator: "\n")
                        throw PartialCreate(detail: detail)
                    }

                    dismiss()
                }

                print("=== WALLET CREATION SUCCESS - networks: \(selectedNetworks.map { $0.displayName }) ===")
            } catch {
                print("=== WALLET CREATION ERROR ===")
                print("Error: \(error)")

                await MainActor.run {
                    self.error = error
                    isCreating = false
                    // Pop the pushed `SeedBackupView` so the error alert
                    // (bound to this view) is actually visible — otherwise
                    // the backup screen sits on top with its submit button
                    // stuck disabled and no feedback.
                    showBackupScreen = false
                }
            }
        }
    }
}

// Custom checkbox style for better visual
struct CheckboxToggleStyle: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack {
            Image(systemName: configuration.isOn ? "checkmark.square.fill" : "square")
                .foregroundColor(configuration.isOn ? .blue : .secondary)
                .onTapGesture {
                    configuration.isOn.toggle()
                }

            configuration.label

            Spacer()
        }
    }
}

struct CreateWalletView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            CreateWalletView()
        }
    }
}
