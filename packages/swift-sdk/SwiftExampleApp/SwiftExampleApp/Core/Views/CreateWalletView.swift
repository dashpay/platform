import SwiftUI

struct CreateWalletView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var walletService: WalletService
    
    @State private var walletLabel: String = ""
    @State private var showImportOption: Bool = false
    @State private var importMnemonic: String = ""
    @State private var walletPin: String = ""
    @State private var confirmPin: String = ""
    @State private var isCreating: Bool = false
    @State private var error: Error? = nil
    @FocusState private var focusedField: Field?
    
    enum Field: Hashable {
        case walletName
        case pin
        case confirmPin
        case mnemonic
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
                .disabled(walletLabel.isEmpty || walletPin.isEmpty || walletPin != confirmPin || isCreating)
            }
        }
        .disabled(isCreating)
        .overlay {
            if isCreating {
                ProgressView("Creating wallet...")
                    .padding()
                    .background(Color.gray.opacity(0.9))
                    .cornerRadius(10)
            }
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
    }
    
    private func createWallet() {
        guard !walletLabel.isEmpty else {
            error = WalletError.notImplemented("Wallet name is required")
            return
        }
        
        guard !walletPin.isEmpty else {
            error = WalletError.notImplemented("PIN is required")
            return
        }
        
        guard walletPin == confirmPin else {
            error = WalletError.notImplemented("PINs do not match")
            return
        }
        
        guard walletPin.count >= 4 && walletPin.count <= 6 else {
            error = WalletError.notImplemented("PIN must be 4-6 digits")
            return
        }
        
        isCreating = true
        
        Task {
            do {
                let mnemonic = showImportOption && !importMnemonic.isEmpty ? importMnemonic : nil
                print("=== WALLET CREATION START ===")
                print("Label: \(walletLabel)")
                print("Has mnemonic: \(mnemonic != nil)")
                print("PIN length: \(walletPin.count)")
                print("Import option enabled: \(showImportOption)")
                
                let wallet = try await walletService.createWallet(label: walletLabel, mnemonic: mnemonic, pin: walletPin)
                
                print("Wallet created successfully: \(wallet.id)")
                print("=== WALLET CREATION SUCCESS ===")
                
                await MainActor.run {
                    dismiss()
                }
            } catch {
                print("=== WALLET CREATION FAILED ===")
                print("Error type: \(type(of: error))")
                print("Error: \(error)")
                print("Error localized: \(error.localizedDescription)")
                
                await MainActor.run {
                    self.error = error
                    self.isCreating = false
                }
            }
        }
    }
}