//! Document create operations on `IdentityWallet`.
//!
//! Lives on `IdentityWallet` (rather than in `rs-sdk-ffi`) for the
//! same reason as `contract.rs`: creating + broadcasting a document is
//! a wallet-level operation. It spans an identity (the owner), needs
//! the wallet's external signer, and broadcasts a document state
//! transition whose signature key is selected from the in-memory
//! wallet manager. Per `swift-sdk/CLAUDE.md`, "anything that spans
//! identities / platform balances / ... belongs in the
//! `platform-wallet` crate"; the Swift side only renders the form,
//! marshals the values, and persists the confirmed document.
//!
//! Mirrors the post-#3541 identity-flow shape:
//!   - The library function takes a `Signer<IdentityPublicKey>`
//!     reference so the FFI's external `KeychainSigner` trampoline can
//!     route signing back to Swift / Keychain without crossing seed
//!     bytes.
//!   - The document content arrives as a properties JSON string and is
//!     turned into a platform `Value` map, then a revision-1 `Document`
//!     via `DocumentType::create_document_from_data` — the same path
//!     `rs-sdk-ffi/src/document/create.rs` uses.
//!   - Broadcast goes through
//!     `dash_sdk::platform::transition::put_document::PutDocument::put_to_platform_and_wait_for_response`
//!     on the platform-wallet runtime (8 MB worker stack) instead of
//!     the rs-sdk-ffi runtime (mobile-tuned default stack) — the same
//!     stack-overflow avoidance `contract.rs` documents for the
//!     post-broadcast GroveDB proof-verification recursion.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::Document;
use dpp::document::DocumentV0Setters;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{DataContract, Identifier};
use dpp::ProtocolError;

use dash_sdk::platform::documents::transitions::{
    DocumentDeleteResult, DocumentDeleteTransitionBuilder, DocumentPurchaseResult,
    DocumentPurchaseTransitionBuilder, DocumentReplaceResult, DocumentReplaceTransitionBuilder,
    DocumentSetPriceResult, DocumentSetPriceTransitionBuilder, DocumentTransferResult,
    DocumentTransferTransitionBuilder,
};
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::{ContextProvider, DocumentQuery, Fetch};

use crate::error::PlatformWalletError;

use super::*;

/// Borrowed-signer adapter — same shape as the local `SignerRef` in
/// `contract.rs` / `dpns.rs` / `transfer.rs`. Lets the
/// `Signer<IdentityPublicKey>` trait bound on the SDK's `PutDocument`
/// extension be satisfied with a `&S` instead of forcing the caller to
/// hand over ownership / wrap in an `Arc` per call.
struct SignerRef<'a, S: ?Sized>(&'a S);

impl<'a, S: ?Sized> std::fmt::Debug for SignerRef<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerRef")
    }
}

#[async_trait]
impl<'a, K, S> Signer<K> for SignerRef<'a, S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.0.sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

/// Build the set of `SecurityLevel`s an AUTHENTICATION key may carry to
/// satisfy a document state-transition whose document type requires
/// `requirement`.
///
/// This reproduces the consensus rule the network enforces in
/// `BatchTransition::combined_security_level_requirement`
/// (`rs-dpp/.../batch_transition/methods/v0/mod.rs`): the signing key's
/// security level must be **stronger-or-equal** to the document type's
/// requirement, expressed as the inclusive range `CRITICAL..=requirement`
/// over the `MASTER(0) < CRITICAL(1) < HIGH(2) < MEDIUM(3)` ordering —
/// with `MASTER` handled as its own degenerate `[MASTER]` set. MASTER is
/// otherwise excluded: it is reserved for identity-self-modification and
/// the document-batch purpose requirement (`vec![AUTHENTICATION]`) never
/// admits it for an ordinary document create.
///
/// Picking the key against this exact set (rather than a hardcoded
/// CRITICAL, which the contract-create path uses) is what makes the
/// flow correct for *any* document type — e.g. DPNS `preorder` requires
/// `HIGH`, so both `CRITICAL` and `HIGH` keys qualify, but `MEDIUM` does
/// not.
fn allowed_signing_security_levels(requirement: SecurityLevel) -> Vec<SecurityLevel> {
    if requirement == SecurityLevel::MASTER {
        return vec![SecurityLevel::MASTER];
    }
    // `CRITICAL as u8 == 1`; iterate down to (and including) the
    // requirement. `SecurityLevel::try_from` only fails for values
    // outside 0..=3, and every value in this range is valid by
    // construction.
    (SecurityLevel::CRITICAL as u8..=requirement as u8)
        .filter_map(|level| SecurityLevel::try_from(level).ok())
        .collect()
}

impl IdentityWallet {
    /// Register a freshly-fetched data contract into the SDK's shared
    /// context provider so the returned-proof verification after a document
    /// state-transition broadcast can resolve it. Without this, the mobile
    /// `TrustedHttpContextProvider` — which never fetches contracts itself —
    /// returns `None` for the contract and proof verification fails with
    /// "unknown contract ... in document verification", even though the
    /// write landed on-chain.
    fn register_contract_for_proof_verification(&self, contract: &DataContract) {
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::new(contract.clone()));
        }
    }

    /// Create a new revision-1 document on `contract_id`'s
    /// `document_type_name` owned by `owner_identity_id`, and broadcast
    /// it to Platform.
    ///
    /// The function:
    ///   1. Fetches the on-chain `DataContract` for `contract_id` via
    ///      `self.sdk` and resolves the owned `DocumentType` for
    ///      `document_type_name`.
    ///   2. Parses `properties_json` into a platform `Value` map,
    ///      sanitizes it against the schema (hex/base64 byte arrays,
    ///      base58/hex identifiers — same as `rs-sdk-ffi`'s document
    ///      create), and builds a revision-1 `Document` via
    ///      `DocumentType::create_document_from_data`. Entropy is
    ///      generated by the SDK at broadcast time (we pass `None`), so
    ///      the canonical document id is derived from
    ///      `(contract, owner, type, entropy)` there — the placeholder
    ///      id this build step assigns is overwritten.
    ///   3. Selects the signing `IdentityPublicKey` from the in-memory
    ///      wallet manager: an AUTHENTICATION-purpose, ECDSA_SECP256K1
    ///      key whose security level satisfies the document type's
    ///      `security_level_requirement()` (see
    ///      [`allowed_signing_security_levels`]). Returns a clear error
    ///      if the owner identity isn't loaded or carries no qualifying
    ///      key.
    ///   4. Broadcasts via
    ///      `Document::put_to_platform_and_wait_for_response` on the
    ///      platform-wallet 8 MB-stack worker and returns the confirmed
    ///      `Document` from Platform.
    ///
    /// `properties_json` is a JSON object keyed by property name. Byte-
    /// array fields are supplied as hex (or base64) strings and
    /// identifier fields as base58 (or hex) strings; the schema-driven
    /// sanitize step converts them to the protocol's native `Bytes` /
    /// `Identifier` values. An empty object (`"{}"`) is valid for a
    /// document type with no required properties.
    pub async fn create_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        properties_json: &str,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let platform_version = self.sdk.version();

        // 1. Fetch the on-chain contract + resolve the document type.
        let data_contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contract {contract_id} for document create: {e}"
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot create document"
                ))
            })?;

        // Make the contract resolvable for the post-broadcast proof check.
        self.register_contract_for_proof_verification(&data_contract);

        // Owned `DocumentType` — `put_to_platform_and_wait_for_response`
        // takes the document type by value.
        let document_type = data_contract
            .document_type_cloned_for_name(document_type_name)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document type {document_type_name:?} not found on contract {contract_id}: {e}"
                ))
            })?;

        // 2. Parse properties JSON -> platform Value map, sanitize
        //    against the schema, and build a revision-1 document.
        let properties_value: serde_json::Value =
            serde_json::from_str(properties_json).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Invalid document properties JSON: {e}"
                ))
            })?;
        let mut properties: BTreeMap<String, Value> = serde_json::from_value(properties_value)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document properties must be a JSON object keyed by property name: {e}"
                ))
            })?;
        document_type
            .as_ref()
            .sanitize_document_properties(&mut properties);

        // Entropy is generated by the SDK at broadcast time (we pass
        // `None` to `put_to_platform_and_wait_for_response`), which
        // overwrites the document id with the canonical
        // `(contract, owner, type, entropy)` derivation. The entropy
        // supplied here only feeds the placeholder id, so a fixed zero
        // value is fine — it is never broadcast.
        let document = document_type
            .as_ref()
            .create_document_from_data(
                properties.into(),
                *owner_identity_id,
                0, // block_height — set by Platform
                0, // core_block_height — set by Platform
                [0u8; 32],
                platform_version,
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Failed to build document: {e}"))
            })?;

        // 3. Owner identity + signing key from the wallet manager. The
        //    document state transition must be signed by an
        //    AUTHENTICATION + ECDSA key whose security level satisfies
        //    the document type's requirement (NOT a fixed CRITICAL like
        //    the contract-create path).
        let required_level = document_type.security_level_requirement();
        let allowed_levels = allowed_signing_security_levels(required_level);
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(owner_identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*owner_identity_id))?;
            identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    allowed_levels.iter().copied().collect(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "No ECDSA authentication key at a security level satisfying \
                         {required_level} found on owner identity {owner_identity_id} \
                         (required to sign a {document_type_name} document state transition)"
                    ))
                })?
                .clone()
        };

        // 4. Broadcast via `PutDocument` on the platform-wallet 8 MB
        //    worker stack. `None` entropy -> the SDK generates entropy
        //    and the canonical document id for this revision-1 create;
        //    `None` token-payment-info -> no token gating.
        let confirmed = document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                document_type,
                None,
                signing_key,
                None,
                &SignerRef(signer),
                None,
            )
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`
                // (dashpay/platform#4183 review).
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to put document to platform: {e}"
                    ))
                })
            })?;

        Ok(confirmed)
    }

    /// Fetch the on-chain `DataContract` for `contract_id` (wrapped in
    /// an `Arc`, the shape the document-transition builders take) and
    /// resolve+verify that `document_type_name` exists on it.
    ///
    /// Shared by the mutate-existing-document flows (replace / delete /
    /// transfer / set-price / purchase) — each needs the contract as an
    /// `Arc<DataContract>` for both the single-document fetch query and
    /// the transition builder.
    async fn fetch_contract_arc_for_document_op(
        &self,
        contract_id: &Identifier,
        document_type_name: &str,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        let data_contract = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contract {contract_id} for document operation: {e}"
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot operate on document"
                ))
            })?;
        // Validate the document type exists up front so the caller gets
        // a clear error before a fetch/broadcast round-trip.
        data_contract
            .document_type_for_name(document_type_name)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document type {document_type_name:?} not found on contract {contract_id}: {e}"
                ))
            })?;
        // Make the contract resolvable for the post-broadcast proof check
        // (covers replace/delete/transfer/set-price/purchase).
        self.register_contract_for_proof_verification(&data_contract);
        Ok(Arc::new(data_contract))
    }

    /// Fetch the single current on-chain `Document` for
    /// `(contract, document_type_name, document_id)`.
    ///
    /// The mutate flows that carry the full document into their
    /// transition builder (replace / transfer / set-price / purchase)
    /// need the *current* revision + base data — they clone it, bump the
    /// revision, and (for replace) overwrite properties. Fetching here
    /// (rather than trusting a Swift-supplied document) keeps the
    /// revision authoritative and matches `rs-sdk-ffi`'s builder path,
    /// which also operates on a fetched/known document.
    async fn fetch_current_document(
        &self,
        data_contract: &Arc<DataContract>,
        document_type_name: &str,
        document_id: &Identifier,
    ) -> Result<Document, PlatformWalletError> {
        let query = DocumentQuery::new(Arc::clone(data_contract), document_type_name)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to build document query: {e}"
                ))
            })?
            .with_document_id(document_id);
        Document::fetch(&self.sdk, query)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch document {document_id}: {e}"
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document {document_id} not found on Platform; cannot operate on it"
                ))
            })
    }

    /// Resolve the AUTHENTICATION signing key `signing_key_id` on
    /// `owner_identity_id` from the in-process wallet manager.
    ///
    /// Unlike `create_document_with_signer` (which auto-selects an
    /// AUTHENTICATION key by security level), the mutate flows take an
    /// explicit `signing_key_id` chosen by the caller's key picker, so
    /// the user keeps control of which key signs. We still enforce the
    /// document state-transition signing rule here: the key must exist,
    /// be AUTHENTICATION-purpose, and be ECDSA_SECP256K1 — the same
    /// purpose `create` uses (see
    /// `project_document_signing_key_purpose_bug`: signing with a
    /// non-AUTHENTICATION key, e.g. a TRANSFER/CRITICAL key, is rejected
    /// by consensus with "requires AUTHENTICATION").
    async fn resolve_authentication_signing_key(
        &self,
        owner_identity_id: &Identifier,
        signing_key_id: u32,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        let manager = &info.identity_manager;
        let identity = manager
            .identity(owner_identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*owner_identity_id))?;
        let key = identity
            .get_public_key_by_id(signing_key_id)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Signing key {signing_key_id} not found on identity {owner_identity_id}"
                ))
            })?
            .clone();
        if key.purpose() != Purpose::AUTHENTICATION {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "Signing key {signing_key_id} on identity {owner_identity_id} has purpose {:?}, \
                 but a document state transition must be signed with an AUTHENTICATION key",
                key.purpose()
            )));
        }
        if key.key_type() != KeyType::ECDSA_SECP256K1 {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "Signing key {signing_key_id} on identity {owner_identity_id} has key type {:?}, \
                 but a document state transition must be signed with an ECDSA_SECP256K1 key",
                key.key_type()
            )));
        }
        Ok(key)
    }

    /// Replace an existing document's properties on
    /// `contract_id`'s `document_type_name` and broadcast.
    ///
    /// Fetches the current document, applies `properties_json` (parsed
    /// + schema-sanitized exactly like the create path), bumps the
    /// revision, signs with the explicit `signing_key_id`
    /// (AUTHENTICATION + ECDSA), broadcasts via `Sdk::document_replace`
    /// on the platform-wallet 8 MB worker stack, and returns the
    /// confirmed `Document`.
    #[allow(clippy::too_many_arguments)]
    pub async fn replace_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        properties_json: &str,
        signing_key_id: u32,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        // Owned `DocumentType` to sanitize the supplied properties
        // against the schema — same conversion the create path runs.
        let document_type = data_contract
            .document_type_cloned_for_name(document_type_name)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document type {document_type_name:?} not found on contract {contract_id}: {e}"
                ))
            })?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;

        // Parse + sanitize the new properties, then overwrite the
        // fetched document's property map. The system fields (id, owner,
        // timestamps, revision) are preserved from the fetched document.
        let properties_value: serde_json::Value =
            serde_json::from_str(properties_json).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Invalid document properties JSON: {e}"
                ))
            })?;
        let mut properties: BTreeMap<String, Value> = serde_json::from_value(properties_value)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document properties must be a JSON object keyed by property name: {e}"
                ))
            })?;
        document_type
            .as_ref()
            .sanitize_document_properties(&mut properties);
        document.set_properties(properties);

        // Bump the revision for the replacement (mirrors the rs-sdk-ffi
        // replace builder, which increments before building).
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .resolve_authentication_signing_key(owner_identity_id, signing_key_id)
            .await?;

        let builder = DocumentReplaceTransitionBuilder::new(
            data_contract,
            document_type_name.to_string(),
            document,
        );
        let DocumentReplaceResult::Document(confirmed) = self
            .sdk
            .document_replace(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`
                // (dashpay/platform#4183 review).
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to replace document: {e}"
                    ))
                })
            })?;
        Ok(confirmed)
    }

    /// Delete an existing document on `contract_id`'s
    /// `document_type_name` and broadcast.
    ///
    /// Signs with the explicit `signing_key_id` (AUTHENTICATION +
    /// ECDSA) and broadcasts via `Sdk::document_delete` on the
    /// platform-wallet 8 MB worker stack. Returns the deleted document's
    /// `Identifier` on confirmation.
    pub async fn delete_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        signing_key_id: u32,
        signer: &S,
    ) -> Result<Identifier, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        let signing_key = self
            .resolve_authentication_signing_key(owner_identity_id, signing_key_id)
            .await?;

        // Delete is keyed by (document_id, owner_id); no current-document
        // fetch is required.
        let builder = DocumentDeleteTransitionBuilder::new(
            data_contract,
            document_type_name.to_string(),
            *document_id,
            *owner_identity_id,
        );
        let DocumentDeleteResult::Deleted(deleted_id) = self
            .sdk
            .document_delete(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`
                // (dashpay/platform#4183 review).
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to delete document: {e}"
                    ))
                })
            })?;
        Ok(deleted_id)
    }

    /// Transfer an existing document on `contract_id`'s
    /// `document_type_name` to `recipient_id` and broadcast.
    ///
    /// Fetches the current document, bumps the revision, signs with the
    /// explicit `signing_key_id` (AUTHENTICATION + ECDSA), broadcasts
    /// via `Sdk::document_transfer` on the platform-wallet 8 MB worker
    /// stack, and returns the confirmed `Document` (now owned by
    /// `recipient_id`).
    #[allow(clippy::too_many_arguments)]
    pub async fn transfer_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        recipient_id: &Identifier,
        signing_key_id: u32,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .resolve_authentication_signing_key(owner_identity_id, signing_key_id)
            .await?;

        let builder = DocumentTransferTransitionBuilder::new(
            data_contract,
            document_type_name.to_string(),
            document,
            *recipient_id,
        );
        let DocumentTransferResult::Document(confirmed) = self
            .sdk
            .document_transfer(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`
                // (dashpay/platform#4183 review).
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to transfer document: {e}"
                    ))
                })
            })?;
        Ok(confirmed)
    }

    /// Set (update) the trade price of an existing document on
    /// `contract_id`'s `document_type_name` and broadcast.
    ///
    /// Fetches the current document, bumps the revision, signs with the
    /// explicit `signing_key_id` (AUTHENTICATION + ECDSA), broadcasts
    /// via `Sdk::document_set_price` on the platform-wallet 8 MB worker
    /// stack, and returns the confirmed `Document` (now carrying
    /// `$price`).
    #[allow(clippy::too_many_arguments)]
    pub async fn set_document_price_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        price: u64,
        signing_key_id: u32,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .resolve_authentication_signing_key(owner_identity_id, signing_key_id)
            .await?;

        let builder = DocumentSetPriceTransitionBuilder::new(
            data_contract,
            document_type_name.to_string(),
            document,
            price as Credits,
        );
        let DocumentSetPriceResult::Document(confirmed) = self
            .sdk
            .document_set_price(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`
                // (dashpay/platform#4183 review).
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to set document price: {e}"
                    ))
                })
            })?;
        Ok(confirmed)
    }

    /// Purchase an existing for-sale document on `contract_id`'s
    /// `document_type_name` and broadcast.
    ///
    /// `purchaser_identity_id` is the buyer (which becomes the new
    /// owner) and signs the transition with `signing_key_id`
    /// (AUTHENTICATION + ECDSA) — so the signing key is resolved on the
    /// purchaser, not the current owner. Fetches the current document,
    /// bumps the revision, broadcasts via `Sdk::document_purchase` on
    /// the platform-wallet 8 MB worker stack, and returns the confirmed
    /// `Document` (now owned by the purchaser). Consensus rejects a
    /// purchase where the buyer is the current owner — the caller's UI
    /// gates against that.
    #[allow(clippy::too_many_arguments)]
    pub async fn purchase_document_with_signer<S>(
        &self,
        purchaser_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        price: u64,
        signing_key_id: u32,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .resolve_authentication_signing_key(purchaser_identity_id, signing_key_id)
            .await?;

        let builder = DocumentPurchaseTransitionBuilder::new(
            data_contract,
            document_type_name.to_string(),
            document,
            *purchaser_identity_id,
            price as Credits,
        );
        let DocumentPurchaseResult::Document(confirmed) = self
            .sdk
            .document_purchase(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`
                // (dashpay/platform#4183 review).
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to purchase document: {e}"
                    ))
                })
            })?;
        Ok(confirmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_levels_high_requirement_admits_critical_and_high_only() {
        // DPNS `preorder` requires HIGH: CRITICAL + HIGH qualify,
        // MEDIUM does not, MASTER is excluded.
        let levels = allowed_signing_security_levels(SecurityLevel::HIGH);
        assert_eq!(levels, vec![SecurityLevel::CRITICAL, SecurityLevel::HIGH]);
    }

    #[test]
    fn allowed_levels_medium_requirement_admits_critical_high_medium() {
        let levels = allowed_signing_security_levels(SecurityLevel::MEDIUM);
        assert_eq!(
            levels,
            vec![
                SecurityLevel::CRITICAL,
                SecurityLevel::HIGH,
                SecurityLevel::MEDIUM
            ]
        );
    }

    #[test]
    fn allowed_levels_critical_requirement_admits_only_critical() {
        let levels = allowed_signing_security_levels(SecurityLevel::CRITICAL);
        assert_eq!(levels, vec![SecurityLevel::CRITICAL]);
    }

    #[test]
    fn allowed_levels_master_requirement_is_master_only() {
        // Degenerate case mirrored from the consensus rule: a MASTER
        // requirement collapses to the single `[MASTER]` set rather
        // than the CRITICAL..=MASTER range (which would be empty).
        let levels = allowed_signing_security_levels(SecurityLevel::MASTER);
        assert_eq!(levels, vec![SecurityLevel::MASTER]);
    }
}
