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
                    SecureField("PIN (4-6 digits)", text: $walletPin)
                        .keyboardType(.numberPad)
                    SecureField("Confirm PIN", text: $confirmPin)
                        .keyboardType(.numberPad)
                } header: {
                    Text("Security")
                } footer: {
                    Text("Choose a PIN to secure your wallet")
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
        isCreating = true
        
        Task {
            do {
                let mnemonic = showImportOption && !importMnemonic.isEmpty ? importMnemonic : nil
                _ = try await walletService.createWallet(label: walletLabel, mnemonic: mnemonic, pin: walletPin)
                await MainActor.run {
                    dismiss()
                }
            } catch {
                await MainActor.run {
                    self.error = error
                    self.isCreating = false
                }
            }
        }
    }
}