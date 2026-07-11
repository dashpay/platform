//! Wallet `txMetadata` document self-encryption.
//!
//! **WIRE-COMPATIBLE with the legacy `org.dashj.platform` stack**
//! (`BlockchainIdentity.publishTxMetaData` / `getTxMetaData`, dash-sdk-kotlin
//! 4.0.0-RC2) so documents written by either stack decrypt with the other —
//! migrated users must not lose their tx-metadata history (memos, tax
//! categories, exchange-rate records, gift cards). The scheme below was
//! recovered byte-for-byte from the legacy jars (`BlockchainIdentity`,
//! `TxMetadataDocument`) and `org.bitcoinj.crypto.KeyCrypterAESCBC`
//! (dashj-core 22.0.3).
//!
//! ## Scheme
//!
//! - **AES key**: the RAW 32-byte secp256k1 private scalar of a hardened HD
//!   child — NOT ECDH and NOT HKDF. This mirrors
//!   `KeyCrypterAESCBC.deriveKey(ECKey)`, which is literally
//!   `new KeyParameter(ecKey.getPrivKeyBytes())`. (Contrast the DIP-15
//!   DashPay fields in [`super::contact_info`], which DO use ECDH — a
//!   different scheme that must not be reused here.)
//! - **Derivation path**: the identity-auth path of the identity's encryption
//!   key (its key id is the document's `keyIndex` field) extended by two
//!   hardened children `/ 32769' / encryptionKeyIndex'`. In dashj terms:
//!   `<BLOCKCHAIN_IDENTITY accountPath> / keyIndex' / 32769' / encryptionKeyIndex'`.
//!   Rust's [`identity_auth_derivation_path_for_type`] reproduces the dashj
//!   `blockchainIdentityECDSADerivationPath(keyIndex)` prefix for the primary
//!   identity (identity_index 0), so appending the two children reconstructs
//!   the exact legacy key. This is the SAME base-path machinery a registered
//!   identity's keys use, and the SAME extend-by-two-hardened-children shape
//!   as [`super::contact_info::derive_contact_info_keys`].
//! - **Cipher**: AES-256-CBC / PKCS7, random 16-byte IV (BouncyCastle
//!   `PaddedBufferedBlockCipher(CBCBlockCipher(AESEngine))` in the legacy stack).
//! - **Stored `encryptedMetadata` blob layout** (the authoritative
//!   `createTxMetadata` / `decryptTxMetadata` framing — NOT the alternate,
//!   unused `TxMetadataDocument.decrypt` helper):
//!
//!   ```text
//!   byte[0]      = version   (0 = CBOR, 1 = protobuf)  -- NOT encrypted
//!   byte[1..17)  = IV (16 bytes)                        -- NOT encrypted
//!   byte[17..)   = AES-256-CBC(key, IV, plaintext)      -- PKCS7 padded
//!   ```
//!
//! ## Payload boundary (SDK owns the envelope, app owns the item schema)
//!
//! The decrypted plaintext is a protobuf `TxMetadataBatch` (version 1) or a
//! CBOR list (version 0) of the wallet's `TxMetadataItem`s. That item schema
//! (memo / taxCategory / exchangeRate / service / giftCard …) is an
//! APP-level concern — the legacy stack kept it in `org.dashj.platform.wallet`
//! and the app batches items itself. This crate therefore treats the plaintext
//! payload as OPAQUE bytes: [`seal_tx_metadata`] takes already-serialized
//! payload bytes + the version byte, and [`open_tx_metadata`] returns the
//! decrypted payload bytes + version byte. The caller (dash-wallet) keeps
//! ownership of the protobuf (de)serialization and the batching policy, exactly
//! as it did on the legacy stack.

use key_wallet::bip32::ChildNumber;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey, KeyDerivationType};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use zeroize::Zeroizing;

use crate::error::PlatformWalletError;
use crate::wallet::identity::network::identity_auth_derivation_path_for_type;

/// The fixed hardened child index between `keyIndex` and `encryptionKeyIndex`
/// in the tx-metadata key path (`ChildNumber(32769, hardened)` in the legacy
/// `TxMetadataDocument` static init — `0x8001`). "To discount other potential
/// derivations of this key in other applications", as with DIP-15's `1 << 16`.
pub const TX_METADATA_ENCRYPTION_CHILD: u32 = 32769;

/// `encryptedMetadata` version byte: the plaintext is a CBOR list of items.
pub const VERSION_CBOR: u8 = 0;

/// `encryptedMetadata` version byte: the plaintext is a protobuf
/// `TxMetadataBatch`. This is what the wallet writes
/// (`TxMetadataDocument.VERSION_PROTOBUF`).
pub const VERSION_PROTOBUF: u8 = 1;

/// Layout overhead of the stored blob: 1 version byte + 16 IV bytes.
const BLOB_HEADER_LEN: usize = 1 + 16;

/// AES block size — the ciphertext must be a non-zero multiple of this.
const AES_BLOCK_LEN: usize = 16;

/// Build the full tx-metadata key derivation path
/// `identity_auth_path(identity_index, key_index) / 32769' / encryption_key_index'`
/// — the single path both key sources ([`derive_tx_metadata_key`] and
/// [`derive_tx_metadata_key_from_master`]) derive at, so the resident-wallet
/// and resolver-master paths can never drift apart.
pub fn tx_metadata_derivation_path(
    network: Network,
    identity_index: u32,
    key_index: u32,
    encryption_key_index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    let root_path = identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    )?;

    Ok(root_path.extend([
        ChildNumber::from_hardened_idx(TX_METADATA_ENCRYPTION_CHILD).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Invalid txMetadata encryption child index: {e}"
            ))
        })?,
        ChildNumber::from_hardened_idx(encryption_key_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Invalid txMetadata encryptionKeyIndex: {e}"
            ))
        })?,
    ]))
}

/// Derive the AES-256 key for one `txMetadata` document from the wallet seed.
///
/// `key_index` is the document's `keyIndex` field (the identity's registered
/// ENCRYPTION key id); `encryption_key_index` is the document's
/// `encryptionKeyIndex` field (the app's per-document index). The derived key
/// is the raw private scalar at
/// `identity_auth_path(identity_index, key_index) / 32769' / encryption_key_index'`.
///
/// Requires a key-resident wallet (mnemonic / seed / xprv). An
/// external-signable or watch-only wallet has no in-process private keys and
/// fails here with `External signable wallet has no private key` — the caller
/// must resolve the wallet's mnemonic host-side (the platform mnemonic
/// resolver) and use [`derive_tx_metadata_key_from_master`] instead. This is
/// exactly the shape the Android/iOS apps run: their SDK wallets are
/// external-signable and every key derives on demand through the resolver.
pub fn derive_tx_metadata_key(
    wallet: &Wallet,
    network: Network,
    identity_index: u32,
    key_index: u32,
    encryption_key_index: u32,
) -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
    let path =
        tx_metadata_derivation_path(network, identity_index, key_index, encryption_key_index)?;

    let ext = wallet.derive_extended_private_key(&path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Failed to derive txMetadata key: {e}"))
    })?;
    Ok(Zeroizing::new(ext.private_key.secret_bytes()))
}

/// Derive the AES-256 key for one `txMetadata` document from a caller-supplied
/// master extended private key — the external-signable-wallet counterpart of
/// [`derive_tx_metadata_key`], deriving the identical path from the identical
/// seed material (see the cross-path agreement test).
///
/// This is the tx-metadata leg of the codebase's resolver convention (mirrors
/// `derive_ecdsa_identity_auth_keypair_from_master` and the discovery /
/// key-preview paths): when the in-process wallet is external-signable /
/// watch-only, the FFI layer resolves the wallet's mnemonic on demand via the
/// host `MnemonicResolverHandle`, builds the master xprv, calls this, and
/// wipes the master (`master.private_key.non_secure_erase()`) before
/// returning — atomic derive + use + zeroize. The returned scalar is
/// [`Zeroizing`], so the key itself is scrubbed on drop as well.
pub fn derive_tx_metadata_key_from_master(
    master: &ExtendedPrivKey,
    network: Network,
    identity_index: u32,
    key_index: u32,
    encryption_key_index: u32,
) -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
    use dashcore::secp256k1::Secp256k1;

    let path =
        tx_metadata_derivation_path(network, identity_index, key_index, encryption_key_index)?;

    let secp = Secp256k1::new();
    // `ExtendedPrivKey` has no `Drop`/`Zeroize`; its inner
    // `secp256k1::SecretKey` memzeroes on drop, and the scalar copy we
    // return is wrapped in `Zeroizing` (same hygiene note as
    // `derive_ecdsa_identity_auth_keypair_from_master`).
    let derived = master.derive_priv(&secp, &path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to derive txMetadata key from master: {e}"
        ))
    })?;
    Ok(Zeroizing::new(derived.private_key.secret_bytes()))
}

/// Seal an already-serialized `txMetadata` payload into the stored
/// `encryptedMetadata` blob: `version(1) ‖ IV(16) ‖ AES-256-CBC(payload)`.
///
/// `payload` is the app's opaque plaintext (a protobuf `TxMetadataBatch` when
/// `version == VERSION_PROTOBUF`); this crate does not parse it. `iv` MUST be a
/// fresh random 16 bytes per document (the legacy stack draws it from
/// `SecureRandom`).
pub fn seal_tx_metadata(key: &[u8; 32], version: u8, iv: &[u8; 16], payload: &[u8]) -> Vec<u8> {
    let ciphertext = platform_encryption::encrypt_aes_256_cbc(key, iv, payload);
    let mut blob = Vec::with_capacity(BLOB_HEADER_LEN + ciphertext.len());
    blob.push(version);
    blob.extend_from_slice(iv);
    blob.extend_from_slice(&ciphertext);
    blob
}

/// The plaintext recovered from a stored `encryptedMetadata` blob.
///
/// `Debug` is hand-written (NOT derived) so a stray `{:?}` / `dbg!()` / tracing
/// statement can never leak the decrypted financial plaintext into a log — the
/// same redaction as [`super::super::network::encrypted_document::DecryptedEncryptedDocument`].
/// The payload is redacted to its length.
#[derive(Clone, PartialEq, Eq)]
pub struct OpenedTxMetadata {
    /// The blob's leading version byte (0 = CBOR, 1 = protobuf). The app
    /// dispatches its payload parse on this.
    pub version: u8,
    /// The decrypted, PKCS7-unpadded payload bytes — opaque to this crate.
    pub payload: Vec<u8>,
}

impl std::fmt::Debug for OpenedTxMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenedTxMetadata")
            .field("version", &self.version)
            // Redacted: never render the decrypted plaintext.
            .field("payload", &format_args!("<{} bytes redacted>", self.payload.len()))
            .finish()
    }
}

/// Open a stored `encryptedMetadata` blob: split off the version byte + IV and
/// AES-256-CBC-decrypt the remainder, returning the version + opaque payload.
///
/// Errors (never panics) on a malformed blob — too short, a ciphertext length
/// that is not a positive multiple of the AES block size, or a decrypt/unpad
/// failure (e.g. the wrong key, which PKCS7 rejects). A malformed or
/// wrong-keyed document must be skipped by the caller, not abort a sync.
pub fn open_tx_metadata(
    key: &[u8; 32],
    blob: &[u8],
) -> Result<OpenedTxMetadata, PlatformWalletError> {
    if blob.len() < BLOB_HEADER_LEN + AES_BLOCK_LEN {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "txMetadata encryptedMetadata is {} bytes; below the {}-byte minimum \
             (version + IV + one AES block)",
            blob.len(),
            BLOB_HEADER_LEN + AES_BLOCK_LEN
        )));
    }
    let ciphertext = &blob[BLOB_HEADER_LEN..];
    if !ciphertext.len().is_multiple_of(AES_BLOCK_LEN) {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "txMetadata ciphertext length {} is not a multiple of the AES block size",
            ciphertext.len()
        )));
    }

    let version = blob[0];
    let iv: [u8; 16] = blob[1..BLOB_HEADER_LEN]
        .try_into()
        .expect("slice [1..17) is exactly 16 bytes");

    let payload = platform_encryption::decrypt_aes_256_cbc(key, &iv, ciphertext).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("txMetadata decrypt failed: {e}"))
    })?;

    Ok(OpenedTxMetadata { version, payload })
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    fn test_wallet() -> Wallet {
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet")
    }

    /// Key derivation is deterministic and every path component
    /// (`key_index`, `encryption_key_index`) is load-bearing.
    #[test]
    fn key_derivation_is_deterministic_and_index_separated() {
        let wallet = test_wallet();

        let a = derive_tx_metadata_key(&wallet, Network::Testnet, 0, 3, 1).expect("derive");
        let a2 = derive_tx_metadata_key(&wallet, Network::Testnet, 0, 3, 1).expect("derive");
        assert_eq!(*a, *a2, "same inputs must yield the same key");

        let diff_enc = derive_tx_metadata_key(&wallet, Network::Testnet, 0, 3, 2).expect("derive");
        assert_ne!(
            *a, *diff_enc,
            "encryptionKeyIndex must change the derived key"
        );

        let diff_key = derive_tx_metadata_key(&wallet, Network::Testnet, 0, 4, 1).expect("derive");
        assert_ne!(*a, *diff_key, "keyIndex must change the derived key");
    }

    /// Full seal → open round-trip across both version bytes.
    #[test]
    fn seal_open_round_trips() {
        let key = [0x11u8; 32];
        let iv = [0x22u8; 16];
        for version in [VERSION_CBOR, VERSION_PROTOBUF] {
            let payload = b"opaque protobuf TxMetadataBatch bytes".to_vec();
            let blob = seal_tx_metadata(&key, version, &iv, &payload);
            // Framing: version at [0], IV at [1..17), ciphertext after.
            assert_eq!(blob[0], version);
            assert_eq!(&blob[1..17], &iv);
            let opened = open_tx_metadata(&key, &blob).expect("open");
            assert_eq!(opened.version, version);
            assert_eq!(opened.payload, payload);
        }
    }

    /// A wrong key can never recover the plaintext: PKCS7 rejects it (Err), or
    /// on the rare valid-padding collision the payload differs — never the
    /// original. Must not panic.
    #[test]
    fn wrong_key_open_fails_cleanly() {
        let key = [0x33u8; 32];
        let wrong = [0x44u8; 32];
        let iv = [0x55u8; 16];
        let payload = b"secret memo".to_vec();
        let blob = seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &payload);

        match open_tx_metadata(&wrong, &blob) {
            Err(_) => {}
            Ok(opened) => assert_ne!(
                opened.payload, payload,
                "a wrong key must not recover the original plaintext"
            ),
        }
    }

    /// Malformed blobs error rather than panic.
    #[test]
    fn open_rejects_malformed_blobs() {
        let key = [0u8; 32];
        // Too short (only version + partial IV).
        assert!(open_tx_metadata(&key, &[1u8; 10]).is_err());
        // Version + IV but ciphertext not block-aligned (17 + 5 bytes).
        assert!(open_tx_metadata(&key, &[0u8; 22]).is_err());
    }

    /// The two key sources — resident wallet vs a resolver-supplied master
    /// xprv from the SAME mnemonic — must derive the IDENTICAL key at every
    /// `(identity_index, key_index, encryption_key_index)` slot. This pins
    /// the external-signable-wallet fix (the Android/iOS shape derives via
    /// the mnemonic resolver → master; test fixtures derive in-wallet):
    /// if the two paths ever drift, decrypt breaks silently on-device.
    #[test]
    fn master_derivation_matches_resident_wallet_derivation() {
        use key_wallet::mnemonic::{Language, Mnemonic};

        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("valid test mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet = Wallet::from_mnemonic(
            mnemonic,
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("wallet from mnemonic");
        // The exact master the FFI's `resolve_master_from_resolver` builds
        // from the host-resolved mnemonic (`to_seed("") → new_master`).
        let master =
            ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master from seed");

        for (identity_index, key_index, encryption_key_index) in
            [(0, 2, 1), (0, 2, 7), (0, 3, 1), (1, 2, 1)]
        {
            let resident = derive_tx_metadata_key(
                &wallet,
                Network::Testnet,
                identity_index,
                key_index,
                encryption_key_index,
            )
            .expect("resident derive");
            let from_master = derive_tx_metadata_key_from_master(
                &master,
                Network::Testnet,
                identity_index,
                key_index,
                encryption_key_index,
            )
            .expect("master derive");
            assert_eq!(
                *resident, *from_master,
                "resident-wallet and resolver-master key derivations must agree at \
                 ({identity_index},{key_index},{encryption_key_index})"
            );
        }
    }

    /// The external-signable wallet shape (the Android/iOS apps: NO resident
    /// private keys — every key derives host-side through the mnemonic
    /// resolver): the in-wallet derive must fail (this exact failure zeroed
    /// the on-device decrypt-proof), and the resolver-master path — fed by a
    /// stub "resolver" supplying the test mnemonic — must decrypt a blob the
    /// resident stack sealed. Round-trips seal(resident) → open(master) and
    /// seal(master) → open(resident), proving an external-signable device
    /// wallet reads and writes documents interchangeably with a key-resident
    /// wallet on the same mnemonic.
    #[test]
    fn external_signable_wallet_derives_via_resolver_master() {
        use key_wallet::account::AccountCollection;
        use key_wallet::mnemonic::{Language, Mnemonic};

        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("valid test mnemonic");
        let seed = mnemonic.to_seed("");

        // The device shape: an external-signable wallet with no in-process
        // private keys.
        let external_wallet = Wallet::new_external_signable(
            Network::Testnet,
            [0x42u8; 32],
            AccountCollection::new(),
        );
        let err = derive_tx_metadata_key(&external_wallet, Network::Testnet, 0, 2, 1)
            .expect_err("an external-signable wallet has no in-process key to derive from");
        assert!(
            err.to_string().contains("no private key"),
            "must fail with the no-private-key shape the device hit, got: {err}"
        );

        // The resolver stub: the host returns the wallet's mnemonic; the FFI
        // builds the master exactly like this and derives from it.
        let master =
            ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master from seed");
        let master_key = derive_tx_metadata_key_from_master(&master, Network::Testnet, 0, 2, 1)
            .expect("master derive");

        // A resident wallet on the same mnemonic (the legacy stack / a test
        // fixture) seals; the external-signable wallet (via the resolver
        // master) opens — and vice versa.
        let resident_wallet = Wallet::from_mnemonic(
            Mnemonic::from_phrase(
                "abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon about",
                Language::English,
            )
            .expect("valid test mnemonic"),
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("wallet from mnemonic");
        let resident_key = derive_tx_metadata_key(&resident_wallet, Network::Testnet, 0, 2, 1)
            .expect("resident derive");

        let payload = b"external-signable round-trip".to_vec();
        let iv = [0x66u8; 16];

        let sealed_by_resident = seal_tx_metadata(&resident_key, VERSION_PROTOBUF, &iv, &payload);
        let opened_by_master =
            open_tx_metadata(&master_key, &sealed_by_resident).expect("master key opens");
        assert_eq!(opened_by_master.payload, payload);

        let sealed_by_master = seal_tx_metadata(&master_key, VERSION_PROTOBUF, &iv, &payload);
        let opened_by_resident =
            open_tx_metadata(&resident_key, &sealed_by_master).expect("resident key opens");
        assert_eq!(opened_by_resident.payload, payload);
    }

    /// Secondary cross-stack check of the AES-256-CBC core + blob framing,
    /// pinned to a PUBLISHED third-party vector (NIST SP 800-38A F.2.5,
    /// CBC-AES256.Encrypt). Any conformant AES-256-CBC implementation —
    /// including the legacy stack's BouncyCastle `KeyCrypterAESCBC` — produces
    /// this exact first ciphertext block for this (key, IV, plaintext-block).
    /// PKCS7 appends a full padding block for a 16-byte plaintext but does NOT
    /// alter the first block, so the leading 16 ciphertext bytes match NIST
    /// byte-for-byte. This isolates the ENVELOPE (cipher + `version ‖ IV ‖
    /// ciphertext` layout) against a standards body.
    ///
    /// The end-to-end HD-derivation + envelope wire-compat guarantee is pinned
    /// by [`legacy_dashj_wire_compat_vector`], whose vector was generated by the
    /// real dashj stack; this NIST test is the narrower cipher-conformance leg.
    #[test]
    fn nist_cbc_aes256_cross_stack_vector() {
        // NIST SP 800-38A F.2.5.
        let key: [u8; 32] =
            hex_lit("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        let iv: [u8; 16] = hex_lit("000102030405060708090a0b0c0d0e0f");
        let plaintext_block: [u8; 16] = hex_lit("6bc1bee22e409f96e93d7e117393172a");
        let expected_ct_block1: [u8; 16] = hex_lit("f58c4c04d6e5f1ba779eabfb5f7bfbd6");

        let blob = seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &plaintext_block);

        // version ‖ IV ‖ ciphertext(2 blocks: data + PKCS7 pad).
        assert_eq!(blob.len(), 1 + 16 + 32, "1 version + 16 IV + 2 AES blocks");
        assert_eq!(blob[0], VERSION_PROTOBUF, "version byte at offset 0");
        assert_eq!(&blob[1..17], &iv, "IV at offset 1..17");
        assert_eq!(
            &blob[17..33],
            &expected_ct_block1,
            "first ciphertext block must match the NIST CBC-AES256 vector"
        );

        // And the framing round-trips back to the original block.
        let opened = open_tx_metadata(&key, &blob).expect("open");
        assert_eq!(opened.version, VERSION_PROTOBUF);
        assert_eq!(opened.payload, plaintext_block);
    }

    /// Tiny fixed-size hex decoder for the test vectors (no extra dep).
    fn hex_lit<const N: usize>(s: &str) -> [u8; N] {
        let bytes = hex::decode(s).expect("valid hex");
        bytes.try_into().expect("length matches")
    }

    /// **The wire-compat anchor**: an end-to-end vector generated by the ACTUAL
    /// legacy stack (dash-sdk-kotlin 4.0.0-RC2 + dashj-core 22.0.3, run under a
    /// JVM), proving the mnemonic→AES-key HD derivation AND the full
    /// `version ‖ IV ‖ AES-256-CBC(payload)` envelope match dashj byte-for-byte.
    /// This pins the one piece static analysis of the jars alone could not (the
    /// derivation-path account prefix): it is now reconstructed exactly and
    /// checked in CI, so a future refactor that moves the path drifts loudly.
    ///
    /// ## How the vector was generated (reproducible)
    ///
    /// A JVM scratch program built the legacy key + blob for the BIP-39 test
    /// mnemonic `abandon abandon … about` (empty passphrase), Testnet:
    ///
    /// 1. `seed = MnemonicCode.toSeed(words, "")`;
    ///    `root = HDKeyDerivation.createMasterPrivateKey(seed)`.
    /// 2. `accountPath = DerivationPathFactory(TestNet3Params)`
    ///    `.blockchainIdentityECDSADerivationPath()` = `m/9'/1'/5'/0'/0'/0'`
    ///    (this is the account path the `BLOCKCHAIN_IDENTITY`
    ///    `AuthenticationKeyChain` is built with, via
    ///    `AuthenticationGroupExtension.getDefaultPath`).
    /// 3. Reproducing `BlockchainIdentity.privateKeyAtPath(keyId, childNumber,`
    ///    `encryptionKeyIndex, ECDSA, …)`, the full path is
    ///    `accountPath / keyId' / 32769' / encryptionKeyIndex'` with
    ///    `keyId = 2` (the id of the identity's `ENCRYPTION`/`MEDIUM` public key
    ///    in `BlockchainIdentity.createIdentityPublicKeys`: keys are
    ///    id0=AUTH/MASTER, id1=AUTH/HIGH, **id2=ENCRYPTION/MEDIUM**,
    ///    id3=TRANSFER/CRITICAL), `32769'` = `TxMetadataDocument.childNumber`,
    ///    and `encryptionKeyIndex = 1` (dash-wallet's first
    ///    `1 + countAllRequests()`). The derived key is
    ///    `key = hierarchy.get(fullPath, false, true).getPrivKeyBytes()`.
    /// 4. The blob was built exactly as `BlockchainIdentity.createTxMetadata`
    ///    does: `KeyCrypterAESCBC().deriveKey(ECKey.fromPrivate(key))`
    ///    (`= new KeyParameter(key)`), `KeyCrypterAESCBC.encrypt(payload, aes)`,
    ///    then framed `version(1) ‖ IV(16) ‖ encryptedBytes`.
    ///
    /// Legacy source of record (the wire-compat reference this crate mirrors):
    /// `org.dashj.platform.dashpay.BlockchainIdentity.{createTxMetadata,`
    /// `decryptTxMetadata,privateKeyAtPath}`,
    /// `org.bitcoinj.wallet.DerivationPathFactory.blockchainIdentityECDSADerivationPath`,
    /// `org.dashj.platform.contracts.wallet.TxMetadataDocument.childNumber`,
    /// `org.bitcoinj.crypto.KeyCrypterAESCBC.{deriveKey,encrypt}`.
    #[test]
    fn legacy_dashj_wire_compat_vector() {
        use key_wallet::mnemonic::{Language, Mnemonic};

        // BIP-39 standard test mnemonic, empty passphrase, Testnet.
        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("valid test mnemonic");
        let wallet = Wallet::from_mnemonic(mnemonic, Network::Testnet, WalletAccountCreationOptions::None)
            .expect("wallet from mnemonic");

        // identity_index 0 (the wallet's single identity), key_index 2 (the
        // ENCRYPTION/MEDIUM key id), encryptionKeyIndex 1 (first document).
        let key = derive_tx_metadata_key(&wallet, Network::Testnet, 0, 2, 1).expect("derive");

        // The AES key dashj derived at m/9'/1'/5'/0'/0'/0'/2'/32769'/1'.
        let legacy_key: [u8; 32] =
            hex_lit("4a2eaec1ad959105738996b49e0327f96a80b765249d2c9af8cf6aa689aa84d7");
        assert_eq!(
            *key, legacy_key,
            "tx-metadata HD key derivation must match the legacy dashj stack byte-for-byte"
        );

        // The resolver-master path (the on-device external-signable shape)
        // must hit the same dashj key — pins the fix's derivation to the
        // legacy vector, not just to the resident path.
        let master = ExtendedPrivKey::new_master(
            Network::Testnet,
            &key_wallet::mnemonic::Mnemonic::from_phrase(
                "abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon about",
                key_wallet::mnemonic::Language::English,
            )
            .expect("valid test mnemonic")
            .to_seed(""),
        )
        .expect("master from seed");
        let key_via_master =
            derive_tx_metadata_key_from_master(&master, Network::Testnet, 0, 2, 1)
                .expect("master derive");
        assert_eq!(
            *key_via_master, legacy_key,
            "resolver-master tx-metadata derivation must match the legacy dashj stack too"
        );

        // The full stored blob dashj produced (KeyCrypterAESCBC over the
        // plaintext below, framed version ‖ IV ‖ ciphertext). Rust must open it
        // and recover the exact plaintext — proving key + cipher + framing are
        // all wire-compatible end to end.
        let legacy_blob = hex::decode(
            "01b79799f5f18c171741700d9906925eae84f1144e0e532e1981b99cf4fffb8ff\
             13754d5a5408c24f1c51185fe53e3b8ae086aa57c30653c52907da21f18ec473c",
        )
        .expect("valid hex");
        let expected_plaintext = b"legacy-txmetadata-wire-compat-vector".to_vec();

        let opened = open_tx_metadata(&key, &legacy_blob).expect("open legacy blob");
        assert_eq!(opened.version, VERSION_PROTOBUF, "version byte");
        assert_eq!(
            opened.payload, expected_plaintext,
            "Rust must decrypt a dashj-produced txMetadata blob to the original plaintext"
        );
    }

    /// **The nonzero-`identity_index` wire-compat anchor** (dashpay/platform#4091,
    /// blocking review finding). The primary [`legacy_dashj_wire_compat_vector`]
    /// pins `identity_index = 0`, but `KeyDerivationType::ECDSA` is also `0` and
    /// sits at the path position immediately before `identity_index`
    /// (`base / key_type' / identity_index' / key_index' / …`, see
    /// [`identity_auth_derivation_path_for_type`]), so at index 0 those two
    /// adjacent `0'` components are indistinguishable — that vector would pass
    /// even if `identity_index` were dropped, swapped, or misplaced. This vector
    /// uses `identity_index = 1` so the derived path
    /// `m/9'/1'/5'/0'/0'/1'/2'/32769'/1'` differs from the index-0 path in
    /// exactly the `identity_index` component, and the resulting legacy key
    /// (`8cda…5196`) is provably distinct from the index-0 key (`4a2e…84d7`) —
    /// empirically proving the component is wired to the correct path slot.
    ///
    /// ## How the vector was generated (reproducible)
    ///
    /// Same real legacy stack (dash-sdk-kotlin 4.0.0-RC2 semantics + dashj-core
    /// 22.0.3, run under a JVM) as [`legacy_dashj_wire_compat_vector`], via the
    /// checked-in generator `LegacyKeyN.java` (see this crate's
    /// `tests/legacy_wire_compat/README.md`):
    ///
    /// ```text
    /// javac -cp <dashj-core-22.0.3.jar:bcprov:guava:slf4j> LegacyKeyN.java
    /// java  -cp .:<same cp> LegacyKeyN 1 2 1
    ///   fullPath=m/9'/1'/5'/0'/0'/1'/2'/32769'/1'
    ///   AES_KEY=8cdadb6b8bcf8defd416f2f032255173df89478c971bb96ae9f3511aae355196
    ///   BLOB=01496ce7…2cba627383   (random per run — the IV differs; key is fixed)
    /// ```
    ///
    /// `identity_index = 1` (the wallet's second Platform identity), `key_index`
    /// (keyId) `2` (ENCRYPTION/MEDIUM), `encryptionKeyIndex` `1`. The BIP-39 test
    /// mnemonic `abandon abandon … about`, empty passphrase, Testnet. The key is
    /// deterministic; the blob's IV is fresh `SecureRandom` per generation, so
    /// the exact blob bytes below are one captured run (any IV opens fine).
    #[test]
    fn legacy_dashj_wire_compat_vector_nonzero_identity_index() {
        use key_wallet::mnemonic::{Language, Mnemonic};

        const PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about";

        let wallet = Wallet::from_mnemonic(
            Mnemonic::from_phrase(PHRASE, Language::English).expect("valid test mnemonic"),
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("wallet from mnemonic");

        // identity_index 1 (a NON-primary identity), key_index 2, encryptionKeyIndex 1.
        let key = derive_tx_metadata_key(&wallet, Network::Testnet, 1, 2, 1).expect("derive");

        // The AES key dashj derived at m/9'/1'/5'/0'/0'/1'/2'/32769'/1'.
        let legacy_key: [u8; 32] =
            hex_lit("8cdadb6b8bcf8defd416f2f032255173df89478c971bb96ae9f3511aae355196");
        // Distinct from the identity_index=0 key — the whole point of this vector.
        let index0_key: [u8; 32] =
            hex_lit("4a2eaec1ad959105738996b49e0327f96a80b765249d2c9af8cf6aa689aa84d7");
        assert_ne!(
            legacy_key, index0_key,
            "identity_index=1 must derive a different key than identity_index=0"
        );
        assert_eq!(
            *key, legacy_key,
            "nonzero-identity_index tx-metadata HD key must match the legacy dashj stack \
             byte-for-byte (proves identity_index is wired to the correct path slot)"
        );

        // The resolver-master path (on-device external-signable shape) must hit
        // the same dashj key at the nonzero identity_index too.
        let master = ExtendedPrivKey::new_master(
            Network::Testnet,
            &Mnemonic::from_phrase(PHRASE, Language::English)
                .expect("valid test mnemonic")
                .to_seed(""),
        )
        .expect("master from seed");
        let key_via_master =
            derive_tx_metadata_key_from_master(&master, Network::Testnet, 1, 2, 1)
                .expect("master derive");
        assert_eq!(
            *key_via_master, legacy_key,
            "resolver-master derivation must match the legacy dashj stack at identity_index=1"
        );

        // The full stored blob dashj produced at this slot — Rust must open it.
        let legacy_blob = hex::decode(
            "01496ce7b7aa8baa910eb278dc38aee86522e841414d7b273da86df2106b0548e\
             ee7b6957bb1789512cd00bf90663690cae4202bd1f9ae5f84859b8d2cba627383",
        )
        .expect("valid hex");
        let expected_plaintext = b"legacy-txmetadata-wire-compat-vector".to_vec();

        let opened = open_tx_metadata(&key, &legacy_blob).expect("open legacy blob");
        assert_eq!(opened.version, VERSION_PROTOBUF, "version byte");
        assert_eq!(
            opened.payload, expected_plaintext,
            "Rust must decrypt a dashj-produced nonzero-identity_index blob to the plaintext"
        );
    }
}
