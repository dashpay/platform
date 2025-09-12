import SwiftUI
import SwiftDashSDK

struct CreateWalletView: View {
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    
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
    @State private var createForDevnet: Bool = false
    
    enum Field: Hashable {
        case walletName
        case pin
        case confirmPin
        case mnemonic
    }
    
    var currentNetwork: Network {
        unifiedAppState.platformState.currentNetwork
    }
    
    // Only show devnet option if currently on devnet
    var shouldShowDevnet: Bool {
        currentNetwork == .devnet
    }
    
    var body: some View {
        Form {
            Section {
                TextField("Wallet Name", text: $walletLabel)
                    .textInputAutocapitalization(.words)
                    .focused($focusedField, equals: .walletName)
                    .submitLabel(.next)
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
                }
                
                HStack {
                    Text("Confirm PIN:")
                        .frame(width: 100, alignment: .leading)
                    SecureField("4-6 digits", text: $confirmPin)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
                        .autocorrectionDisabled()
                        .focused($focusedField, equals: .confirmPin)
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
        createForMainnet || createForTestnet || createForDevnet
    }
    
    private func setupInitialNetworkSelection() {
        // Set the current network as selected by default
        switch currentNetwork {
        case .mainnet:
            createForMainnet = true
        case .testnet:
            createForTestnet = true
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

    private func createWallet(using mnemonic: String?) {
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
                
                let mnemonic: String? = (showImportOption ? importMnemonic : mnemonic)
                print("Has mnemonic: \(mnemonic != nil)")
                print("PIN length: \(walletPin.count)")
                print("Import option enabled: \(showImportOption)")
                
                // Determine primary network to create the wallet in (SDK enforces unique wallet per mnemonic)
                let selectedNetworks: [Network] = [
                    createForMainnet ? Network.mainnet : nil,
                    createForTestnet ? Network.testnet : nil,
                    (createForDevnet && shouldShowDevnet) ? Network.devnet : nil,
                ].compactMap { $0 }

                guard let primaryNetwork = selectedNetworks.first else {
                    throw WalletError.walletError("No network selected")
                }

                // Create exactly one wallet in the SDK; do not append network to label
                let wallet = try await walletService.createWallet(
                    label: walletLabel,
                    mnemonic: mnemonic,
                    pin: walletPin,
                    network: primaryNetwork,
                    networks: selectedNetworks
                )

                // Update wallet.networks bitfield to reflect all user selections
                var networksBitfield: UInt32 = 0
                if createForMainnet { networksBitfield |= 1 }
                if createForTestnet { networksBitfield |= 2 }
                if createForDevnet && shouldShowDevnet { networksBitfield |= 8 }
                wallet.networks = networksBitfield
                try? modelContext.save()

                print("=== WALLET CREATION SUCCESS - Created 1 wallet for \(primaryNetwork.displayName) ===")
                
                await MainActor.run {
                    dismiss()
                }
            } catch {
                print("=== WALLET CREATION ERROR ===")
                print("Error: \(error)")
                
                await MainActor.run {
                    self.error = error
                    isCreating = false
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
