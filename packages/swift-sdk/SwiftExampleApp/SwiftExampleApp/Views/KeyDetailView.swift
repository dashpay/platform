import SwiftUI
import SwiftDashSDK

struct KeyDetailView: View {
    let identity: PersistentIdentity
    let publicKey: IdentityPublicKey
    @State private var privateKeyInput = ""
    @State private var isValidating = false
    @State private var validationError: String?
    @State private var showSuccessAlert = false
    @State private var showForgetKeyAlert = false
    // Disable-key flow state.
    @State private var showDisableConfirm = false
    @State private var isDisabling = false
    @State private var disableError: String?
    @State private var forgetError: String?
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager

    var hasPrivateKey: Bool {
        let result = KeychainManager.shared.hasPrivateKey(identityId: identity.identityId, keyIndex: Int32(publicKey.id))
        print("🔑 KeyDetailView: hasPrivateKey for key \(publicKey.id) = \(result)")
        return result
    }

    /// Pre-flight gate for disabling this key. Evaluated against the
    /// identity's current key set so the result reflects "is this the
    /// last enabled auth/transfer key" relative to everything else.
    private var disableEvaluation: KeyDisableGate.Evaluation {
        KeyDisableGate.evaluate(
            target: publicKey,
            allKeys: identity.identityPublicKeys
        )
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

            // Key Status / danger section
            keyStatusSection

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
        .alert("Disable Key #\(publicKey.id)?", isPresented: $showDisableConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Disable Key", role: .destructive) {
                Task { await disableKey() }
            }
        } message: {
            Text("This permanently and irreversibly disables key #\(publicKey.id) on-chain. It can never be re-enabled — you would have to add a new key instead. Continue?")
        }
        .alert(
            "Disable Failed",
            isPresented: Binding(
                get: { disableError != nil },
                set: { if !$0 { disableError = nil } }
            )
        ) {
            Button("OK", role: .cancel) { disableError = nil }
        } message: {
            Text(disableError ?? "")
        }
        .alert(
            "Forget Key Failed",
            isPresented: Binding(
                get: { forgetError != nil },
                set: { if !$0 { forgetError = nil } }
            )
        ) {
            Button("OK", role: .cancel) { forgetError = nil }
        } message: {
            Text(forgetError ?? "")
        }
    }

    // MARK: - Key Status section

    /// "Key Status" section: a read-only "Disabled" row when the key is
    /// already disabled, otherwise a gated destructive "Disable Key"
    /// action. When a safety gate fails, the button is rendered
    /// disabled with an inline caption explaining why.
    @ViewBuilder
    private var keyStatusSection: some View {
        Section("Key Status") {
            switch disableEvaluation {
            case .alreadyDisabled:
                HStack {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.red)
                    Text("Disabled")
                        .fontWeight(.medium)
                    Spacer()
                    Text("Permanent")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

            case .allowed:
                Button(role: .destructive) {
                    showDisableConfirm = true
                } label: {
                    HStack {
                        if isDisabling {
                            ProgressView()
                                .controlSize(.small)
                        }
                        Label("Disable Key", systemImage: "xmark.circle")
                    }
                }
                .disabled(isDisabling)
                Text("Disabling a key is permanent and irreversible on-chain.")
                    .font(.caption)
                    .foregroundColor(.secondary)

            case .forbidden(let reason):
                Button(role: .destructive) {
                    // No-op: button is disabled.
                } label: {
                    Label("Disable Key", systemImage: "xmark.circle")
                }
                .disabled(true)
                Text(reason)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
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
                print("🔑 Storing private key for identity: \(identity.identityId.toHexString()), keyId: \(publicKey.id)")
                let stored = KeychainManager.shared.storePrivateKey(
                    privateKeyData,
                    identityId: identity.identityId,
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

    /// Submit an `IdentityUpdate` that disables this key, signed by the
    /// identity's MASTER auth key via the wallet's `KeychainSigner`.
    ///
    /// Mirrors `AddIdentityKeyView.submit()`'s wallet + signer
    /// resolution: resolve the wallet from the identity's wallet
    /// linkage, build a `KeychainSigner` over the same model container,
    /// and pin it across the `await` with the `_ = signer` keepalive.
    /// On success, re-fetch the identity's keys (so the disabled badge
    /// appears) and pop back to the key list.
    @MainActor
    private func disableKey() async {
        // Re-check the gate at submit time — the key set could have
        // changed since the view rendered (a background sync, a
        // sibling disable). Never broadcast a doomed transition.
        guard case .allowed = disableEvaluation else { return }

        guard let walletId = identity.wallet?.walletId else {
            disableError = "Identity has no wallet linkage; cannot sign the disable transition."
            return
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            disableError = "Wallet not loaded in the wallet manager."
            return
        }
        guard let sdk = appState.sdk else {
            disableError = "SDK not initialized."
            return
        }

        isDisabling = true
        disableError = nil

        do {
            let signer = KeychainSigner(modelContainer: modelContext.container)
            try await wallet.updateIdentity(
                identityId: identity.identityId,
                addPublicKeys: [],
                disablePublicKeyIds: [publicKey.id],
                signer: signer
            )
            _ = signer  // keepalive: see KeychainSigner lifetime contract.

            // Re-fetch balance + keys so the disabled badge appears
            // without a manual pull-to-refresh.
            try? await IdentityKeyRefresher.refreshBalanceAndKeys(
                identity: identity,
                sdk: sdk,
                modelContext: modelContext
            )

            isDisabling = false
            dismiss()
        } catch {
            isDisabling = false
            disableError = error.localizedDescription
        }
    }

    private func forgetPrivateKey() {
        // Two storage schemes coexist (see `KeysListView`'s
        // `getPrivateKey(for:)`). `forgetIdentityKeyMaterial` deletes
        // under BOTH — including a breadcrumb-independent public-key
        // sweep for pre-migration rows whose breadcrumb columns are
        // still nil — and verifies nothing remains retrievable before
        // we clear the stored reference.
        let persistedKey = identity.publicKeys.first { $0.keyId == Int32(publicKey.id) }
        let removed = KeychainManager.shared.forgetIdentityKeyMaterial(
            identityId: identity.identityId,
            keyIndex: Int32(publicKey.id),
            publicKeyHex: publicKey.data.toHexString(),
            breadcrumbWalletId: persistedKey?.walletId,
            derivationPath: persistedKey?.identityDerivationPath
        )
        guard removed else {
            // Keep the stored reference — clearing it while the
            // secret is still retrievable would leave the key
            // orphaned in the Keychain with no handle pointing at it.
            forgetError = "The private key could not be fully removed from the Keychain — nothing was changed."
            return
        }

        // Clear the keychain reference on the matching
        // PersistentPublicKey so the UI no longer thinks this
        // key has a stored private key.
        if let persistedKey {
            persistedKey.privateKeyKeychainIdentifier = nil
            // Forgetting the last imported key can end the
            // identity's local status — recompute from what
            // remains (wallet linkage / other key material).
            identity.recomputeIsLocalAfterKeyRemoval()
            do {
                try modelContext.save()
            } catch {
                // The Keychain item is already gone (that delete
                // is irreversible), so a silent save failure
                // would leave a stale reference + isLocal on the
                // next launch with no way to re-run this flow.
                // Surface it and keep the sheet open.
                forgetError = "The key was removed from the Keychain, but saving the change failed: \(error.localizedDescription)"
                return
            }
        }
        dismiss()
    }
}
