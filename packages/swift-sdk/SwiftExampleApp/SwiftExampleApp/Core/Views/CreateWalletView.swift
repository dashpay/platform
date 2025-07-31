import SwiftUI

struct CreateWalletView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletService: WalletService
    
    @State private var walletLabel = ""
    @State private var showImportOption = false
    @State private var importMnemonic = ""
    @State private var walletPin = ""
    @State private var confirmPin = ""
    @State private var isCreating = false
    @State private var error: Error?
    
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Wallet Name", text: $walletLabel)
                        .textInputAutocapitalization(.words)
                } header: {
                    Text("Wallet Information")
                }
                
                Section {
                    TextField("PIN (4-6 digits)", text: $walletPin)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
                    TextField("Confirm PIN", text: $confirmPin)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
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
    }
    
    private func createWallet() {
        // Validate inputs first
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
                print("Creating wallet with label: \(walletLabel), has mnemonic: \(mnemonic != nil), PIN length: \(walletPin.count)")
                
                _ = try await walletService.createWallet(label: walletLabel, mnemonic: mnemonic, pin: walletPin)
                await MainActor.run {
                    dismiss()
                }
            } catch {
                print("Wallet creation failed: \(error)")
                await MainActor.run {
                    self.error = error
                    self.isCreating = false
                }
            }
        }
    }
}