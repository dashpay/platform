import SwiftDashSDK
import SwiftUI

struct KeysListView: View {
  struct IdentifiableInt: Identifiable { let id: Int }
  let identity: PersistentIdentity
  @Environment(\.modelContext) private var modelContext
  @EnvironmentObject var appState: AppState
  @EnvironmentObject var walletManager: PlatformWalletManager
  @State private var showingPrivateKey: IdentifiableInt? = nil
  @State private var copiedKeyId: Int? = nil
  /// Drives the Add-Key sheet. Bool flag rather than presentation
  /// state because the sheet's content is parameter-free — the
  /// view binds to the surrounding `identity` directly.
  @State private var showingAddKey = false
  /// The key the user swiped to disable, pending confirmation. Drives
  /// a value-based `.confirmationDialog`. The canonical disable home is
  /// `KeyDetailView`; this swipe action is a shortcut that also reaches
  /// keys whose row taps into `PrivateKeyView` instead of the detail.
  @State private var pendingDisableKey: IdentityPublicKey?
  @State private var isDisabling = false
  @State private var disableError: String?

  private var publicKeys: [IdentityPublicKey] {
    identity.identityPublicKeys
  }

  private var privateKeysAvailableCount: Int {
    publicKeys.filter { publicKey in
      hasPrivateKey(for: publicKey)
    }.count
  }

  var body: some View {
    List {
      // Public Keys Section
      Section("Public Keys") {
        ForEach(publicKeys.sorted(by: { $0.id < $1.id }), id: \.id) { publicKey in
          if hasPrivateKey(for: publicKey) {
            // For keys with private keys, use a button instead of NavigationLink
            Button(action: {
              print("🔑 View Private button pressed for key \(publicKey.id)")
              showingPrivateKey = IdentifiableInt(id: Int(publicKey.id))
            }) {
              KeyRowView(
                publicKey: publicKey,
                privateKeyAvailable: true
              )
            }
            .foregroundColor(.primary)
            .swipeActions(edge: .trailing, allowsFullSwipe: false) {
              disableSwipeButton(for: publicKey)
            }
          } else {
            // For keys without private keys, use NavigationLink
            NavigationLink(destination: KeyDetailView(identity: identity, publicKey: publicKey)) {
              KeyRowView(
                publicKey: publicKey,
                privateKeyAvailable: false
              )
            }
            .swipeActions(edge: .trailing, allowsFullSwipe: false) {
              disableSwipeButton(for: publicKey)
            }
          }
        }
      }

      // Summary Section
      Section("Key Summary") {
        HStack {
          Label("Total Public Keys", systemImage: "key")
          Spacer()
          Text("\(publicKeys.count)")
            .foregroundColor(.secondary)
        }

        HStack {
          Label("Private Keys Available", systemImage: "key.fill")
          Spacer()
          Text("\(privateKeysAvailableCount)")
            .foregroundColor(.green)
        }

        if identity.votingPrivateKeyIdentifier != nil {
          HStack {
            Label("Voting Key", systemImage: "hand.raised.fill")
            Spacer()
            Text("Available")
              .foregroundColor(.green)
          }
        }

        if identity.ownerPrivateKeyIdentifier != nil {
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
    .toolbar {
      ToolbarItem(placement: .navigationBarTrailing) {
        Button {
          showingAddKey = true
        } label: {
          Label("Add Key", systemImage: "plus")
        }
        .accessibilityLabel("Add Identity Key")
      }
    }
    .sheet(isPresented: $showingAddKey) {
      AddIdentityKeyView(identity: identity)
    }
    .sheet(item: $showingPrivateKey) { keyId in
      PrivateKeyView(
        identity: identity,
        keyId: UInt32(keyId.id),
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
    .confirmationDialog(
      pendingDisableKey.map { "Disable Key #\($0.id)?" } ?? "Disable Key?",
      isPresented: Binding(
        get: { pendingDisableKey != nil },
        set: { if !$0 { pendingDisableKey = nil } }
      ),
      titleVisibility: .visible
    ) {
      if let key = pendingDisableKey {
        Button("Disable Key", role: .destructive) {
          Task { await disableKey(key) }
        }
      }
      Button("Cancel", role: .cancel) { pendingDisableKey = nil }
    } message: {
      if let key = pendingDisableKey {
        Text("This permanently and irreversibly disables key #\(key.id) on-chain. It can never be re-enabled — you would have to add a new key instead.")
      }
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
  }

  /// Trailing-edge swipe button for a row. Only shown for keys that
  /// pass the disable gate; already-disabled or gate-failing keys get
  /// no swipe action (the full reason is surfaced in `KeyDetailView`).
  @ViewBuilder
  private func disableSwipeButton(for publicKey: IdentityPublicKey) -> some View {
    if case .allowed = KeyDisableGate.evaluate(
      target: publicKey,
      allKeys: publicKeys
    ) {
      Button(role: .destructive) {
        pendingDisableKey = publicKey
      } label: {
        Label("Disable", systemImage: "xmark.circle")
      }
      .disabled(isDisabling)
    }
  }

  /// Submit an `IdentityUpdate` disabling `publicKey`, mirroring
  /// `KeyDetailView.disableKey()` / `AddIdentityKeyView.submit()`: same
  /// wallet + signer resolution, same `_ = signer` keepalive, same
  /// post-submit key refresh so the disabled badge appears.
  @MainActor
  private func disableKey(_ publicKey: IdentityPublicKey) async {
    pendingDisableKey = nil

    // Re-check the gate at submit time — the key set could have
    // changed since the swipe (background sync, a sibling disable).
    guard case .allowed = KeyDisableGate.evaluate(
      target: publicKey,
      allKeys: publicKeys
    ) else { return }

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
    defer { isDisabling = false }

    do {
      let signer = KeychainSigner(modelContainer: modelContext.container)
      try await wallet.updateIdentity(
        identityId: identity.identityId,
        addPublicKeys: [],
        disablePublicKeyIds: [publicKey.id],
        signer: signer
      )
      _ = signer  // keepalive: see KeychainSigner lifetime contract.

      try? await IdentityKeyRefresher.refreshBalanceAndKeys(
        identity: identity,
        sdk: sdk,
        modelContext: modelContext
      )
    } catch {
      disableError = error.localizedDescription
    }
  }

  private func hasPrivateKey(for publicKey: IdentityPublicKey) -> Bool {
    // Two private-key storage schemes coexist on the device. The
    // legacy scheme is keyed by `(identityId, keyIndex)`; the new
    // wallet-derived scheme is keyed by `identity_privkey.<derivation
    // path>` with the public-key hex carried in metadata. Wallet-
    // derived keys (the modern flow) only show up under the second
    // scheme — checking only the first produces a confusing "we
    // definitely have them but the UI says we don't" diagnostic
    // mismatch.
    let km = KeychainManager.shared
    if km.hasPrivateKey(identityId: identity.identityId, keyIndex: Int32(publicKey.id)) {
      return true
    }
    let publicKeyHex = publicKey.data.toHexString()
    return km.hasIdentityPrivateKey(publicKeyHex: publicKeyHex)
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
  let identity: PersistentIdentity
  let keyId: UInt32
  let onCopy: (Int) -> Void
  @Environment(\.dismiss) var dismiss
  @Environment(\.modelContext) private var modelContext
  @EnvironmentObject var appState: AppState
  @State private var showingPrivateKey = false
  @State private var showForgetKeyAlert = false

  var body: some View {
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

          if let publicKey = identity.identityPublicKeys.first(where: { $0.id == keyId }) {
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

        // Public Key — always visible, doesn't require the
        // "reveal" gate the private key hides behind. Helps the
        // user confirm which key the private bytes pair with
        // (e.g. when copying both into another tool that asks for
        // both halves).
        if let publicKey = identity.identityPublicKeys.first(where: { $0.id == keyId }) {
          VStack(alignment: .leading, spacing: 8) {
            Text("Public Key (Hex):")
              .font(.caption)
              .fontWeight(.medium)

            Text(publicKey.data.toHexString())
              .font(.system(.caption, design: .monospaced))
              .padding()
              .frame(maxWidth: .infinity, alignment: .leading)
              .background(Color.black.opacity(0.05))
              .cornerRadius(8)
              .textSelection(.enabled)
              .fixedSize(horizontal: false, vertical: true)

            Button(action: {
              UIPasteboard.general.string = publicKey.data.toHexString()
              onCopy(Int(keyId))
            }) {
              Label("Copy Public Key", systemImage: "doc.on.doc")
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.bordered)
          }
          .padding()
          .background(Color.gray.opacity(0.1))
          .cornerRadius(12)
        }

        // Private Key Display
        if showingPrivateKey {
          if let privateKeyData = getPrivateKey(for: keyId),
             let publicKey = identity.identityPublicKeys.first(where: { $0.id == keyId }) {
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
    let removed = KeychainManager.shared.deletePrivateKey(identityId: identity.identityId, keyIndex: Int32(keyId))

    if removed {
      // Clear the keychain reference on the matching
      // PersistentPublicKey. The @Query observing the parent
      // identity will re-render this row automatically.
      if let persistedKey = identity.publicKeys.first(
        where: { $0.keyId == Int32(keyId) }
      ) {
        persistedKey.privateKeyKeychainIdentifier = nil
        // Forgetting the last imported key can end the identity's
        // local status — recompute from what remains (wallet
        // linkage / other key material).
        identity.recomputeIsLocalAfterKeyRemoval()
        try? modelContext.save()
      }
      dismiss()
    }
  }

  @MainActor
  private func getPrivateKey(for keyId: UInt32) -> Data? {
    // Two private-key storage schemes coexist on the device — see
    // `KeysListView.hasPrivateKey(for:)` for the long-form note.
    // Try the legacy `(identityId, keyIndex)` lookup first; if it
    // misses, fall back to the wallet-derived `identity_privkey.*`
    // scheme keyed by public-key hex.
    let km = KeychainManager.shared
    if let legacy = km.retrievePrivateKey(identityId: identity.identityId, keyIndex: Int32(keyId)) {
      print("🔑 Retrieved private key for keyId \(keyId) via legacy (id, index) scheme")
      return legacy
    }
    if let publicKey = identity.identityPublicKeys.first(where: { $0.id == keyId }) {
      let publicKeyHex = publicKey.data.toHexString()
      if let derived = km.retrieveIdentityPrivateKey(publicKeyHex: publicKeyHex) {
        print("🔑 Retrieved private key for keyId \(keyId) via wallet-derived (publicKeyHex) scheme")
        return derived
      }
    }
    print("🔑 No private key found for keyId \(keyId) under either scheme")
    return nil
  }

  private func getWIFForPrivateKey(_ privateKeyData: Data) -> String? {
    // Mainnet → `X…` compressed prefix; every other network → `c…`.
    // Fall back to testnet when the SDK isn't loaded so the call
    // still produces *some* WIF rather than `nil`.
    let network = appState.sdk?.network ?? .testnet
    return WIFParser.encodeToWIF(privateKeyData, network: network)
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

// Int Identifiable workaround removed; using wrapper type instead
