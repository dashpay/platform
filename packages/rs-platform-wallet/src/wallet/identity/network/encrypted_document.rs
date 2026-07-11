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
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};

use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::tx_metadata::{
    derive_tx_metadata_key, open_tx_metadata, seal_tx_metadata,
};

use super::*;

/// Wallet-contract document field names (wire-compatible with the legacy
/// `TxMetadataDocument` schema — `wallet-utils-contract` `tx_metadata`).
const FIELD_KEY_INDEX: &str = "keyIndex";
const FIELD_ENCRYPTION_KEY_INDEX: &str = "encryptionKeyIndex";
const FIELD_ENCRYPTED_METADATA: &str = "encryptedMetadata";

/// One decrypted encrypted-document, returned to the caller (serialized to
/// JSON at the FFI boundary). The `payload` is the opaque, decrypted plaintext
/// the app parses itself (a protobuf `TxMetadataBatch` for `version == 1`).
#[derive(Debug, Clone)]
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
    ) -> Result<(dpp::identity::Identity, u32, key_wallet::wallet::Wallet), PlatformWalletError> {
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

    /// Create + broadcast an ENCRYPTED `txMetadata`-style document on
    /// `contract_id`'s `document_type_name`, owned by `owner_identity_id`.
    ///
    /// The SDK derives the identity encryption key, seals `payload` into the
    /// wire-compatible `version ‖ IV ‖ AES-256-CBC` blob, and writes the
    /// `{keyIndex, encryptionKeyIndex, encryptedMetadata}` document — the exact
    /// shape the legacy `publishTxMetaData` wrote, so the legacy stack decrypts
    /// it and vice versa.
    ///
    /// The caller supplies:
    /// - `encryption_key_index`: the per-document index (dash-wallet's
    ///   monotonic `1 + countAllRequests()` counter). Batching stays app-side.
    /// - `version`: the payload version byte (`1` = protobuf, as the wallet
    ///   writes).
    /// - `payload`: the already-serialized opaque plaintext (a protobuf
    ///   `TxMetadataBatch`) — the SDK does not parse it.
    ///
    /// The `keyIndex` field (the identity encryption key id) is selected
    /// SDK-side to match the legacy stack. Returns the confirmed `Document`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_encrypted_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        encryption_key_index: u32,
        version: u8,
        payload: &[u8],
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        use dashcore::secp256k1::rand::{thread_rng, RngCore};

        let (identity, identity_index, wallet) =
            self.resolve_encryption_context(owner_identity_id).await?;
        let key_index = Self::select_encryption_key_id(&identity)?;

        // Derive the AES key and seal the payload into the wire blob.
        let aes_key = derive_tx_metadata_key(
            &wallet,
            self.sdk.network,
            identity_index,
            key_index,
            encryption_key_index,
        )?;
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);
        let blob = seal_tx_metadata(&aes_key, version, &iv, payload);

        // Reuse the generic create path: it fetches the contract, sanitizes the
        // hex `encryptedMetadata` into `Bytes` against the schema, auto-selects
        // the AUTHENTICATION signing key, and broadcasts on the 8 MB worker
        // stack. Byte-array fields are accepted as hex strings there.
        let properties_json = serde_json::json!({
            FIELD_KEY_INDEX: key_index,
            FIELD_ENCRYPTION_KEY_INDEX: encryption_key_index,
            FIELD_ENCRYPTED_METADATA: hex::encode(&blob),
        })
        .to_string();

        self.create_document_with_signer(
            owner_identity_id,
            contract_id,
            document_type_name,
            &properties_json,
            signer,
        )
        .await
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
    ) -> Result<Vec<DecryptedEncryptedDocument>, PlatformWalletError> {
        use dash_sdk::platform::{ContextProvider, Fetch};

        // Fetch the contract and register it so `fetch_many`'s proof
        // verification can resolve it through the context provider (the mobile
        // provider never fetches contracts itself).
        let contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(PlatformWalletError::Sdk)?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot fetch documents"
                ))
            })?;
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::new(contract.clone()));
        }
        let contract = Arc::new(contract);

        let (_identity, identity_index, wallet) =
            self.resolve_encryption_context(owner_identity_id).await?;

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
        .await?;

        let mut out = Vec::new();
        for (doc_id, maybe_doc) in raw_docs.iter() {
            let Some(doc) = maybe_doc else { continue };
            let props = doc.properties();
            let (Some(key_index), Some(encryption_key_index)) = (
                props
                    .get(FIELD_KEY_INDEX)
                    .and_then(|v: &Value| v.to_integer::<u32>().ok()),
                props
                    .get(FIELD_ENCRYPTION_KEY_INDEX)
                    .and_then(|v: &Value| v.to_integer::<u32>().ok()),
            ) else {
                tracing::warn!(owner = %owner_identity_id, doc = %doc_id, "encrypted document missing key indices; skipping");
                continue;
            };
            let Some(blob) = props
                .get(FIELD_ENCRYPTED_METADATA)
                .and_then(|v: &Value| v.to_binary_bytes().ok())
            else {
                tracing::warn!(owner = %owner_identity_id, doc = %doc_id, "encrypted document missing encryptedMetadata; skipping");
                continue;
            };

            let aes_key = match derive_tx_metadata_key(
                &wallet,
                self.sdk.network,
                identity_index,
                key_index,
                encryption_key_index,
            ) {
                Ok(k) => k,
                Err(e) => {
                    tracing::warn!(owner = %owner_identity_id, doc = %doc_id, error = %e, "txMetadata key derivation failed; skipping");
                    continue;
                }
            };
            let opened = match open_tx_metadata(&aes_key, &blob) {
                Ok(o) => o,
                Err(e) => {
                    tracing::warn!(owner = %owner_identity_id, doc = %doc_id, error = %e, "txMetadata decrypt failed; skipping");
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

        let page = Document::fetch_many(sdk, query)
            .await
            .map_err(PlatformWalletError::Sdk)?;
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
    // nothing. Log the raw count (BEFORE decrypt) so an `adb logcat` run pins
    // the empty result to the query vs the decrypt stage without guessing.
    tracing::info!(
        owner = %owner_identity_id,
        document_type = document_type_name,
        since_ms,
        raw_count = raw_docs.len(),
        materialized = raw_docs.iter().filter(|(_, d)| d.is_some()).count(),
        "fetched raw encrypted documents"
    );
    Ok(raw_docs)
}
