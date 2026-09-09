import Foundation
import SwiftDashSDK

/// Identity-registration key derivation used by the invitation-claim flow.
///
/// Claiming an invitation is ordinary identity registration funded by the
/// imported voucher, so the invitee's keys are derived exactly as for a normal
/// new-identity registration — including the DashPay Encryption/Decryption pair
/// that lets the new identity send a contact request back to the inviter.
///
/// `makeDashpayKeyPair` is intentionally a MIRROR of
/// `CreateIdentityView.makeDashpayKeyPair`; any change to the DashPay-key
/// derivation policy must be made in both (a future refactor should unify them).
enum IdentityRegistrationKeys {
    /// DashPay data-contract id — the enc/dec keys are contract-bounded to it.
    static let dashpayContractId = Data([
        162, 161, 180, 172, 111, 239, 34, 234,
        42, 26, 104, 232, 18, 54, 68, 179,
        87, 135, 95, 107, 65, 44, 24, 16,
        146, 129, 193, 70, 231, 178, 113, 188,
    ])

    /// DashPay document type these keys are bound to — the only one in the
    /// contract that declares `requiresIdentityEncryptionBoundedKey`.
    static let dashpayContactRequestDocumentType = "contactRequest"

    /// Derive + persist the DashPay Encryption (kid=firstKeyId) + Decryption
    /// (kid=firstKeyId+1) key pair for a registering/claiming identity, bounded
    /// to DashPay's `contactRequest` document type, MEDIUM security level,
    /// ECDSA-secp256k1. Each key is cross-checked against its public key and
    /// written to the Keychain by pubkey hash so the signing trampoline can find
    /// it, then returned as `IdentityPubkey` rows to append to the registration /
    /// claim key set.
    @MainActor
    static func makeDashpayKeyPair(
        managedWallet: ManagedPlatformWallet,
        walletId: Data,
        identityIndex: UInt32,
        firstKeyId: UInt32,
        network: Network
    ) throws -> [ManagedPlatformWallet.IdentityPubkey] {
        let purposes: [(keyId: UInt32, purpose: KeyPurpose)] = [
            (firstKeyId, .encryption),
            (firstKeyId + 1, .decryption),
        ]
        let bounds: ManagedPlatformWallet.ContractBounds = .singleContractDocumentType(
            id: dashpayContractId,
            documentTypeName: dashpayContactRequestDocumentType
        )
        let walletIdHex = walletId.toHexString()
        // Overwritten by the persister callback when the identity actually lands
        // on-chain; the metadata written here only needs to satisfy the keychain
        // round-trip lookup by pubkey hex.
        let identityIdPlaceholder = ""

        var rows: [ManagedPlatformWallet.IdentityPubkey] = []
        rows.reserveCapacity(purposes.count)

        for (keyId, purpose) in purposes {
            let preview = try managedWallet.deriveIdentityAuthKeyAtSlot(
                identityIndex: identityIndex,
                keyId: keyId,
                network: network
            )

            // Defence against derivation drift / FFI marshalling bugs. A
            // mismatched DashPay key lands on Platform as a key the trampoline
            // can't sign with and surfaces as an opaque "encrypted xpub" failure
            // on the first contact-request flow — much harder to debug after the
            // fact than failing fast here.
            guard
                KeyValidation.validatePrivateKeyForPublicKey(
                    privateKeyHex: preview.privateKeyData.toHexString(),
                    publicKeyHex: preview.publicKeyHex,
                    keyType: .ecdsaSecp256k1,
                    network: network
                )
            else {
                throw PlatformWalletError.walletOperation(
                    "Derived DashPay key (kid \(keyId), purpose \(purpose.name)) didn't match its public key — refusing to persist"
                )
            }

            let pubKeyHashHex = SwiftDashSDK.KeychainManager.computePublicKeyHashHex(
                preview.publicKeyData
            )
            let metadata = IdentityPrivateKeyMetadata(
                identityId: identityIdPlaceholder,
                keyId: keyId,
                walletId: walletIdHex,
                identityIndex: identityIndex,
                keyIndex: keyId,
                derivationPath: preview.derivationPath,
                publicKey: preview.publicKeyHex,
                publicKeyHash: pubKeyHashHex,
                keyType: KeyType.ecdsaSecp256k1.rawValue,
                purpose: purpose.rawValue,
                securityLevel: SecurityLevel.medium.rawValue
            )
            guard KeychainManager.shared.storeIdentityPrivateKey(
                preview.privateKeyData,
                derivationPath: preview.derivationPath,
                metadata: metadata
            ) != nil else {
                throw PlatformWalletError.walletOperation(
                    "Could not persist DashPay key (kid \(keyId), purpose \(purpose.name)) to Keychain"
                )
            }
            rows.append(
                ManagedPlatformWallet.IdentityPubkey(
                    keyId: keyId,
                    keyType: .ecdsaSecp256k1,
                    purpose: purpose,
                    securityLevel: .medium,
                    pubkeyBytes: preview.publicKeyData,
                    contractBounds: bounds
                )
            )
        }
        return rows
    }
}
