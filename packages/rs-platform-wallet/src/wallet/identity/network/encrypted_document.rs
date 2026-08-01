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

use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::tx_metadata::{
    derive_tx_metadata_key, derive_tx_metadata_key_from_master, ensure_tx_metadata_payload_fits,
    ensure_tx_metadata_version_supported, open_tx_metadata, seal_tx_metadata,
};

use super::*;

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
///   with `External signable wallet has no private key` (the exact on-device
///   failure that zeroed the decrypt-proof). For that shape the FFI resolves
///   the wallet's mnemonic via the host `MnemonicResolverHandle`, builds the
///   master xprv, passes [`TxMetadataKeySource::Master`], and wipes the
///   master after the call — atomic derive + use + zeroize.
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
    pub payload: Vec<u8>,
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

    /// Resolve `(identity, identity_index, wallet)` for `owner_identity_id`
    /// from the in-process wallet manager — the inputs the tx-metadata key
    /// derivation needs. Errors for a watch-only / out-of-wallet identity (no
    /// resident HD slot); the dash-wallet migration uses a resident mnemonic
    /// wallet.
    async fn resolve_encryption_context(
        &self,
        owner_identity_id: &Identifier,
    ) -> Result<(dpp::identity::Identity, u32, key_wallet::wallet::Wallet), PlatformWalletError>
    {
        let wm = self.wallet_manager.read().await;
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
        Ok((identity, identity_index, wallet))
    }

    /// Synchronous (`blocking_read`) counterpart of
    /// [`Self::resolve_encryption_context`], resolving
    /// `(identity, identity_index, wallet)` without crossing an `.await`. MUST
    /// be called from a sync context — never inside an async task (`blocking_read`
    /// panics there). Used by [`Self::prepare_encrypted_txmetadata_properties`]
    /// so the master xprv can be wiped BEFORE any network round-trip.
    fn resolve_encryption_context_blocking(
        &self,
        owner_identity_id: &Identifier,
    ) -> Result<(dpp::identity::Identity, u32, key_wallet::wallet::Wallet), PlatformWalletError>
    {
        let wm = self.wallet_manager.blocking_read();
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
        Ok((identity, identity_index, wallet))
    }

    /// Synchronously derive the identity encryption key and seal `payload` into
    /// the wire-compatible `version ‖ IV ‖ AES-256-CBC` blob, returning the
    /// `{keyIndex, encryptionKeyIndex, encryptedMetadata}` properties JSON ready
    /// for [`Self::create_document_with_signer`] — the exact document shape the
    /// legacy `publishTxMetaData` wrote, so the legacy stack decrypts it.
    ///
    /// **Crosses no `.await`** (resolves via `blocking_read`, derives, seals) so
    /// the FFI caller can WIPE the resolved master xprv before the network
    /// broadcast: the master never lives across an await
    /// Call from a sync context only. The subsequent
    /// generic [`Self::create_document_with_signer`] then broadcasts the returned
    /// properties with no key material in scope.
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
    pub fn prepare_encrypted_txmetadata_properties(
        &self,
        owner_identity_id: &Identifier,
        encryption_key_index: u32,
        version: u8,
        payload: &[u8],
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<String, PlatformWalletError> {
        use dashcore::secp256k1::rand::{thread_rng, RngCore};

        // Both conditions are decidable from the arguments alone, so they are
        // rejected before this call resolves an encryption context, selects a
        // key, derives AES material or draws an IV. A payload that cannot fit
        // the encryptedMetadata field, or a version the legacy stack cannot
        // decode, would otherwise do all of that work and only then fail — the
        // size case at broadcast with an opaque schema error, the version case
        // at the sealing choke point.
        ensure_tx_metadata_payload_fits(payload.len())?;
        ensure_tx_metadata_version_supported(version)?;

        let (identity, identity_index, wallet) =
            self.resolve_encryption_context_blocking(owner_identity_id)?;
        let key_index = Self::select_encryption_key_id(&identity)?;

        // Derive the AES key and seal the payload into the wire blob — the only
        // step that touches `key_source`'s master, done here synchronously so the
        // caller can wipe it before broadcasting.
        let aes_key = key_source
            .derive(
                &wallet,
                self.sdk.network,
                identity_index,
                key_index,
                encryption_key_index,
            )
            .inspect_err(|e| {
                breadcrumb_error(&format!(
                    "prepare_encrypted_txmetadata: key derivation failed \
                     key_source={} error_kind={}",
                    key_source.label(),
                    error_kind(e)
                ));
            })?;
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);
        // Re-checks the wire version and the payload size as the choke-point
        // last line of defense, so nothing can seal a document the legacy stack
        // cannot decode or one that overflows the field.
        let blob = seal_tx_metadata(&aes_key, version, &iv, payload)?;

        // Byte-array fields are accepted as hex strings by the generic create
        // path, which sanitizes them into `Bytes` against the schema.
        Ok(serde_json::json!({
            FIELD_KEY_INDEX: key_index,
            FIELD_ENCRYPTION_KEY_INDEX: encryption_key_index,
            FIELD_ENCRYPTED_METADATA: hex::encode(&blob),
        })
        .to_string())
    }

    /// Fetch every encrypted `txMetadata`-style document owned by
    /// `owner_identity_id` on `contract_id`'s `document_type_name` updated at or
    /// after `since_ms`, and DECRYPT each with the identity's derived key.
    ///
    /// Mirrors the legacy `getTxMetaData(sinceTime, key)`: the query is
    /// `$ownerId == owner AND $updatedAt >= since_ms` ordered by `$updatedAt`
    /// ascending, paginated so a wallet with many documents isn't truncated. A
    /// document whose key can't be derived or whose blob doesn't decrypt is
    /// SKIPPED with a warning (a malformed document must not abort the sync),
    /// matching the resident `contactInfo` sweep.
    pub async fn fetch_encrypted_documents(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        since_ms: u64,
        key_source: TxMetadataKeySource<'_>,
    ) -> Result<Vec<DecryptedEncryptedDocument>, PlatformWalletError> {
        use dash_sdk::platform::{ContextProvider, Fetch};

        // Stage breadcrumbs for this fetch. An empty result on this path is
        // indistinguishable from a failure without them: the query can return
        // nothing, a document can fail to materialize, or a decrypt can be
        // skipped, and each stage below records which one happened.
        breadcrumb(&format!(
            "fetch_encrypted_documents: entry since_ms={since_ms} key_source={}",
            key_source.label()
        ));

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

        let (_identity, identity_index, wallet) = self
            .resolve_encryption_context(owner_identity_id)
            .await
            .inspect_err(|e| {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: encryption-context resolution failed \
                     error_kind={}",
                    error_kind(e)
                ));
            })?;

        // The wire query, split out so its exact shape is integration-testable
        // against testnet without a resident wallet/identity (see
        // `tests/txmetadata_fetch.rs`).
        let raw_docs = query_owned_encrypted_documents(
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
        })?;

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
                &wallet,
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
        Ok(out)
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
    breadcrumb(&format!(
        "query_owned_encrypted_documents: entry since_ms={since_ms}"
    ));
    let mut raw_docs: Vec<(Identifier, Option<Document>)> = Vec::new();
    let mut start: Option<Start> = None;
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
            start: start.clone(),
        };

        let page = Document::fetch_many(sdk, query).await.map_err(|e| {
            breadcrumb_error("query_owned_encrypted_documents: fetch_many failed error_kind=sdk");
            PlatformWalletError::Sdk(e)
        })?;
        let page_len = page.len();
        let last_id = page.keys().last().copied();
        raw_docs.extend(page);

        if page_len < PAGE as usize {
            break;
        }
        match last_id {
            Some(id) => start = Some(Start::StartAfter(id.to_buffer().to_vec())),
            None => break,
        }
    }

    // Both counts are recorded BEFORE any decrypt, so an empty end result can be
    // attributed to the query returning nothing, to documents the SDK could not
    // materialize, or to the decrypt stage that runs after this — three causes
    // that are otherwise indistinguishable from one another.
    breadcrumb(&format!(
        "query_owned_encrypted_documents: fetched raw encrypted documents \
         since_ms={since_ms} raw_count={} materialized={}",
        raw_docs.len(),
        raw_docs.iter().filter(|(_, d)| d.is_some()).count()
    ));
    Ok(raw_docs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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

    /// The owner and contract identifiers the breadcrumbs are given.
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

    // ── Version gate ordering ───────────────────────────────────────────────

    use crate::changeset::{PersistenceError, PlatformWalletPersistence};
    use crate::wallet::WalletId;
    use crate::ClientStartState;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

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

    /// An unsupported wire version is decidable from the argument alone, so it
    /// must be rejected before this call resolves the encryption context,
    /// selects a key, derives AES material or draws an IV.
    ///
    /// The wallet here carries no managed identity, so any path that reaches
    /// context resolution fails with an identity error instead — which is what
    /// makes the ordering observable without a network or a live host.
    #[test]
    fn prepare_rejects_an_unsupported_version_before_context_or_key_work() {
        use key_wallet::mnemonic::{Language, Mnemonic};

        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let seed = Mnemonic::from_entropy(&[0u8; 16], Language::English)
            .expect("16 bytes of entropy")
            .to_seed("");

        let wallet = runtime.block_on(async {
            let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
            let manager = Arc::new(crate::PlatformWalletManager::new(
                sdk,
                Arc::new(NoopPersister),
                Arc::new(NoopEventHandler) as Arc<dyn crate::PlatformEventHandler>,
            ));
            manager
                .create_wallet_from_seed_bytes(
                    key_wallet::Network::Testnet,
                    &seed,
                    WalletAccountCreationOptions::None,
                    Some(0),
                )
                .await
                .expect("wallet creation on a mock sdk")
        });

        // Called outside the runtime: this path resolves its context with a
        // blocking read and must not run inside an async context.
        let error = wallet
            .identity()
            .prepare_encrypted_txmetadata_properties(
                &TEST_OWNER,
                0,
                2,
                b"opaque",
                TxMetadataKeySource::ResidentWallet,
            )
            .expect_err("version 2 is not wire-decodable");

        assert!(
            matches!(
                error,
                PlatformWalletError::UnsupportedTxMetadataVersion { version: 2 }
            ),
            "an unsupported version must be rejected before the encryption context \
             is resolved; got {error:?}"
        );
    }

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
}
