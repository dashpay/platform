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

    /// System contracts that declare `requiresIdentityEncryptionBoundedKey`
    /// somewhere — either contract-level or on at least one document
    /// type. These are network-agnostic (same canonical 32-byte ID
    /// on every network) so they're always selectable in the bounds
    /// picker — the user shouldn't have to fetch them onto the
    /// device first just to bind an encryption key to them.
    ///
    /// Today only DashPay qualifies (its `contactRequest` document
    /// type declares the requirement). DPNS / withdrawals /
    /// masternode-reward-shares / token-history / keyword-search
    /// don't bind encryption keys.
    ///
    /// **Important:** DPP rejects `ContractBounds::SingleContract`
    /// unless the *contract config* itself declares the requirement
    /// (see `packages/rs-drive-abci/.../validate_identity_public_key_contract_bounds/v0/mod.rs:82`).
    /// DashPay declares it only at the document-type level, so
    /// `allowsContractScope` is `false` and the document-type
    /// picker is required (not optional).
    ///
    /// ID source: `packages/dashpay-contract/src/lib.rs::ID_BYTES`.
    /// Document-type names: `packages/dashpay-contract/schema/v1/dashpay.schema.json`.
    private static let systemContractsAllowingKeyBounds: [SystemContractEntry] = [
        SystemContractEntry(
            name: "DashPay",
            id: Data([
                162, 161, 180, 172, 111, 239, 34, 234,
                42, 26, 104, 232, 18, 54, 68, 179,
                87, 135, 95, 107, 65, 44, 24, 16,
                146, 129, 193, 70, 231, 178, 113, 188,
            ]),
            allowsContractScope: false,
            documentTypesAllowingBounds: ["contactRequest"]
        ),
    ]

    /// Combined picker entries: system contracts first (always
    /// available), then user-saved contracts filtered to the
    /// current network. Saved-contract rows that happen to match
    /// a system contract by ID are skipped to avoid a duplicate.
    ///
    /// Caveat: for user-saved (`PersistentDataContract`) entries
    /// we don't currently know which document types declare the
    /// bounded-key requirement, so we conservatively expose ALL
    /// of them and allow contract-scope. Picking an invalid
    /// combination there will fail at submit time with a DPP
    /// validation error. Tightening this would require parsing
    /// the per-document-type schema flag into SwiftData rows.
    private var pickerEntries: [BoundsPickerEntry] {
        // System metadata wins over user-saved rows: the system
        // registry knows precisely which document types within the
        // contract carry the bounded-key requirement (DashPay's
        // `contactRequest`, for example, vs the rest of the
        // contract). If we let a saved row override the system
        // entry — even when the IDs match — the picker would fall
        // back to the "expose every document type" behaviour and
        // an invalid (contract, doc-type) combo could slip through.
        let systemIds = Set(Self.systemContractsAllowingKeyBounds.map(\.id))
        let system = Self.systemContractsAllowingKeyBounds.map {
            BoundsPickerEntry(
                id: $0.id,
                displayName: "\($0.name) (System)",
                allowsContractScope: $0.allowsContractScope,
                documentTypesAllowingBounds: $0.documentTypesAllowingBounds
            )
        }
        let saved = contractsForNetwork
            .filter { !systemIds.contains($0.id) }
            .map {
                BoundsPickerEntry(
                    id: $0.id,
                    displayName: $0.name,
                    allowsContractScope: true,
                    documentTypesAllowingBounds: ($0.documentTypes ?? []).map(\.name).sorted()
                )
            }
        return system + saved
    }

    /// The currently-selected picker entry, if any.
    private var selectedEntry: BoundsPickerEntry? {
        guard let id = boundContractId else { return nil }
        return pickerEntries.first { $0.id == id }
    }

    /// Document-type names valid for binding on the selected contract.
    private var documentTypesForSelectedContract: [String] {
        selectedEntry?.documentTypesAllowingBounds ?? []
    }

    /// Whether the user must pick a document type (i.e. the selected
    /// contract does not allow a contract-scope binding). Drives
    /// the "Any document type" option visibility and gates submit.
    private var documentTypeRequired: Bool {
        guard let entry = selectedEntry else { return false }
        return !entry.allowsContractScope
    }

    /// Effective security level for the current purpose. Several
    /// purposes are protocol-locked:
    ///   - `transfer`            → Critical
    ///   - `encryption`/`decryption` → Medium  (DPP enforces only
    ///     `SecurityLevel::MEDIUM`; see
    ///     `validate_identity_public_keys_structure/v0/mod.rs`)
    /// Auth-style purposes (Authentication today) are user-pickable.
    private var effectiveSecurityLevel: SecurityLevel {
        switch purpose {
        case .transfer: return .critical
        case .encryption, .decryption: return .medium
        default: return authSecurityLevel
        }
    }

    /// Effective key type for the current purpose. ENCRYPTION /
    /// DECRYPTION are locked to ECDSA secp256k1: the rest of the
    /// stack does ECDH via `dashcore::secp256k1::PublicKey` (see
    /// `packages/rs-platform-wallet/.../contacts.rs`), HASH160
    /// stores only the hash so there's no full pubkey to ECDH
    /// against, and BLS is the wrong curve.
    private var effectiveKeyType: KeyType {
        switch purpose {
        case .encryption, .decryption: return .ecdsaSecp256k1
        default: return keyType
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
    /// Also covers the case where the selected contract requires a
    /// document type (DashPay does) but the user hasn't picked one.
    private var contractBoundsMissing: Bool {
        switch purpose {
        case .encryption, .decryption:
            guard boundContractId != nil else { return true }
            if documentTypeRequired {
                let trimmed = boundDocumentTypeName.trimmingCharacters(in: .whitespacesAndNewlines)
                return trimmed.isEmpty
            }
            return false
        default:
            return false
        }
    }

    private var canSubmit: Bool {
        !isSubmitting && effectiveKeyType != .bls12_381 && !contractBoundsMissing
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("New Key") {
                    keyTypePicker
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

                if effectiveKeyType == .bls12_381 {
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
                // schemas). When the new contract requires a
                // document-type binding and only one valid choice
                // exists, auto-pick it so the user can submit
                // immediately (DashPay → `contactRequest`). When
                // contract-scope is allowed, default to the empty
                // "Any document type" option.
                if let entry = selectedEntry,
                   !entry.allowsContractScope,
                   entry.documentTypesAllowingBounds.count == 1
                {
                    boundDocumentTypeName = entry.documentTypesAllowingBounds[0]
                } else {
                    boundDocumentTypeName = ""
                }
            }
        }
    }

    // MARK: Sub-views

    /// Key-type picker. Locked to ECDSA secp256k1 for Encryption /
    /// Decryption purposes since ECDH only works against a full
    /// secp256k1 pubkey. Pickable for all other purposes; the
    /// `effectiveKeyType` computed property carries the locked
    /// value through to the submit path.
    @ViewBuilder
    private var keyTypePicker: some View {
        switch purpose {
        case .encryption, .decryption:
            LabeledContent("Key Type") {
                Text(KeyType.ecdsaSecp256k1.name)
                    .foregroundColor(.secondary)
            }
        default:
            Picker("Key Type", selection: $keyType) {
                ForEach(Self.pickableKeyTypes, id: \.self) { type in
                    Text(type.name).tag(type)
                }
            }
        }
    }

    /// Security-level picker. Locked rows for purposes the protocol
    /// constrains to a single level (Transfer → Critical;
    /// Encryption / Decryption → Medium per DPP
    /// `validate_identity_public_keys_structure/v0/mod.rs`). Pickable
    /// for Auth-style purposes with Master excluded.
    @ViewBuilder
    private var securityLevelPicker: some View {
        switch purpose {
        case .transfer:
            LabeledContent("Security Level") {
                Text("Critical")
                    .foregroundColor(.secondary)
            }
        case .encryption, .decryption:
            LabeledContent("Security Level") {
                Text("Medium")
                    .foregroundColor(.secondary)
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
            // `pickerEntries` always contains the system-contract
            // entries (DashPay today), so the picker is never empty
            // even on a fresh install. The Contracts tab is still
            // the way to add non-system contracts to the list.
            Picker("Contract", selection: $boundContractId) {
                Text("Select a contract").tag(Data?.none)
                ForEach(pickerEntries, id: \.id) { entry in
                    Text(entry.displayName).tag(Optional(entry.id))
                }
            }

            if !documentTypesForSelectedContract.isEmpty {
                // When the contract permits a contract-scope bound
                // (the `requires_identity_encryption_bounded_key`
                // flag is set on the contract config itself), the
                // user can leave the document type unset → results
                // in `SingleContract` bounds. When it isn't set —
                // DashPay's case, where the flag lives only on
                // `contactRequest` — the user MUST pick one of the
                // listed document types. DPP rejects
                // `SingleContract` bounds against DashPay with
                // `DataContractBoundsNotPresentError`.
                Picker(
                    documentTypeRequired
                        ? "Document Type (required)"
                        : "Document Type (optional)",
                    selection: $boundDocumentTypeName
                ) {
                    if !documentTypeRequired {
                        Text("Any document type").tag("")
                    }
                    ForEach(documentTypesForSelectedContract, id: \.self) { name in
                        Text(name).tag(name)
                    }
                }
            }

            if documentTypeRequired {
                Text("This contract requires the key to be bound to a specific document type — picking a contract scope alone would be rejected at submit.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
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
        let network = appState.sdk?.network ?? .testnet
        let chosenKeyId = nextKeyId
        let chosenSecurityLevel = effectiveSecurityLevel
        // Use the effective (purpose-aware) key type so encryption /
        // decryption submissions always carry ECDSA secp256k1
        // regardless of what's in the `keyType` @State from a prior
        // purpose selection.
        let chosenKeyType = effectiveKeyType

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
            guard
                KeyValidation.validatePrivateKeyForPublicKey(
                    privateKeyHex: preview.privateKeyData.toHexString(),
                    publicKeyHex: preview.publicKeyHex,
                    keyType: .ecdsaSecp256k1,
                    network: network
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
                chosenKeyType == .ecdsaHash160 ? pubKeyHashHex : preview.publicKeyHex
            let metadata = IdentityPrivateKeyMetadata(
                identityId: identity.identityIdString,
                keyId: chosenKeyId,
                walletId: walletId.toHexString(),
                identityIndex: identity.identityIndex,
                keyIndex: chosenKeyId,
                derivationPath: preview.derivationPath,
                publicKey: metadataPublicKeyHex,
                publicKeyHash: pubKeyHashHex,
                keyType: chosenKeyType.rawValue,
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
            if chosenKeyType == .ecdsaHash160 {
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
                keyType: chosenKeyType,
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

// MARK: - Picker support types

/// A system data contract that should always be available in the
/// contract-bounds picker. Network-agnostic — system contracts have
/// the same canonical 32-byte ID on every network.
///
/// `allowsContractScope` mirrors the contract-level
/// `requiresIdentityEncryptionBoundedKey` flag — when `false`,
/// `ContractBounds::SingleContract` is rejected by DPP and the
/// picker has to force a document-type selection.
///
/// `documentTypesAllowingBounds` lists only the document types
/// that themselves declare `requiresIdentityEncryptionBoundedKey`,
/// since picking any other DT would fail DPP validation with
/// `DataContractBoundsNotPresentError`.
private struct SystemContractEntry {
    let name: String
    let id: Data
    let allowsContractScope: Bool
    let documentTypesAllowingBounds: [String]
}

/// Unified row shape for the bounds picker — covers both the static
/// `SystemContractEntry` registry and per-network
/// `PersistentDataContract` rows. Lets the picker iterate one list
/// regardless of origin.
private struct BoundsPickerEntry {
    let id: Data
    let displayName: String
    let allowsContractScope: Bool
    let documentTypesAllowingBounds: [String]
}
