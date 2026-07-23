//! Encrypted `txMetadata` document create + decrypt-on-fetch on
//! `IdentityWallet`.
//!
//! Implements the wallet-contract encrypted-document surface the Android
//! wallet needs to retire the legacy `org.dashj.platform` stack
//! (dashpay/platform#4086 create, #4087 decrypt-on-fetch;
//! dashpay/dash-wallet#1507). The encryption ENVELOPE — key derivation, the
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
    open_tx_metadata, seal_tx_metadata,
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
/// These per-poll stage lines carry identity / contract / document ids, so now
/// that the `sdkFetched=0` root cause is fixed (external-signable txMetadata
/// derive, dashpay/platform#4091) they are deliberately DEBUG — they must NOT
/// persist identity-correlated data to logcat on every successful fetch. Genuine
/// failures use [`breadcrumb_error`] (WARN) so they stay visible on-device.
fn breadcrumb(line: &str) {
    tracing::debug!("{line}");
    log::debug!("{line}");
}

/// Emit a FAILURE breadcrumb through both logging facades at **WARN** level, so
/// a genuine error or skip stays visible in Android logcat (`android_logger`
/// Info+). Use ONLY for actual failure / skip paths — never per-poll
/// informational stages, which belong on [`breadcrumb`] (DEBUG) to keep
/// identity-correlated data out of the device log.
fn breadcrumb_error(line: &str) {
    tracing::warn!("{line}");
    log::warn!("{line}");
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
    /// (dashpay/platform#4091). Call from a sync context only. The subsequent
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

        // Reject an over-large payload BEFORE any key derivation or network
        // work: a plaintext that cannot fit the encryptedMetadata field once
        // sealed would otherwise derive the key, seal, and only then die at
        // broadcast with an opaque DPP schema error. Fail fast with a typed
        // error instead (dashpay/platform#4091).
        ensure_tx_metadata_payload_fits(payload.len())?;

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
                     key_source={} owner={owner_identity_id} error={e}",
                    key_source.label()
                ));
            })?;
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);
        // Rejects a non-wire-decodable version byte (only 0/1) before it can be
        // sealed into a document the legacy stack can't decode, and enforces the
        // payload-size limit as the choke-point last line of defense
        // (dashpay/platform#4091).
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

        // On-device diagnostic breadcrumbs, dual-emitted at warn level (see
        // [`breadcrumb`]): this call sits under an active `sdkFetched=0`
        // investigation — every stage must be provably visible in `adb logcat`.
        breadcrumb(&format!(
            "fetch_encrypted_documents: entry owner={owner_identity_id} \
             contract={contract_id} type={document_type_name} since_ms={since_ms} \
             key_source={}",
            key_source.label()
        ));

        // Fetch the contract and register it so `fetch_many`'s proof
        // verification can resolve it through the context provider (the mobile
        // provider never fetches contracts itself).
        let contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(|e| {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: contract fetch failed contract={contract_id} error={e}"
                ));
                PlatformWalletError::Sdk(e)
            })?
            .ok_or_else(|| {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: contract not found on Platform contract={contract_id}"
                ));
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
                     owner={owner_identity_id} error={e}"
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
                "fetch_encrypted_documents: document query failed owner={owner_identity_id} error={e}"
            ));
        })?;

        let mut out = Vec::new();
        for (doc_id, maybe_doc) in raw_docs.iter() {
            let Some(doc) = maybe_doc else {
                // A raw entry the SDK could not materialize (e.g. a proved
                // fetch returning an id without a document). Previously a
                // SILENT skip — under proofs this is exactly the shape that
                // turns "2 documents exist" into an empty result with no
                // error, so it must leave a trail.
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: raw entry NOT materialized doc={doc_id} \
                     owner={owner_identity_id}; skipping"
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
                    "fetch_encrypted_documents: document missing key indices doc={doc_id} \
                     owner={owner_identity_id}; skipping"
                ));
                continue;
            };
            let Some(blob) = props
                .get(FIELD_ENCRYPTED_METADATA)
                .and_then(|v: &Value| v.to_binary_bytes().ok())
            else {
                breadcrumb_error(&format!(
                    "fetch_encrypted_documents: document missing encryptedMetadata doc={doc_id} \
                     owner={owner_identity_id}; skipping"
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
                        "fetch_encrypted_documents: txMetadata key derivation failed doc={doc_id} \
                         owner={owner_identity_id} key_source={} error={e}; skipping",
                        key_source.label()
                    ));
                    continue;
                }
            };
            let opened = match open_tx_metadata(&aes_key, &blob) {
                Ok(o) => o,
                Err(e) => {
                    breadcrumb_error(&format!(
                        "fetch_encrypted_documents: txMetadata decrypt failed doc={doc_id} \
                         owner={owner_identity_id} error={e}; skipping"
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
            "fetch_encrypted_documents: returning decrypted documents owner={owner_identity_id} \
             raw={} decrypted={}",
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
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::platform_value::platform_value;

    const PAGE: u32 = 100;
    breadcrumb(&format!(
        "query_owned_encrypted_documents: entry owner={owner_identity_id} contract={} \
         type={document_type_name} since_ms={since_ms}",
        contract.id()
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
            breadcrumb_error(&format!(
                "query_owned_encrypted_documents: fetch_many failed owner={owner_identity_id} \
                 type={document_type_name} error={e}"
            ));
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

    // On-device diagnostic breadcrumb: the probe reported `sdkFetched=0` with
    // ZERO decrypt-skip warnings, which can only mean the query itself returned
    // nothing OR nothing materialized. Log the raw count (BEFORE decrypt) so an
    // `adb logcat` run pins the empty result to the query vs the
    // materialization vs the decrypt stage without guessing.
    breadcrumb(&format!(
        "query_owned_encrypted_documents: fetched raw encrypted documents \
         owner={owner_identity_id} type={document_type_name} since_ms={since_ms} \
         raw_count={} materialized={}",
        raw_docs.len(),
        raw_docs.iter().filter(|(_, d)| d.is_some()).count()
    ));
    Ok(raw_docs)
}
