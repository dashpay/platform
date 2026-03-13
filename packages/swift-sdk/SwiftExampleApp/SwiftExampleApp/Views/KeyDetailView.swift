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
            // Parse the private key using centralized parser
            let parseResult = PrivateKeyParser.parse(privateKeyInput)

            guard let privateKeyData = parseResult.data else {
                await MainActor.run {
                    validationError = parseResult.error ?? "Invalid private key format"
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

            // Validate the private key matches the public key using centralized validator
            let validationResult = KeyValidator.validatePrivateKey(
                privateKeyData,
                against: [publicKey]
            )

            if validationResult.isValid {
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
                    validationError = validationResult.error ?? "Private key does not match the public key"
                    isValidating = false
                }
            }
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
