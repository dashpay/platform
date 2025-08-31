import SwiftUI

struct CreateWalletView: View {
    @Environment(\.dismiss) var dismiss
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
            
            if showImportOption {
                Section {
                    TextField("Enter 12-word mnemonic phrase", text: $importMnemonic, axis: .vertical)
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
                    createWallet()
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
    
    private func createWallet() {
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
                
                let mnemonic: String? = showImportOption && !importMnemonic.isEmpty ? importMnemonic : nil
                print("Has mnemonic: \(mnemonic != nil)")
                print("PIN length: \(walletPin.count)")
                print("Import option enabled: \(showImportOption)")
                
                // Create wallets for selected networks
                var createdWalletCount = 0
                
                if createForMainnet {
                    let wallet = try await walletService.createWallet(
                        label: "\(walletLabel) (Mainnet)",
                        mnemonic: mnemonic,
                        pin: walletPin,
                        network: DashNetwork.mainnet
                    )
                    print("Mainnet wallet created: \(wallet.id)")
                    createdWalletCount += 1
                }
                
                if createForTestnet {
                    let wallet = try await walletService.createWallet(
                        label: "\(walletLabel) (Testnet)",
                        mnemonic: mnemonic,
                        pin: walletPin,
                        network: DashNetwork.testnet
                    )
                    print("Testnet wallet created: \(wallet.id)")
                    createdWalletCount += 1
                }
                
                if createForDevnet && shouldShowDevnet {
                    let wallet = try await walletService.createWallet(
                        label: "\(walletLabel) (Devnet)",
                        mnemonic: mnemonic,
                        pin: walletPin,
                        network: DashNetwork.devnet
                    )
                    print("Devnet wallet created: \(wallet.id)")
                    createdWalletCount += 1
                }
                
                print("=== WALLET CREATION SUCCESS - Created \(createdWalletCount) wallet(s) ===")
                
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