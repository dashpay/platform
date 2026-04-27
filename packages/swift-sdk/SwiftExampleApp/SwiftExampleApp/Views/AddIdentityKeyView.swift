import SwiftDashSDK
import SwiftData
import SwiftUI

/// Form for adding a new public key to an existing identity.
///
/// The form constrains the user's choices to combinations Drive
/// actually accepts:
///
/// | Purpose         | Security level         | Contract bounds |
/// |-----------------|------------------------|-----------------|
/// | Authentication  | Critical / High / Medium | none            |
/// | Encryption      | Critical / High / Medium | required        |
/// | Decryption      | Critical / High / Medium | required        |
/// | Transfer        | Critical (forced)        | none            |
///
/// Master / System / Voting / Owner aren't pickable here — the
/// first three are minted at registration time, and Owner keys
/// belong to masternode tooling.
///
/// The flow runs in three FFI-bridged steps:
///   1. **Derive** the new keypair at the next free `keyId` slot
///      via `wallet.deriveIdentityAuthKeyAtSlot(...)` (ECDSA-only
///      for the moment; BLS still needs Rust derivation work).
///   2. **Pre-persist** the 32-byte private scalar to the iOS
///      Keychain so the `KeychainSigner` trampoline can sign the
///      resulting transition.
///   3. **Submit** `updateIdentity(addPublicKeys:)` with the new
///      `IdentityPubkey` — including its `ContractBounds` for
///      Encryption / Decryption keys. Rust signs + broadcasts;
///      the persister callback writes the SwiftData row when the
///      transition lands on-chain.
///
/// Per swift-sdk/CLAUDE.md, all derivation policy lives in
/// `rs-platform-wallet`; this view only marshals user choices into
/// FFI calls.
struct AddIdentityKeyView: View {
    let identity: PersistentIdentity

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    /// Saved contracts on this device — feeds the Encryption /
    /// Decryption "contract bounds" picker. Filtering by network
    /// keeps a testnet identity from accidentally binding to a
    /// mainnet contract row.
    @Query(sort: \PersistentDataContract.lastAccessedAt, order: .reverse)
    private var allContracts: [PersistentDataContract]

    @State private var keyType: KeyType = .ecdsaSecp256k1
    @State private var purpose: KeyPurpose = .authentication
    @State private var authSecurityLevel: SecurityLevel = .high
    @State private var encryptionSecurityLevel: SecurityLevel = .high
    /// Selected contract id for Encryption / Decryption bounds, in
    /// the canonical 32-byte form. `nil` until the user picks one.
    @State private var boundContractId: Data?
    /// Optional document type narrowing the bounds. Empty string
    /// means "any document type within the contract", which maps
    /// to `.singleContract`. A non-empty string maps to
    /// `.singleContractDocumentType`.
    @State private var boundDocumentTypeName: String = ""
    @State private var isSubmitting = false
    @State private var errorMessage: String?

    /// User-pickable purposes per the form's contract. Master /
    /// System / Voting / Owner are intentionally absent.
    private static let pickablePurposes: [KeyPurpose] = [
        .authentication, .encryption, .decryption, .transfer,
    ]

    /// Pickable key types. ECDSA covers the two variants the FFI
    /// derives today; BLS is shown so the option's discoverable but
    /// stays disabled until Rust BLS derivation lands.
    private static let pickableKeyTypes: [KeyType] = [
        .ecdsaSecp256k1, .ecdsaHash160, .bls12_381,
    ]

    /// Security levels offered for Authentication / Encryption /
    /// Decryption keys — Master is excluded by design (only the
    /// registration master key gets that level).
    private static let nonMasterSecurityLevels: [SecurityLevel] = [
        .critical, .high, .medium,
    ]

    /// Filtered subset of `allContracts` matching the active
    /// network. Identities are network-scoped, so binding a key
    /// to a contract on a different network would never be valid.
    private var contractsForNetwork: [PersistentDataContract] {
        allContracts.filter { $0.network == appState.currentNetwork }
    }

    /// Document-type names declared by the currently-selected
    /// contract. Drives the optional document-type picker.
    private var documentTypesForSelectedContract: [String] {
        guard let id = boundContractId,
              let contract = contractsForNetwork.first(where: { $0.id == id })
        else {
            return []
        }
        return (contract.documentTypes ?? []).map { $0.name }.sorted()
    }

    /// Effective security level for the current purpose. Transfer
    /// is locked at Critical per the form's contract; everything
    /// else picks from the user's selected value.
    private var effectiveSecurityLevel: SecurityLevel {
        switch purpose {
        case .transfer: return .critical
        case .encryption, .decryption: return encryptionSecurityLevel
        default: return authSecurityLevel
        }
    }

    /// `keyId` to assign to the new key — `max(existing) + 1`.
    /// Auto-assigned (the user can't pick) so the new key never
    /// collides with an existing slot.
    private var nextKeyId: UInt32 {
        let existing = identity.identityPublicKeys.map { $0.id }.max() ?? 0
        return existing + 1
    }

    /// Whether Encryption / Decryption purposes are missing their
    /// required contract bounds. Surfaces a disabled-Submit hint
    /// rather than letting the user submit a doomed transition.
    private var contractBoundsMissing: Bool {
        switch purpose {
        case .encryption, .decryption:
            return boundContractId == nil
        default:
            return false
        }
    }

    private var canSubmit: Bool {
        !isSubmitting && keyType != .bls12_381 && !contractBoundsMissing
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("New Key") {
                    Picker("Key Type", selection: $keyType) {
                        ForEach(Self.pickableKeyTypes, id: \.self) { type in
                            Text(type.name).tag(type)
                        }
                    }
                    Picker("Purpose", selection: $purpose) {
                        ForEach(Self.pickablePurposes, id: \.self) { p in
                            Text(p.name).tag(p)
                        }
                    }
                    securityLevelPicker
                }

                if purpose == .encryption || purpose == .decryption {
                    contractBoundsSection
                }

                Section("Slot") {
                    LabeledContent("Auto-assigned key id") {
                        Text("#\(nextKeyId)").foregroundColor(.secondary)
                    }
                    Text("Picked as `max(existingKeyIds) + 1`. Slots are non-recyclable — disabled keys leave a hole in the range; new keys always extend past the highest ever used.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                if keyType == .bls12_381 {
                    Section {
                        Label(
                            "BLS derivation is not yet wired through the FFI for this flow. Pick ECDSA secp256k1 or ECDSA Hash160 to add a key now.",
                            systemImage: "exclamationmark.triangle.fill"
                        )
                        .font(.caption)
                        .foregroundColor(.orange)
                    }
                }

                Section {
                    Button {
                        Task { await submit() }
                    } label: {
                        HStack {
                            if isSubmitting {
                                ProgressView().controlSize(.small)
                            }
                            Label("Add Key", systemImage: "plus.circle.fill")
                                .frame(maxWidth: .infinity)
                        }
                    }
                    .disabled(!canSubmit)
                }

                if let errorMessage = errorMessage {
                    Section {
                        Text(errorMessage)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Add Identity Key")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .onChange(of: purpose) { _, newValue in
                // Reset contract-bounds state if the user toggles
                // away from Encryption / Decryption — otherwise
                // closing + reopening the section silently re-uses
                // a stale binding.
                if newValue != .encryption && newValue != .decryption {
                    boundContractId = nil
                    boundDocumentTypeName = ""
                }
            }
            .onChange(of: boundContractId) { _, _ in
                // Switching contracts invalidates the document-type
                // selection (different contracts have different
                // schemas).
                boundDocumentTypeName = ""
            }
        }
    }

    // MARK: Sub-views

    /// Security-level picker. Hidden + auto-Critical for Transfer
    /// (per the form's contract); pickable for Auth / Encryption /
    /// Decryption with Master excluded.
    @ViewBuilder
    private var securityLevelPicker: some View {
        switch purpose {
        case .transfer:
            LabeledContent("Security Level") {
                Text("Critical")
                    .foregroundColor(.secondary)
            }
        case .encryption, .decryption:
            Picker("Security Level", selection: $encryptionSecurityLevel) {
                ForEach(Self.nonMasterSecurityLevels, id: \.self) { lvl in
                    Text(lvl.name).tag(lvl)
                }
            }
        default:
            Picker("Security Level", selection: $authSecurityLevel) {
                ForEach(Self.nonMasterSecurityLevels, id: \.self) { lvl in
                    Text(lvl.name).tag(lvl)
                }
            }
        }
    }

    /// Contract-bounds section for Encryption / Decryption keys.
    /// Required by Drive; the form blocks submission if the user
    /// hasn't picked a contract.
    @ViewBuilder
    private var contractBoundsSection: some View {
        Section("Contract Bounds (required)") {
            if contractsForNetwork.isEmpty {
                Text("No contracts saved on this device. Add or fetch a contract on the Contracts tab first.")
                    .font(.caption)
                    .foregroundColor(.orange)
            } else {
                Picker("Contract", selection: $boundContractId) {
                    Text("Select a contract").tag(Data?.none)
                    ForEach(contractsForNetwork, id: \.id) { contract in
                        Text(contract.name).tag(Optional(contract.id))
                    }
                }

                if !documentTypesForSelectedContract.isEmpty {
                    Picker("Document Type (optional)", selection: $boundDocumentTypeName) {
                        Text("Any document type").tag("")
                        ForEach(documentTypesForSelectedContract, id: \.self) { name in
                            Text(name).tag(name)
                        }
                    }
                }

                Text("Encryption / decryption keys must be scoped to a specific contract. A document type narrows the scope further; leaving it blank lets the key operate across all of the contract's document types.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: Submit

    @MainActor
    private func submit() async {
        guard let walletId = identity.wallet?.walletId else {
            errorMessage = "Identity has no wallet linkage; cannot derive new keys."
            return
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "Wallet not loaded in the wallet manager."
            return
        }
        let network = appState.sdk?.network ?? DashSDKNetwork(rawValue: 1)
        let chosenKeyId = nextKeyId
        let chosenSecurityLevel = effectiveSecurityLevel

        // Build the contract bounds shape from the picker state.
        // Encryption / Decryption are gated above so we can assume
        // `boundContractId` is non-nil here for those purposes.
        let contractBounds: ManagedPlatformWallet.ContractBounds? = {
            switch purpose {
            case .encryption, .decryption:
                guard let id = boundContractId else { return nil }
                let trimmed = boundDocumentTypeName.trimmingCharacters(in: .whitespacesAndNewlines)
                if trimmed.isEmpty {
                    return .singleContract(id: id)
                }
                return .singleContractDocumentType(id: id, documentTypeName: trimmed)
            default:
                return nil
            }
        }()

        isSubmitting = true
        errorMessage = nil

        do {
            // 1. Derive the new keypair via the FFI. The library does
            //    the path build + secp256k1 derive; we get the public
            //    bytes + the 32-byte private scalar back.
            let preview = try wallet.deriveIdentityAuthKeyAtSlot(
                walletId: walletId,
                identityIndex: identity.identityIndex,
                keyId: chosenKeyId,
                network: network
            )

            // 2. Verify the derived scalar matches the returned
            //    public key. Cheap defence against derivation drift
            //    or bit-rot in the FFI marshalling — mismatched key
            //    material lands as a key Drive can't sign with, and
            //    surfaces as a mid-state-transition validation
            //    failure that's painful to debug after-the-fact.
            //    Validation goes against the compressed pubkey
            //    (33 bytes) regardless of `keyType`; the HASH160
            //    variant just stores a different on-chain payload.
            // `DashSDKNetwork` is a `(rawValue: UInt32)` struct;
            // 0 = mainnet, 1 = testnet (see SDK.swift's default).
            let isMainnet = (network.rawValue == 0)
            guard
                KeyValidation.validatePrivateKeyForPublicKey(
                    privateKeyHex: preview.privateKeyData.toHexString(),
                    publicKeyHex: preview.publicKeyHex,
                    keyType: .ecdsaSecp256k1,
                    isTestnet: !isMainnet
                )
            else {
                isSubmitting = false
                errorMessage =
                    "Derived key didn't match its public key — refusing to broadcast."
                return
            }

            // 3. Pre-persist private bytes to Keychain under the
            //    wallet-derived `identity_privkey.<path>` scheme so
            //    the `KeychainSigner` trampoline can find them when
            //    Rust signs the resulting `updateIdentity` transition.
            //
            //    The trampoline looks up the privkey by hex match
            //    against `metadata.publicKey`, comparing whatever
            //    bytes Rust ships in via the FFI. For
            //    `ECDSA_SECP256K1` that's the 33-byte compressed
            //    pubkey; for `ECDSA_HASH160` it's the 20-byte
            //    HASH160. So the `metadata.publicKey` we stamp
            //    here must match the on-chain payload shape, not
            //    the raw derived pubkey. `publicKeyHash` is always
            //    the HASH160 hex (kept around for the explorer UI's
            //    cross-referencing).
            let pubKeyHashHex =
                SwiftDashSDK.KeychainManager.computePublicKeyHashHex(preview.publicKeyData)
            let metadataPublicKeyHex: String =
                keyType == .ecdsaHash160 ? pubKeyHashHex : preview.publicKeyHex
            let metadata = IdentityPrivateKeyMetadata(
                identityId: identity.identityIdString,
                keyId: chosenKeyId,
                walletId: walletId.toHexString(),
                identityIndex: identity.identityIndex,
                keyIndex: chosenKeyId,
                derivationPath: preview.derivationPath,
                publicKey: metadataPublicKeyHex,
                publicKeyHash: pubKeyHashHex,
                keyType: keyType.rawValue,
                purpose: purpose.rawValue,
                securityLevel: chosenSecurityLevel.rawValue
            )
            // `storeIdentityPrivateKey` returns nil on Keychain
            // failure. Earlier revisions ignored the return, so a
            // failed write would silently broadcast a key whose
            // matching scalar wasn't actually persisted — the
            // signer trampoline would then fail to sign anything
            // for the new key. Treat the nil return as fatal.
            guard
                KeychainManager.shared.storeIdentityPrivateKey(
                    preview.privateKeyData,
                    derivationPath: preview.derivationPath,
                    metadata: metadata
                ) != nil
            else {
                isSubmitting = false
                errorMessage =
                    "Could not persist new key to the iOS Keychain — aborted before broadcast."
                return
            }

            // 4. Submit `updateIdentity(addPublicKeys:)`. Rust signs
            //    via the trampoline and broadcasts; the persister
            //    callback writes the matching `PersistentPublicKey`
            //    row when the transition lands on-chain.
            //
            //    For `ECDSA_HASH160` the on-chain payload is the
            //    20-byte HASH160 of the compressed pubkey — not
            //    the pubkey itself. Compute it here so the FFI row
            //    matches the selected key type's expected byte
            //    length.
            let pubkeyBytesForFFI: Data
            if keyType == .ecdsaHash160 {
                guard let hashBytes = Data(hexString: pubKeyHashHex), hashBytes.count == 20 else {
                    isSubmitting = false
                    errorMessage =
                        "Could not compute HASH160 of derived pubkey — aborted before broadcast."
                    return
                }
                pubkeyBytesForFFI = hashBytes
            } else {
                pubkeyBytesForFFI = preview.publicKeyData
            }

            let newKey = ManagedPlatformWallet.IdentityPubkey(
                keyId: chosenKeyId,
                keyType: keyType,
                purpose: purpose,
                securityLevel: chosenSecurityLevel,
                pubkeyBytes: pubkeyBytesForFFI,
                contractBounds: contractBounds
            )
            let signer = KeychainSigner(modelContainer: modelContext.container)
            try await wallet.updateIdentity(
                identityId: identity.identityId,
                addPublicKeys: [newKey],
                signer: signer
            )
            _ = signer  // keepalive: see KeychainSigner lifetime contract.

            isSubmitting = false
            dismiss()
        } catch {
            isSubmitting = false
            errorMessage = error.localizedDescription
        }
    }
}
