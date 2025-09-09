import SwiftUI
import SwiftDashSDK
import SwiftDashSDK

struct KeysListView: View {
    let identity: IdentityModel
    @State private var showingPrivateKey: Int? = nil
    @State private var copiedKeyId: Int? = nil
    
    private var privateKeysAvailableCount: Int {
        identity.publicKeys.filter { publicKey in
            hasPrivateKey(for: publicKey.id)
        }.count
    }
    
    var body: some View {
        List {
            // Public Keys Section
            Section("Public Keys") {
                ForEach(identity.publicKeys.sorted(by: { $0.id < $1.id }), id: \.id) { publicKey in
                    if hasPrivateKey(for: publicKey.id) {
                        // For keys with private keys, use a button instead of NavigationLink
                        Button(action: {
                            print("🔑 View Private button pressed for key \(publicKey.id)")
                            showingPrivateKey = Int(publicKey.id)
                        }) {
                            KeyRowView(
                                publicKey: publicKey,
                                privateKeyAvailable: true
                            )
                        }
                        .foregroundColor(.primary)
                    } else {
                        // For keys without private keys, use NavigationLink
                        NavigationLink(destination: KeyDetailView(identity: identity, publicKey: publicKey)) {
                            KeyRowView(
                                publicKey: publicKey,
                                privateKeyAvailable: false
                            )
                        }
                    }
                }
            }
            
            // Summary Section
            Section("Key Summary") {
                HStack {
                    Label("Total Public Keys", systemImage: "key")
                    Spacer()
                    Text("\(identity.publicKeys.count)")
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Label("Private Keys Available", systemImage: "key.fill")
                    Spacer()
                    Text("\(privateKeysAvailableCount)")
                        .foregroundColor(.green)
                }
                
                if let votingKey = identity.votingPrivateKey {
                    HStack {
                        Label("Voting Key", systemImage: "hand.raised.fill")
                        Spacer()
                        Text("Available")
                            .foregroundColor(.green)
                    }
                }
                
                if let ownerKey = identity.ownerPrivateKey {
                    HStack {
                        Label("Owner Key", systemImage: "person.badge.key.fill")
                        Spacer()
                        Text("Available")
                            .foregroundColor(.green)
                    }
                }
            }
        }
        .navigationTitle("Identity Keys")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $showingPrivateKey) { keyId in
            let _ = print("🔑 Sheet presenting for keyId: \(keyId)")
            PrivateKeyView(
                identity: identity,
                keyId: UInt32(keyId),
                onCopy: { keyId in
                    copiedKeyId = keyId
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                        copiedKeyId = nil
                    }
                }
            )
        }
        .overlay(alignment: .bottom) {
            if let copiedId = copiedKeyId {
                CopiedToast(message: "Private key #\(copiedId) copied")
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
    }
    
    private func hasPrivateKey(for keyId: UInt32) -> Bool {
        // Check if we have a private key for this key ID in keychain
        let hasKey = KeychainManager.shared.hasPrivateKey(identityId: identity.id, keyIndex: Int32(keyId))
        print("🔑 Checking private key for keyId: \(keyId) - found: \(hasKey)")
        return hasKey
    }
}

struct KeyRowView: View {
    let publicKey: IdentityPublicKey
    let privateKeyAvailable: Bool
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Key Header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Key #\(publicKey.id)")
                        .font(.headline)
                    Text(publicKey.purpose.name)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing, spacing: 2) {
                    SecurityLevelBadge(level: publicKey.securityLevel)
                    if privateKeyAvailable {
                        Label("View Private", systemImage: "eye.fill")
                            .font(.caption2)
                            .foregroundColor(.blue)
                    }
                }
            }
            
            // Key Type and Properties
            HStack(spacing: 12) {
                Label(publicKey.keyType.name, systemImage: "signature")
                    .font(.caption2)
                
                if publicKey.readOnly {
                    Label("Read Only", systemImage: "lock.fill")
                        .font(.caption2)
                        .foregroundColor(.orange)
                }
                
                if publicKey.disabledAt != nil {
                    Label("Disabled", systemImage: "xmark.circle.fill")
                        .font(.caption2)
                        .foregroundColor(.red)
                }
            }
            
            // Public Key Data
            VStack(alignment: .leading, spacing: 4) {
                Text("Public Key:")
                    .font(.caption2)
                    .fontWeight(.medium)
                Text(publicKey.data.toHexString())
                    .font(.system(.caption2, design: .monospaced))
                    .lineLimit(2)
                    .truncationMode(.middle)
                    .foregroundColor(.secondary)
            }
            .padding(.top, 4)
        }
        .padding(.vertical, 4)
    }
}

struct PrivateKeyView: View {
    let identity: IdentityModel
    let keyId: UInt32
    let onCopy: (Int) -> Void
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState
    @State private var showingPrivateKey = false
    @State private var showForgetKeyAlert = false
    
    var body: some View {
        let _ = print("🔑 PrivateKeyView initialized for keyId: \(keyId)")
        NavigationView {
            VStack(spacing: 20) {
                // Warning
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.largeTitle)
                        .foregroundColor(.orange)
                    
                    Text("Private Key Warning")
                        .font(.headline)
                    
                    Text("Never share your private key with anyone. Anyone with access to this key can control your identity and spend your funds.")
                        .multilineTextAlignment(.center)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding()
                .background(Color.orange.opacity(0.1))
                .cornerRadius(12)
                
                // Key Info
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Key ID:")
                        Spacer()
                        Text("#\(keyId)")
                            .fontWeight(.medium)
                    }
                    
                    if let publicKey = identity.publicKeys.first(where: { $0.id == keyId }) {
                        HStack {
                            Text("Purpose:")
                            Spacer()
                            Text(publicKey.purpose.name)
                                .fontWeight(.medium)
                        }
                        
                        HStack {
                            Text("Type:")
                            Spacer()
                            Text(publicKey.keyType.name)
                                .fontWeight(.medium)
                        }
                    }
                }
                .padding()
                .background(Color.gray.opacity(0.1))
                .cornerRadius(12)
                
                // Private Key Display
                if showingPrivateKey {
                    if let privateKeyData = getPrivateKey(for: keyId),
                       let publicKey = identity.publicKeys.first(where: { $0.id == keyId }) {
                        VStack(alignment: .leading, spacing: 16) {
                            // Hex Format
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Private Key (Hex):")
                                    .font(.caption)
                                    .fontWeight(.medium)
                                
                                Text(privateKeyData.toHexString())
                                    .font(.system(.caption, design: .monospaced))
                                    .padding()
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .background(Color.black.opacity(0.05))
                                    .cornerRadius(8)
                                    .textSelection(.enabled)
                                    .fixedSize(horizontal: false, vertical: true)
                                
                                Button(action: {
                                    UIPasteboard.general.string = privateKeyData.toHexString()
                                    onCopy(Int(keyId))
                                }) {
                                    Label("Copy Hex", systemImage: "doc.on.doc")
                                        .frame(maxWidth: .infinity)
                                }
                                .buttonStyle(.bordered)
                            }
                            
                            // WIF Format - only for ECDSA key types
                            if publicKey.keyType == .ecdsaSecp256k1 || publicKey.keyType == .ecdsaHash160 {
                                VStack(alignment: .leading, spacing: 8) {
                                    Text("Private Key (WIF):")
                                        .font(.caption)
                                        .fontWeight(.medium)
                                    
                                    if let wif = getWIFForPrivateKey(privateKeyData) {
                                        Text(wif)
                                            .font(.system(.caption, design: .monospaced))
                                            .padding()
                                            .frame(maxWidth: .infinity, alignment: .leading)
                                            .background(Color.black.opacity(0.05))
                                            .cornerRadius(8)
                                            .textSelection(.enabled)
                                            .fixedSize(horizontal: false, vertical: true)
                                        
                                        Button(action: {
                                            UIPasteboard.general.string = wif
                                            onCopy(Int(keyId))
                                        }) {
                                            Label("Copy WIF", systemImage: "doc.on.doc")
                                                .frame(maxWidth: .infinity)
                                        }
                                        .buttonStyle(.bordered)
                                    } else {
                                        Text("Unable to encode to WIF format")
                                            .foregroundColor(.red)
                                            .font(.caption)
                                    }
                                }
                            }
                            
                            Button(action: {
                                dismiss()
                            }) {
                                Label("Done", systemImage: "checkmark.circle")
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.borderedProminent)
                            
                            Button(action: {
                                showForgetKeyAlert = true
                            }) {
                                Label("Forget Private Key", systemImage: "trash")
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.bordered)
                            .foregroundColor(.red)
                        }
                    } else {
                        Text("Private key not available")
                            .foregroundColor(.red)
                    }
                } else {
                    Button(action: {
                        print("🔑 Reveal button pressed for keyId: \(keyId)")
                        showingPrivateKey = true
                    }) {
                        Label("Reveal Private Key", systemImage: "eye.fill")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(.orange)
                }
                
                Spacer()
            }
            .padding()
            .navigationTitle("Private Key #\(keyId)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
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
    }
    
    private func forgetPrivateKey() {
        // Remove from keychain
        let removed = KeychainManager.shared.deletePrivateKey(identityId: identity.id, keyIndex: Int32(keyId))
        
        if removed {
            // Update the persistent public key to clear the reference
            appState.removePrivateKeyReference(identityId: identity.id, keyId: Int32(keyId))
            dismiss()
        }
    }
    
    private func getPrivateKey(for keyId: UInt32) -> Data? {
        // Retrieve the actual stored private key from keychain
        let privateKey = KeychainManager.shared.retrievePrivateKey(identityId: identity.id, keyIndex: Int32(keyId))
        print("🔑 Retrieving private key for identity: \(identity.id.toHexString()), keyId: \(keyId)")
        print("🔑 Private key found: \(privateKey != nil ? "Yes (\(privateKey!.count) bytes)" : "No")")
        return privateKey
    }
    
    private func getWIFForPrivateKey(_ privateKeyData: Data) -> String? {
        return WIFParser.encodeToWIF(privateKeyData, isTestnet: true)
    }
}

struct SecurityLevelBadge: View {
    let level: SecurityLevel
    
    var body: some View {
        Text(level.name.uppercased())
            .font(.caption2)
            .padding(.horizontal, 8)
            .padding(.vertical, 2)
            .background(backgroundColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }
    
    private var backgroundColor: Color {
        switch level {
        case .master: return .red
        case .critical: return .orange
        case .high: return .blue
        case .medium: return .green
        }
    }
}

struct CopiedToast: View {
    let message: String
    
    var body: some View {
        Text(message)
            .font(.caption)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(Color.black.opacity(0.8))
            .foregroundColor(.white)
            .cornerRadius(20)
            .padding(.bottom, 50)
    }
}


// Extension to make Int identifiable for sheet presentation
extension Int: Identifiable {
    public var id: Int { self }
}