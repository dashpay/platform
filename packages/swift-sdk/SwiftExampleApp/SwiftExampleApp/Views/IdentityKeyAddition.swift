import Foundation
import SwiftData
import SwiftDashSDK

/// Shared "add a key to an existing identity" plumbing.
///
/// This is the exact derive → validate → persist → build-`IdentityPubkey`
/// sequence `AddIdentityKeyView.submit()` runs, lifted into a reusable
/// helper so the generic transition builder
/// (`TransitionDetailView.executeIdentityUpdate`) can drive the same
/// IdentityUpdate flow without duplicating the crypto / Keychain steps.
///
/// Per `swift-sdk/CLAUDE.md`, all derivation policy lives in
/// `rs-platform-wallet`; this helper only marshals choices into the
/// `wallet.deriveIdentityAuthKeyAtSlot(...)` /
/// `wallet.updateIdentity(...)` FFI calls and owns the iOS-Keychain
/// persist step (the one operation Rust can't perform from its side).
enum IdentityKeyAddition {
    /// A single key the caller wants to add, described by its DPP
    /// attributes. The keypair itself is derived Rust-side from the
    /// owning wallet's mnemonic — the caller never supplies private
    /// material, because a key the app can't sign with later would be
    /// useless on the identity.
    struct KeySpec {
        let keyType: KeyType
        let purpose: KeyPurpose
        let securityLevel: SecurityLevel
        let contractBounds: ManagedPlatformWallet.ContractBounds?

        init(
            keyType: KeyType,
            purpose: KeyPurpose,
            securityLevel: SecurityLevel,
            contractBounds: ManagedPlatformWallet.ContractBounds? = nil
        ) {
            self.keyType = keyType
            self.purpose = purpose
            self.securityLevel = securityLevel
            self.contractBounds = contractBounds
        }
    }

    enum KeyAdditionError: LocalizedError {
        case derivationMismatch
        case keychainWriteFailed
        case hash160ComputationFailed
        case blsUnsupported

        var errorDescription: String? {
            switch self {
            case .derivationMismatch:
                return "Derived key didn't match its public key — refusing to broadcast."
            case .keychainWriteFailed:
                return "Could not persist new key to the iOS Keychain — aborted before broadcast."
            case .hash160ComputationFailed:
                return "Could not compute HASH160 of derived pubkey — aborted before broadcast."
            case .blsUnsupported:
                return "BLS derivation is not yet wired through the FFI for this flow. Use ECDSA secp256k1 or ECDSA Hash160."
            }
        }
    }

    /// Derive each requested key against the wallet, pre-persist its
    /// private scalar to the iOS Keychain, and build the matching
    /// `IdentityPubkey` row — without broadcasting. Returns the rows in
    /// the same order as `specs`, with `keyId` slots auto-assigned as
    /// `max(existingKeyIds) + 1`, `+2`, ... so they never collide with
    /// an existing slot.
    ///
    /// Runs on the main actor because `deriveIdentityAuthKeyAtSlot` and
    /// the Keychain writes are main-actor-bound in this app.
    @MainActor
    static func prepareKeys(
        specs: [KeySpec],
        identity: PersistentIdentity,
        wallet: ManagedPlatformWallet,
        walletId: Data,
        network: Network
    ) throws -> [ManagedPlatformWallet.IdentityPubkey] {
        var firstFreeKeyId = (identity.identityPublicKeys.map { $0.id }.max() ?? 0) + 1
        var rows: [ManagedPlatformWallet.IdentityPubkey] = []
        rows.reserveCapacity(specs.count)

        for spec in specs {
            // ECDSA only for the moment — BLS derivation still needs
            // Rust work (mirrors AddIdentityKeyView's gating).
            guard spec.keyType != .bls12_381 else {
                throw KeyAdditionError.blsUnsupported
            }

            let chosenKeyId = firstFreeKeyId
            firstFreeKeyId += 1

            // 1. Derive the keypair via the FFI. The library does the
            //    path build + secp256k1 derive; we get the public bytes
            //    + the 32-byte private scalar back.
            let preview = try wallet.deriveIdentityAuthKeyAtSlot(
                identityIndex: identity.identityIndex,
                keyId: chosenKeyId,
                network: network
            )

            // 2. Verify the derived scalar matches the returned public
            //    key — cheap defence against derivation drift. Validation
            //    goes against the compressed pubkey (33 bytes) regardless
            //    of keyType; the HASH160 variant just stores a different
            //    on-chain payload.
            guard
                KeyValidation.validatePrivateKeyForPublicKey(
                    privateKeyHex: preview.privateKeyData.toHexString(),
                    publicKeyHex: preview.publicKeyHex,
                    keyType: .ecdsaSecp256k1,
                    network: network
                )
            else {
                throw KeyAdditionError.derivationMismatch
            }

            // 3. Pre-persist private bytes to Keychain so the
            //    KeychainSigner trampoline can sign the resulting
            //    updateIdentity transition. The trampoline matches on
            //    metadata.publicKey: for ECDSA_HASH160 that's the
            //    20-byte HASH160 hex, otherwise the 33-byte compressed
            //    pubkey hex.
            let pubKeyHashHex =
                SwiftDashSDK.KeychainManager.computePublicKeyHashHex(preview.publicKeyData)
            let metadataPublicKeyHex: String =
                spec.keyType == .ecdsaHash160 ? pubKeyHashHex : preview.publicKeyHex
            let metadata = IdentityPrivateKeyMetadata(
                identityId: identity.identityIdString,
                keyId: chosenKeyId,
                walletId: walletId.toHexString(),
                identityIndex: identity.identityIndex,
                keyIndex: chosenKeyId,
                derivationPath: preview.derivationPath,
                publicKey: metadataPublicKeyHex,
                publicKeyHash: pubKeyHashHex,
                keyType: spec.keyType.rawValue,
                purpose: spec.purpose.rawValue,
                securityLevel: spec.securityLevel.rawValue
            )
            guard
                KeychainManager.shared.storeIdentityPrivateKey(
                    preview.privateKeyData,
                    derivationPath: preview.derivationPath,
                    metadata: metadata
                ) != nil
            else {
                throw KeyAdditionError.keychainWriteFailed
            }

            // 4. Build the IdentityPubkey. For ECDSA_HASH160 the
            //    on-chain payload is the 20-byte HASH160 of the
            //    compressed pubkey, not the pubkey itself.
            let pubkeyBytesForFFI: Data
            if spec.keyType == .ecdsaHash160 {
                guard let hashBytes = Data(hexString: pubKeyHashHex), hashBytes.count == 20 else {
                    throw KeyAdditionError.hash160ComputationFailed
                }
                pubkeyBytesForFFI = hashBytes
            } else {
                pubkeyBytesForFFI = preview.publicKeyData
            }

            rows.append(
                ManagedPlatformWallet.IdentityPubkey(
                    keyId: chosenKeyId,
                    keyType: spec.keyType,
                    purpose: spec.purpose,
                    securityLevel: spec.securityLevel,
                    pubkeyBytes: pubkeyBytesForFFI,
                    contractBounds: spec.contractBounds
                )
            )
        }

        return rows
    }
}

// MARK: - DPP string parsing

/// Parse the canonical DPP enum token strings the generic transition
/// builder's `addPublicKeys` JSON uses (e.g. `"ECDSA_HASH160"`,
/// `"AUTHENTICATION"`, `"MEDIUM"`). Case-insensitive; tolerant of
/// hyphen/space separators so `"ecdsa-secp256k1"` also resolves.

/// Upper-case and strip `_`, `-`, and space so the switch arms below
/// can match a single canonical form regardless of input punctuation.
private func normalizeDPPToken(_ raw: String) -> String {
    raw.uppercased()
        .replacingOccurrences(of: "_", with: "")
        .replacingOccurrences(of: "-", with: "")
        .replacingOccurrences(of: " ", with: "")
}

extension KeyType {
    init?(dppToken raw: String) {
        switch normalizeDPPToken(raw) {
        case "ECDSASECP256K1", "ECDSA", "SECP256K1": self = .ecdsaSecp256k1
        case "BLS12381", "BLS": self = .bls12_381
        case "ECDSAHASH160", "HASH160": self = .ecdsaHash160
        case "BIP13SCRIPTHASH", "BIP13": self = .bip13ScriptHash
        case "EDDSA25519HASH160", "EDDSA": self = .eddsa25519Hash160
        default: return nil
        }
    }
}

extension KeyPurpose {
    init?(dppToken raw: String) {
        switch normalizeDPPToken(raw) {
        case "AUTHENTICATION", "AUTH": self = .authentication
        case "ENCRYPTION": self = .encryption
        case "DECRYPTION": self = .decryption
        case "TRANSFER": self = .transfer
        case "SYSTEM": self = .system
        case "VOTING": self = .voting
        case "OWNER": self = .owner
        default: return nil
        }
    }
}

extension SecurityLevel {
    init?(dppToken raw: String) {
        switch normalizeDPPToken(raw) {
        case "MASTER": self = .master
        case "CRITICAL": self = .critical
        case "HIGH": self = .high
        case "MEDIUM": self = .medium
        default: return nil
        }
    }
}
