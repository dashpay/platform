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
//! Mirrors the identity-flow shape:
//!   - The library function takes a `Signer<IdentityPublicKey>`
//!     reference so the FFI's external `KeychainSigner` trampoline can
//!     route signing back to Swift / Keychain without crossing seed
//!     bytes.
//!   - The document content arrives as a properties JSON string and is
//!     turned into a platform `Value` map, then a revision-1 `Document`
//!     via `DocumentType::create_document_from_data` — the same path
//!     `rs-sdk-ffi/src/document/create.rs` uses.
//!   - Broadcast goes through
//!     `dash_sdk::platform::transition::put_document::PutDocument::put_to_platform_and_wait_for_response_with_owner_balance_and_metadata`
//!     on the platform-wallet runtime (8 MB worker stack) instead of
//!     the rs-sdk-ffi runtime (mobile-tuned default stack) — the same
//!     stack-overflow avoidance `contract.rs` documents for the
//!     post-broadcast GroveDB proof-verification recursion. The owner's
//!     balance the proof carries (from protocol version 14) is kept.

use super::key_selection::{
    check_signing_key, usable_signing_key, SigningScope, PLATFORM_SIGNING_KEY_TYPES,
    WALLET_SIGNING_KEY_TYPES,
};
use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::property_names::CREATED_AT;
use dpp::document::Document;
use dpp::document::DocumentV0Setters;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{DataContract, Identifier};
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::ProtocolError;

use dash_sdk::platform::documents::transitions::{
    DocumentDeleteResult, DocumentDeleteTransitionBuilder, DocumentPurchaseResult,
    DocumentPurchaseTransitionBuilder, DocumentReplaceResult, DocumentReplaceTransitionBuilder,
    DocumentSetPriceResult, DocumentSetPriceTransitionBuilder, DocumentTransferResult,
    DocumentTransferTransitionBuilder,
};
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::{ContextProvider, DocumentQuery, Fetch};

use crate::error::{preserve_signer_key_unavailable_or, PlatformWalletError};

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

/// The key that signs a document batch in `scope` whose document type requires
/// `requirement`: an AUTHENTICATION, ECDSA_SECP256K1 key at a level consensus accepts.
pub(super) fn document_signing_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
    requirement: SecurityLevel,
    scope: SigningScope<'_>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    usable_signing_key(
        identity,
        signer,
        Purpose::AUTHENTICATION,
        &SecurityLevel::accepted_for_requirement(requirement),
        &WALLET_SIGNING_KEY_TYPES,
        scope,
    )
}

/// Check a key the caller chose to sign a document batch in `scope` whose document type
/// requires `requirement`, by the rules [`document_signing_key`] applies, except that any key
/// type Platform verifies is accepted.
pub(super) fn check_document_signing_key(
    key: &IdentityPublicKey,
    signer: &impl Signer<IdentityPublicKey>,
    requirement: SecurityLevel,
    scope: SigningScope<'_>,
) -> Result<(), dash_sdk::Error> {
    check_signing_key(
        key,
        signer,
        &[Purpose::AUTHENTICATION],
        &SecurityLevel::accepted_for_requirement(requirement),
        &PLATFORM_SIGNING_KEY_TYPES,
        scope,
    )
}

impl IdentityWallet {
    /// Register a freshly-fetched data contract into the SDK's shared
    /// context provider so the returned-proof verification after a document
    /// state-transition broadcast can resolve it. Without this, the mobile
    /// `TrustedHttpContextProvider` — which never fetches contracts itself —
    /// returns `None` for the contract and proof verification fails with
    /// "unknown contract ... in document verification", even though the
    /// write landed on-chain.
    pub(super) fn register_contract_for_proof_verification(&self, contract: &DataContract) {
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
    ///      `security_level_requirement()` and whose bounds cover the
    ///      document type (see [`document_signing_key`]). Returns a clear error
    ///      if the owner identity isn't loaded or carries no qualifying
    ///      key.
    ///   4. Broadcasts via
    ///      `Document::put_to_platform_and_wait_for_response_with_owner_balance_and_metadata`
    ///      on the platform-wallet 8 MB-stack worker, keeps the owner's
    ///      balance the proof carries (from protocol version 14) and
    ///      returns the confirmed `Document` from Platform.
    ///
    /// `properties_json` is a JSON object keyed by property name. Byte-
    /// array fields are supplied as hex (or base64) strings and
    /// identifier fields as base58 (or hex) strings; the schema-driven
    /// sanitize step converts them to the protocol's native `Bytes` /
    /// `Identifier` values. An empty object (`"{}"`) is valid for a
    /// document type with no required properties.
    ///
    /// `contest_fund` is the most a contested document pays into the
    /// contest it joins (the fund to join doubles once a contest holds 250
    /// contenders and again for every 50 more); `None` states the fund to
    /// join read just before the document is submitted. A document that
    /// joins no contest ignores it.
    pub async fn create_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        properties_json: &str,
        contest_fund: Option<Credits>,
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
            drop(wm);
            document_signing_key(
                &identity,
                signer,
                required_level,
                SigningScope::DocumentCreate {
                    document_type: document_type.as_ref(),
                },
            )?
            .ok_or_else(|| PlatformWalletError::NoUsableSigningKey {
                identity_id: *owner_identity_id,
                purpose: Purpose::AUTHENTICATION,
                security_levels: SecurityLevel::accepted_for_requirement(required_level),
                key_types: WALLET_SIGNING_KEY_TYPES.to_vec(),
                transition: format!("a {document_type_name} document transition"),
            })?
            .clone()
        };

        // 4. Broadcast via `PutDocument` on the platform-wallet 8 MB
        //    worker stack. `None` entropy -> the SDK generates entropy
        //    and the canonical document id for this revision-1 create;
        //    `None` token-payment-info -> no token gating.
        let settings = contest_fund.map(|contest_fund| PutSettings {
            state_transition_creation_options: Some(StateTransitionCreationOptions {
                contest_fund: Some(contest_fund),
                ..Default::default()
            }),
            ..Default::default()
        });
        let (confirmed, owner_balance, metadata) = document
            .put_to_platform_and_wait_for_response_with_owner_balance_and_metadata(
                &self.sdk,
                document_type,
                None,
                signing_key,
                None,
                &SignerRef(signer),
                settings,
            )
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`.
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to put document to platform: {e}"
                    ))
                })
            })?;
        self.persist_proven_owner_balance(owner_identity_id, owner_balance, metadata)
            .await;

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
    pub(super) async fn fetch_contract_arc_for_document_op(
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
    /// document state-transition signing rules here: the key must exist
    /// and pass [`check_document_signing_key`] — AUTHENTICATION-purpose,
    /// ECDSA_SECP256K1, at an accepted level, enabled, unexpired and inside
    /// its contract bounds — the same rules `create` uses (see
    /// `project_document_signing_key_purpose_bug`: signing with a
    /// non-AUTHENTICATION key, e.g. a TRANSFER/CRITICAL key, is rejected
    /// by consensus with "requires AUTHENTICATION").
    async fn resolve_authentication_signing_key(
        &self,
        owner_identity_id: &Identifier,
        signing_key_id: u32,
        data_contract: &DataContract,
        document_type_name: &str,
        signer: &impl Signer<IdentityPublicKey>,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let requirement = data_contract
            .document_type_for_name(document_type_name)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Document type {document_type_name:?} not found on contract {}: {e}",
                    data_contract.id()
                ))
            })?
            .security_level_requirement();
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
        drop(wm);
        let key = identity
            .get_public_key_by_id(signing_key_id)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Signing key {signing_key_id} not found on identity {owner_identity_id}"
                ))
            })?
            .clone();
        check_document_signing_key(
            &key,
            signer,
            requirement,
            SigningScope::DocumentType {
                contract_id: data_contract.id(),
                document_type_name,
            },
        )?;
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

        let signing_key = self
            .resolve_authentication_signing_key(
                owner_identity_id,
                signing_key_id,
                &data_contract,
                document_type_name,
                signer,
            )
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
                // operation failures get stringified into `InvalidIdentityData`.
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
            .resolve_authentication_signing_key(
                owner_identity_id,
                signing_key_id,
                &data_contract,
                document_type_name,
                signer,
            )
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
                // operation failures get stringified into `InvalidIdentityData`.
                crate::error::preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to delete document: {e}"
                    ))
                })
            })?;
        Ok(deleted_id)
    }

    /// Delete a document by its values and broadcast.
    ///
    /// The delete an `indexOnly` document type needs: such a type keeps no
    /// document row, so its delete transition carries the values that
    /// identify the entries, and [`Self::delete_document_with_signer`]
    /// (ids only) is refused for it. Any other document type is refused
    /// here before anything is signed: its delete goes by id and would not
    /// look at the values. So is a type whose documents can not be deleted.
    ///
    /// `properties_json` is the document's own properties as they were
    /// created, in the format [`Self::create_document_with_signer`] takes;
    /// `document_id` is the id that create returned. A type whose entries
    /// commit to `$createdAt` also needs that timestamp, as a
    /// `"$createdAt"` number of milliseconds in the same object; no other
    /// `$` field and no property the type does not declare is accepted, and
    /// every required property has to be there. A changed value does not
    /// match the stored entry and Platform rejects the delete.
    ///
    /// Signs with the explicit `signing_key_id` (AUTHENTICATION + ECDSA)
    /// and broadcasts via `Sdk::document_delete` on the platform-wallet
    /// 8 MB worker stack. Returns the deleted document's `Identifier` on
    /// confirmation.
    #[allow(clippy::too_many_arguments)]
    pub async fn delete_document_by_values_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        properties_json: &str,
        signing_key_id: u32,
        signer: &S,
    ) -> Result<Identifier, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        let (values, created_at) = values_from_json(properties_json)?;
        let builder = DocumentDeleteTransitionBuilder::from_values(
            Arc::clone(&data_contract),
            document_type_name.to_string(),
            *document_id,
            *owner_identity_id,
            values,
            created_at,
            self.sdk.version(),
        )
        .map_err(delete_preparation_error)?;

        let signing_key = self
            .resolve_authentication_signing_key(
                owner_identity_id,
                signing_key_id,
                &data_contract,
                document_type_name,
                signer,
            )
            .await?;

        let DocumentDeleteResult::Deleted(deleted_id) = self
            .sdk
            .document_delete(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                preserve_signer_key_unavailable_or(e, |e| {
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

        let signing_key = self
            .resolve_authentication_signing_key(
                owner_identity_id,
                signing_key_id,
                &data_contract,
                document_type_name,
                signer,
            )
            .await?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

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
                // Typed trade rejections (not-for-sale / price-changed /
                // insufficient credits) and the structured key-unavailable
                // signer failure survive; only genuine operation failures
                // get stringified into `InvalidIdentityData`.
                crate::error::promote_document_trade_error_or(e, |e| {
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

        let signing_key = self
            .resolve_authentication_signing_key(
                owner_identity_id,
                signing_key_id,
                &data_contract,
                document_type_name,
                signer,
            )
            .await?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

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
                // Typed trade rejections and the structured key-unavailable
                // signer failure survive; only genuine operation failures
                // get stringified into `InvalidIdentityData`.
                crate::error::promote_document_trade_error_or(e, |e| {
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

        let signing_key = self
            .resolve_authentication_signing_key(
                purchaser_identity_id,
                signing_key_id,
                &data_contract,
                document_type_name,
                signer,
            )
            .await?;

        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

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
                // Typed trade rejections — crucially the price-changed race
                // (40109), where the consensus equality check is the backstop
                // behind the wallet's pre-flight — and the structured
                // key-unavailable signer failure survive; only genuine
                // operation failures get stringified into
                // `InvalidIdentityData`.
                crate::error::promote_document_trade_error_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to purchase document: {e}"
                    ))
                })
            })?;
        Ok(confirmed)
    }
}

/// The values a delete-by-values names, parsed from `properties_json`, and
/// its `$createdAt`, taken out of them.
fn values_from_json(
    properties_json: &str,
) -> Result<(BTreeMap<String, Value>, Option<u64>), PlatformWalletError> {
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
    let created_at = match properties.remove(CREATED_AT) {
        None => None,
        Some(value) => Some(value.to_integer::<u64>().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "{CREATED_AT} must be a number of milliseconds: {e}"
            ))
        })?),
    };
    Ok((properties, created_at))
}

/// A refusal of
/// [`index_only_delete_document`](dash_sdk::platform::documents::transitions::index_only_delete_document)
/// carries its own text; any other SDK error keeps its rendering.
fn delete_preparation_error(error: dash_sdk::Error) -> PlatformWalletError {
    match error {
        dash_sdk::Error::Generic(message) => PlatformWalletError::InvalidIdentityData(message),
        other => PlatformWalletError::InvalidIdentityData(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::key_selection::tests::signature_refusal_of;
    use super::super::signing_key::tests::{
        identity as identity_with_keys, lock_checking_signer, wallet_with_identity_on,
        wallet_with_signing_keys,
    };
    use super::*;
    use dpp::consensus::signature::SignatureError;
    use dpp::identity::accessors::IdentityGettersV0;

    /// A wallet whose SDK answers only the fetch of the DashPay contract, owning an identity
    /// whose key 1 is disabled.
    async fn wallet_answering_only_the_contract() -> (Arc<crate::PlatformWallet>, Identifier) {
        let contract = super::super::dashpay_contract().unwrap();
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
        sdk.mock()
            .expect_fetch::<DataContract, Identifier>(contract.id(), Some((*contract).clone()))
            .await
            .unwrap();
        let mut identity = identity_with_keys(
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
        );
        identity
            .public_keys_mut()
            .get_mut(&1)
            .unwrap()
            .set_disabled_at(1);
        let owner = identity.id();
        (wallet_with_identity_on(sdk, identity).await, owner)
    }

    fn assert_disabled_refusal(result: Result<Document, PlatformWalletError>) {
        assert!(
            matches!(
                &result,
                Err(PlatformWalletError::Sdk(error))
                    if matches!(
                        signature_refusal_of(error),
                        Some(SignatureError::PublicKeyIsDisabledError(_))
                    )
            ),
            "the key is refused before the document is fetched: {result:?}"
        );
    }

    #[tokio::test]
    async fn should_refuse_an_explicit_key_before_fetching_the_document() {
        let contract_id = super::super::dashpay_contract().unwrap().id();
        let document_id = Identifier::from([9; 32]);
        let signer = KeyFilter(|_| true);

        let (wallet, owner) = wallet_answering_only_the_contract().await;
        assert_disabled_refusal(
            wallet
                .identity()
                .replace_document_with_signer(
                    &owner,
                    &contract_id,
                    "profile",
                    &document_id,
                    "{}",
                    1,
                    &signer,
                )
                .await,
        );
        let (wallet, owner) = wallet_answering_only_the_contract().await;
        assert_disabled_refusal(
            wallet
                .identity()
                .transfer_document_with_signer(
                    &owner,
                    &contract_id,
                    "profile",
                    &document_id,
                    &Identifier::from([3; 32]),
                    1,
                    &signer,
                )
                .await,
        );
        let (wallet, owner) = wallet_answering_only_the_contract().await;
        assert_disabled_refusal(
            wallet
                .identity()
                .set_document_price_with_signer(
                    &owner,
                    &contract_id,
                    "profile",
                    &document_id,
                    5,
                    1,
                    &signer,
                )
                .await,
        );
        let (wallet, owner) = wallet_answering_only_the_contract().await;
        assert_disabled_refusal(
            wallet
                .identity()
                .purchase_document_with_signer(
                    &owner,
                    &contract_id,
                    "profile",
                    &document_id,
                    5,
                    1,
                    &signer,
                )
                .await,
        );
    }

    #[tokio::test]
    async fn should_reject_unavailable_explicit_document_key_outside_wallet_lock() {
        let wallet = wallet_with_signing_keys().await;
        let signer = lock_checking_signer(wallet.identity());
        let error = wallet
            .identity()
            .resolve_authentication_signing_key(
                &Identifier::default(),
                1,
                &super::super::dashpay_contract().unwrap(),
                "contactRequest",
                &signer,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::Sdk(dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(ref message)))
                if message.starts_with(crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX)),
            "{error:?}"
        );
    }

    #[test]
    fn should_refuse_values_that_are_not_a_json_object() {
        let error = values_from_json("[1, 2]").unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::InvalidIdentityData(ref message)
                if message.contains("JSON object")),
            "{error:?}"
        );
    }

    #[test]
    fn should_take_the_creation_time_out_of_the_values() {
        let mut properties = serde_json::json!({ "encryptedPayload": "33" });

        let (values, created_at) = values_from_json(&properties.to_string()).expect("parses");
        assert_eq!(created_at, None);
        assert_eq!(
            values.get("encryptedPayload"),
            Some(&Value::Text("33".to_string()))
        );

        properties["$createdAt"] = serde_json::json!(1_700_000_000_000u64);
        let (values, created_at) = values_from_json(&properties.to_string()).expect("parses");
        assert_eq!(created_at, Some(1_700_000_000_000));
        assert!(!values.contains_key("$createdAt"));

        properties["$createdAt"] = serde_json::json!("yesterday");
        let error = values_from_json(&properties.to_string()).unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::InvalidIdentityData(ref message)
                if message.contains("must be a number of milliseconds")),
            "{error:?}"
        );
    }

    use super::super::key_selection::tests::{bound_key, identity, key};
    use super::super::signing_key::tests::KeyFilter;
    use dpp::identity::identity_public_key::accessors::v0::{
        IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
    };
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::KeyType;

    const CONTRACT: [u8; 32] = [0xDA; 32];

    fn scope() -> SigningScope<'static> {
        SigningScope::DocumentType {
            contract_id: Identifier::from(CONTRACT),
            document_type_name: "note",
        }
    }

    fn pick(identity: &Identity, requirement: SecurityLevel) -> Option<u32> {
        document_signing_key(identity, &KeyFilter(|_| true), requirement, scope())
            .unwrap()
            .map(|key| key.id())
    }

    fn bound_elsewhere(id: u32, level: SecurityLevel) -> IdentityPublicKey {
        bound_key(
            id,
            level,
            Some(ContractBounds::SingleContract {
                id: Identifier::from([0x0E; 32]),
            }),
        )
    }

    #[test]
    fn should_choose_a_document_key_whose_bounds_cover_the_document_type() {
        let identity = identity(vec![
            bound_elsewhere(1, SecurityLevel::CRITICAL),
            key(2, SecurityLevel::HIGH),
        ]);
        assert_eq!(pick(&identity, SecurityLevel::HIGH), Some(2));
    }

    #[test]
    fn should_choose_an_unlimited_document_key_before_a_limited_one() {
        let identity = identity(vec![
            key(1, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            key(2, SecurityLevel::HIGH),
        ]);
        assert_eq!(pick(&identity, SecurityLevel::HIGH), Some(2));
    }

    #[test]
    fn should_choose_only_levels_the_document_type_accepts() {
        let identity = identity(vec![
            key(0, SecurityLevel::MASTER),
            key(1, SecurityLevel::MEDIUM),
            key(2, SecurityLevel::HIGH),
        ]);
        assert_eq!(pick(&identity, SecurityLevel::HIGH), Some(2));
        assert_eq!(pick(&identity, SecurityLevel::CRITICAL), None);
        assert_eq!(pick(&identity, SecurityLevel::MEDIUM), Some(1));
    }

    #[test]
    fn should_refuse_an_explicit_document_key_platform_would_refuse() {
        let check = |key: &IdentityPublicKey| {
            check_document_signing_key(key, &KeyFilter(|_| true), SecurityLevel::HIGH, scope())
        };
        check(&key(1, SecurityLevel::HIGH)).expect("a HIGH key meets a HIGH requirement");
        assert!(
            check(&key(1, SecurityLevel::MEDIUM)).is_err(),
            "MEDIUM is below the requirement"
        );
        assert!(
            check(&bound_elsewhere(1, SecurityLevel::HIGH)).is_err(),
            "the key is bound to another contract"
        );
        assert!(
            check(&key(1, SecurityLevel::HIGH).with_limits(None, Some(1))).is_err(),
            "the key has expired"
        );
        for key_type in [KeyType::ECDSA_HASH160, KeyType::BLS12_381] {
            let mut explicit = key(1, SecurityLevel::HIGH);
            explicit.set_key_type(key_type);
            check(&explicit).expect("Platform verifies a signature by this key type");
        }
        let mut eddsa = key(1, SecurityLevel::HIGH);
        eddsa.set_key_type(KeyType::EDDSA_25519_HASH160);
        assert!(
            check(&eddsa).is_err(),
            "Platform cannot verify an identity signature by this key type"
        );
    }
}
