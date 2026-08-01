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
//!   `blockchainIdentityECDSADerivationPath()` prefix for the primary
//!   identity (identity_index 0), so appending the two children reconstructs
//!   the exact legacy key. This is the SAME base-path machinery a registered
//!   identity's keys use, and the SAME extend-by-two-hardened-children shape
//!   as [`super::contact_info::derive_contact_info_keys`].
//!
//!   **Wire-compat holds only at `identity_index == 0`.** The legacy
//!   `createTxMetadata` flow always derives against the wallet's PRIMARY
//!   blockchain identity (`AuthenticationGroupExtension.getDefaultPath` calls
//!   `blockchainIdentityECDSADerivationPath()` with no argument = index 0), so
//!   the legacy scheme has NO identity-index component. Rust exposes an
//!   `identity_index` parameter for forward compatibility, but only the
//!   `identity_index == 0` derivation corresponds to a key any legacy wallet
//!   ever wrote. See [`derive_tx_metadata_key`] and the
//!   `legacy_dashj_wire_compat_vector` test (verified byte-for-byte against the
//!   real dashj `DerivationPathFactory`).
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

/// `maxItems` of the wallet-utils contract's `encryptedMetadata` byteArray
/// field: the stored blob must not exceed this or the document is rejected by
/// DPP schema validation at broadcast.
const ENCRYPTED_METADATA_FIELD_MAX: usize = 4096;

/// The largest plaintext payload [`seal_tx_metadata`] can accept while keeping
/// the stored blob within the [`ENCRYPTED_METADATA_FIELD_MAX`]-byte field limit.
///
/// The blob is `version(1) ‖ IV(16) ‖ AES-256-CBC/PKCS7(plaintext)`. PKCS7
/// **always** appends a full padding block when the plaintext is block-aligned,
/// so the ciphertext length for a plaintext of `L` bytes is
/// `16 * (L / 16 + 1)` — the next multiple of 16 strictly greater than `L`.
/// The blob length is therefore `17 + 16 * (L / 16 + 1)`.
///
/// Derived (not hardcoded) from the field limit so the boundary stays correct
/// if the framing ever changes: the largest ciphertext that fits is
/// `((4096 - 17) / 16) * 16 = 254 * 16 = 4064` bytes, and because PKCS7 spends
/// at least one byte of the final block on padding, the largest plaintext is one
/// less — **4063**. That plaintext frames to `17 + 4064 = 4081` bytes, which
/// fits. `L = 4064` is block-aligned, so PKCS7 adds a whole 16-byte block →
/// ciphertext 4080 → blob `17 + 4080 = 4097`, which overflows. So 4063 is the
/// true maximum and 4064 the first rejected length.
///
/// Note the envelope for the maximum plaintext is 4081 bytes, not 4096: the
/// gap 4082..=4096 is unreachable because the next plaintext byte (4064) forces
/// a fresh padding block that jumps straight to 4097. (The 4063/4064 boundary
/// itself matches the reviewer's figure; the "4063 → 4096" envelope size in the
/// review does not — see the module tests, which pin the real 4081-byte blob.)
pub const MAX_TX_METADATA_PLAINTEXT_LEN: usize = {
    // Largest whole ciphertext (a multiple of the AES block) that still fits
    // the field alongside the version+IV header.
    let max_ciphertext =
        ((ENCRYPTED_METADATA_FIELD_MAX - BLOB_HEADER_LEN) / AES_BLOCK_LEN) * AES_BLOCK_LEN;
    // PKCS7 always consumes ≥ 1 byte of the final block for padding, so the
    // plaintext is at most one byte short of that ciphertext length.
    max_ciphertext - 1
};

/// Reject a `txMetadata` plaintext that cannot fit the `encryptedMetadata`
/// field once sealed, BEFORE any key derivation or network work.
///
/// Callers on the create path (`prepare_encrypted_txmetadata_properties`, and
/// the FFI/JNI entry points) run this first so an over-large batch fails with a
/// typed [`PlatformWalletError::TxMetadataPayloadTooLarge`] up front instead of
/// deriving the key, sealing, and dying at broadcast with an opaque DPP schema
/// error. [`seal_tx_metadata`] also enforces it as the choke-point last line of
/// defense.
pub fn ensure_tx_metadata_payload_fits(payload_len: usize) -> Result<(), PlatformWalletError> {
    if payload_len > MAX_TX_METADATA_PLAINTEXT_LEN {
        return Err(PlatformWalletError::TxMetadataPayloadTooLarge {
            len: payload_len,
            max: MAX_TX_METADATA_PLAINTEXT_LEN,
        });
    }
    Ok(())
}

/// Reject a `txMetadata` wire version byte the legacy stack cannot decode.
///
/// Decidable from the argument alone, so entry points run it before touching a
/// payload, a wallet, a host key resolver or the network: consulting a device
/// keychain — which on some hosts prompts the user — for a request that can
/// never be sealed is work nobody asked for. [`seal_tx_metadata`] keeps the
/// same check as its choke-point last line of defense, so a caller that skips
/// the early gate still cannot produce an undecodable document.
pub fn ensure_tx_metadata_version_supported(version: u8) -> Result<(), PlatformWalletError> {
    if version != VERSION_CBOR && version != VERSION_PROTOBUF {
        return Err(PlatformWalletError::UnsupportedTxMetadataVersion { version });
    }
    Ok(())
}

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
/// ## Legacy wire-compat is guaranteed ONLY at `identity_index == 0`
///
/// The legacy dashj `createTxMetadata` flow has no identity-index parameter —
/// it always derives against the primary blockchain identity
/// (`blockchainIdentityECDSADerivationPath()`, index 0). Only
/// `derive_tx_metadata_key(_, _, 0, key_index, enc)` reproduces a key a legacy
/// wallet could have written; it matches the real dashj-derived key
/// byte-for-byte (see `legacy_dashj_wire_compat_vector`, whose value was
/// checked against the actual `DerivationPathFactory`). A nonzero
/// `identity_index` derives a valid, deterministic, distinct key for THIS
/// stack's own future use, but it corresponds to no legacy-written document —
/// there is no legacy path that reaches it. Do not treat a nonzero-index key as
/// a cross-stack compatibility guarantee.
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
///
/// `version` MUST be [`VERSION_CBOR`] (0) or [`VERSION_PROTOBUF`] (1) — the only
/// two values the legacy dashj `decryptTxMetadata` switches on. Sealing any
/// other byte would produce a document that installs fine but the legacy stack
/// cannot decode, silently breaking the bidirectional wire-compat guarantee. It
/// is rejected HERE, at the one choke point every layer funnels through, so no
/// caller can produce such a document by reaching this function directly; entry
/// points reject it earlier via [`ensure_tx_metadata_version_supported`].
///
/// `payload` must be at most [`MAX_TX_METADATA_PLAINTEXT_LEN`] bytes: a larger
/// plaintext seals into a blob that overflows the `encryptedMetadata` field and
/// would be rejected at broadcast with an opaque DPP schema error. This is the
/// last-line-of-defense enforcement of the same limit the create path
/// pre-checks up front (see [`ensure_tx_metadata_payload_fits`]); over-large
/// payloads fail with a typed [`PlatformWalletError::TxMetadataPayloadTooLarge`].
pub fn seal_tx_metadata(
    key: &[u8; 32],
    version: u8,
    iv: &[u8; 16],
    payload: &[u8],
) -> Result<Vec<u8>, PlatformWalletError> {
    // Choke-point guards. Entry points reject both conditions earlier, from the
    // arguments alone; repeating them here means a caller that reaches this
    // function by another route still cannot seal an undecodable or oversized
    // document.
    ensure_tx_metadata_version_supported(version)?;
    ensure_tx_metadata_payload_fits(payload.len())?;
    let ciphertext = platform_encryption::encrypt_aes_256_cbc(key, iv, payload);
    let mut blob = Vec::with_capacity(BLOB_HEADER_LEN + ciphertext.len());
    blob.push(version);
    blob.extend_from_slice(iv);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
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
            .field(
                "payload",
                &format_args!("<{} bytes redacted>", self.payload.len()),
            )
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
            let blob = seal_tx_metadata(&key, version, &iv, &payload).expect("valid version");
            // Framing: version at [0], IV at [1..17), ciphertext after.
            assert_eq!(blob[0], version);
            assert_eq!(&blob[1..17], &iv);
            let opened = open_tx_metadata(&key, &blob).expect("open");
            assert_eq!(opened.version, version);
            assert_eq!(opened.payload, payload);
        }
    }

    /// The choke-point wire-version guard: `seal_tx_metadata` accepts only the
    /// two versions the legacy `decryptTxMetadata` understands (0 = CBOR,
    /// 1 = protobuf) and rejects everything else, so the guarantee holds for
    /// any caller that reaches it directly rather than through an entry point
    /// that gates the version earlier.
    #[test]
    fn seal_rejects_non_wire_versions() {
        let key = [0x11u8; 32];
        let iv = [0x22u8; 16];
        let payload = b"opaque".to_vec();

        // The two legal versions seal successfully.
        assert!(seal_tx_metadata(&key, VERSION_CBOR, &iv, &payload).is_ok());
        assert!(seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &payload).is_ok());

        // Every other byte (2..=255) is rejected — none can be produced by
        // sealing, so a non-decodable document can never reach the wire. The
        // rejection carries the dedicated typed variant, which is what lets the
        // FFI boundary surface it as a caller-input error instead of flattening
        // it into the generic unknown-failure code.
        for version in 2u8..=255 {
            match seal_tx_metadata(&key, version, &iv, &payload) {
                Err(PlatformWalletError::UnsupportedTxMetadataVersion { version: reported }) => {
                    assert_eq!(
                        reported, version,
                        "the rejection must report the version it rejected"
                    );
                }
                other => panic!(
                    "version {version} must be rejected as UnsupportedTxMetadataVersion, \
                     got {other:?}"
                ),
            }
        }
    }

    /// Payload-size boundary: the largest plaintext the
    /// `encryptedMetadata` field (`maxItems` 4096) can hold once framed is
    /// [`MAX_TX_METADATA_PLAINTEXT_LEN`] = 4063, and 4064 is the first rejected
    /// length. Pins the REAL PKCS7 envelope math against the code, not the
    /// reviewer's "4063 → 4096" arithmetic: because PKCS7 adds a whole padding
    /// block when the plaintext is block-aligned, a 4063-byte plaintext frames to
    /// a 4081-byte blob (1 version + 16 IV + 4064 ciphertext), and a 4064-byte
    /// plaintext jumps to 4097 (4080 ciphertext) — overflowing the field.
    #[test]
    fn seal_rejects_payload_above_size_limit() {
        let key = [0x11u8; 32];
        let iv = [0x22u8; 16];

        assert_eq!(MAX_TX_METADATA_PLAINTEXT_LEN, 4063);

        // 4063 bytes: seals, and the blob is exactly 4081 bytes (≤ 4096) — the
        // real envelope, NOT 4096. It also round-trips.
        let max_payload = vec![0xabu8; MAX_TX_METADATA_PLAINTEXT_LEN];
        let blob = seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &max_payload)
            .expect("the maximum-size payload must seal");
        assert_eq!(
            blob.len(),
            4081,
            "1 version + 16 IV + 4064 PKCS7 ciphertext = 4081 (fits the 4096 field)"
        );
        assert!(
            blob.len() <= ENCRYPTED_METADATA_FIELD_MAX,
            "the max-payload blob must fit the encryptedMetadata field"
        );
        let opened = open_tx_metadata(&key, &blob).expect("max-size blob round-trips");
        assert_eq!(opened.payload, max_payload);

        // 4064 bytes: rejected up front with the typed error, before any cipher
        // work — it would frame to a 4097-byte blob and be refused at broadcast.
        let over_payload = vec![0xabu8; MAX_TX_METADATA_PLAINTEXT_LEN + 1];
        match seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &over_payload) {
            Err(PlatformWalletError::TxMetadataPayloadTooLarge { len, max }) => {
                assert_eq!(len, 4064);
                assert_eq!(max, 4063);
            }
            other => panic!("expected TxMetadataPayloadTooLarge, got {other:?}"),
        }

        // The standalone precheck agrees on the boundary.
        assert!(ensure_tx_metadata_payload_fits(MAX_TX_METADATA_PLAINTEXT_LEN).is_ok());
        assert!(ensure_tx_metadata_payload_fits(MAX_TX_METADATA_PLAINTEXT_LEN + 1).is_err());
    }

    /// [`ENCRYPTED_METADATA_FIELD_MAX`] duplicates the `encryptedMetadata`
    /// `maxItems` from the wallet-utils contract schema (the crate exports no
    /// limit constant to anchor to), so pin it against the schema JSON itself:
    /// if the contract ever changes the field limit, this fails instead of the
    /// size precheck silently drifting.
    #[test]
    fn field_max_matches_wallet_utils_contract_schema() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../wallet-utils-contract/schema/v1/wallet-utils-contract-documents.json"
        )))
        .expect("wallet-utils contract schema parses");
        let max_items = schema["txMetadata"]["properties"]["encryptedMetadata"]["maxItems"]
            .as_u64()
            .expect("encryptedMetadata.maxItems present in schema");
        assert_eq!(
            ENCRYPTED_METADATA_FIELD_MAX as u64, max_items,
            "ENCRYPTED_METADATA_FIELD_MAX must track the contract's encryptedMetadata maxItems"
        );
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
        let blob = seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &payload).expect("valid version");

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
        let external_wallet =
            Wallet::new_external_signable(Network::Testnet, [0x42u8; 32], AccountCollection::new());
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

        let sealed_by_resident = seal_tx_metadata(&resident_key, VERSION_PROTOBUF, &iv, &payload)
            .expect("valid version");
        let opened_by_master =
            open_tx_metadata(&master_key, &sealed_by_resident).expect("master key opens");
        assert_eq!(opened_by_master.payload, payload);

        let sealed_by_master =
            seal_tx_metadata(&master_key, VERSION_PROTOBUF, &iv, &payload).expect("valid version");
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

        let blob =
            seal_tx_metadata(&key, VERSION_PROTOBUF, &iv, &plaintext_block).expect("valid version");

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

    /// **The wire-compat anchor** (identity_index 0 — the ONLY point at which
    /// legacy wire-compat is defined; see [`derive_tx_metadata_key`] and the
    /// module docs): an end-to-end vector generated by the ACTUAL legacy stack
    /// (dash-sdk-kotlin 4.0.0-RC2 + dashj-core 22.0.3, run under a JVM), proving
    /// the mnemonic→AES-key HD derivation AND the full
    /// `version ‖ IV ‖ AES-256-CBC(payload)` envelope match dashj byte-for-byte.
    /// This pins the one piece static analysis of the jars alone could not (the
    /// derivation-path account prefix): it is now reconstructed exactly and
    /// checked in CI, so a future refactor that moves the path drifts loudly.
    ///
    /// ## Provenance verified against the REAL `DerivationPathFactory`
    ///
    /// The account prefix here is not hand-asserted: the `4a2eaec1…` key was
    /// re-derived by driving the actual dashj
    /// `org.bitcoinj.wallet.DerivationPathFactory(TestNet3Params)`
    /// `.blockchainIdentityECDSADerivationPath()` — the same method
    /// `AuthenticationGroupExtension.getDefaultPath` feeds the
    /// `BLOCKCHAIN_IDENTITY` key chain — and reading the `32769'` child straight
    /// off `org.dashj.platform.contracts.wallet.TxMetadataDocument`, then
    /// deriving `key = hierarchy.get(path, …).getPrivKeyBytes()`. The factory
    /// chose the full path `m/9'/1'/5'/0'/0'/0'/keyId'/32769'/encryptionKeyIndex'`
    /// (`keyId = 2`, `encryptionKeyIndex = 1`) independently of anything this
    /// crate constructs, and it produced exactly `4a2eaec1…`. So this vector's
    /// path is proven by the legacy library, not merely mirrored back from
    /// Rust's own `tx_metadata_derivation_path`.
    /// Note the factory has NO identity-index argument — the
    /// legacy tx-metadata path is fixed at the primary identity, which is why
    /// wire-compat is defined here and only here.
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
        let wallet = Wallet::from_mnemonic(
            mnemonic,
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
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
        let key_via_master = derive_tx_metadata_key_from_master(&master, Network::Testnet, 0, 2, 1)
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

    /// **Independent legacy-INSTALL wire-compat vector: one check that decrypts
    /// a blob produced by a real legacy dash-wallet install.**
    ///
    /// Unlike [`legacy_dashj_wire_compat_vector`] and
    /// [`nonzero_identity_index_derivation_slot_is_internally_consistent`] —
    /// which are generated by driving dashj-core's crypto primitives from a JVM
    /// scratch program (`tests/legacy_wire_compat/LegacyKeyN.java`) — this
    /// vector was not produced by this repo at all. It is a blob a real
    /// **dash-wallet 11.9 Android install** (the shipping dashj crypto path)
    /// created on TESTNET, encrypted, and published to Dash Platform. It was
    /// fetched back off testnet once and is decrypted here with the Rust crypto,
    /// closing the loop the JVM-generated vectors cannot: those prove Rust ⟷
    /// dashj-core agree on primitives this repo invokes; THIS proves the Rust
    /// `open` path decrypts a document that a stock legacy app, running end to
    /// end, actually wrote to the network.
    ///
    /// ## Provenance
    ///
    /// The wallet is a DESIGNATED THROWAWAY, testnet-only, provided explicitly
    /// for this fixture; its recovery phrase is public by intent. On a stock
    /// dash-wallet 11.9 testnet install it registered the DPNS username
    /// `yabba2`, did a send + a receive, and saved transaction metadata; the app
    /// encrypted that metadata and published one `txMetadata` document to
    /// Platform under identity
    /// `ESR1nfF3bj4TR2ZkLmDuSeu6r7VzpTurYi47BV6XwsoP`. That document was
    /// fetched from testnet once and its values hard-coded below, so this test
    /// needs no network: `keyIndex = 2` (the identity's registered
    /// ENCRYPTION/MEDIUM key), `encryptionKeyIndex = 1`,
    /// `$updatedAt = 1784666696610`, blob version byte `1` (protobuf).
    ///
    /// ## What the decrypted plaintext is (real metadata, not a scratch string)
    ///
    /// The recovered plaintext is a genuine dash-wallet protobuf `TxMetadataBatch`
    /// carrying two per-transaction items (the send + the receive), each with a
    /// 32-byte transaction id, a millisecond timestamp, a memo string
    /// (`"username"` and `"faucet"`), an exchange-rate double (USD-per-DASH), and
    /// a `"USD"` currency code — the tax-category / memo / exchange-rate shape the
    /// app persists. This test only asserts byte-for-byte decrypt equality; it
    /// does not depend on the protobuf schema (the payload is opaque to this
    /// crate), so it stays green regardless of future proto field changes.
    ///
    /// The key is derived from the throwaway recovery phrase with
    /// [`derive_tx_metadata_key`] at `identity_index = 0` (the only slot a
    /// legacy `createTxMetadata` flow writes — see [`derive_tx_metadata_key`]),
    /// using the document's own `keyIndex`/`encryptionKeyIndex`. This is entirely
    /// network-free: the blob is the real captured bytes, and decryption
    /// succeeding under PKCS7 is itself the proof the derivation matches the
    /// legacy install byte-for-byte.
    #[test]
    fn legacy_install_yabba2_wire_compat_vector() {
        use key_wallet::mnemonic::{Language, Mnemonic};

        // The DESIGNATED THROWAWAY testnet wallet the legacy dash-wallet 11.9
        // install ran under (public by intent for this fixture).
        const PHRASE: &str =
            "across jungle only rocket promote mule behave siren crush pole awful deposit";

        let wallet = Wallet::from_mnemonic(
            Mnemonic::from_phrase(PHRASE, Language::English).expect("valid recovery phrase"),
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("wallet from recovery phrase");

        // The captured document's own indices: identity_index 0 (legacy always
        // derives against the primary identity), keyIndex 2 (ENCRYPTION/MEDIUM),
        // encryptionKeyIndex 1 (the document's `encryptionKeyIndex` field).
        let key = derive_tx_metadata_key(&wallet, Network::Testnet, 0, 2, 1).expect("derive");

        // The resolver-master path (the on-device external-signable shape) must
        // derive the identical key — pins the fetched-blob decrypt to both key
        // sources, not just the resident wallet.
        let master = ExtendedPrivKey::new_master(
            Network::Testnet,
            &Mnemonic::from_phrase(PHRASE, Language::English)
                .expect("valid recovery phrase")
                .to_seed(""),
        )
        .expect("master from seed");
        let key_via_master = derive_tx_metadata_key_from_master(&master, Network::Testnet, 0, 2, 1)
            .expect("master derive");
        assert_eq!(
            *key, *key_via_master,
            "resident and resolver-master derivation must agree for the legacy-install document"
        );

        // The REAL `encryptedMetadata` blob dash-wallet 11.9 published to testnet
        // (version 1 ‖ IV(16) ‖ AES-256-CBC), fetched back off Platform.
        let legacy_blob = hex::decode(
            "0189b0af73dd2fdeee0141b225580d18dba09ca495c95ee14e5bc23d2683\
             626c0e7522dc45ad1316900543ef9a63da3d3bb4893ac8df3e6a3ca94051\
             b2521e5a4bfd7db87d2f2352b64d8a216781386155b9e2d1cfccc194c98a\
             51e436438b0eaea15fdded112a8c55d286818f82a7f2fce80c7688e8fbed\
             4fab85e8f1da7ee0f2929066274add52f86082f37f52bbf3da21723b5b97\
             46b3d9a42cc528f236ab39",
        )
        .expect("valid hex");

        // The exact protobuf `TxMetadataBatch` plaintext the legacy app encrypted
        // (two items: memos "username"/"faucet", USD exchange-rate doubles).
        let expected_plaintext = hex::decode(
            "0a410a20ba248e210822fea2f26bc78368331dbcb45bfa08c7a4ef19e969\
             8b06b568b93110b8d09fb3f8331a08757365726e616d6521f38e5374246f\
             41402a035553440a3f0a2072615b227e464acd4b8fc6cd03f29a093e9e2e\
             e49e9e92e5e11eed04faf9b91d10d2d49cb3f8331a06666175636574212d\
             211ff46c6e41402a03555344",
        )
        .expect("valid hex");

        let opened = open_tx_metadata(&key, &legacy_blob).expect("open legacy-install blob");
        assert_eq!(
            opened.version, VERSION_PROTOBUF,
            "the legacy install published a protobuf (version 1) txMetadata blob"
        );
        assert_eq!(
            opened.payload, expected_plaintext,
            "the new Rust crypto must decrypt a real dash-wallet 11.9 install's testnet \
             txMetadata blob to its exact published plaintext, byte-for-byte"
        );
    }

    /// **Internal derivation-slot consistency at a nonzero `identity_index` —
    /// NOT a legacy wire-compat claim.** This
    /// exercises that the `identity_index` parameter lands in
    /// the correct path slot and is deterministic across both key sources, so a
    /// refactor that dropped, swapped, or misplaced it would fail loudly. It does
    /// NOT assert cross-stack compatibility, because the legacy stack has no
    /// identity-index component: `createTxMetadata` always derives against the
    /// primary identity (`blockchainIdentityECDSADerivationPath()`, index 0), so
    /// NO legacy wallet ever wrote a document keyed at `identity_index = 1`.
    /// Legacy wire-compat is proven separately and exclusively by
    /// [`legacy_dashj_wire_compat_vector`] at index 0.
    ///
    /// Why index 0 alone can't cover the slot: `KeyDerivationType::ECDSA` is also
    /// `0` and sits immediately before `identity_index`
    /// (`base / key_type' / identity_index' / key_index' / …`, see
    /// [`identity_auth_derivation_path_for_type`]), so at index 0 those two
    /// adjacent `0'` components are indistinguishable. Using `identity_index = 1`
    /// makes the path `m/9'/1'/5'/0'/0'/1'/2'/32769'/1'` differ from the index-0
    /// path in exactly that component, and the resulting key (`8cda…5196`) is
    /// provably distinct from the index-0 key (`4a2e…84d7`).
    ///
    /// ## Provenance of the `8cda…5196` value: SELF-REFERENTIAL
    ///
    /// This value is generated by `LegacyKeyN.java` (see
    /// `tests/legacy_wire_compat/README.md`), which HAND-BUILDS the account path
    /// `m/9'/1'/5'/0'/0'/identityIndex'` — it does NOT call the real dashj
    /// `DerivationPathFactory` (contrast [`legacy_dashj_wire_compat_vector`],
    /// whose index-0 path the factory itself chose). So for a nonzero index the
    /// generator merely re-derives, under dashj-core's raw `HDKeyDerivation`, the
    /// very path this crate's `tx_metadata_derivation_path` constructs. It
    /// confirms Rust and dashj-core agree on the key for a given path — an
    /// internal consistency check — but supplies no independent evidence that any
    /// legacy platform code selects that path. Treat `8cda…5196` as a regression
    /// pin on Rust's own slot placement, not a legacy sample.
    ///
    /// ```text
    /// javac -cp <dashj-core-22.0.3.jar:bcprov:guava:slf4j> LegacyKeyN.java
    /// java  -cp .:<same cp> LegacyKeyN 1 2 1
    ///   fullPath=m/9'/1'/5'/0'/0'/1'/2'/32769'/1'   (hand-built, not from the factory)
    ///   AES_KEY=8cdadb6b8bcf8defd416f2f032255173df89478c971bb96ae9f3511aae355196
    ///   BLOB=01496ce7…2cba627383   (random per run — the IV differs; key is fixed)
    /// ```
    ///
    /// `identity_index = 1`, `key_index` (keyId) `2` (ENCRYPTION/MEDIUM),
    /// `encryptionKeyIndex` `1`. The BIP-39 test mnemonic `abandon abandon …
    /// about`, empty passphrase, Testnet. The key is deterministic; the blob's IV
    /// is fresh `SecureRandom` per generation, so the exact blob bytes below are
    /// one captured run (any IV opens fine).
    #[test]
    fn nonzero_identity_index_derivation_slot_is_internally_consistent() {
        use key_wallet::mnemonic::{Language, Mnemonic};

        const PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about";

        let wallet = Wallet::from_mnemonic(
            Mnemonic::from_phrase(PHRASE, Language::English).expect("valid test mnemonic"),
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("wallet from mnemonic");

        // identity_index 1 (a NON-primary slot), key_index 2, encryptionKeyIndex 1.
        let key = derive_tx_metadata_key(&wallet, Network::Testnet, 1, 2, 1).expect("derive");

        // The key at the hand-built path m/9'/1'/5'/0'/0'/1'/2'/32769'/1'. This
        // is a self-referential cross-check (LegacyKeyN.java re-derives the same
        // path Rust constructs), NOT a legacy-written sample — see the doc above.
        let slot1_key: [u8; 32] =
            hex_lit("8cdadb6b8bcf8defd416f2f032255173df89478c971bb96ae9f3511aae355196");
        // Distinct from the identity_index=0 key — proves the slot is exercised.
        let index0_key: [u8; 32] =
            hex_lit("4a2eaec1ad959105738996b49e0327f96a80b765249d2c9af8cf6aa689aa84d7");
        assert_ne!(
            slot1_key, index0_key,
            "identity_index=1 must derive a different key than identity_index=0 \
             (the identity_index component must occupy its own path slot)"
        );
        assert_eq!(
            *key, slot1_key,
            "derivation at identity_index=1 must be deterministic and match the \
             hand-built dashj-core path (internal slot-consistency pin, not legacy wire-compat)"
        );

        // The resolver-master path (on-device external-signable shape) must hit
        // the same key at this slot too — resident and master must never drift.
        let master = ExtendedPrivKey::new_master(
            Network::Testnet,
            &Mnemonic::from_phrase(PHRASE, Language::English)
                .expect("valid test mnemonic")
                .to_seed(""),
        )
        .expect("master from seed");
        let key_via_master = derive_tx_metadata_key_from_master(&master, Network::Testnet, 1, 2, 1)
            .expect("master derive");
        assert_eq!(
            *key_via_master, slot1_key,
            "resolver-master derivation must match the resident derivation at identity_index=1"
        );

        // A blob sealed under this slot's key must round-trip through open — the
        // cipher/framing works identically at any slot (blob captured from the
        // same generator; any IV opens fine).
        let slot1_blob = hex::decode(
            "01496ce7b7aa8baa910eb278dc38aee86522e841414d7b273da86df2106b0548e\
             ee7b6957bb1789512cd00bf90663690cae4202bd1f9ae5f84859b8d2cba627383",
        )
        .expect("valid hex");
        let expected_plaintext = b"legacy-txmetadata-wire-compat-vector".to_vec();

        let opened = open_tx_metadata(&key, &slot1_blob).expect("open slot-1 blob");
        assert_eq!(opened.version, VERSION_PROTOBUF, "version byte");
        assert_eq!(
            opened.payload, expected_plaintext,
            "Rust must decrypt a blob sealed at identity_index=1 to the original plaintext"
        );
    }
}
