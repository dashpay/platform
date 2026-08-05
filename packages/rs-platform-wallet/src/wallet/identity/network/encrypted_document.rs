//! Encrypted `txMetadata` document create + decrypt-on-fetch on
//! `IdentityWallet`.
//!
//! Implements the wallet-contract encrypted-document surface the Android
//! wallet needs to retire the legacy `org.dashj.platform` stack
//! for create and decrypt-on-fetch. The encryption ENVELOPE — key derivation, the
//! `version ‖ IV ‖ AES-256-CBC(payload)` blob, and the `keyIndex` /
//! `encryptionKeyIndex` / `encryptedMetadata` document fields — is
//! wire-compatible with the legacy `BlockchainIdentity.publishTxMetaData` /
//! `getTxMetaData` (see [`crate::wallet::identity::crypto::tx_metadata`] for the
//! byte-level scheme). The PAYLOAD inside the blob is opaque to the SDK: the
//! app owns the protobuf `TxMetadataBatch` item schema and the batching policy,
//! exactly as it did on the legacy stack.
//!
//! Lives on `IdentityWallet` (like `document.rs` / `contact_info.rs`) because
//! it spans an identity, needs the wallet's HD tree to derive the self-
//! encryption key, and broadcasts a document state transition through the
//! external signer.

use std::sync::Arc;

use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{KeyType, Purpose, SecurityLevel};
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use key_wallet_manager::WalletManager;
use zeroize::Zeroizing;

use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::tx_metadata::{
    derive_tx_metadata_key, derive_tx_metadata_key_from_master,
    ensure_tx_metadata_create_inputs_valid, ensure_tx_metadata_payload_fits, open_tx_metadata,
    seal_tx_metadata, MAX_TX_METADATA_ENCRYPTION_KEY_INDEX,
};
use crate::wallet::platform_wallet::PlatformWalletInfo;

use super::*;

/// The series one `encryptionKeyIndex` high-water belongs to.
///
/// A high-water is only meaningful for the exact set of documents its seed
/// counted, and that count is scoped to one owner identity, one contract and one
/// document type — all three of which the create API accepts from the caller.
/// Keying the map by the whole triple keeps each series' `1 + count` contract
/// true instead of letting one series continue another's numbering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EncryptionKeyIndexScope {
    owner_identity_id: Identifier,
    contract_id: Identifier,
    document_type_name: String,
}

impl EncryptionKeyIndexScope {
    pub(crate) fn new(
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
    ) -> Self {
        Self {
            owner_identity_id: *owner_identity_id,
            contract_id: *contract_id,
            document_type_name: document_type_name.to_string(),
        }
    }
}

/// In-process high-water map for txMetadata `encryptionKeyIndex` allocation:
/// each [`EncryptionKeyIndexScope`] maps to the NEXT index to hand out for that
/// series. Wrapped in an `Arc<tokio::sync::Mutex<..>>` so it is shared across
/// every clone of [`IdentityWallet`] and serializes concurrent allocations.
pub(crate) type EncryptionKeyIndexAllocator =
    Arc<tokio::sync::Mutex<std::collections::HashMap<EncryptionKeyIndexScope, u32>>>;

/// The `encryptionKeyIndex` for the NEXT txMetadata document given the count of
/// documents that already exist in the series.
///
/// This is the legacy wallet's `1 + countAllRequests()`
/// (`SELECT COUNT(*) FROM transaction_metadata_platform`): empty state
/// (`count == 0`) → `1`; `n` existing documents → `n + 1`. It is `count + 1`,
/// NOT `max(index) + 1`, so a wallet migrating from the legacy stack keeps
/// producing the same series it produced before.
///
/// A count with no representable successor is an error rather than a clamp: the
/// clamped value would be an index the series has already used.
pub(crate) fn next_encryption_key_index_from_count(count: u32) -> Result<u32, PlatformWalletError> {
    count
        .checked_add(1)
        .ok_or(PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted)
}

/// Reserve the next `encryptionKeyIndex` for `scope` from the shared
/// `allocator`, so two creates through the same wallet process never pick the
/// same index.
///
/// The first allocation for a scope seeds the high-water from `seed` — the
/// Platform-derived `1 + count` — and every subsequent allocation hands out a
/// monotonically increasing index with no further network work. The stored value
/// is always `handed_out + 1`.
///
/// The seed runs OUTSIDE the allocator lock. It is an unbounded Platform round
/// trip (the SDK sets no request timeout) and this allocator is shared by every
/// identity in the process, so holding the lock across it would let one
/// unresponsive node block encrypted-document creates for every other identity
/// too. Racing callers are reconciled after the fact instead: whichever seed
/// lands first owns the series, and the others adopt it rather than overwrite
/// it, so a scope's high-water only ever moves forward and no index is handed
/// out twice.
///
/// Cross-DEVICE uniqueness is not guaranteed; see
/// [`IdentityWallet::allocate_encryption_key_index`] for why that stays safe.
///
/// A create that fails after allocating leaves a harmless index GAP — never a
/// collision — because the high-water is not rolled back.
pub(crate) async fn reserve_next_index<S>(
    allocator: &tokio::sync::Mutex<std::collections::HashMap<EncryptionKeyIndexScope, u32>>,
    scope: &EncryptionKeyIndexScope,
    seed: S,
) -> Result<u32, PlatformWalletError>
where
    S: std::future::Future<Output = Result<u32, PlatformWalletError>>,
{
    // A seeded scope needs no network work, so it is answered under a guard held
    // only for the map access itself.
    {
        let mut guard = allocator.lock().await;
        if let Some(next) = guard.get(scope).copied() {
            return hand_out(&mut guard, scope, next);
        }
    }

    let seeded = seed.await?;

    let mut guard = allocator.lock().await;
    // Another caller may have seeded this scope while the round trip above was
    // in flight. Its value already reflects hand-outs this caller cannot see, so
    // adopting it — rather than overwriting with a count taken before those
    // hand-outs — is what keeps the two callers from picking the same index.
    let next = guard.get(scope).copied().unwrap_or(seeded);
    hand_out(&mut guard, scope, next)
}

/// Record that `next` has been handed out for `scope` and return it.
///
/// The series ends at [`MAX_TX_METADATA_ENCRYPTION_KEY_INDEX`] — above it the
/// index addresses no derivable key, so handing one out would produce a document
/// nothing can open. The ceiling value itself is usable; it is the value AFTER
/// it that is refused, which is why the stored successor may sit one past the
/// maximum and is only rejected when a later caller tries to use it.
fn hand_out(
    allocated: &mut std::collections::HashMap<EncryptionKeyIndexScope, u32>,
    scope: &EncryptionKeyIndexScope,
    next: u32,
) -> Result<u32, PlatformWalletError> {
    if next > MAX_TX_METADATA_ENCRYPTION_KEY_INDEX {
        return Err(PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted);
    }
    // Cannot overflow: `next` is at most the maximum, which is far below `u32::MAX`.
    allocated.insert(scope.clone(), next + 1);
    Ok(next)
}

/// [`reserve_next_index`] with the deterministic payload-size gate run FIRST, so
/// an over-large payload — one that MUST fail — never consumes an index.
///
/// The size check ([`ensure_tx_metadata_payload_fits`]) is a pure bound that
/// needs no network and no key material. Running it before the allocator is
/// touched means an oversized payload is rejected without seeding or advancing
/// the high-water, so an always-doomed request leaves no gap behind it.
pub(crate) async fn reserve_next_index_checked<S>(
    allocator: &tokio::sync::Mutex<std::collections::HashMap<EncryptionKeyIndexScope, u32>>,
    scope: &EncryptionKeyIndexScope,
    payload_len: usize,
    seed: S,
) -> Result<u32, PlatformWalletError>
where
    S: std::future::Future<Output = Result<u32, PlatformWalletError>>,
{
    ensure_tx_metadata_payload_fits(payload_len)?;
    reserve_next_index(allocator, scope, seed).await
}

/// Where one encrypted-document call derives the per-document txMetadata AES
/// key from. Selected by the CALLER (the FFI layer) from the wallet's shape —
/// the same capability convention as the identity discovery / key-preview
/// paths (`identity_key_preview.rs`):
///
/// - a wallet with resident private keys (mnemonic / seed / xprv — test
///   fixtures, desktop wallets) derives in-process
///   ([`TxMetadataKeySource::ResidentWallet`], the historical path);
/// - an external-signable / watch-only wallet (the Android/iOS apps: the seed
///   lives host-side, keys derive on demand through the registered mnemonic
///   resolver) holds NO in-process private keys — the in-wallet derive fails
///   with `External signable wallet has no private key`. For that shape the FFI
///   resolves the wallet's mnemonic via the host `MnemonicResolverHandle`,
///   builds the master xprv, passes [`TxMetadataKeySource::Master`], and wipes
///   the master after the call — atomic derive + use + zeroize.
///
/// Both sources derive the IDENTICAL path
/// ([`crate::wallet::identity::crypto::tx_metadata::tx_metadata_derivation_path`]),
/// pinned equal by unit test.
#[derive(Clone, Copy)]
pub enum TxMetadataKeySource<'a> {
    /// Derive from the in-process resident wallet's private keys.
    ResidentWallet,
    /// Derive from this caller-resolved master extended private key
    /// (external-signable / watch-only wallet). The caller owns the master's
    /// lifecycle and MUST wipe it (`private_key.non_secure_erase()`) once the
    /// call returns.
    Master(&'a key_wallet::bip32::ExtendedPrivKey),
}

impl TxMetadataKeySource<'_> {
    /// Compact breadcrumb label.
    fn label(&self) -> &'static str {
        match self {
            TxMetadataKeySource::ResidentWallet => "resident-wallet",
            TxMetadataKeySource::Master(_) => "resolver-master",
        }
    }

    /// Derive the AES key for one document from this source. `wallet` is the
    /// in-process wallet (only consulted by the resident variant).
    fn derive(
        &self,
        wallet: &key_wallet::wallet::Wallet,
        network: key_wallet::Network,
        identity_index: u32,
        key_index: u32,
        encryption_key_index: u32,
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, PlatformWalletError> {
        match self {
            TxMetadataKeySource::ResidentWallet => derive_tx_metadata_key(
                wallet,
                network,
                identity_index,
                key_index,
                encryption_key_index,
            ),
            TxMetadataKeySource::Master(master) => derive_tx_metadata_key_from_master(
                master,
                network,
                identity_index,
                key_index,
                encryption_key_index,
            ),
        }
    }
}

/// A fully resolved txMetadata encryption operation that needs only the
/// caller's plaintext to produce document properties.
///
/// Construction validates the declared payload shape, resolves the managed
/// owner and encryption key id, and derives the per-document AES key. Keeping
/// those fallible steps separate from [`Self::seal`] lets FFI callers finish
/// all wallet and key work before they materialize a host-owned payload.
///
/// The AES key is private and zeroized on drop. This type is neither `Clone`
/// nor `Copy`, and its manual `Debug` rendering never exposes the key.
pub struct PreparedTxMetadataEncryption {
    key_index: u32,
    encryption_key_index: u32,
    version: u8,
    payload_len: usize,
    aes_key: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for PreparedTxMetadataEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedTxMetadataEncryption")
            .field("key_index", &self.key_index)
            .field("encryption_key_index", &self.encryption_key_index)
            .field("version", &self.version)
            .field("payload_len", &self.payload_len)
            .field("aes_key", &"<redacted>")
            .finish()
    }
}

impl PreparedTxMetadataEncryption {
    /// Seal the exact payload shape this context was prepared for.
    ///
    /// A fresh IV is drawn for every call, so even an accidental repeated seal
    /// with one context never reuses an AES-CBC key/IV pair. The context remains
    /// zeroizing and its caller should drop it immediately after this returns.
    pub fn seal(&self, payload: &[u8]) -> Result<String, PlatformWalletError> {
        use dashcore::secp256k1::rand::{thread_rng, RngCore};

        if payload.len() != self.payload_len {
            return Err(PlatformWalletError::TxMetadataPayloadLengthMismatch {
                declared: self.payload_len,
                actual: payload.len(),
            });
        }

        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);
        let blob = seal_tx_metadata(&self.aes_key, self.version, &iv, payload)?;

        // The generic document-create path sanitizes hex strings into the
        // schema's byte-array field before broadcasting.
        Ok(serde_json::json!({
            FIELD_KEY_INDEX: self.key_index,
            FIELD_ENCRYPTION_KEY_INDEX: self.encryption_key_index,
            FIELD_ENCRYPTED_METADATA: hex::encode(&blob),
        })
        .to_string())
    }
}

#[cfg(test)]
mod prepared_encryption_tests {
    use super::*;

    fn prepared(payload_len: usize) -> PreparedTxMetadataEncryption {
        PreparedTxMetadataEncryption {
            key_index: 2,
            encryption_key_index: 7,
            version: 1,
            payload_len,
            aes_key: Zeroizing::new([0xa5; 32]),
        }
    }

    #[test]
    fn prepared_encryption_seals_wire_compatible_properties() {
        let payload = b"txMetadata prepared-context payload";
        let properties = prepared(payload.len()).seal(payload).expect("seal");
        let properties: serde_json::Value =
            serde_json::from_str(&properties).expect("properties JSON");

        assert_eq!(properties[FIELD_KEY_INDEX], 2);
        assert_eq!(properties[FIELD_ENCRYPTION_KEY_INDEX], 7);
        let blob = hex::decode(
            properties[FIELD_ENCRYPTED_METADATA]
                .as_str()
                .expect("encryptedMetadata is hex"),
        )
        .expect("valid hex");
        let opened = open_tx_metadata(&[0xa5; 32], &blob).expect("open prepared blob");
        assert_eq!(opened.version, 1);
        assert_eq!(opened.payload.as_slice(), payload);
    }

    #[test]
    fn prepared_encryption_rejects_a_different_materialized_length() {
        let error = prepared(3)
            .seal(&[1, 2])
            .expect_err("the materializer must honor its declared length");

        assert!(matches!(
            error,
            PlatformWalletError::TxMetadataPayloadLengthMismatch {
                declared: 3,
                actual: 2
            }
        ));
    }

    #[test]
    fn prepared_encryption_draws_a_fresh_iv_for_every_seal() {
        let payload = b"repeat seal";
        let prepared = prepared(payload.len());
        let blobs = [
            prepared.seal(payload).expect("first seal"),
            prepared.seal(payload).expect("second seal"),
        ]
        .map(|properties| {
            let properties: serde_json::Value =
                serde_json::from_str(&properties).expect("properties JSON");
            hex::decode(
                properties[FIELD_ENCRYPTED_METADATA]
                    .as_str()
                    .expect("encryptedMetadata is hex"),
            )
            .expect("valid hex")
        });

        assert_ne!(&blobs[0][1..17], &blobs[1][1..17]);
        for blob in blobs {
            let opened = open_tx_metadata(&[0xa5; 32], &blob).expect("open prepared blob");
            assert_eq!(opened.payload.as_slice(), payload);
        }
    }

    #[test]
    fn prepared_encryption_debug_redacts_the_aes_key() {
        let rendered = format!("{:?}", prepared(3));
        let key_rendering = format!("{:?}", [0xa5u8; 32]);

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains(&key_rendering));
    }
}

/// Wallet-contract document field names (wire-compatible with the legacy
/// `TxMetadataDocument` schema — `wallet-utils-contract` `tx_metadata`).
const FIELD_KEY_INDEX: &str = "keyIndex";
const FIELD_ENCRYPTION_KEY_INDEX: &str = "encryptionKeyIndex";
const FIELD_ENCRYPTED_METADATA: &str = "encryptedMetadata";

/// Emit an INFORMATIONAL stage breadcrumb through both logging facades at
/// **DEBUG** level.
///
/// On Android the two facades diverge: the JNI layer's `JNI_OnLoad` installs
/// `android_logger` as the global `log` logger (logcat tag `DashSDK`) but at
/// `LevelFilter::Info`, while the only `tracing` subscriber the Kotlin SDK
/// installs (`dash_sdk_enable_logging`, a `tracing_subscriber::fmt` layer)
/// writes to STDOUT, which Android discards. Consequence: a DEBUG line reaches
/// NEITHER on-device sink, while host tests / desktop file logging still capture
/// it through `tracing`.
///
/// Genuine failures use [`breadcrumb_error`] (WARN) so they stay visible
/// on-device; routine stage lines stay at DEBUG.
///
/// No breadcrumb on this path may carry an owner, contract or document
/// identifier, or an error's `Display`. Logcat is readable by any process
/// holding READ_LOGS and is captured in bug reports, so a full identifier there
/// correlates a device to an on-chain identity, and an echoed error body is
/// unbounded and can carry query shapes and contract internals. Stage names,
/// [`error_kind`] classifications, counts and booleans are what belong here.
fn breadcrumb(line: &str) {
    tracing::debug!("{line}");
    log::debug!("{line}");
}

/// Emit a FAILURE breadcrumb through both logging facades at **WARN** level, so
/// a genuine error or skip stays visible in Android logcat (`android_logger`
/// Info+). Use ONLY for actual failure / skip paths — never per-poll
/// informational stages, which belong on [`breadcrumb`] (DEBUG). The same
/// redaction rules apply at every level.
fn breadcrumb_error(line: &str) {
    tracing::warn!("{line}");
    log::warn!("{line}");
}

/// A stable, bounded classification of a failure, for breadcrumbs that must not
/// transcribe an error's `Display`. The returned token names the failure class
/// only — it never contains caller data, an identifier, or a message body — and
/// is stable enough to tell the stages apart in a device log.
fn error_kind(error: &PlatformWalletError) -> &'static str {
    match error {
        PlatformWalletError::Sdk(_) => "sdk",
        PlatformWalletError::WalletNotFound(_) => "wallet-not-found",
        PlatformWalletError::IdentityNotFound(_) => "identity-not-found",
        PlatformWalletError::UnsupportedTxMetadataVersion { .. } => "unsupported-version",
        PlatformWalletError::TxMetadataPayloadTooLarge { .. } => "payload-too-large",
        PlatformWalletError::InvalidIdentityData(_) => "invalid-identity-data",
        _ => "other",
    }
}

/// One decrypted encrypted-document, returned to the caller (serialized to
/// JSON at the FFI boundary). The `payload` is the opaque, decrypted plaintext
/// the app parses itself (a protobuf `TxMetadataBatch` for `version == 1`).
///
/// `Debug` is hand-written (NOT derived) so a stray `{:?}` / `dbg!()` / tracing
/// statement can never leak the decrypted financial payload (memos, tax
/// categories, exchange-rate records, gift cards) into a log — mirroring the
/// deliberate omission of `Debug` on secret-bearing sibling types like
/// `DerivedIdentityAuthKey`. The plaintext is redacted to its length.
#[derive(Clone)]
pub struct DecryptedEncryptedDocument {
    /// Canonical 32-byte document id.
    pub document_id: Identifier,
    /// Document owner ($ownerId).
    pub owner_id: Identifier,
    /// The document's `keyIndex` field (the identity's ENCRYPTION key id used
    /// to derive the decryption key).
    pub key_index: u32,
    /// The document's `encryptionKeyIndex` field (the app's per-document index).
    pub encryption_key_index: u32,
    /// The blob's leading version byte (0 = CBOR, 1 = protobuf).
    pub version: u8,
    /// $updatedAt in epoch-millis, if the document carries it. The app tracks
    /// this as its since-timestamp high-water mark for the next fetch.
    pub updated_at_ms: Option<u64>,
    /// The decrypted, opaque payload bytes.
    pub payload: Zeroizing<Vec<u8>>,
}

impl std::fmt::Debug for DecryptedEncryptedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecryptedEncryptedDocument")
            .field("document_id", &self.document_id)
            .field("owner_id", &self.owner_id)
            .field("key_index", &self.key_index)
            .field("encryption_key_index", &self.encryption_key_index)
            .field("version", &self.version)
            .field("updated_at_ms", &self.updated_at_ms)
            // Redacted: never render the decrypted financial plaintext.
            .field(
                "payload",
                &format_args!("<{} bytes redacted>", self.payload.len()),
            )
            .finish()
    }
}

#[cfg(test)]
mod decrypted_document_tests {
    use super::*;

    /// `DecryptedEncryptedDocument`'s `Debug` never renders the decrypted
    /// plaintext.
    ///
    /// Same reasoning as the sibling redaction on `OpenedTxMetadata`: `Debug` is
    /// hand-written so a stray `{:?}` cannot put financial plaintext in a log,
    /// and a derive would print it verbatim. The identifying, non-secret fields
    /// must still be rendered or the redaction would make the type useless to
    /// debug with.
    #[test]
    fn decrypted_document_debug_redacts_the_plaintext() {
        const MARKER: &str = "s3cr3t-memo-marker";
        let payload = format!("memo={MARKER}").into_bytes();
        let document = DecryptedEncryptedDocument {
            document_id: Identifier::new([9u8; 32]),
            owner_id: Identifier::new([8u8; 32]),
            key_index: 2,
            encryption_key_index: 1,
            version: 1,
            updated_at_ms: Some(1_700_000_000_000),
            payload: Zeroizing::new(payload.clone()),
        };

        let rendered = format!("{document:?}");

        assert!(
            !rendered.contains(MARKER),
            "Debug leaked the decrypted plaintext: {rendered}"
        );
        assert!(
            !rendered.contains(&format!("{:?}", payload.as_slice())),
            "Debug leaked the raw payload bytes: {rendered}"
        );
        assert!(
            rendered.contains(&payload.len().to_string()),
            "the redaction must keep the length: {rendered}"
        );
        for field in [
            "key_index",
            "encryption_key_index",
            "version",
            "updated_at_ms",
        ] {
            assert!(
                rendered.contains(field),
                "non-secret field {field} must survive redaction: {rendered}"
            );
        }
    }

    #[test]
    fn should_use_zeroizing_storage_for_decrypted_payload() {
        let document = DecryptedEncryptedDocument {
            document_id: Identifier::from([1; 32]),
            owner_id: Identifier::from([2; 32]),
            key_index: 3,
            encryption_key_index: 4,
            version: 1,
            updated_at_ms: Some(5),
            payload: b"txMetadata plaintext".to_vec().into(),
        };

        fn assert_zeroizing(_: &Zeroizing<Vec<u8>>) {}
        assert_zeroizing(&document.payload);
    }
}

/// Everything the tx-metadata key derivation needs about one owner, resolved
/// from the wallet manager in a single lock acquisition: the identity (whose
/// keys pick the document's `keyIndex`), the HD slot it lives at, and the
/// wallet the AES key derives from.
///
/// Carries no key material of its own — the wallet is the same handle the
/// manager holds — so it is only as sensitive as the wallet it came from.
struct TxMetadataEncryptionContext {
    identity: dpp::identity::Identity,
    identity_index: u32,
    wallet: key_wallet::wallet::Wallet,
}

impl IdentityWallet {
    /// Select the identity's encryption key id (the document's `keyIndex`
    /// field): an `ECDSA_SECP256K1` `Purpose::ENCRYPTION` / `MEDIUM` key, falling
    /// back to an `AUTHENTICATION` / `HIGH` key — mirroring the legacy
    /// `BlockchainIdentity.createTxMetadata` selection
    /// (`getFirstPublicKey(ENCRYPTION, MEDIUM)` → `getHighAuthenticationKey`).
    fn select_encryption_key_id(
        identity: &dpp::identity::Identity,
    ) -> Result<u32, PlatformWalletError> {
        identity
            .get_first_public_key_matching(
                Purpose::ENCRYPTION,
                [SecurityLevel::MEDIUM].into(),
                [KeyType::ECDSA_SECP256K1].into(),
                false,
            )
            .or_else(|| {
                identity.get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::HIGH].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
            })
            .map(|k| k.id())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Identity has no ECDSA_SECP256K1 ENCRYPTION (MEDIUM) or AUTHENTICATION \
                     (HIGH) key to derive the txMetadata encryption key"
                        .to_string(),
                )
            })
    }

    /// Count the identity's existing txMetadata-style documents on Platform —
    /// the authoritative equivalent of dash-wallet's local
    /// `transactionMetadataDocumentDao.countAllRequests()`
    /// (`SELECT COUNT(*) FROM transaction_metadata_platform`). Fetches +
    /// registers the contract, then runs the owner-scoped scan with
    /// `since_ms == 0` (every document, since `$updatedAt >= 0` always holds)
    /// and returns the number of documents found.
    ///
    /// Every returned entry counts, materialized or not: an un-materialized id
    /// still denotes an existing document, so the count never under-reports and
    /// the next index never re-collides with an existing one.
    ///
    /// NOTE: this counts by fetching the owned documents (the same paginated
    /// query the fetch path uses) rather than a dedicated drive `COUNT` query —
    /// a wallet's txMetadata document set is small, so the extra surface a
    /// count-only query would add is not worth it here.
    async fn count_owned_txmetadata_documents(
        &self,
        contract_id: &Identifier,
        owner_identity_id: &Identifier,
        document_type_name: &str,
    ) -> Result<u32, PlatformWalletError> {
        use dash_sdk::platform::{ContextProvider, Fetch};

        let contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(PlatformWalletError::Sdk)?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; \
                     cannot allocate encryptionKeyIndex"
                ))
            })?;
        let contract = Arc::new(contract);
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::clone(&contract));
        }
        let raw = query_owned_encrypted_documents(
            &self.sdk,
            contract,
            owner_identity_id,
            document_type_name,
            0,
        )
        .await?;
        // A count that does not fit the index's own width cannot produce a
        // usable index, so it fails here rather than being clamped into one the
        // series has already used.
        u32::try_from(raw.len())
            .map_err(|_| PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted)
    }

    /// Allocate the next `encryptionKeyIndex` for an encrypted-document create
    /// when the host supplies none, keeping the index-selection policy in the
    /// SDK rather than asking every host to reimplement it.
    ///
    /// Semantics match the legacy wallet counter exactly: the index is
    /// `1 + count`, where the count is
    /// [`Self::count_owned_txmetadata_documents`] read from Platform at create
    /// time instead of the app's local table. Empty state → `1`; `n` existing
    /// documents → `n + 1`.
    ///
    /// Allocation is serialized through the wallet's shared
    /// [`EncryptionKeyIndexAllocator`], keyed by owner identity, contract and
    /// document type: two concurrent creates in the same series through the same
    /// wallet process never pick the same index — the first seeds the in-process
    /// high-water from Platform, the second hands out the next value without a
    /// second query.
    ///
    /// ## Uniqueness is per device, and the index is not a document sequence
    /// Uniqueness is guaranteed only PER DEVICE, and only for creates that come
    /// through this allocator. Two devices sharing an identity can seed from the
    /// same base before either's write is visible to the other, and a caller
    /// that supplies its own index (the migration/test path) bypasses the
    /// high-water entirely, so the same `encryptionKeyIndex` can legitimately
    /// appear on two documents.
    ///
    /// That is safe, not lossy: every encrypted document stores its OWN
    /// `keyIndex` and `encryptionKeyIndex`, and the reader
    /// ([`Self::fetch_encrypted_documents`]) derives each document's key from
    /// that document's own stored indices — so two documents sharing an index
    /// each carry a fresh random IV, decrypt independently, and are both
    /// returned. No document is overwritten or shadowed.
    ///
    /// What follows from that: the index is an encryption-key selector, NOT a
    /// document sequence number. It must not be used to order documents, detect
    /// gaps, count them, or address one — only the document's own stored fields
    /// decide how it decrypts.
    ///
    /// ## Size validated BEFORE allocating (no index consumed on failure)
    /// `payload_len` is the plaintext length of the document about to be sealed.
    /// It is checked up front — a pure, network-free bound — so an oversized
    /// payload fails without ever counting on Platform or advancing the
    /// high-water, leaving no index gap behind an always-doomed request.
    pub async fn allocate_encryption_key_index(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        payload_len: usize,
    ) -> Result<u32, PlatformWalletError> {
        let scope =
            EncryptionKeyIndexScope::new(owner_identity_id, contract_id, document_type_name);
        reserve_next_index_checked(&self.enc_key_index_allocator, &scope, payload_len, async {
            let count = self
                .count_owned_txmetadata_documents(
                    contract_id,
                    owner_identity_id,
                    document_type_name,
                )
                .await?;
            let index = next_encryption_key_index_from_count(count)?;
            breadcrumb(&format!(
                "allocate_encryption_key_index: seeded existing_count={count} \
                 next_index={index}"
            ));
            Ok(index)
        })
        .await
    }

    /// Resolve the [`TxMetadataEncryptionContext`] for `owner_identity_id` from
    /// an ALREADY-HELD wallet-manager read guard. Errors for a watch-only /
    /// out-of-wallet identity (no resident HD slot); the dash-wallet migration
    /// uses a resident mnemonic wallet.
    ///
    /// Shared by the async and blocking resolvers below, which differ ONLY in
    /// how the guard is taken, so the two can never drift apart on what counts
    /// as a resolvable owner.
    fn encryption_context_from_manager(
        &self,
        wm: &WalletManager<PlatformWalletInfo>,
        owner_identity_id: &Identifier,
    ) -> Result<TxMetadataEncryptionContext, PlatformWalletError> {
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = info
            .identity_manager
            .managed_identity(owner_identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*owner_identity_id))?;
        let identity_index = managed.identity_index.ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Identity {owner_identity_id} is watch-only (no resident HD slot); \
                 cannot derive its txMetadata encryption key in-process"
            ))
        })?;
        let identity = managed.identity.clone();
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .clone();
        Ok(TxMetadataEncryptionContext {
            identity,
            identity_index,
            wallet,
        })
    }

    /// Await the wallet-manager read lock, then resolve the
    /// [`TxMetadataEncryptionContext`] for `owner_identity_id`.
    async fn resolve_encryption_context(
        &self,
        owner_identity_id: &Identifier,
    ) -> Result<TxMetadataEncryptionContext, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        self.encryption_context_from_manager(&wm, owner_identity_id)
    }

    /// Synchronous counterpart of [`Self::resolve_encryption_context`],
    /// resolving the same context without crossing an `.await` — that is what
    /// lets the `*_blocking` entry points wipe a resolved master xprv BEFORE any
    /// network round-trip.
    ///
    /// ## Never panics, including when called from async code
    /// `blocking_read` parks the calling thread, and Tokio refuses to park a
    /// thread that is currently driving tasks — it panics, which
    /// `panic = "abort"` turns into an immediate process death. Whether the
    /// current thread MAY park is not observable from outside Tokio (a
    /// `spawn_blocking` thread may, a runtime worker may not), so this splits on
    /// what is observable:
    ///
    /// * **No runtime in scope** — the FFI's own C thread, and any plain
    ///   synchronous caller. Parking is unconditionally safe, so park.
    /// * **A runtime in scope** — do not park at all. Take the lock without
    ///   waiting and report contention as the typed, retryable
    ///   [`PlatformWalletError::WalletManagerBusy`], naming `api` so the caller
    ///   knows which entry point to move onto its async counterpart.
    fn resolve_encryption_context_blocking(
        &self,
        owner_identity_id: &Identifier,
        api: &'static str,
    ) -> Result<TxMetadataEncryptionContext, PlatformWalletError> {
        match tokio::runtime::Handle::try_current() {
            Err(_) => {
                let wm = self.wallet_manager.blocking_read();
                self.encryption_context_from_manager(&wm, owner_identity_id)
            }
            Ok(_) => {
                let wm = self
                    .wallet_manager
                    .try_read()
                    .map_err(|_| PlatformWalletError::WalletManagerBusy { api })?;
                self.encryption_context_from_manager(&wm, owner_identity_id)
            }
        }
    }

    /// Select the identity encryption key and derive the per-document AES key
    /// from an already-resolved encryption context.
    ///
    /// Pure and synchronous: no lock, no network, no awaits. Shared by the
    /// async and blocking preparation entry points so the two produce
    /// byte-identical contexts.
    fn prepared_encryption_from_context(
        &self,
        context: &TxMetadataEncryptionContext,
        encryption_key_index: u32,
        version: u8,
        payload_len: usize,
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<PreparedTxMetadataEncryption, PlatformWalletError> {
        let key_index = Self::select_encryption_key_id(&context.identity)?;
        let aes_key = key_source
            .derive(
                &context.wallet,
                self.sdk.network,
                context.identity_index,
                key_index,
                encryption_key_index,
            )
            .inspect_err(|e| {
                breadcrumb_error(&format!(
                    "prepare_txmetadata_encryption: key derivation failed \
                     key_source={} error_kind={}",
                    key_source.label(),
                    error_kind(e)
                ));
            })?;

        Ok(PreparedTxMetadataEncryption {
            key_index,
            encryption_key_index,
            version,
            payload_len,
            aes_key,
        })
    }

    /// Resolve every fallible input needed to encrypt one txMetadata payload,
    /// without taking or copying the plaintext itself.
    ///
    /// The returned context contains the selected identity key id and a
    /// zeroizing per-document AES key. A host bridge can therefore finish this
    /// operation, release any master xprv used to derive it, and only then
    /// materialize the payload for [`PreparedTxMetadataEncryption::seal`].
    ///
    /// The ONLY await is the wallet-manager read lock, taken before anything is
    /// derived — no key material and no plaintext exists yet at that point, and
    /// nothing here touches the network. A caller holding a master xprv across
    /// this call therefore holds it across a local lock acquisition and nothing
    /// else. A caller that must not even do that (the FFI, which wipes the
    /// master before the broadcast await) uses
    /// [`Self::prepare_txmetadata_encryption_blocking`] instead.
    pub async fn prepare_txmetadata_encryption(
        &self,
        owner_identity_id: &Identifier,
        encryption_key_index: u32,
        version: u8,
        payload_len: usize,
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<PreparedTxMetadataEncryption, PlatformWalletError> {
        ensure_tx_metadata_create_inputs_valid(payload_len, version, Some(encryption_key_index))?;

        let context = self.resolve_encryption_context(owner_identity_id).await?;
        self.prepared_encryption_from_context(
            &context,
            encryption_key_index,
            version,
            payload_len,
            key_source,
        )
    }

    /// Synchronous counterpart of [`Self::prepare_txmetadata_encryption`].
    ///
    /// **Crosses no `.await` at all** — it resolves through a blocking read —
    /// so a host bridge can derive here, wipe the master xprv, and only then
    /// enter an async broadcast. Call it from a synchronous context; called
    /// with a Tokio runtime in scope it never parks and never panics, returning
    /// [`PlatformWalletError::WalletManagerBusy`] if the lock is contended.
    pub fn prepare_txmetadata_encryption_blocking(
        &self,
        owner_identity_id: &Identifier,
        encryption_key_index: u32,
        version: u8,
        payload_len: usize,
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<PreparedTxMetadataEncryption, PlatformWalletError> {
        ensure_tx_metadata_create_inputs_valid(payload_len, version, Some(encryption_key_index))?;

        let context = self.resolve_encryption_context_blocking(
            owner_identity_id,
            "IdentityWallet::prepare_txmetadata_encryption_blocking",
        )?;
        self.prepared_encryption_from_context(
            &context,
            encryption_key_index,
            version,
            payload_len,
            key_source,
        )
    }

    /// Derive the identity encryption key and seal `payload` into the
    /// wire-compatible `version ‖ IV ‖ AES-256-CBC` blob, returning the
    /// `{keyIndex, encryptionKeyIndex, encryptedMetadata}` properties JSON ready
    /// for [`Self::create_document_with_signer`] — the exact document shape the
    /// legacy `publishTxMetaData` wrote, so the legacy stack decrypts it.
    ///
    /// Awaits only the wallet-manager read lock (see
    /// [`Self::prepare_txmetadata_encryption`]); the derivation and the seal are
    /// synchronous, and the subsequent generic
    /// [`Self::create_document_with_signer`] broadcasts the returned properties
    /// with no key material in scope. A caller that must keep its master off
    /// every await uses [`Self::prepare_encrypted_txmetadata_properties_blocking`].
    ///
    /// The caller supplies:
    /// - `encryption_key_index`: the per-document index (dash-wallet's monotonic
    ///   `1 + countAllRequests()` counter). Batching stays app-side.
    /// - `version`: the payload version byte (`1` = protobuf, as the wallet
    ///   writes).
    /// - `payload`: the already-serialized opaque plaintext (a protobuf
    ///   `TxMetadataBatch`) — the SDK does not parse it.
    ///
    /// The `keyIndex` field (the identity encryption key id) is selected SDK-side
    /// to match the legacy stack; `key_source` selects where the AES key derives
    /// from (see [`TxMetadataKeySource`]).
    pub async fn prepare_encrypted_txmetadata_properties(
        &self,
        owner_identity_id: &Identifier,
        encryption_key_index: u32,
        version: u8,
        payload: &[u8],
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<String, PlatformWalletError> {
        self.prepare_txmetadata_encryption(
            owner_identity_id,
            encryption_key_index,
            version,
            payload.len(),
            key_source,
        )
        .await?
        .seal(payload)
    }

    /// Synchronous counterpart of
    /// [`Self::prepare_encrypted_txmetadata_properties`], resolving through a
    /// blocking read so nothing it derives crosses an `.await`. Same
    /// sync-context contract as
    /// [`Self::prepare_txmetadata_encryption_blocking`].
    pub fn prepare_encrypted_txmetadata_properties_blocking(
        &self,
        owner_identity_id: &Identifier,
        encryption_key_index: u32,
        version: u8,
        payload: &[u8],
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<String, PlatformWalletError> {
        self.prepare_txmetadata_encryption_blocking(
            owner_identity_id,
            encryption_key_index,
            version,
            payload.len(),
            key_source,
        )?
        .seal(payload)
    }

    /// The NETWORK half of the encrypted-document fetch: resolve the contract
    /// and run the paginated owner-scoped scan, returning the raw entries
    /// exactly as Drive returned them.
    ///
    /// The query mirrors the legacy `getTxMetaData(sinceTime, key)`:
    /// `$ownerId == owner AND $updatedAt >= since_ms` ordered by `$updatedAt`
    /// ascending, paginated so a wallet with many documents isn't truncated.
    ///
    /// Touches no key material at all, so a caller that must not acquire a key
    /// before it knows there is something to decrypt can await this first and
    /// only then resolve one. That matters for hosts whose key acquisition runs
    /// a user-visible prompt, and because acquired material would otherwise have
    /// to survive this scan — an unbounded wait, since the SDK sets no request
    /// timeout.
    ///
    /// Pairs with [`Self::decrypt_fetched_documents`], or with
    /// [`Self::decrypt_fetched_documents_blocking`] for a caller whose key
    /// material must not cross any await at all.
    pub async fn fetch_raw_encrypted_documents(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        since_ms: u64,
    ) -> Result<Vec<(Identifier, Option<Document>)>, PlatformWalletError> {
        use dash_sdk::platform::{ContextProvider, Fetch};

        // Fetch the contract and register it so `fetch_many`'s proof
        // verification can resolve it through the context provider (the mobile
        // provider never fetches contracts itself).
        let contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(|e| {
                breadcrumb_error("fetch_encrypted_documents: contract fetch failed error_kind=sdk");
                PlatformWalletError::Sdk(e)
            })?
            .ok_or_else(|| {
                breadcrumb_error("fetch_encrypted_documents: contract not found on Platform");
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot fetch documents"
                ))
            })?;
        // Wrap once and share the cheap `Arc` handle with the context provider
        // rather than deep-cloning the whole `DataContract` (document-type/index
        // metadata) a second time.
        let contract = Arc::new(contract);
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::clone(&contract));
        }

        // The wire query, split out so its exact shape is integration-testable
        // against testnet without a resident wallet/identity (see
        // `tests/txmetadata_fetch.rs`).
        query_owned_encrypted_documents(
            &self.sdk,
            Arc::clone(&contract),
            owner_identity_id,
            document_type_name,
            since_ms,
        )
        .await
        .inspect_err(|e| {
            breadcrumb_error(&format!(
                "fetch_encrypted_documents: document query failed error_kind={}",
                error_kind(e)
            ));
        })
    }

    /// The DECRYPT half: turn raw entries from
    /// [`Self::fetch_raw_encrypted_documents`] into decrypted documents.
    ///
    /// Awaits only the wallet-manager read lock; the derivation and every
    /// decrypt are synchronous and touch no network. A caller may therefore
    /// acquire key material, call this, and wipe that material immediately,
    /// without it ever being live across a network round trip. A caller that
    /// must keep its key material off every await uses
    /// [`Self::decrypt_fetched_documents_blocking`].
    ///
    /// Returns an empty vec for empty input without resolving anything, so a
    /// caller that skipped acquisition on an empty scan stays correct if it
    /// calls this anyway.
    pub async fn decrypt_fetched_documents(
        &self,
        owner_identity_id: &Identifier,
        raw_docs: &[(Identifier, Option<Document>)],
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<Vec<DecryptedEncryptedDocument>, PlatformWalletError> {
        if raw_docs.is_empty() {
            return Ok(Vec::new());
        }
        let context = self.resolve_encryption_context(owner_identity_id).await?;
        Ok(self.decrypt_raw_documents(
            raw_docs,
            context.identity_index,
            &context.wallet,
            key_source,
        ))
    }

    /// Synchronous counterpart of [`Self::decrypt_fetched_documents`].
    ///
    /// **Crosses no `.await`** — it resolves its context with a blocking read
    /// and derives synchronously — which is what lets the FFI hold a resolved
    /// master for the length of one plain function call and wipe it on the next
    /// line. Same sync-context contract as
    /// [`Self::prepare_txmetadata_encryption_blocking`]: with a Tokio runtime in
    /// scope it never parks and never panics, returning
    /// [`PlatformWalletError::WalletManagerBusy`] on contention instead.
    pub fn decrypt_fetched_documents_blocking(
        &self,
        owner_identity_id: &Identifier,
        raw_docs: &[(Identifier, Option<Document>)],
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<Vec<DecryptedEncryptedDocument>, PlatformWalletError> {
        if raw_docs.is_empty() {
            return Ok(Vec::new());
        }
        let context = self.resolve_encryption_context_blocking(
            owner_identity_id,
            "IdentityWallet::decrypt_fetched_documents_blocking",
        )?;
        Ok(self.decrypt_raw_documents(
            raw_docs,
            context.identity_index,
            &context.wallet,
            key_source,
        ))
    }

    /// Fetch and decrypt in one call, for wallets that hold their keys in
    /// process.
    ///
    /// RESIDENT-KEY ONLY, deliberately: it takes no key source, because
    /// accepting a caller-supplied master would mean holding that master across
    /// the raw network scan this method awaits internally — an unbounded wait,
    /// since the SDK sets no request timeout. A wallet with resident keys has
    /// nothing to hold: the key derives from the wallet itself, synchronously,
    /// at decrypt time.
    ///
    /// An external-signable caller — anything whose key comes from a host
    /// resolver or an externally supplied xprv — must use the two stages
    /// instead: [`Self::fetch_raw_encrypted_documents`] first, then acquire the
    /// key, then [`Self::decrypt_fetched_documents`], then wipe. That ordering
    /// is what keeps the secret off the network path, and it cannot be expressed
    /// through this convenience.
    pub async fn fetch_encrypted_documents(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        since_ms: u64,
    ) -> Result<Vec<DecryptedEncryptedDocument>, PlatformWalletError> {
        // Resident-only by construction: a caller-supplied master would have to
        // be held across the raw scan below, which is exactly what the split
        // exists to prevent. A wallet with resident keys has nothing to hold —
        // the key derives from the wallet itself, at decrypt time.
        let key_source = TxMetadataKeySource::ResidentWallet;

        // Stage breadcrumbs for this fetch. An empty result on this path is
        // indistinguishable from a failure without them: the query can return
        // nothing, a document can fail to materialize, or a decrypt can be
        // skipped, and each stage below records which one happened.
        breadcrumb(&format!(
            "fetch_encrypted_documents: entry key_source={}",
            key_source.label()
        ));

        let raw_docs = self
            .fetch_raw_encrypted_documents(
                owner_identity_id,
                contract_id,
                document_type_name,
                since_ms,
            )
            .await?;

        // Nothing came back, so there is nothing to decrypt and no reason to
        // touch a key at all.
        if raw_docs.is_empty() {
            breadcrumb("fetch_encrypted_documents: query returned no documents; no key acquired");
            return Ok(Vec::new());
        }

        // Candidates exist: acquire the key context now, with every network
        // await already behind us. Everything from here to the end of the loop
        // is synchronous, so the resolved material never crosses an await.
        let context = self
            .resolve_encryption_context(owner_identity_id)
            .await
            .inspect_err(|e| {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: encryption-context resolution failed \
                     error_kind={}",
                    error_kind(e)
                ));
            })?;

        Ok(self.decrypt_raw_documents(
            &raw_docs,
            context.identity_index,
            &context.wallet,
            key_source,
        ))
    }

    /// Decrypt raw entries with an already-resolved context.
    ///
    /// Pure and synchronous: no network, no context resolution, no awaits. A
    /// document that cannot be materialized, is missing its fields, carries an
    /// unsupported wire version, or fails to decrypt is SKIPPED with a
    /// breadcrumb — one bad document must never abort a sync.
    fn decrypt_raw_documents(
        &self,
        raw_docs: &[(Identifier, Option<Document>)],
        identity_index: u32,
        wallet: &key_wallet::wallet::Wallet,
        key_source: TxMetadataKeySource<'_>,
    ) -> Vec<DecryptedEncryptedDocument> {
        let mut out = Vec::new();
        for (position, (doc_id, maybe_doc)) in raw_docs.iter().enumerate() {
            let Some(doc) = maybe_doc else {
                // A raw entry the SDK could not materialize (e.g. a proved
                // fetch returning an id without a document). Skipped, but never
                // silently: under proofs this is exactly the shape that turns
                // "documents exist" into an empty result with no error, so it
                // must leave a trail.
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: raw entry NOT materialized \
                     position={position}; skipping"
                ));
                continue;
            };
            let props = doc.properties();
            let (Some(key_index), Some(encryption_key_index)) = (
                props
                    .get(FIELD_KEY_INDEX)
                    .and_then(|v: &Value| v.to_integer::<u32>().ok()),
                props
                    .get(FIELD_ENCRYPTION_KEY_INDEX)
                    .and_then(|v: &Value| v.to_integer::<u32>().ok()),
            ) else {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: document missing key indices \
                     position={position}; skipping"
                ));
                continue;
            };
            let Some(blob) = props
                .get(FIELD_ENCRYPTED_METADATA)
                .and_then(|v: &Value| v.to_binary_bytes().ok())
            else {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: document missing encryptedMetadata \
                     position={position}; skipping"
                ));
                continue;
            };

            let aes_key = match key_source.derive(
                wallet,
                self.sdk.network,
                identity_index,
                key_index,
                encryption_key_index,
            ) {
                Ok(k) => k,
                Err(e) => {
                    breadcrumb_error(&format!(
                        "fetch_encrypted_documents: txMetadata key derivation failed \
                         position={position} key_source={} error_kind={}; skipping",
                        key_source.label(),
                        error_kind(&e)
                    ));
                    continue;
                }
            };
            let opened = match open_tx_metadata(&aes_key, &blob) {
                Ok(o) => o,
                Err(e) => {
                    breadcrumb_error(&format!(
                        "fetch_encrypted_documents: txMetadata decrypt failed \
                         position={position} error_kind={}; skipping",
                        error_kind(&e)
                    ));
                    continue;
                }
            };

            out.push(DecryptedEncryptedDocument {
                document_id: *doc_id,
                owner_id: doc.owner_id(),
                key_index,
                encryption_key_index,
                version: opened.version,
                updated_at_ms: doc.updated_at(),
                payload: opened.payload,
            });
        }
        breadcrumb(&format!(
            "fetch_encrypted_documents: returning decrypted documents raw={} decrypted={}",
            raw_docs.len(),
            out.len()
        ));
        out
    }
}

/// What a paginated scan does once it has read a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextPage {
    /// The page just read was the last one; the scan is complete.
    Done,
    /// Request the next page continuing after this cursor.
    ContinueAfter(Identifier),
}

/// Decides, from page shape alone, whether a paginated scan is still advancing.
///
/// Every full page hands back the cursor the next request continues from. A
/// cursor that has already been used means the source is repeating itself, and
/// paging on would refetch the same documents without end while the result grew
/// without bound. Yielding what was collected so far would be worse than
/// failing: a caller cannot tell a truncated history from a complete one, and
/// for transaction metadata that difference matters — so a repeat becomes a
/// typed error instead.
///
/// Every cursor is remembered, not just the previous one, so a scan that cycles
/// through several pages before returning to an earlier cursor is caught on the
/// same terms as one that immediately repeats itself.
///
/// Deliberately pure, synchronous and finite: the decision depends only on how
/// many entries a page held and which key ended it, never on the network or on
/// elapsed time. That is what lets the stall contract be exercised directly,
/// rather than by starting a scan against an always-ready source and relying on
/// a timeout to stop it.
#[derive(Debug, Default)]
struct PaginationProgress {
    /// Cursors the scan has already continued from.
    issued_cursors: std::collections::HashSet<Identifier>,
    /// Pages read so far, reported with a stall so the failure says how far the
    /// scan got.
    pages_read: usize,
}

impl PaginationProgress {
    /// Record one page and decide what the scan does next.
    ///
    /// `page_len` is how many entries the source returned and `page_limit` the
    /// number requested, so a short page ends the scan. `last_id` is the page's
    /// final key in the order the source returned it, which is the cursor the
    /// next request would continue from.
    fn record_page(
        &mut self,
        page_len: usize,
        page_limit: usize,
        last_id: Option<Identifier>,
    ) -> Result<NextPage, PlatformWalletError> {
        self.pages_read += 1;

        // A page the source could not fill is the last page.
        if page_len < page_limit {
            return Ok(NextPage::Done);
        }

        match last_id {
            // `insert` reports whether the cursor is new; a cursor already used
            // means this page did not move the scan forward.
            Some(id) if self.issued_cursors.insert(id) => Ok(NextPage::ContinueAfter(id)),
            Some(_) => Err(PlatformWalletError::EncryptedDocumentPaginationStalled {
                pages: self.pages_read,
            }),
            // A full page with no final key yields no cursor to continue from.
            None => Ok(NextPage::Done),
        }
    }
}

/// Run the paginated owner-scoped, since-timestamp document scan that
/// [`IdentityWallet::fetch_encrypted_documents`] fetches from — split out
/// (taking only the `Sdk` + the already-fetched `contract`) so the exact wire
/// query is integration-testable against testnet without a resident
/// wallet/identity: the decrypt half needs the wallet mnemonic, this half does
/// not. Covered by `tests/txmetadata_fetch.rs`.
///
/// Query shape (verified byte-for-byte against the legacy `TxMetadata.get`
/// builder and confirmed to return the real testnet documents): `$ownerId ==`
/// owner + `$updatedAt >= since_ms`, ordered `$updatedAt asc`. The order-by is
/// load-bearing, not cosmetic — drive answers a bare secondary-index equality
/// or an un-ordered range with a proof of ABSENCE (the same trap the
/// `contactInfo` sweep documents), and it also gives the deterministic order
/// pagination relies on. Returns the raw, still-encrypted documents; a
/// `None` entry is a proof of a document the SDK could not materialize and is
/// preserved so the caller's count/telemetry never silently under-reports.
pub async fn query_owned_encrypted_documents(
    sdk: &dash_sdk::Sdk,
    contract: Arc<DataContract>,
    owner_identity_id: &Identifier,
    document_type_name: &str,
    since_ms: u64,
) -> Result<Vec<(Identifier, Option<Document>)>, PlatformWalletError> {
    use dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
    use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
    use dash_sdk::platform::FetchMany;
    use dpp::platform_value::platform_value;

    const PAGE: u32 = 100;
    // `since_ms` is caller-supplied and a timestamp correlates a device to
    // when it last synced, so the value is not rendered — only that the scan
    // started.
    breadcrumb("query_owned_encrypted_documents: entry");
    let mut raw_docs: Vec<(Identifier, Option<Document>)> = Vec::new();
    let mut start: Option<Start> = None;
    let mut progress = PaginationProgress::default();
    loop {
        let query = dash_sdk::platform::DocumentQuery {
            select: dash_sdk::drive::query::SelectProjection::documents(),
            data_contract: Arc::clone(&contract),
            document_type_name: document_type_name.to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(owner_identity_id),
                },
                WhereClause {
                    field: "$updatedAt".to_string(),
                    operator: WhereOperator::GreaterThanOrEquals,
                    value: platform_value!(since_ms),
                },
            ],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![OrderClause {
                field: "$updatedAt".to_string(),
                ascending: true,
            }],
            limit: PAGE,
            // Paging is by insertion-order cursor (`start`), not by row offset:
            // `offset` is served only on the ranked-aggregate surface, and a
            // skip-count page would silently drop documents whenever the owner
            // writes between round-trips.
            offset: None,
            start: start.clone(),
        };

        let page = Document::fetch_many(sdk, query).await.map_err(|e| {
            breadcrumb_error("query_owned_encrypted_documents: fetch_many failed error_kind=sdk");
            PlatformWalletError::Sdk(e)
        })?;
        let page_len = page.len();
        let last_id = page.keys().last().copied();
        raw_docs.extend(page);

        // Decided before the next request is built, so a stalled scan costs no
        // further round-trips.
        match progress
            .record_page(page_len, PAGE as usize, last_id)
            .inspect_err(|_| {
                breadcrumb_error("query_owned_encrypted_documents: page cursor repeated; stopping")
            })? {
            NextPage::Done => break,
            NextPage::ContinueAfter(id) => {
                start = Some(Start::StartAfter(id.to_buffer().to_vec()));
            }
        }
    }

    // Both counts are recorded BEFORE any decrypt, so an empty end result can be
    // attributed to the query returning nothing, to documents the SDK could not
    // materialize, or to the decrypt stage that runs after this — three causes
    // that are otherwise indistinguishable from one another.
    breadcrumb(&format!(
        "query_owned_encrypted_documents: fetched raw encrypted documents \
         raw_count={} materialized={}",
        raw_docs.len(),
        raw_docs.iter().filter(|(_, d)| d.is_some()).count()
    ));
    Ok(raw_docs)
}

#[cfg(test)]
mod allocator_tests {
    //! The `encryptionKeyIndex` allocator and the pagination scan it seeds from.
    //!
    //! Both are exercised without a live SDK: the Platform-derived seed is
    //! injected as a plain future and the pagination decision is driven
    //! directly, so every case here is finite by construction rather than
    //! bounded by a wall-clock timeout.
    use super::*;

    /// The identity these cases allocate for.
    const TEST_OWNER: Identifier = Identifier::new([7u8; 32]);
    /// Two contracts an identity could hold encrypted documents on. The
    /// allocator seeds from a count taken for ONE contract and document type, so
    /// these exist to prove a high-water never crosses into another scope.
    const TEST_CONTRACT_A: Identifier = Identifier::new([11u8; 32]);
    const TEST_CONTRACT_B: Identifier = Identifier::new([12u8; 32]);

    fn empty_allocator() -> EncryptionKeyIndexAllocator {
        Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()))
    }

    /// The allocator key a hand-out belongs to, built the way production builds
    /// it. Tests go through this helper so what the allocator considers "the
    /// same series" is stated in exactly one place.
    fn test_scope(
        owner: Identifier,
        contract: Identifier,
        document_type_name: &str,
    ) -> EncryptionKeyIndexScope {
        EncryptionKeyIndexScope::new(&owner, &contract, document_type_name)
    }

    /// The seeding formula is the legacy wallet's, exactly.
    ///
    /// A migrating install keeps numbering where its old local counter left off
    /// only because this is `count + 1` and not `max(index) + 1` — the two agree
    /// on a dense series and diverge the moment one has a gap, and a divergence
    /// here silently changes which key every later document is sealed under.
    #[test]
    fn the_seed_formula_is_one_plus_the_existing_count() {
        assert_eq!(
            next_encryption_key_index_from_count(0).expect("empty state"),
            1
        );
        assert_eq!(next_encryption_key_index_from_count(1).expect("one"), 2);
        assert_eq!(next_encryption_key_index_from_count(5).expect("five"), 6);
        assert!(
            matches!(
                next_encryption_key_index_from_count(u32::MAX),
                Err(PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted)
            ),
            "a count with no representable successor must fail rather than clamp \
             onto an index the series already used"
        );
    }

    /// Empty state seeds to `1 + count(0) == 1`, then hands out 2, 3 … without
    /// re-seeding: once the high-water exists, no further network work may
    /// happen, so the seed future must never be polled again.
    #[tokio::test]
    async fn empty_state_seeds_to_one_then_increments() {
        let allocator = empty_allocator();
        let scope = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");

        let first = reserve_next_index(&allocator, &scope, async { Ok(1) })
            .await
            .expect("the first allocation seeds");
        assert_eq!(first, 1, "empty state must allocate index 1");

        let must_not_seed =
            || async { unreachable!("a seeded scope must not query Platform again") };
        assert_eq!(
            reserve_next_index(&allocator, &scope, must_not_seed())
                .await
                .expect("second allocation"),
            2
        );
        assert_eq!(
            reserve_next_index(&allocator, &scope, must_not_seed())
                .await
                .expect("third allocation"),
            3
        );
    }

    /// The last derivable index is usable, and the series ends immediately after.
    ///
    /// The index is a hardened derivation element, so a hand-out above
    /// [`MAX_TX_METADATA_ENCRYPTION_KEY_INDEX`] would seal a document with a key
    /// nothing can re-derive — worse than refusing, because the failure would
    /// surface only when someone later tried to read it. The boundary has to be
    /// exact in both directions: one too low silently denies a usable index, one
    /// too high hands out an unusable one.
    #[tokio::test]
    async fn the_last_derivable_index_is_handed_out_once_then_the_series_is_exhausted() {
        let allocator = empty_allocator();
        let scope = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");

        let last = reserve_next_index(&allocator, &scope, async {
            Ok(MAX_TX_METADATA_ENCRYPTION_KEY_INDEX)
        })
        .await
        .expect("the maximum derivable index is usable and must be handed out");
        assert_eq!(last, MAX_TX_METADATA_ENCRYPTION_KEY_INDEX);

        let outcome = reserve_next_index(&allocator, &scope, async {
            unreachable!("the scope is already seeded")
        })
        .await;
        assert!(
            matches!(
                outcome,
                Err(PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted)
            ),
            "the index after the last derivable one must be refused rather than \
             handed out; got {outcome:?}"
        );

        // Exhaustion is terminal, not a one-off: a later caller must not find a
        // usable high-water sitting past the end of the series.
        let outcome_again = reserve_next_index(&allocator, &scope, async {
            unreachable!("the scope is already seeded")
        })
        .await;
        assert!(
            matches!(
                outcome_again,
                Err(PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted)
            ),
            "an exhausted scope must keep failing; got {outcome_again:?}"
        );
    }

    /// A seed already past the derivable range never hands out anything.
    ///
    /// The seed comes from a Platform document count, so a corrupted or
    /// adversarial count is the one way a scope can start beyond the end of the
    /// series rather than walking to it.
    #[tokio::test]
    async fn a_seed_past_the_derivable_range_is_refused_outright() {
        let allocator = empty_allocator();
        let scope = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");

        for seeded in [MAX_TX_METADATA_ENCRYPTION_KEY_INDEX + 1, u32::MAX] {
            let outcome = reserve_next_index(&allocator, &scope, async move { Ok(seeded) }).await;
            assert!(
                matches!(
                    outcome,
                    Err(PlatformWalletError::TxMetadataEncryptionKeyIndexExhausted)
                ),
                "a seed of {seeded} is past the derivable range and must be refused; \
                 got {outcome:?}"
            );
        }
    }

    /// A high-water is only valid for the scope it was counted from.
    ///
    /// The seed counts the documents of ONE (owner, contract, document type)
    /// triple, and both the FFI exports and the host APIs accept an arbitrary
    /// contract and document type. Reusing one triple's high-water for another
    /// would hand out an index derived from a count that never described it —
    /// breaking the `1 + count` contract for the second series.
    #[tokio::test]
    async fn each_owner_contract_and_document_type_seeds_independently() {
        let allocator = empty_allocator();

        let a = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");
        assert_eq!(
            reserve_next_index(&allocator, &a, async { Ok(4) })
                .await
                .expect("contract A seeds from its own count of 3"),
            4
        );

        // Same owner, different contract: a fresh series, seeded from its own
        // (empty) count rather than continuing contract A's.
        let b = test_scope(TEST_OWNER, TEST_CONTRACT_B, "txMetadata");
        assert_eq!(
            reserve_next_index(&allocator, &b, async { Ok(1) })
                .await
                .expect("contract B seeds independently"),
            1,
            "a second contract must seed from its own count, not continue the first's"
        );

        // Same owner and contract, different document type: likewise its own
        // series.
        let other_type = test_scope(TEST_OWNER, TEST_CONTRACT_A, "otherEncryptedType");
        assert_eq!(
            reserve_next_index(&allocator, &other_type, async { Ok(1) })
                .await
                .expect("the other document type seeds independently"),
            1,
            "a second document type must seed from its own count, not continue \
             the first's"
        );

        // The original series is untouched by either of them.
        assert_eq!(
            reserve_next_index(&allocator, &a, async {
                unreachable!("contract A is already seeded")
            })
            .await
            .expect("contract A continues"),
            5
        );
    }

    /// The core concurrency guarantee: two allocations racing on the SAME scope
    /// get DISTINCT indices, even when both of their seed futures actually run.
    ///
    /// Both seeds observing the same pre-write Platform count is the expected
    /// case — the count query is not serialized with the create — so the
    /// allocator, not the seed, is what makes the two hand-outs differ.
    #[tokio::test]
    async fn concurrent_first_allocations_never_collide_even_when_both_seeds_run() {
        let allocator = empty_allocator();
        let scope = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");

        // Both seeds yield first, so each is guaranteed to be in flight while
        // the other runs, and both compute the same value from the same count.
        let seed = || async {
            tokio::task::yield_now().await;
            Ok(1)
        };
        let (first, second) = tokio::join!(
            reserve_next_index(&allocator, &scope, seed()),
            reserve_next_index(&allocator, &scope, seed()),
        );
        let mut handed_out = [
            first.expect("first allocation"),
            second.expect("second allocation"),
        ];
        handed_out.sort_unstable();
        assert_eq!(
            handed_out,
            [1, 2],
            "two racing allocations for one scope must hand out two different indices"
        );
    }

    /// A seed that never answers must not freeze the whole wallet.
    ///
    /// The seed is a Platform round trip and the SDK sets no request timeout, so
    /// a node that accepts the connection and never replies stalls it forever.
    /// The allocator state is shared by every identity in the process: if the
    /// shared lock were held across that round trip, one unresponsive node would
    /// block every other encrypted-document create in the wallet instead of just
    /// the one waiting on it.
    #[tokio::test(start_paused = true)]
    async fn a_stalled_seed_does_not_block_other_scopes_or_cached_allocations() {
        /// Long enough that only a genuinely blocked allocation reaches it;
        /// virtual time makes it elapse instantly when nothing can progress.
        const STALL_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

        let allocator = empty_allocator();
        let stalled_scope = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");
        let cached_scope = test_scope(TEST_OWNER, TEST_CONTRACT_B, "txMetadata");
        let fresh_scope = test_scope(TEST_OWNER, TEST_CONTRACT_B, "otherEncryptedType");

        // Seed one scope up front so its later hand-out needs no network at all.
        reserve_next_index(&allocator, &cached_scope, async { Ok(1) })
            .await
            .expect("cached scope seeds");

        // `_never_answers` is held to the end of the test, so the seed below
        // stays pending rather than resolving with a channel error.
        let (_never_answers, never_answered) = tokio::sync::oneshot::channel::<u32>();
        let stalled = reserve_next_index(&allocator, &stalled_scope, async {
            Ok(never_answered
                .await
                .expect("the stalled seed never answers"))
        });
        tokio::pin!(stalled);

        // Drive the stalled allocation up to its seed await, which is where a
        // lock-holding implementation would be holding the shared lock.
        tokio::select! {
            _ = &mut stalled => panic!("the stalled seed must not complete"),
            _ = tokio::task::yield_now() => {}
        }

        let cached = tokio::time::timeout(
            STALL_BUDGET,
            reserve_next_index(&allocator, &cached_scope, async {
                unreachable!("the cached scope is already seeded")
            }),
        )
        .await;
        assert_eq!(
            cached
                .expect("a hand-out from an already-seeded scope must not wait on another scope's network call")
                .expect("cached allocation"),
            2
        );

        let fresh = tokio::time::timeout(
            STALL_BUDGET,
            reserve_next_index(&allocator, &fresh_scope, async { Ok(1) }),
        )
        .await;
        assert_eq!(
            fresh
                .expect(
                    "another scope's first allocation must not wait on an unrelated stalled seed"
                )
                .expect("fresh allocation"),
            1
        );
    }

    /// An oversized payload is rejected before the allocator is touched.
    ///
    /// The size bound is deterministic and needs no network, so a request that
    /// must fail should not seed the high-water or consume an index — otherwise
    /// every rejected batch would burn an index and leave a gap in a series the
    /// legacy stack expects to be dense.
    #[tokio::test]
    async fn an_oversized_payload_does_not_seed_or_advance_the_high_water() {
        use crate::wallet::identity::crypto::tx_metadata::MAX_TX_METADATA_PLAINTEXT_LEN;

        let allocator = empty_allocator();
        let scope = test_scope(TEST_OWNER, TEST_CONTRACT_A, "txMetadata");

        let outcome = reserve_next_index_checked(
            &allocator,
            &scope,
            MAX_TX_METADATA_PLAINTEXT_LEN + 1,
            async { unreachable!("an oversized payload must not reach the seed") },
        )
        .await;
        match outcome {
            Err(PlatformWalletError::TxMetadataPayloadTooLarge { len, max }) => {
                assert_eq!(len, MAX_TX_METADATA_PLAINTEXT_LEN + 1);
                assert_eq!(max, MAX_TX_METADATA_PLAINTEXT_LEN);
            }
            other => panic!("expected TxMetadataPayloadTooLarge, got {other:?}"),
        }

        // The rejected request left nothing behind in the map.
        assert!(
            allocator.lock().await.get(&scope).is_none(),
            "an oversized payload must not seed or advance the high-water"
        );

        // The next well-sized request still seeds fresh at 1: the rejected one
        // left no reservation and no gap.
        let index = reserve_next_index_checked(&allocator, &scope, 0, async { Ok(1) })
            .await
            .expect("a well-sized payload allocates");
        assert_eq!(
            index, 1,
            "the first index after a rejected oversized payload must still be 1"
        );
    }

    // ── Pagination stall detection ──────────────────────────────────────────
    //
    // These drive [`PaginationProgress`] — the same decision the production scan
    // makes after every page — directly. Feeding it page shapes is finite by
    // construction: each call returns, so a scan that would never stop shows up
    // as the wrong return value rather than as a test that has to be cut short.
    // Exercising it through a source that always answers would instead need a
    // timeout, which reports "still running when time ran out" and not "the
    // repeat was detected".

    /// Page limit these cases page at. Small on purpose: the stall contract
    /// depends on a page being FULL, not on how many entries that takes, so two
    /// keeps each scenario readable as a sequence of cursors.
    const STALL_PAGE_LIMIT: usize = 2;

    /// Distinct page cursors, named so a scan reads as the sequence it is.
    const CURSOR_A: Identifier = Identifier::new([0xA1; 32]);
    const CURSOR_B: Identifier = Identifier::new([0xB2; 32]);

    /// A source that answers every request with the same full page is reported,
    /// not paged forever.
    ///
    /// The first page yields cursor A and the scan continues after it. The
    /// source hands back a full page ending at A again, so the scan is not
    /// advancing: continuing would refetch the same documents indefinitely and
    /// grow the result without bound. Yielding what was collected would be worse
    /// than failing, because a caller cannot distinguish a truncated history
    /// from a complete one — so it is a typed error, reported on the second
    /// page, which is the first one that could prove the repeat.
    #[test]
    fn a_page_cursor_that_immediately_repeats_is_reported_as_a_stall() {
        let mut progress = PaginationProgress::default();

        assert_eq!(
            progress
                .record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, Some(CURSOR_A))
                .expect("the first page cannot repeat anything and must continue"),
            NextPage::ContinueAfter(CURSOR_A),
            "a full page must continue after the cursor it ended on"
        );

        match progress.record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, Some(CURSOR_A)) {
            Err(PlatformWalletError::EncryptedDocumentPaginationStalled { pages }) => assert_eq!(
                pages, 2,
                "the stall is reported on the page that proved the repeat, and both \
                 pages were read to get there"
            ),
            other => panic!(
                "a repeated page cursor must be reported as its own error rather than \
                 continued or reported as something else; got {other:?}"
            ),
        }
    }

    /// A cursor cycle that passes through another page is reported on the same
    /// terms as one that repeats immediately.
    ///
    /// The scan runs A, then B, then A again. Only comparing against the
    /// PREVIOUS cursor would see B follow A and A follow B and call both an
    /// advance, so the scan would loop over the same two pages forever. Every
    /// cursor the scan has continued from is remembered, so returning to A is a
    /// stall no matter how many pages the cycle spans.
    #[test]
    fn a_page_cursor_that_repeats_after_an_intervening_page_is_reported_as_a_stall() {
        let mut progress = PaginationProgress::default();

        assert_eq!(
            progress
                .record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, Some(CURSOR_A))
                .expect("the first page must continue"),
            NextPage::ContinueAfter(CURSOR_A)
        );
        assert_eq!(
            progress
                .record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, Some(CURSOR_B))
                .expect("a new cursor is an advance and must continue"),
            NextPage::ContinueAfter(CURSOR_B),
            "a cursor the scan has not used before must not be mistaken for a stall"
        );

        match progress.record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, Some(CURSOR_A)) {
            Err(PlatformWalletError::EncryptedDocumentPaginationStalled { pages }) => assert_eq!(
                pages, 3,
                "the cycle took three pages to close, and the count must say so"
            ),
            other => panic!(
                "returning to an earlier cursor must be reported as a stall even with a \
                 page in between; got {other:?}"
            ),
        }
    }

    /// A scan that keeps advancing runs to its natural end.
    ///
    /// Guards the detector against the opposite failure: rejecting healthy
    /// scans. Distinct cursors continue, and the short page that follows ends
    /// the scan rather than asking for a cursor it has no reason to distrust.
    #[test]
    fn an_advancing_scan_runs_to_a_short_page_without_a_stall() {
        let mut progress = PaginationProgress::default();

        for cursor in [CURSOR_A, CURSOR_B] {
            assert_eq!(
                progress
                    .record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, Some(cursor))
                    .expect("distinct cursors are an advancing scan, never a stall"),
                NextPage::ContinueAfter(cursor)
            );
        }

        assert_eq!(
            progress
                .record_page(STALL_PAGE_LIMIT - 1, STALL_PAGE_LIMIT, Some(CURSOR_A))
                .expect("a short page ends the scan and cannot stall it"),
            NextPage::Done,
            "a page the source could not fill is the last page, so its key is never \
             used as a cursor and repeating one is not a stall"
        );
    }

    /// A full page carrying no final key ends the scan.
    ///
    /// There is no cursor to continue from, so the only alternative to stopping
    /// would be reissuing the previous request unchanged.
    #[test]
    fn a_full_page_without_a_final_key_ends_the_scan() {
        let mut progress = PaginationProgress::default();

        assert_eq!(
            progress
                .record_page(STALL_PAGE_LIMIT, STALL_PAGE_LIMIT, None)
                .expect("a missing cursor ends the scan rather than failing it"),
            NextPage::Done
        );
    }
}

#[cfg(test)]
mod query_tests {
    //! The query path against a mocked Platform: what its breadcrumbs may say,
    //! how it walks pages, and what the allocator's seed counts. All offline —
    //! every expectation is registered on a mock SDK, so nothing here reaches a
    //! network.
    use super::*;
    use std::sync::Mutex;

    use crate::changeset::{PersistenceError, PlatformWalletPersistence};
    use crate::wallet::WalletId;
    use crate::ClientStartState;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    // ── Breadcrumb redaction ────────────────────────────────────────────────
    //
    // The encrypted-document breadcrumbs are dual-emitted to Android logcat.
    // Logcat is readable by any process holding READ_LOGS and survives in bug
    // reports, so a breadcrumb must never persist data that correlates a device
    // to an on-chain identity, nor echo a raw error body (which can carry query
    // shapes, contract internals, or decrypted context). Stable codes, booleans
    // and bounded non-sensitive context are fine; full identifiers are not.

    /// Captures every `tracing` event's level and rendered `message` so a test
    /// can assert on what the breadcrumbs actually emit.
    #[derive(Clone, Default)]
    struct CapturedBreadcrumbs(Arc<Mutex<Vec<(tracing::Level, String)>>>);

    impl CapturedBreadcrumbs {
        fn lines(&self) -> Vec<(tracing::Level, String)> {
            self.0.lock().expect("capture buffer not poisoned").clone()
        }
    }

    /// Pulls the `message` field out of an event, which is where both
    /// [`breadcrumb`] and [`breadcrumb_error`] put their whole formatted line.
    struct MessageVisitor(String);

    impl tracing::field::Visit for MessageVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CapturedBreadcrumbs {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut visitor = MessageVisitor(String::new());
            event.record(&mut visitor);
            self.0
                .lock()
                .expect("capture buffer not poisoned")
                .push((*event.metadata().level(), visitor.0));
        }
    }

    /// The owner identifier the breadcrumbs and queries are given.
    const TEST_OWNER: Identifier = Identifier::new([7u8; 32]);

    /// Outcome of one captured, deterministically-failing query run.
    struct CapturedQuery {
        lines: Vec<(tracing::Level, String)>,
        contract_id: Identifier,
        error: PlatformWalletError,
    }

    /// Drive the real query path against a mock SDK carrying NO registered
    /// expectation. The contract is supplied directly, so the query runs and
    /// `fetch_many` fails deterministically — exercising the entry breadcrumb
    /// and the failure breadcrumb in a single call, with no network.
    async fn capture_failing_query_for_type(document_type_name: &str) -> CapturedQuery {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let sdk = dash_sdk::Sdk::new_mock();
        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = contract.id();

        let captured = CapturedBreadcrumbs::default();
        let collected = captured.clone();
        let error = {
            let _guard = tracing_subscriber::registry().with(captured).set_default();
            query_owned_encrypted_documents(&sdk, contract, &TEST_OWNER, document_type_name, 0)
                .await
                .expect_err("a mock SDK with no expectation must fail the page fetch")
        };

        let lines = collected.lines();
        assert!(
            !lines.is_empty(),
            "the query path must emit breadcrumbs for these assertions to mean anything"
        );
        CapturedQuery {
            lines,
            contract_id,
            error,
        }
    }

    /// The ordinary document type this module is written for.
    async fn capture_failing_query() -> CapturedQuery {
        capture_failing_query_for_type("txMetadata").await
    }

    struct NoopPersister;
    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: crate::changeset::PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEventHandler;
    impl crate::events::EventHandler for NoopEventHandler {}
    impl crate::PlatformEventHandler for NoopEventHandler {}

    /// The document type is caller-supplied and travels straight from the host
    /// into this module. Nothing bounds its length, character set, or content,
    /// so a breadcrumb that interpolates it raw lets a caller write arbitrary
    /// text — including secret-looking material and embedded newlines that
    /// forge additional log lines — into a device log that any process holding
    /// READ_LOGS can read.
    #[tokio::test]
    async fn query_breadcrumbs_do_not_echo_the_caller_supplied_document_type() {
        // A hostile document type: an embedded newline to forge a log line, and
        // a marker standing in for whatever the caller chose to put here.
        const MARKER: &str = "s3cr3t-marker-do-not-log";
        let hostile = format!("txMetadata\nFORGED WARN line {MARKER}");

        let captured = capture_failing_query_for_type(&hostile).await;

        for (level, line) in &captured.lines {
            assert!(
                !line.contains(MARKER),
                "{level} breadcrumb echoes caller-supplied document-type content: {line}"
            );
            assert!(
                !line.contains('\n'),
                "{level} breadcrumb contains an embedded newline, letting a caller \
                 forge additional log lines: {line}"
            );
        }
    }

    /// No breadcrumb, at any level, may carry a full owner or contract
    /// identifier: logcat is readable by any process holding READ_LOGS and
    /// survives in bug reports, so a full identifier there correlates a device
    /// to an on-chain identity.
    #[tokio::test]
    async fn query_breadcrumbs_redact_owner_and_contract_identifiers() {
        let captured = capture_failing_query().await;

        // Rendered exactly the way the breadcrumbs interpolate them (`Display`).
        let owner_rendered = format!("{TEST_OWNER}");
        let contract_rendered = format!("{}", captured.contract_id);

        for (level, line) in &captured.lines {
            assert!(
                !line.contains(&owner_rendered),
                "{level} breadcrumb carries the full owner identity id: {line}"
            );
            assert!(
                !line.contains(&contract_rendered),
                "{level} breadcrumb carries the full contract id: {line}"
            );
        }
    }

    /// A failure breadcrumb must classify, not transcribe. The SDK error body
    /// is unbounded and carries query and contract internals, so the exact
    /// `Display` of the error the call returned must not appear in the WARN
    /// line. The label itself is not the problem — the verbatim body is — so
    /// this compares against the real error string rather than banning a token.
    #[tokio::test]
    async fn query_failure_breadcrumb_redacts_the_raw_sdk_error_body() {
        let captured = capture_failing_query().await;

        // The exact body the breadcrumb would transcribe: the inner SDK error's
        // own `Display`, taken from the very error this call returned.
        let raw_body = match &captured.error {
            PlatformWalletError::Sdk(sdk_error) => format!("{sdk_error}"),
            other => panic!("expected the page fetch to fail as Sdk(_), got {other:?}"),
        };
        assert!(
            !raw_body.is_empty(),
            "the SDK error must render to something for this assertion to bite"
        );

        let warnings: Vec<_> = captured
            .lines
            .iter()
            .filter(|(level, _)| *level == tracing::Level::WARN)
            .collect();
        assert!(
            !warnings.is_empty(),
            "the failed page fetch must emit a WARN breadcrumb"
        );

        for (level, line) in warnings {
            assert!(
                !line.contains(&raw_body),
                "{level} breadcrumb transcribes the raw SDK error body verbatim \
                 instead of a stable classification.\n  raw body: {raw_body}\n  line: {line}"
            );
        }
    }

    // ── Pagination ──────────────────────────────────────────────────────────

    /// Page size the query paginates at, mirrored from the production loop.
    const PAGE_SIZE: usize = 100;

    /// Rebuild the exact `DocumentQuery` the production loop issues for a given
    /// cursor, so mock expectations key on the same request the code sends.
    fn expected_page_query(
        contract: Arc<DataContract>,
        owner: &Identifier,
        start: Option<
            dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start,
        >,
    ) -> dash_sdk::platform::DocumentQuery {
        use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
        use dpp::platform_value::platform_value;

        dash_sdk::platform::DocumentQuery {
            select: dash_sdk::drive::query::SelectProjection::documents(),
            data_contract: contract,
            document_type_name: "txMetadata".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(owner),
                },
                WhereClause {
                    field: "$updatedAt".to_string(),
                    operator: WhereOperator::GreaterThanOrEquals,
                    value: platform_value!(0u64),
                },
            ],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![OrderClause {
                field: "$updatedAt".to_string(),
                ascending: true,
            }],
            limit: PAGE_SIZE as u32,
            offset: None,
            start,
        }
    }

    /// A document carrying the given id and `$updatedAt`.
    fn document_at(id: Identifier, updated_at_ms: u64) -> Document {
        Document::V0(dpp::document::DocumentV0 {
            id,
            owner_id: TEST_OWNER,
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: Some(updated_at_ms),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    /// Full pagination walk over a boundary-sized first page, offline.
    ///
    /// The scenario is the one that separates an order-preserving cursor from a
    /// sorted one: every document on page one shares the SAME `$updatedAt`, so
    /// the `$updatedAt asc` ordering cannot disambiguate them, and the ids are
    /// assigned in DESCENDING order so the final returned document is also the
    /// numerically smallest. That last entry is additionally unmaterialized
    /// (`None`), the shape a proved fetch returns for a document it could not
    /// produce. The cursor must still be that final entry's key: a sorted map
    /// would hand back the largest id instead and silently skip every document
    /// between them on the next page.
    ///
    /// Termination is proved by construction — only two page requests are
    /// registered, so a third would find no expectation and fail the call.
    #[tokio::test]
    async fn paginates_by_final_insertion_order_key_across_a_full_page() {
        use dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;

        // Pin the protocol version so the wire encoding of page two matches the
        // expectation registered for it; an unpinned mock ratchets to the
        // latest version after the first response and re-encodes the request.
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");

        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        // Page one: exactly PAGE_SIZE entries, identical timestamps, descending
        // ids, final entry unmaterialized.
        const SHARED_TIMESTAMP: u64 = 1_700_000_000_000;
        let page_one_ids: Vec<Identifier> = (0..PAGE_SIZE)
            .map(|i| Identifier::from([(200 - i) as u8; 32]))
            .collect();
        let mut page_one: drive_proof_verifier::types::Documents = Default::default();
        for (position, id) in page_one_ids.iter().enumerate() {
            let is_final = position == PAGE_SIZE - 1;
            page_one.insert(
                *id,
                if is_final {
                    None
                } else {
                    Some(document_at(*id, SHARED_TIMESTAMP))
                },
            );
        }
        let final_page_one_key = *page_one_ids.last().expect("page one is not empty");

        // Page two: short, so the loop terminates after consuming it.
        let page_two_ids: Vec<Identifier> = (0..3)
            .map(|i| Identifier::from([(50 - i) as u8; 32]))
            .collect();
        let mut page_two: drive_proof_verifier::types::Documents = Default::default();
        for id in &page_two_ids {
            page_two.insert(*id, Some(document_at(*id, SHARED_TIMESTAMP + 1)));
        }

        sdk.mock()
            .expect_fetch_many(
                expected_page_query(Arc::clone(&contract), &TEST_OWNER, None),
                Some(page_one),
            )
            .await
            .expect("register page one");
        sdk.mock()
            .expect_fetch_many(
                expected_page_query(
                    Arc::clone(&contract),
                    &TEST_OWNER,
                    Some(Start::StartAfter(final_page_one_key.to_buffer().to_vec())),
                ),
                Some(page_two),
            )
            .await
            .expect("register page two");

        let fetched = query_owned_encrypted_documents(
            &sdk,
            Arc::clone(&contract),
            &TEST_OWNER,
            "txMetadata",
            0,
        )
        .await
        .expect(
            "both pages are registered; a failure here means the cursor did not select the \
             final insertion-order key, so page two was requested with the wrong StartAfter",
        );

        // Every document, exactly once, in Drive's returned order.
        let expected_order: Vec<Identifier> = page_one_ids
            .iter()
            .chain(page_two_ids.iter())
            .copied()
            .collect();
        let actual_order: Vec<Identifier> = fetched.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            actual_order, expected_order,
            "results must preserve Drive's returned order across the page boundary"
        );
        assert_eq!(
            fetched.len(),
            PAGE_SIZE + page_two_ids.len(),
            "every document is returned exactly once"
        );

        // The unmaterialized entry is preserved rather than dropped, so callers
        // never silently under-report.
        assert!(
            fetched[PAGE_SIZE - 1].1.is_none(),
            "the final page-one entry was unmaterialized and must be preserved as None"
        );
        assert_eq!(
            fetched.iter().filter(|(_, doc)| doc.is_none()).count(),
            1,
            "exactly one entry was unmaterialized"
        );
    }

    // ── Key acquisition happens after the query, never before ───────────────
    //
    // Acquiring the txMetadata key context can consult the host key resolver —
    // which on some platforms prompts the user — and whatever it yields would
    // then have to survive the paginated scan, an unbounded wait. A scan that
    // fails, or that finds nothing, must therefore cost no key acquisition at
    // all.
    //
    // The wallet these cases build has NO managed identity, so any attempt to
    // resolve the encryption context fails with an identity error. That is what
    // makes the ordering observable: an identity error proves acquisition was
    // reached, and its absence proves it was not.

    /// Build a wallet on a mock SDK with no managed identity.
    async fn wallet_without_managed_identity(
        sdk: dash_sdk::Sdk,
    ) -> std::sync::Arc<crate::PlatformWallet> {
        use key_wallet::mnemonic::{Language, Mnemonic};

        let manager = Arc::new(crate::PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoopPersister),
            Arc::new(NoopEventHandler) as Arc<dyn crate::PlatformEventHandler>,
        ));
        let seed = Mnemonic::from_entropy(&[0u8; 16], Language::English)
            .expect("16 bytes of entropy")
            .to_seed("");
        manager
            .create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &seed,
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .expect("wallet creation on a mock sdk")
    }

    /// A query that fails surfaces the query's own error and acquires no key.
    ///
    /// If the context were resolved first, this wallet's missing identity would
    /// fail before the query ever ran and the caller would see an identity error
    /// instead — so the error's own kind is the proof of ordering.
    #[tokio::test]
    async fn a_failing_query_reports_the_query_error_and_acquires_no_key() {
        let mut sdk = dash_sdk::Sdk::new_mock();
        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = {
            use dpp::data_contract::accessors::v0::DataContractV0Getters;
            contract.id()
        };
        // No `expect_fetch_many` is registered, so the page fetch fails.
        sdk.mock()
            .expect_fetch(contract_id, Some((*contract).clone()))
            .await
            .expect("register the contract fetch");

        let wallet = wallet_without_managed_identity(sdk).await;
        let error = wallet
            .identity()
            .fetch_encrypted_documents(&TEST_OWNER, &contract_id, "txMetadata", 0)
            .await
            .expect_err("the unregistered page fetch must fail");

        assert!(
            matches!(error, PlatformWalletError::Sdk(_)),
            "a failing scan must surface the scan's own error, not an identity \
             error — an identity error would mean the key context was resolved \
             before the query ran; got {error:?}"
        );
    }

    /// A query that returns nothing yields an empty result and acquires no key.
    ///
    /// This wallet cannot resolve an encryption context at all, so the call
    /// succeeding is itself the proof that no acquisition was attempted.
    #[tokio::test]
    async fn an_empty_query_returns_no_documents_and_acquires_no_key() {
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");
        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = {
            use dpp::data_contract::accessors::v0::DataContractV0Getters;
            contract.id()
        };
        sdk.mock()
            .expect_fetch(contract_id, Some((*contract).clone()))
            .await
            .expect("register the contract fetch");
        // A short (empty) page ends the scan immediately.
        let empty: drive_proof_verifier::types::Documents = Default::default();
        sdk.mock()
            .expect_fetch_many(
                expected_page_query(Arc::clone(&contract), &TEST_OWNER, None),
                Some(empty),
            )
            .await
            .expect("register the empty page");

        let wallet = wallet_without_managed_identity(sdk).await;
        let fetched = wallet
            .identity()
            .fetch_encrypted_documents(&TEST_OWNER, &contract_id, "txMetadata", 0)
            .await
            .expect(
                "an empty scan must succeed without acquiring a key; this wallet has no \
                 managed identity, so any acquisition attempt would have failed here",
            );

        assert!(
            fetched.is_empty(),
            "no documents were returned by the query"
        );
    }

    /// A query that DOES return candidates goes on to acquire the key context.
    ///
    /// The mirror of the two cases above: with something to decrypt, acquisition
    /// must be reached — and on this identity-less wallet that surfaces as an
    /// identity error. Without this, the two negative cases could also be
    /// satisfied by never acquiring a key at all.
    #[tokio::test]
    async fn a_non_empty_query_goes_on_to_acquire_the_key_context() {
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");
        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = {
            use dpp::data_contract::accessors::v0::DataContractV0Getters;
            contract.id()
        };
        sdk.mock()
            .expect_fetch(contract_id, Some((*contract).clone()))
            .await
            .expect("register the contract fetch");

        let id = Identifier::from([0x5Au8; 32]);
        let mut page: drive_proof_verifier::types::Documents = Default::default();
        page.insert(id, Some(document_at(id, 1_700_000_000_000)));
        sdk.mock()
            .expect_fetch_many(
                expected_page_query(Arc::clone(&contract), &TEST_OWNER, None),
                Some(page),
            )
            .await
            .expect("register the single-document page");

        let wallet = wallet_without_managed_identity(sdk).await;
        let error = wallet
            .identity()
            .fetch_encrypted_documents(&TEST_OWNER, &contract_id, "txMetadata", 0)
            .await
            .expect_err("this wallet cannot resolve an encryption context");

        assert!(
            !matches!(error, PlatformWalletError::Sdk(_)),
            "with a candidate document present the key context must be acquired, \
             which on this wallet fails with an identity error rather than a scan \
             error; got {error:?}"
        );
    }

    /// A document carrying the given id, `$updatedAt` and txMetadata fields.
    fn encrypted_document_at(
        id: Identifier,
        updated_at_ms: u64,
        key_index: u32,
        encryption_key_index: u32,
        blob: Vec<u8>,
    ) -> Document {
        let mut properties: std::collections::BTreeMap<String, Value> = Default::default();
        properties.insert(FIELD_KEY_INDEX.to_string(), Value::U32(key_index));
        properties.insert(
            FIELD_ENCRYPTION_KEY_INDEX.to_string(),
            Value::U32(encryption_key_index),
        );
        properties.insert(FIELD_ENCRYPTED_METADATA.to_string(), Value::Bytes(blob));

        Document::V0(dpp::document::DocumentV0 {
            id,
            owner_id: TEST_OWNER,
            properties,
            revision: Some(1),
            created_at: None,
            updated_at: Some(updated_at_ms),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    /// A wallet whose manager holds a managed identity at a resident HD slot,
    /// carrying the ECDSA ENCRYPTION key the txMetadata reader selects.
    ///
    /// Returns the wallet plus the `(identity_index, key_index)` the reader will
    /// derive at, so a fixture can seal a blob with the reader's own derivation
    /// instead of guessing it.
    async fn wallet_with_managed_identity(
        sdk: dash_sdk::Sdk,
        owner: Identifier,
    ) -> (std::sync::Arc<crate::PlatformWallet>, u32, u32) {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{IdentityPublicKey, IdentityV0};
        use key_wallet::mnemonic::{Language, Mnemonic};

        const IDENTITY_INDEX: u32 = 0;
        const KEY_INDEX: u32 = 2;

        // The wallet must live on the SDK's own network: the txMetadata
        // derivation path is network-dependent and the reader takes its network
        // from the SDK, so a wallet built on another one derives different keys
        // and even a different wallet id.
        let network = sdk.network;
        let manager = Arc::new(crate::PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoopPersister),
            Arc::new(NoopEventHandler) as Arc<dyn crate::PlatformEventHandler>,
        ));
        let seed = Mnemonic::from_entropy(&[0u8; 16], Language::English)
            .expect("16 bytes of entropy")
            .to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                &seed,
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .expect("wallet creation on a mock sdk");

        // The reader selects an ECDSA ENCRYPTION/MEDIUM key, so the fixture
        // identity must carry one at the id the blob will be sealed under.
        let encryption_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: KEY_INDEX,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: KeyType::ECDSA_SECP256K1,
            contract_bounds: None,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![0x02; 33]),
            disabled_at: None,
        });
        let identity = dpp::identity::Identity::V0(IdentityV0 {
            id: owner,
            public_keys: [(KEY_INDEX, encryption_key)].into_iter().collect(),
            balance: 0,
            revision: 1,
        });

        let identity_wallet = wallet.identity();
        let wallet_id = identity_wallet.wallet_id;
        let persister = identity_wallet.persister.clone();
        {
            let mut wm = identity_wallet.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("the wallet just created is registered");
            info.identity_manager
                .add_identity(identity, IDENTITY_INDEX, wallet_id, &persister)
                .expect("register the managed identity");
        }

        (wallet, IDENTITY_INDEX, KEY_INDEX)
    }

    /// The BIP-39 seed every wallet fixture in this module is built from.
    fn fixture_seed() -> [u8; 64] {
        use key_wallet::mnemonic::{Language, Mnemonic};
        Mnemonic::from_entropy(&[0u8; 16], Language::English)
            .expect("16 bytes of entropy")
            .to_seed("")
    }

    /// Restore resident private keys on an already-registered wallet.
    ///
    /// Registration downgrades a wallet to external-signable, which is the
    /// mobile shape. A desktop or test wallet that keeps its keys in process
    /// takes the other branch of the key-source dispatch, and that branch has to
    /// be exercised against a wallet that genuinely holds them — swapping in a
    /// seed-bearing wallet built from the SAME seed keeps the wallet id, and so
    /// the registration, intact.
    async fn make_wallet_resident(wallet: &crate::PlatformWallet) {
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        let identity_wallet = wallet.identity();
        let network = identity_wallet.sdk.network;
        let resident =
            Wallet::from_seed_bytes(fixture_seed(), network, WalletAccountCreationOptions::None)
                .expect("seed-bearing wallet");

        let mut wm = identity_wallet.wallet_manager.write().await;
        let (stored, _info) = wm
            .get_wallet_mut_and_info_mut(&identity_wallet.wallet_id)
            .expect("the wallet is registered");
        assert_eq!(
            stored.wallet_id, resident.wallet_id,
            "the resident wallet must be the same wallet, or the registration \
             and the managed identity would no longer refer to it"
        );
        *stored = resident;
    }

    /// The whole decrypt-on-fetch orchestration, end to end against a mocked
    /// Platform: one document this wallet can open, one whose blob is malformed,
    /// and one whose wire version is unsupported.
    ///
    /// The per-piece tests cover the query shape and the crypto separately, but
    /// only driving the orchestrator shows what a caller actually receives: that
    /// a bad document is SKIPPED rather than aborting the sync, that an
    /// unsupported version is skipped on the same terms, and that the surviving
    /// document arrives with its plaintext and its non-secret metadata intact.
    /// A skip that silently dropped everything would satisfy neither.
    // A plain `#[test]` driving its own runtime: only the network stages are
    // awaited, and the decrypt stage runs outside the runtime entirely — the
    // same split the FFI makes, which is the shape
    // `decrypt_fetched_documents_blocking` exists for.
    #[test]
    fn fetch_decrypts_the_valid_document_and_skips_the_malformed_and_unsupported_ones() {
        use crate::wallet::identity::crypto::tx_metadata::{
            derive_tx_metadata_key_from_master, seal_tx_metadata, VERSION_PROTOBUF,
        };
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use key_wallet::bip32::ExtendedPrivKey;
        use key_wallet::mnemonic::{Language, Mnemonic};

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");

        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");
        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = contract.id();

        let (wallet, identity_index, key_index) =
            runtime.block_on(wallet_with_managed_identity(sdk.clone(), TEST_OWNER));

        // Seal a real blob with the SAME derivation the reader will use, so the
        // valid document is one this wallet genuinely owns.
        const ENCRYPTION_KEY_INDEX: u32 = 1;
        const PLAINTEXT: &[u8] = b"memo=coffee;taxCategory=expense";
        // Seal on the SDK's own network: the reader derives with `sdk.network`,
        // and the derivation path is network-dependent, so a mismatch here would
        // produce a key that cannot open its own blob.
        let network = wallet.identity().sdk.network;

        // Sealing secrets live in this block and nowhere else. The seed is
        // `Zeroizing`, so it is scrubbed when the block ends; the master
        // zeroizes on drop and its scalar is also erased explicitly at the use
        // boundary; the AES key is `Zeroizing` and is dropped with the block.
        // Nothing derived from them is in scope after it, so none of them is
        // live across the raw scan below.
        let (good_blob, unsupported_blob) = {
            let seed = Zeroizing::new(
                Mnemonic::from_entropy(&[0u8; 16], Language::English)
                    .expect("16 bytes of entropy")
                    .to_seed(""),
            );
            let mut master = ExtendedPrivKey::new_master(network, seed.as_ref())
                .expect("master xprv from the wallet's own seed");
            let aes_key = derive_tx_metadata_key_from_master(
                &master,
                network,
                identity_index,
                key_index,
                ENCRYPTION_KEY_INDEX,
            )
            .expect("derive the reader's own key");
            let iv = [0x5Cu8; 16];
            let good = seal_tx_metadata(&aes_key, VERSION_PROTOBUF, &iv, PLAINTEXT).expect("seal");
            // Same ciphertext, version byte changed to one nothing can interpret.
            let mut unsupported = good.clone();
            unsupported[0] = 2;
            // Best-effort erase: removes the stack residue, but cannot reach a
            // register copy the optimizer may have made.
            master.private_key.non_secure_erase();
            (good, unsupported)
        };
        // Too short to be an envelope at all.
        let malformed_blob = vec![VERSION_PROTOBUF, 0x00, 0x01];

        let good_id = Identifier::from([0x11u8; 32]);
        let malformed_id = Identifier::from([0x22u8; 32]);
        let unsupported_id = Identifier::from([0x33u8; 32]);

        let mut page: drive_proof_verifier::types::Documents = Default::default();
        page.insert(
            good_id,
            Some(encrypted_document_at(
                good_id,
                1_700_000_000_000,
                key_index,
                ENCRYPTION_KEY_INDEX,
                good_blob,
            )),
        );
        page.insert(
            malformed_id,
            Some(encrypted_document_at(
                malformed_id,
                1_700_000_000_001,
                key_index,
                ENCRYPTION_KEY_INDEX,
                malformed_blob,
            )),
        );
        page.insert(
            unsupported_id,
            Some(encrypted_document_at(
                unsupported_id,
                1_700_000_000_002,
                key_index,
                ENCRYPTION_KEY_INDEX,
                unsupported_blob,
            )),
        );

        runtime.block_on(async {
            sdk.mock()
                .expect_fetch(contract_id, Some((*contract).clone()))
                .await
                .expect("register the contract fetch");
            sdk.mock()
                .expect_fetch_many(
                    expected_page_query(Arc::clone(&contract), &TEST_OWNER, None),
                    Some(page),
                )
                .await
                .expect("register the page");
        });

        // Stage 1 — network only. No key material is in scope: the sealing block
        // above ended, so nothing it produced is alive across this scan.
        let raw = runtime
            .block_on(async {
                wallet
                    .identity()
                    .fetch_raw_encrypted_documents(&TEST_OWNER, &contract_id, "txMetadata", 0)
                    .await
            })
            .expect("the raw scan must succeed without any key");
        assert_eq!(
            raw.len(),
            3,
            "all three raw entries come back from the scan"
        );

        // Stage 2 — acquire a FRESH master only now that there is something to
        // decrypt, decrypt synchronously, and erase it before leaving the block.
        // This runs outside the runtime, matching the FFI, whose decrypt stage
        // executes on its own calling thread — the context
        // `decrypt_fetched_documents_blocking` is built for, where its blocking
        // read can park freely.
        let fetched = {
            let seed = Zeroizing::new(
                Mnemonic::from_entropy(&[0u8; 16], Language::English)
                    .expect("16 bytes of entropy")
                    .to_seed(""),
            );
            let mut master = ExtendedPrivKey::new_master(network, seed.as_ref())
                .expect("master xprv acquired after the scan");
            let decrypted = wallet.identity().decrypt_fetched_documents_blocking(
                &TEST_OWNER,
                &raw,
                TxMetadataKeySource::Master(&master),
            );
            master.private_key.non_secure_erase();
            decrypted
        }
        .expect("a bad document must never abort the decrypt stage");

        assert_eq!(
            fetched.len(),
            1,
            "exactly the one openable document must be returned; the malformed and \
             unsupported ones are skipped, not surfaced and not fatal"
        );
        let only = &fetched[0];
        assert_eq!(
            only.document_id, good_id,
            "the surviving document is the valid one"
        );
        assert_eq!(
            only.payload.as_slice(),
            PLAINTEXT,
            "the decrypted plaintext must reach the caller intact"
        );
        assert_eq!(only.version, VERSION_PROTOBUF);
        assert_eq!(only.key_index, key_index);
        assert_eq!(only.encryption_key_index, ENCRYPTION_KEY_INDEX);
        assert_eq!(only.updated_at_ms, Some(1_700_000_000_000));
    }

    /// The same orchestration, on a wallet that holds its private keys in
    /// process.
    ///
    /// The sibling case above runs the external-signable shape, where the key
    /// comes from a resolved master. This one takes the OTHER branch of the
    /// key-source dispatch: `ResidentWallet` derives from the wallet itself, so
    /// a defect confined to that branch — a wrong wallet, a wrong network, a
    /// derivation that silently disagrees with the master path — would not show
    /// up in the master case at all.
    #[tokio::test]
    async fn a_resident_key_wallet_decrypts_its_own_document_through_the_fetch_path() {
        use crate::wallet::identity::crypto::tx_metadata::{
            derive_tx_metadata_key, seal_tx_metadata, VERSION_PROTOBUF,
        };
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");
        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = contract.id();

        let (wallet, identity_index, key_index) =
            wallet_with_managed_identity(sdk.clone(), TEST_OWNER).await;
        make_wallet_resident(&wallet).await;

        // Seal with the SAME resident wallet and network the reader resolves,
        // so the blob is one this wallet genuinely owns.
        const ENCRYPTION_KEY_INDEX: u32 = 4;
        const PLAINTEXT: &[u8] = b"memo=resident;taxCategory=income";
        let network = wallet.identity().sdk.network;
        let resident = {
            let wm = wallet.identity().wallet_manager.read().await;
            wm.get_wallet(&wallet.identity().wallet_id)
                .expect("the wallet is registered")
                .clone()
        };
        let aes_key = derive_tx_metadata_key(
            &resident,
            network,
            identity_index,
            key_index,
            ENCRYPTION_KEY_INDEX,
        )
        .expect("a resident wallet derives its own txMetadata key in process");
        let iv = [0x7Bu8; 16];
        let blob = seal_tx_metadata(&aes_key, VERSION_PROTOBUF, &iv, PLAINTEXT).expect("seal");

        let id = Identifier::from([0x44u8; 32]);
        let mut page: drive_proof_verifier::types::Documents = Default::default();
        page.insert(
            id,
            Some(encrypted_document_at(
                id,
                1_700_000_000_003,
                key_index,
                ENCRYPTION_KEY_INDEX,
                blob,
            )),
        );

        sdk.mock()
            .expect_fetch(contract_id, Some((*contract).clone()))
            .await
            .expect("register the contract fetch");
        sdk.mock()
            .expect_fetch_many(
                expected_page_query(Arc::clone(&contract), &TEST_OWNER, None),
                Some(page),
            )
            .await
            .expect("register the page");

        let fetched = wallet
            .identity()
            .fetch_encrypted_documents(&TEST_OWNER, &contract_id, "txMetadata", 0)
            .await
            .expect("a resident-key wallet must decrypt its own document");

        assert_eq!(
            fetched.len(),
            1,
            "the resident branch must return the document it can open; a silent \
             skip here would look identical to a bad document"
        );
        let only = &fetched[0];
        assert_eq!(only.document_id, id);
        assert_eq!(
            only.payload.as_slice(),
            PLAINTEXT,
            "the decrypted plaintext must reach the caller intact"
        );
        assert_eq!(only.version, VERSION_PROTOBUF);
        assert_eq!(only.key_index, key_index);
        assert_eq!(only.encryption_key_index, ENCRYPTION_KEY_INDEX);
        assert_eq!(only.updated_at_ms, Some(1_700_000_000_003));
    }

    /// The authoritative seed path, end to end against a mocked Platform.
    ///
    /// This is the path that turns Drive's answer into the first index, and
    /// every part of it can silently go wrong: a missed page under-counts, a
    /// dropped un-materialized entry under-counts, and an off-by-one in the
    /// formula collides with an existing document. All three failures produce a
    /// plausible-looking index, so only counting real pages end to end pins it.
    #[tokio::test]
    async fn a_first_allocation_counts_every_raw_entry_across_pages() {
        use dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        // Pin the protocol version so page two's registered wire encoding
        // matches what the loop sends after the first response.
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");

        let contract = Arc::new(
            dpp::tests::fixtures::get_data_contract_fixture(None, 0, dpp::version::LATEST_VERSION)
                .data_contract_owned(),
        );
        let contract_id = contract.id();

        // A full first page whose final entry is un-materialized — the shape a
        // proved fetch returns for a document it could not produce, which still
        // denotes an existing document and must still be counted.
        const SHARED_TIMESTAMP: u64 = 1_700_000_000_000;
        let page_one_ids: Vec<Identifier> = (0..PAGE_SIZE)
            .map(|i| Identifier::from([(200 - i) as u8; 32]))
            .collect();
        let mut page_one: drive_proof_verifier::types::Documents = Default::default();
        for (position, id) in page_one_ids.iter().enumerate() {
            let is_final = position == PAGE_SIZE - 1;
            page_one.insert(
                *id,
                if is_final {
                    None
                } else {
                    Some(document_at(*id, SHARED_TIMESTAMP))
                },
            );
        }
        let final_page_one_key = *page_one_ids.last().expect("page one is not empty");

        let page_two_ids: Vec<Identifier> = (0..3)
            .map(|i| Identifier::from([(50 - i) as u8; 32]))
            .collect();
        let mut page_two: drive_proof_verifier::types::Documents = Default::default();
        for id in &page_two_ids {
            page_two.insert(*id, Some(document_at(*id, SHARED_TIMESTAMP + 1)));
        }
        let expected_raw_count = PAGE_SIZE + page_two_ids.len();

        sdk.mock()
            .expect_fetch(contract_id, Some((*contract).clone()))
            .await
            .expect("register the contract fetch");
        sdk.mock()
            .expect_fetch_many(
                expected_page_query(Arc::clone(&contract), &TEST_OWNER, None),
                Some(page_one),
            )
            .await
            .expect("register page one");
        sdk.mock()
            .expect_fetch_many(
                expected_page_query(
                    Arc::clone(&contract),
                    &TEST_OWNER,
                    Some(Start::StartAfter(final_page_one_key.to_buffer().to_vec())),
                ),
                Some(page_two),
            )
            .await
            .expect("register page two");

        let manager = Arc::new(crate::PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoopPersister),
            Arc::new(NoopEventHandler) as Arc<dyn crate::PlatformEventHandler>,
        ));
        let wallet = {
            use key_wallet::mnemonic::{Language, Mnemonic};
            let seed = Mnemonic::from_entropy(&[0u8; 16], Language::English)
                .expect("16 bytes of entropy")
                .to_seed("");
            manager
                .create_wallet_from_seed_bytes(
                    key_wallet::Network::Testnet,
                    &seed,
                    WalletAccountCreationOptions::None,
                    Some(0),
                )
                .await
                .expect("wallet creation on a mock sdk")
        };

        let first = wallet
            .identity()
            .allocate_encryption_key_index(&TEST_OWNER, &contract_id, "txMetadata", 0)
            .await
            .expect("the first allocation counts the owner's documents");
        assert_eq!(
            first,
            expected_raw_count as u32 + 1,
            "the first index must be 1 + every raw entry Drive returned, across \
             both pages and including the un-materialized one"
        );

        // The second allocation continues in process. A re-seed would count the
        // same documents again and hand out the same index twice.
        let second = wallet
            .identity()
            .allocate_encryption_key_index(&TEST_OWNER, &contract_id, "txMetadata", 0)
            .await
            .expect("the second allocation continues from the high-water");
        assert_eq!(
            second,
            expected_raw_count as u32 + 2,
            "a seeded scope must continue in process rather than re-count"
        );
    }

    // ── The blocking entry points never panic, wherever they are called ─────
    //
    // `blocking_read` is how the `*_blocking` APIs keep key material off every
    // await, but Tokio answers it with a PANIC on a thread that is driving
    // tasks — and this workspace builds with `panic = "abort"`, so that panic is
    // immediate process death, not a catchable misuse. A public API that a Rust
    // consumer can reach from an ordinary async task must therefore fail as a
    // value. These cases pin both halves of that: the uncontended call proceeds,
    // and the contended one reports a typed error instead of parking.

    /// A resident wallet whose managed identity carries the ENCRYPTION key,
    /// ready for a `prepare_*` call.
    async fn resident_wallet_for_preparation() -> std::sync::Arc<crate::PlatformWallet> {
        let sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk builds");
        let (wallet, _identity_index, _key_index) =
            wallet_with_managed_identity(sdk, TEST_OWNER).await;
        make_wallet_resident(&wallet).await;
        wallet
    }

    /// Called from inside a Tokio task, the blocking preparation must return —
    /// not abort the process.
    ///
    /// Before the fix this test did not fail, it DIED: `blocking_read` on a
    /// runtime worker panics, and `panic = "abort"` takes the test binary with
    /// it. Nothing is contended here, so the call must also actually succeed.
    #[tokio::test]
    async fn blocking_preparation_inside_an_async_task_completes_instead_of_panicking() {
        use crate::wallet::identity::crypto::tx_metadata::VERSION_PROTOBUF;

        let wallet = resident_wallet_for_preparation().await;

        let prepared = wallet
            .identity()
            .prepare_txmetadata_encryption_blocking(
                &TEST_OWNER,
                1,
                VERSION_PROTOBUF,
                16,
                TxMetadataKeySource::ResidentWallet,
            )
            .expect(
                "an uncontended blocking preparation must complete from an async task, \
                 not panic on the runtime worker",
            );

        assert_eq!(prepared.encryption_key_index, 1);
        assert_eq!(prepared.version, VERSION_PROTOBUF);
        assert_eq!(prepared.payload_len, 16);
    }

    /// Same call, with the wallet-manager lock genuinely held by a writer.
    ///
    /// This is the case that would otherwise have to park: it must neither park
    /// (deadlocking this single task against its own guard) nor panic, but
    /// report the typed retryable error naming the entry point.
    #[tokio::test]
    async fn a_contended_blocking_preparation_reports_a_busy_wallet_manager() {
        use crate::wallet::identity::crypto::tx_metadata::VERSION_PROTOBUF;

        let wallet = resident_wallet_for_preparation().await;
        let identity_wallet = wallet.identity();

        // Held across the blocking call below, so `try_read` cannot succeed.
        let _writer = identity_wallet.wallet_manager.write().await;

        let error = identity_wallet
            .prepare_txmetadata_encryption_blocking(
                &TEST_OWNER,
                1,
                VERSION_PROTOBUF,
                16,
                TxMetadataKeySource::ResidentWallet,
            )
            .expect_err("the wallet-manager lock is write-held for the whole call");

        assert!(
            matches!(
                error,
                PlatformWalletError::WalletManagerBusy {
                    api: "IdentityWallet::prepare_txmetadata_encryption_blocking"
                }
            ),
            "a contended blocking preparation must surface the typed busy error \
             naming its own entry point; got {error:?}"
        );
    }

    /// The decrypt half takes the same bridge, so it must behave the same way.
    ///
    /// A non-empty input is required: the empty-input short circuit returns
    /// before any lock is taken and would pass no matter what the bridge did.
    #[tokio::test]
    async fn a_contended_blocking_decrypt_reports_a_busy_wallet_manager() {
        let wallet = resident_wallet_for_preparation().await;
        let identity_wallet = wallet.identity();

        let id = Identifier::from([0x5Au8; 32]);
        let raw = vec![(id, Some(document_at(id, 1_700_000_000_000)))];

        let _writer = identity_wallet.wallet_manager.write().await;

        let error = identity_wallet
            .decrypt_fetched_documents_blocking(
                &TEST_OWNER,
                &raw,
                TxMetadataKeySource::ResidentWallet,
            )
            .expect_err("the wallet-manager lock is write-held for the whole call");

        assert!(
            matches!(
                error,
                PlatformWalletError::WalletManagerBusy {
                    api: "IdentityWallet::decrypt_fetched_documents_blocking"
                }
            ),
            "a contended blocking decrypt must surface the typed busy error naming \
             its own entry point; got {error:?}"
        );
    }

    /// The async counterparts are the deterministic path for async callers:
    /// they WAIT for the lock instead of reporting contention.
    ///
    /// Proven by holding the write guard until after the async preparation is
    /// already pending — a call that reported busy, or that resolved before the
    /// guard dropped, would not satisfy this. Time is paused, so the timeout
    /// that observes "still waiting" costs no real wall clock.
    #[tokio::test(start_paused = true)]
    async fn the_async_preparation_waits_for_a_contended_wallet_manager() {
        use crate::wallet::identity::crypto::tx_metadata::VERSION_PROTOBUF;

        let wallet = resident_wallet_for_preparation().await;
        let identity_wallet = wallet.identity();

        let writer = identity_wallet.wallet_manager.write().await;

        let mut pending = Box::pin(identity_wallet.prepare_txmetadata_encryption(
            &TEST_OWNER,
            1,
            VERSION_PROTOBUF,
            16,
            TxMetadataKeySource::ResidentWallet,
        ));
        // Polled while the writer still holds the lock: it must be waiting, not
        // failing.
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), &mut pending)
                .await
                .is_err(),
            "the async preparation must wait for the write guard rather than \
             reporting contention"
        );

        drop(writer);
        let prepared = pending
            .await
            .expect("the async preparation completes once the writer releases");
        assert_eq!(prepared.encryption_key_index, 1);
    }
}
