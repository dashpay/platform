import SwiftUI
import SwiftDashSDK

struct KeyDetailView: View {
    let identity: IdentityModel
    let publicKey: IdentityPublicKey
    @State private var privateKeyInput = ""
    @State private var isValidating = false
    @State private var validationError: String?
    @State private var showSuccessAlert = false
    @State private var showForgetKeyAlert = false
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState
    
    var hasPrivateKey: Bool {
        let result = KeychainManager.shared.hasPrivateKey(identityId: identity.id, keyIndex: Int32(publicKey.id))
        print("🔑 KeyDetailView: hasPrivateKey for key \(publicKey.id) = \(result)")
        return result
    }
    
    var body: some View {
        Form {
            // Key Information Section
            Section("Key Information") {
                HStack {
                    Text("Key ID")
                    Spacer()
                    Text("#\(publicKey.id)")
                        .fontWeight(.medium)
                }
                
                HStack {
                    Text("Purpose")
                    Spacer()
                    Text(publicKey.purpose.name)
                        .fontWeight(.medium)
                }
                
                HStack {
                    Text("Type")
                    Spacer()
                    Text(publicKey.keyType.name)
                        .fontWeight(.medium)
                }
                
                HStack {
                    Text("Security Level")
                    Spacer()
                    SecurityLevelBadge(level: publicKey.securityLevel)
                }
            }
            
            // Public Key Section
            Section("Public Key") {
                Text(publicKey.data.toHexString())
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
            }
            
            // Private Key Section
            if hasPrivateKey {
                Section("Private Key") {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("Private key is stored securely")
                    }
                    
                    Button(action: viewPrivateKey) {
                        Label("View Private Key", systemImage: "eye.fill")
                    }
                    
                    Button(action: { showForgetKeyAlert = true }) {
                        Label("Forget Private Key", systemImage: "trash")
                    }
                    .foregroundColor(.red)
                }
            } else {
                Section("Add Private Key") {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("Enter the private key for this public key")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        TextField("Private key (hex or WIF)", text: $privateKeyInput)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                        
                        if let error = validationError {
                            Text(error)
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }
                    
                    Button(action: validateAndStorePrivateKey) {
                        HStack {
                            if isValidating {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                            }
                            Text("Validate and Store")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(privateKeyInput.isEmpty || isValidating)
                }
            }
        }
        .navigationTitle("Key #\(publicKey.id)")
        .navigationBarTitleDisplayMode(.inline)
        .alert("Success", isPresented: $showSuccessAlert) {
            Button("OK") {
                dismiss()
            }
        } message: {
            Text("Private key validated and stored successfully")
        }
        .alert("Forget Private Key?", isPresented: $showForgetKeyAlert) {
            Button("Cancel", role: .cancel) {}
            Button("Forget", role: .destructive) {
                forgetPrivateKey()
            }
        } message: {
            Text("Are you sure you want to forget this private key? This action cannot be undone and you will need to re-enter the key to use it again.")
        }
    }
    
    private func viewPrivateKey() {
        // This will trigger the sheet presentation through the parent view
        // For now, we could show an alert or navigate to a secure view
    }
    
    private func validateAndStorePrivateKey() {
        isValidating = true
        validationError = nil
        
        Task {
                // Parse the private key input
                let trimmedInput = privateKeyInput.trimmingCharacters(in: .whitespacesAndNewlines)
                
                // Convert to Data (hex or WIF format)
                guard let privateKeyData = parsePrivateKey(trimmedInput) else {
                    await MainActor.run {
                        validationError = "Invalid private key format"
                        isValidating = false
                    }
                    return
                }
                
                // Ensure SDK exists
                guard appState.sdk != nil else {
                    await MainActor.run {
                        validationError = "SDK not initialized"
                        isValidating = false
                    }
                    return
                }
                
                // Get the public key data in the correct format
                let publicKeyHex: String
                if publicKey.keyType == .ecdsaHash160 || publicKey.keyType == .eddsa25519Hash160 {
                    // For hash160 types, the data is already the hash
                    publicKeyHex = publicKey.data.toHexString()
                } else {
                    // For other types, we need the full public key
                    publicKeyHex = publicKey.data.toHexString()
                }
                
                // Validate the private key matches the public key
                let isValid = KeyValidation.validatePrivateKeyForPublicKey(
                    privateKeyHex: privateKeyData.toHexString(),
                    publicKeyHex: publicKeyHex,
                    keyType: publicKey.keyType
                )
                
                if isValid {
                    // Store the private key
                    print("🔑 Storing private key for identity: \(identity.id.toHexString()), keyId: \(publicKey.id)")
                    let stored = KeychainManager.shared.storePrivateKey(
                        privateKeyData,
                        identityId: identity.id,
                        keyIndex: Int32(publicKey.id)
                    )
                    print("🔑 Storage result: \(stored != nil ? "Success" : "Failed")")
                    
                    await MainActor.run {
                        showSuccessAlert = true
                        isValidating = false
                    }
                } else {
                    await MainActor.run {
                        validationError = "Private key does not match the public key"
                        isValidating = false
                    }
                }
        }
    }
    
    private func parsePrivateKey(_ input: String) -> Data? {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        
        // Try hex first
        if let hexData = Data(hexString: trimmed) {
            // Validate it's 32 bytes for a private key
            if hexData.count == 32 {
                return hexData
            }
        }
        
        // Try WIF format
        if let wifData = WIFParser.parseWIF(trimmed) {
            return wifData
        }
        
        return nil
    }
    
    private func validateKeySize(_ privateKey: Data, for keyType: KeyType) -> Bool {
        switch keyType {
        case .ecdsaSecp256k1:
            return privateKey.count == 32 // 256 bits
        case .bls12_381:
            return privateKey.count == 32 // 256 bits
        case .ecdsaHash160:
            return privateKey.count == 32 // 256 bits for the actual key
        case .bip13ScriptHash:
            return privateKey.count == 32 // 256 bits
        case .eddsa25519Hash160:
            return privateKey.count == 32 // 256 bits
        }
    }
    
    private func forgetPrivateKey() {
        // Remove from keychain
        let removed = KeychainManager.shared.deletePrivateKey(identityId: identity.id, keyIndex: Int32(publicKey.id))
        
        if removed {
            // Update the persistent public key to clear the reference
            appState.removePrivateKeyReference(identityId: identity.id, keyId: Int32(publicKey.id))
            dismiss()
        }
    }
}
