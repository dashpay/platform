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
                // builds one on demand for the others. `walletId` is
                // network-independent, so the Keychain mnemonic +
                // metadata are written once and the `isImported` flag
                // is stamped on every per-network row.
                try await MainActor.run {
                    var createdWalletId: Data?
                    for net in selectedNetworks {
                        do {
                            let mgr = try walletManagerStore.backgroundManager(for: net)
                            let managed = try mgr.createWallet(
                                mnemonic: mnemonicPhrase,
                                network: net,
                                name: walletLabel
                            )
                            createdWalletId = managed.walletId
                        } catch {
                            // An "already exists" throw for one network
                            // (wallet was previously created there) is
                            // non-fatal — keep creating the others.
                            SDKLogger.error(
                                "Wallet creation skipped for \(net.displayName): \(error.localizedDescription)"
                            )
                        }
                    }

                    guard let walletId = createdWalletId else {
                        struct AllNetworksFailed: LocalizedError {
                            var errorDescription: String? {
                                "Wallet could not be created on any selected network."
                            }
                        }
                        throw AllNetworksFailed()
                    }

                    // Persist the mnemonic in the iOS Keychain keyed
                    // by walletId (network-independent) so the
                    // recovery flow can enumerate wallets on launch.
                    // Best-effort — failure here doesn't block.
                    let storage = WalletStorage()
                    do {
                        try storage.storeMnemonic(mnemonicPhrase, for: walletId)
                    } catch {
                        SDKLogger.error(
                            "Failed to persist mnemonic to keychain: \(error.localizedDescription)"
                        )
                    }

                    // Stamp `isImported` on every per-network row for
                    // this walletId. The persister callbacks run
                    // synchronously from `createWallet` via the
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

                    // Mirror the name + ticked networks + birth height
                    // into the keychain alongside the mnemonic so an
                    // orphan-recovery after a wipe restores the
                    // original label / networks / birth height.
                    do {
                        let metadata = WalletKeychainMetadata(
                            name: walletLabel,
                            walletDescription: nil,
                            networks: selectedNetworks.map { $0.networkName },
                            birthHeight: rows.first?.birthHeight
                        )
                        try storage.setMetadata(metadata, for: walletId)
                    } catch {
                        SDKLogger.error(
                            "Failed to persist wallet metadata to keychain: \(error.localizedDescription)"
                        )
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
