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
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::property_names::CREATED_AT;
use dpp::document::Document;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::prelude::{DataContract, Identifier};
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::ProtocolError;

use dash_sdk::platform::documents::transitions::{
    DocumentDeleteResult, DocumentDeleteTransitionBuilder, DocumentEraseResult,
    DocumentEraseTransitionBuilder, DocumentPurchaseResult, DocumentPurchaseTransitionBuilder,
    DocumentReplaceResult, DocumentReplaceTransitionBuilder, DocumentSetPriceResult,
    DocumentSetPriceTransitionBuilder, DocumentTransferResult, DocumentTransferTransitionBuilder,
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
pub(super) struct SignerRef<'a, S: ?Sized>(pub(super) &'a S);

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
/// `requirement`: an AUTHENTICATION key of one of [`WALLET_SIGNING_KEY_TYPES`] at a level
/// consensus accepts, in [`usable_signing_key`]'s order of preference.
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
    ///      wallet manager: an AUTHENTICATION-purpose ECDSA key
    ///      (`ECDSA_SECP256K1` preferred, `ECDSA_HASH160` accepted) whose security level satisfies the document type's
    ///      `security_level_requirement()` and whose bounds cover the
    ///      document type and every type its create may consume (see
    ///      [`document_signing_key`]). Returns a clear error
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
        let properties = document_properties_from_json(document_type.as_ref(), properties_json)?;

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
        let signing_key = self
            .select_document_signing_key(
                owner_identity_id,
                &document_type,
                SigningScope::DocumentCreate {
                    document_type: document_type.as_ref(),
                },
                signer,
            )
            .await?;

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
        self.put_document_keeping_owner_balance(
            &document,
            document_type,
            None,
            signing_key,
            signer,
            settings,
        )
        .await
        .map_err(|e| {
            // Preserve a structured key-unavailable signer failure so the
            // FFI boundary can still restore code 31; only genuine
            // operation failures get stringified into `InvalidIdentityData`.
            preserve_signer_key_unavailable_or(e, |e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to put document to platform: {e}"
                ))
            })
        })
    }

    /// The key a `document_type` state transition of `signing_identity_id`
    /// in `scope` is signed with when the caller names none:
    /// [`document_signing_key`]'s choice among the owner identity's keys in
    /// the wallet manager. The wallet manager lock is released before
    /// `signer` is asked.
    pub(super) async fn select_document_signing_key<S>(
        &self,
        signing_identity_id: &Identifier,
        document_type: &DocumentType,
        scope: SigningScope<'_>,
        signer: &S,
    ) -> Result<IdentityPublicKey, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey>,
    {
        let required_level = document_type.security_level_requirement();
        let identity = {
            let wm = self.wallet_manager.read().await;
            self.current_wallet_info(&wm)?
                .identity_manager
                .wallet_identity(&self.wallet_id, signing_identity_id)
                .map(|managed| managed.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*signing_identity_id))?
        };
        let signing_key = document_signing_key(&identity, signer, required_level, scope)?
            .ok_or_else(|| PlatformWalletError::NoUsableSigningKey {
                identity_id: *signing_identity_id,
                purposes: vec![Purpose::AUTHENTICATION],
                security_levels: SecurityLevel::accepted_for_requirement(required_level),
                key_types: WALLET_SIGNING_KEY_TYPES.to_vec(),
                transition: format!("a {} document transition", document_type.name()),
            })?;
        Ok(signing_key.clone())
    }

    /// Put `document` (a create at revision 1, a replace above it),
    /// signed with `signing_key`, wait for the proof that it executed,
    /// keep the owner's balance the proof carries (from protocol version
    /// 14) and return the confirmed `Document`.
    ///
    /// The document goes out exactly as given: a replace carries its own
    /// revision, which Platform takes only as the stored revision plus
    /// one, so nothing here reads the document again or moves its
    /// revision. `entropy` is read for a create only; `None` lets the SDK
    /// generate it. Runs on the caller's runtime; the FFI drives it on the
    /// platform-wallet 8 MB worker stack for the post-broadcast GroveDB
    /// proof-verification recursion.
    pub(super) async fn put_document_keeping_owner_balance<S>(
        &self,
        document: &Document,
        document_type: DocumentType,
        entropy: Option<[u8; 32]>,
        signing_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, dash_sdk::Error>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let (confirmed, owner_balance, metadata) = document
            .put_to_platform_and_wait_for_response_with_owner_balance_and_metadata(
                &self.sdk,
                document_type,
                entropy,
                signing_key,
                None,
                &SignerRef(signer),
                settings,
            )
            .await?;
        self.persist_proven_owner_balance(&document.owner_id(), owner_balance, metadata)
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
    /// `owner_identity_id` from the in-process wallet manager, for
    /// [`Self::erase_document_with_signer`], which takes an explicit key.
    ///
    /// The key must exist and pass [`check_document_signing_key`]:
    /// AUTHENTICATION-purpose, a key type Platform verifies, at an accepted
    /// level, enabled, unexpired and inside its contract bounds. Signing with a
    /// non-AUTHENTICATION key, e.g. a TRANSFER/CRITICAL key, is rejected
    /// by consensus with "requires AUTHENTICATION". `signer` must be able
    /// to sign with it.
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

    /// Refuse, before anything is signed, a document that `owner_identity_id`
    /// does not own: Platform would refuse the transition.
    fn require_document_owner(
        document: &Document,
        owner_identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        if document.owner_id() == *owner_identity_id {
            Ok(())
        } else {
            Err(PlatformWalletError::InvalidParameter(format!(
                "Document {} is owned by {}, not by {owner_identity_id}",
                document.id(),
                document.owner_id()
            )))
        }
    }

    /// Replace an existing document's properties on
    /// `contract_id`'s `document_type_name` and broadcast.
    ///
    /// Fetches the current document, applies `properties_json` (parsed
    /// + schema-sanitized exactly like the create path), bumps the
    /// revision, signs with the owner's AUTHENTICATION key that the
    /// document type's security level allows, the key's contract bounds
    /// admit and `signer` can sign with (see
    /// [`Self::select_document_signing_key`]), broadcasts via
    /// `Sdk::document_replace` on the platform-wallet 8 MB worker stack,
    /// and returns the confirmed `Document`. A document the owner does not
    /// hold is refused before anything is signed.
    pub async fn replace_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        properties_json: &str,
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
        Self::require_document_owner(&document, owner_identity_id)?;

        // Parse + sanitize the new properties, then overwrite the
        // fetched document's property map. The system fields (id, owner,
        // timestamps, revision) are preserved from the fetched document.
        let properties = document_properties_from_json(document_type.as_ref(), properties_json)?;
        document.set_properties(properties);

        // Bump the revision for the replacement (mirrors the rs-sdk-ffi
        // replace builder, which increments before building).
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .select_document_signing_key(
                owner_identity_id,
                &document_type,
                SigningScope::DocumentType {
                    contract_id: data_contract.id(),
                    document_type_name,
                },
                signer,
            )
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
    /// Signs with the owner's AUTHENTICATION key that the document
    /// type's security level allows, the key's contract bounds admit and
    /// `signer` can sign with (see [`Self::select_document_signing_key`])
    /// and broadcasts via
    /// `Sdk::document_delete` on the platform-wallet 8 MB worker stack.
    /// Returns the deleted document's `Identifier` on confirmation.
    pub async fn delete_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        signer: &S,
    ) -> Result<Identifier, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        let signing_key = self
            .select_document_signing_key(
                owner_identity_id,
                document_type_of(&data_contract, document_type_name)?,
                SigningScope::DocumentType {
                    contract_id: data_contract.id(),
                    document_type_name,
                },
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
    /// Signs with the owner's AUTHENTICATION key that the document
    /// type's security level allows, the key's contract bounds admit and
    /// `signer` can sign with (see [`Self::select_document_signing_key`])
    /// and broadcasts via
    /// `Sdk::document_delete` on the platform-wallet 8 MB worker stack.
    /// Returns the deleted document's `Identifier` on confirmation.
    pub async fn delete_document_by_values_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        properties_json: &str,
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
            .select_document_signing_key(
                owner_identity_id,
                document_type_of(&data_contract, document_type_name)?,
                SigningScope::DocumentType {
                    contract_id: data_contract.id(),
                    document_type_name,
                },
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

    /// Erase a chunk of the retained revisions of an already deleted
    /// document on `contract_id`'s `document_type_name` and broadcast.
    ///
    /// Sibling of `delete_document_with_signer`. The document type must
    /// keep history and allow erasure, and the document must already be
    /// deleted. The first erase must be signed by the document's owner
    /// and commits the document to erasure; every erase after that may be
    /// signed by any identity, so `owner_identity_id` is the identity
    /// signing and paying for this chunk rather than necessarily the
    /// document's owner. One transition removes a bounded chunk of
    /// revisions; read the document's history to see how many remain and
    /// submit another erase while there are any.
    ///
    /// Signs with the explicit `signing_key_id` (AUTHENTICATION +
    /// ECDSA) and broadcasts via `Sdk::document_erase` on the
    /// platform-wallet 8 MB worker stack. Returns the document's
    /// `Identifier` once Platform has proved it absent from ordinary
    /// reads. That absence predates the erase, so the return is an
    /// observation of the affected state, not evidence that this erase
    /// removed any revision; progress is read from the document's
    /// history, whose lifecycle block carries the state and the exact
    /// number of revisions still retained.
    pub async fn erase_document_with_signer<S>(
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

        // Erase is keyed by (document_id, owner_id) like delete; the
        // document is no longer visible to ordinary reads, so there is
        // nothing to fetch first.
        let builder = DocumentEraseTransitionBuilder::new(
            data_contract,
            document_type_name.to_string(),
            *document_id,
            *owner_identity_id,
        );
        let DocumentEraseResult::AbsentAsOfProof(absent_id) = self
            .sdk
            .document_erase(builder, &signing_key, &SignerRef(signer))
            .await
            .map_err(|e| {
                // Preserve a structured key-unavailable signer failure so the
                // FFI boundary can still restore code 31; only genuine
                // operation failures get stringified into `InvalidIdentityData`.
                preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to erase document: {e}"
                    ))
                })
            })?;
        Ok(absent_id)
    }

    /// Transfer an existing document on `contract_id`'s
    /// `document_type_name` to `recipient_id` and broadcast.
    ///
    /// Fetches the current document, bumps the revision, signs,
    /// broadcasts via `Sdk::document_transfer` on the platform-wallet
    /// 8 MB worker stack, and returns the confirmed `Document` (now owned
    /// by `recipient_id`).
    ///
    /// Signed with the owner's AUTHENTICATION key that the document type's
    /// security level allows, the key's contract bounds admit and `signer`
    /// can sign with (see [`Self::select_document_signing_key`]). A
    /// document the owner does not hold is refused before anything is
    /// signed.
    pub async fn transfer_document_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        recipient_id: &Identifier,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        self.transfer_document_in_contract(
            owner_identity_id,
            data_contract,
            document_type_name,
            document_id,
            recipient_id,
            signer,
        )
        .await
    }

    /// Transfer like [`Self::transfer_document_with_signer`], on `data_contract` as the
    /// caller resolved it: the signing key is selected against that same contract, so the
    /// key and the transition it signs are judged by one contract snapshot.
    pub(super) async fn transfer_document_in_contract<S>(
        &self,
        owner_identity_id: &Identifier,
        data_contract: Arc<DataContract>,
        document_type_name: &str,
        document_id: &Identifier,
        recipient_id: &Identifier,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        Self::require_document_owner(&document, owner_identity_id)?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .select_document_signing_key(
                owner_identity_id,
                document_type_of(&data_contract, document_type_name)?,
                SigningScope::DocumentType {
                    contract_id: data_contract.id(),
                    document_type_name,
                },
                signer,
            )
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
    /// Fetches the current document, bumps the revision, signs,
    /// broadcasts via `Sdk::document_set_price` on the platform-wallet
    /// 8 MB worker stack, and returns the confirmed `Document` (now
    /// carrying `$price`).
    ///
    /// Signed with the owner's key, chosen as in
    /// [`Self::transfer_document_with_signer`]. A document the owner does
    /// not hold is refused before anything is signed.
    pub async fn set_document_price_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        price: u64,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        self.set_document_price_in_contract(
            owner_identity_id,
            data_contract,
            document_type_name,
            document_id,
            price,
            signer,
        )
        .await
    }

    /// Set a price like [`Self::set_document_price_with_signer`], on `data_contract` as the
    /// caller resolved it: the signing key is selected against that same contract, so the
    /// key and the transition it signs are judged by one contract snapshot.
    pub(super) async fn set_document_price_in_contract<S>(
        &self,
        owner_identity_id: &Identifier,
        data_contract: Arc<DataContract>,
        document_type_name: &str,
        document_id: &Identifier,
        price: u64,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        Self::require_document_owner(&document, owner_identity_id)?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .select_document_signing_key(
                owner_identity_id,
                document_type_of(&data_contract, document_type_name)?,
                SigningScope::DocumentType {
                    contract_id: data_contract.id(),
                    document_type_name,
                },
                signer,
            )
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
    /// owner) and signs the transition, so the signing key belongs to
    /// the purchaser, not the current owner: the purchaser's
    /// AUTHENTICATION key that the document type's security level allows,
    /// the key's contract bounds admit and `signer` can sign with (see
    /// [`Self::select_document_signing_key`]).
    /// Fetches the current document, bumps the revision, broadcasts via
    /// `Sdk::document_purchase` on the platform-wallet 8 MB worker stack,
    /// and returns the confirmed `Document` (now owned by the purchaser).
    /// Consensus rejects a purchase where the buyer is the current owner
    /// — the caller's UI gates against that.
    pub async fn purchase_document_with_signer<S>(
        &self,
        purchaser_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        price: u64,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let data_contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;

        self.purchase_document_in_contract(
            data_contract,
            document_type_name,
            document_id,
            purchaser_identity_id,
            price,
            signer,
        )
        .await
    }

    /// Purchase like [`Self::purchase_document_with_signer`], on `data_contract` as the
    /// caller resolved it: the signing key is selected against that same contract, so the
    /// key and the transition it signs are judged by one contract snapshot.
    pub(super) async fn purchase_document_in_contract<S>(
        &self,
        data_contract: Arc<DataContract>,
        document_type_name: &str,
        document_id: &Identifier,
        purchaser_identity_id: &Identifier,
        price: u64,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let mut document = self
            .fetch_current_document(&data_contract, document_type_name, document_id)
            .await?;
        document.increment_revision().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to increment document revision: {e}"
            ))
        })?;

        let signing_key = self
            .select_document_signing_key(
                purchaser_identity_id,
                document_type_of(&data_contract, document_type_name)?,
                SigningScope::DocumentType {
                    contract_id: data_contract.id(),
                    document_type_name,
                },
                signer,
            )
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

/// The properties `properties_json`, a JSON object keyed by property name,
/// holds for a document of `document_type`, sanitized against its schema:
/// byte arrays given as hex (or base64) strings and identifiers given as
/// base58 (or hex) strings become the protocol's native `Bytes` and
/// `Identifier` values, as `rs-sdk-ffi`'s document create converts them.
pub(super) fn document_properties_from_json(
    document_type: DocumentTypeRef<'_>,
    properties_json: &str,
) -> Result<BTreeMap<String, Value>, PlatformWalletError> {
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
    document_type.sanitize_document_properties(&mut properties);
    Ok(properties)
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

/// The `document_type_name` type of `data_contract`, which
/// [`IdentityWallet::select_document_signing_key`] reads the signing rules
/// from.
fn document_type_of<'a>(
    data_contract: &'a DataContract,
    document_type_name: &str,
) -> Result<&'a DocumentType, PlatformWalletError> {
    data_contract
        .document_type_borrowed_for_name(document_type_name)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Document type {document_type_name:?} not found on contract {}: {e}",
                data_contract.id()
            ))
        })
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
    use super::super::decision_proposal::tests::proposals_sdk;
    use super::super::signing_key::tests::{
        available, lock_checking_signer, wallet_with_signing_keys, wallet_with_signing_keys_on,
    };
    use super::*;
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::PlatformWallet;
    use dash_sdk::query_types::IdentityContractNonceFetcher;
    use dpp::data_contract::factory::DataContractFactory;
    use dpp::document::DocumentV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::platform_value::platform_value;
    use dpp::version::PlatformVersion;

    /// The owner identity `wallet_with_signing_keys` holds: CRITICAL
    /// AUTHENTICATION ECDSA_SECP256K1 keys 1 and 2.
    fn owner() -> Identifier {
        Identifier::default()
    }

    /// A contract whose `note` type requires `requirement` to sign.
    fn contract_requiring(requirement: SecurityLevel) -> DataContract {
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .unwrap()
            .create_with_value_config(
                Identifier::from([1; 32]),
                1,
                platform_value!({
                    "note": {
                        "type": "object",
                        "documentsMutable": true,
                        "canBeDeleted": true,
                        "transferable": 1,
                        "tradeMode": 1,
                        "signatureSecurityLevelRequirement": requirement as u8,
                        "properties": {"message": {"type": "string", "maxLength": 64, "position": 0}},
                        "additionalProperties": false
                    }
                }),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned()
    }

    /// The scope of a write to existing documents of `document_type`.
    fn type_scope(document_type: &DocumentType) -> SigningScope<'_> {
        SigningScope::DocumentType {
            contract_id: document_type.data_contract_id(),
            document_type_name: document_type.name(),
        }
    }

    /// The scope of a create of a `document_type` document.
    fn create_scope(document_type: &DocumentType) -> SigningScope<'_> {
        SigningScope::DocumentCreate {
            document_type: document_type.as_ref(),
        }
    }

    /// The `note` type of [`contract_requiring`] `requirement`.
    fn note_type(requirement: SecurityLevel) -> DocumentType {
        contract_requiring(requirement)
            .document_type_cloned_for_name("note")
            .unwrap()
    }

    fn assert_signer_key_unavailable(error: PlatformWalletError) {
        assert!(
            matches!(error, PlatformWalletError::Sdk(dash_sdk::Error::Protocol(ProtocolError::Generic(ref message)))
                if message.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)),
            "{error:?}"
        );
    }

    /// An AUTHENTICATION key `id` at `level` of `key_type`, unbound and
    /// without limits.
    fn auth_key(id: u32, level: SecurityLevel, key_type: KeyType) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: level,
            key_type,
            data: vec![id as u8; key_type.default_size()].into(),
            ..Default::default()
        }
        .into()
    }

    /// Add an identity `identity_id` holding `keys` to `wallet` at
    /// registration index `index` (the owner sits at 0).
    async fn hold_identity(
        wallet: &PlatformWallet,
        identity_id: Identifier,
        index: u32,
        keys: Vec<IdentityPublicKey>,
    ) {
        let identity: Identity = IdentityV0 {
            id: identity_id,
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        }
        .into();
        let mut wm = wallet.identity().wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                identity,
                index,
                wallet.wallet_id(),
                &wallet.identity().persister,
            )
            .unwrap();
    }

    /// The key [`IdentityWallet::select_document_signing_key`] picks for
    /// `identity_id` and a `note` of a HIGH contract, with a signer that
    /// holds every key.
    async fn automatic_key(wallet: &PlatformWallet, identity_id: Identifier) -> u32 {
        let note = note_type(SecurityLevel::HIGH);
        wallet
            .identity()
            .select_document_signing_key(
                &identity_id,
                &note,
                type_scope(&note),
                &KeyFilter(|_| true),
            )
            .await
            .unwrap()
            .id()
    }

    /// A lower-id key that cannot sign this contract's `note` (bound to
    /// another contract, or past its expiry) never shadows a usable later
    /// key, even though the signer holds it: every retry would pick it
    /// again and be refused.
    #[tokio::test]
    async fn should_skip_an_earlier_key_bound_elsewhere_or_expired() {
        let wallet = wallet_with_signing_keys().await;
        let identity_id = Identifier::from([0x31; 32]);
        let mut bound_elsewhere = auth_key(1, SecurityLevel::CRITICAL, KeyType::ECDSA_SECP256K1);
        if let IdentityPublicKey::V0(key) = &mut bound_elsewhere {
            key.contract_bounds = Some(ContractBounds::SingleContract {
                id: Identifier::from([0x0E; 32]),
            });
        }
        hold_identity(
            &wallet,
            identity_id,
            1,
            vec![
                bound_elsewhere,
                auth_key(2, SecurityLevel::CRITICAL, KeyType::ECDSA_SECP256K1)
                    .with_limits(None, Some(1)),
                // Limited too, but unexpired: it sits in the same tier as the
                // expired key 2, so only the expiry check puts it first.
                auth_key(3, SecurityLevel::CRITICAL, KeyType::ECDSA_SECP256K1)
                    .with_limits(None, Some(u64::MAX)),
            ],
        )
        .await;
        assert_eq!(automatic_key(&wallet, identity_id).await, 3);
    }

    /// An ECDSA_HASH160 AUTHENTICATION key signs a document batch as well
    /// as an ECDSA_SECP256K1 one, so an identity whose only qualifying key
    /// is a HASH160 key is not refused.
    #[tokio::test]
    async fn should_select_an_ecdsa_hash160_authentication_key() {
        let wallet = wallet_with_signing_keys().await;
        let identity_id = Identifier::from([0x32; 32]);
        hold_identity(
            &wallet,
            identity_id,
            1,
            vec![
                auth_key(0, SecurityLevel::MASTER, KeyType::ECDSA_SECP256K1),
                auth_key(1, SecurityLevel::HIGH, KeyType::ECDSA_HASH160),
            ],
        )
        .await;
        assert_eq!(automatic_key(&wallet, identity_id).await, 1);
    }

    /// ECDSA_HASH160 is a strict fallback: among limited keys, a later
    /// SECP256K1 key is still taken over an earlier HASH160 key.
    #[tokio::test]
    async fn should_prefer_a_later_secp256k1_key_over_an_earlier_hash160_key() {
        let wallet = wallet_with_signing_keys().await;
        let identity_id = Identifier::from([0x33; 32]);
        hold_identity(
            &wallet,
            identity_id,
            1,
            vec![
                auth_key(1, SecurityLevel::HIGH, KeyType::ECDSA_HASH160)
                    .with_limits(Some(1_000), None),
                auth_key(2, SecurityLevel::HIGH, KeyType::ECDSA_SECP256K1)
                    .with_limits(Some(1_000), None),
            ],
        )
        .await;
        assert_eq!(automatic_key(&wallet, identity_id).await, 2);
    }

    /// An unlimited HASH160 key is taken before a limited SECP256K1 key,
    /// whose budget may already be spent: the tier decides before the type.
    #[tokio::test]
    async fn should_prefer_an_unlimited_hash160_key_over_a_limited_secp256k1_key() {
        let wallet = wallet_with_signing_keys().await;
        let identity_id = Identifier::from([0x35; 32]);
        hold_identity(
            &wallet,
            identity_id,
            1,
            vec![
                auth_key(1, SecurityLevel::HIGH, KeyType::ECDSA_SECP256K1)
                    .with_limits(Some(1_000), None),
                auth_key(2, SecurityLevel::HIGH, KeyType::ECDSA_HASH160),
            ],
        )
        .await;
        assert_eq!(automatic_key(&wallet, identity_id).await, 2);
    }

    /// A SECP256K1 key the signer cannot use does not block the fallback:
    /// an eligible HASH160 key the signer holds signs.
    #[tokio::test]
    async fn should_fall_back_to_a_hash160_key_past_an_unavailable_secp256k1_key() {
        let wallet = wallet_with_signing_keys().await;
        let identity_id = Identifier::from([0x34; 32]);
        hold_identity(
            &wallet,
            identity_id,
            1,
            vec![
                auth_key(1, SecurityLevel::HIGH, KeyType::ECDSA_SECP256K1),
                auth_key(2, SecurityLevel::HIGH, KeyType::ECDSA_HASH160),
            ],
        )
        .await;
        let note = note_type(SecurityLevel::HIGH);
        let key = wallet
            .identity()
            .select_document_signing_key(&identity_id, &note, type_scope(&note), &available(&[2]))
            .await
            .unwrap();
        assert_eq!(key.id(), 2);
    }

    /// A create is signed with a key whose contract bounds cover the type it
    /// creates and every type that create may consume, as Platform checks: a
    /// lower-id key bound to the created type alone is skipped for a later
    /// key bound to the whole contract, and with only the narrowly bound
    /// key the create is refused. A replace of the same type, which consumes
    /// nothing, may use the narrowly bound key.
    #[tokio::test]
    async fn should_sign_a_create_with_a_key_covering_every_type_it_consumes() {
        let contract = dpp::system_data_contracts::load_system_data_contract(
            dpp::data_contracts::SystemDataContract::DPNS,
            PlatformVersion::latest(),
        )
        .expect("bundled DPNS contract");
        let tombstone = contract
            .document_type_borrowed_for_name("tombstone")
            .expect("the tombstone type");
        assert!(
            !tombstone
                .as_ref()
                .consumable_document_type_names()
                .is_empty(),
            "the fixture needs a type whose create consumes another type"
        );
        let bound = |id, bounds| {
            let mut key = auth_key(id, SecurityLevel::CRITICAL, KeyType::ECDSA_SECP256K1);
            if let IdentityPublicKey::V0(key) = &mut key {
                key.contract_bounds = Some(bounds);
            }
            key
        };
        let tombstone_only = || {
            bound(
                1,
                ContractBounds::SingleContractDocumentType {
                    id: contract.id(),
                    document_type_name: "tombstone".to_string(),
                },
            )
        };
        let wallet = wallet_with_signing_keys().await;
        let creator = Identifier::from([0x35; 32]);
        hold_identity(
            &wallet,
            creator,
            1,
            vec![
                tombstone_only(),
                bound(2, ContractBounds::SingleContract { id: contract.id() }),
            ],
        )
        .await;
        let signer = KeyFilter(|_| true);
        let create = wallet
            .identity()
            .select_document_signing_key(&creator, tombstone, create_scope(tombstone), &signer)
            .await
            .unwrap();
        assert_eq!(create.id(), 2);
        let replace = wallet
            .identity()
            .select_document_signing_key(&creator, tombstone, type_scope(tombstone), &signer)
            .await
            .unwrap();
        assert_eq!(replace.id(), 1);

        let narrow = Identifier::from([0x36; 32]);
        hold_identity(&wallet, narrow, 2, vec![tombstone_only()]).await;
        let error = wallet
            .identity()
            .select_document_signing_key(&narrow, tombstone, create_scope(tombstone), &signer)
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::NoUsableSigningKey { .. }),
            "{error:?}"
        );
    }

    /// Records the keys it is asked to sign with, signing with a fixed
    /// signature.
    #[derive(Debug, Default)]
    struct RecordingSigner(std::sync::Mutex<Vec<IdentityPublicKey>>);

    #[async_trait]
    impl Signer<IdentityPublicKey> for RecordingSigner {
        async fn sign(
            &self,
            key: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            self.0.lock().unwrap().push(key.clone());
            Ok(BinaryData::new(vec![7; 65]))
        }

        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "a document transition has no witness".to_string(),
            ))
        }

        fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// The seller [`listed_note`] belongs to; not an identity of the wallet.
    const SELLER: [u8; 32] = [0x52; 32];
    const NOTE_ID: [u8; 32] = [0x4E; 32];

    /// A `note` of [`contract_requiring`] as Platform stores it, owned by
    /// [`SELLER`].
    fn listed_note() -> Document {
        DocumentV0 {
            id: Identifier::from(NOTE_ID),
            owner_id: Identifier::from(SELLER),
            properties: BTreeMap::from([("message".to_string(), Value::Text("hi".to_string()))]),
            revision: Some(1),
            ..Default::default()
        }
        .into()
    }

    /// A wallet on a mock SDK that answers the read of `contract` and of
    /// [`listed_note`], and of `nonce_reader`'s contract nonce when given.
    /// It answers no broadcast.
    async fn trading_wallet(
        contract: &DataContract,
        nonce_reader: Option<Identifier>,
    ) -> Arc<PlatformWallet> {
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract.id(), Some(contract.clone()))
            .await
            .expect("expects the contract read");
        let query = DocumentQuery::new(Arc::new(contract.clone()), "note")
            .expect("the note type")
            .with_document_id(&Identifier::from(NOTE_ID));
        sdk.mock()
            .expect_fetch(query, Some(listed_note()))
            .await
            .expect("expects the document read");
        if let Some(reader) = nonce_reader {
            sdk.mock()
                .expect_fetch(
                    (reader, contract.id()),
                    Some(IdentityContractNonceFetcher(3)),
                )
                .await
                .expect("expects the nonce read");
        }
        wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await
    }

    /// A purchase is signed by the buyer, not by the document's owner: the
    /// buyer's contract nonce is read and the buyer's own key signs. The
    /// seller is not even an identity of the wallet. The mock answers no
    /// broadcast, so the purchase fails after signing.
    #[tokio::test]
    async fn should_sign_a_purchase_with_the_buyers_key() {
        let contract = contract_requiring(SecurityLevel::HIGH);
        let wallet = trading_wallet(&contract, Some(owner())).await;
        let signer = RecordingSigner::default();

        let error = wallet
            .identity()
            .purchase_document_with_signer(
                &owner(),
                &contract.id(),
                "note",
                &Identifier::from(NOTE_ID),
                1_000,
                &signer,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(
            matches!(error, PlatformWalletError::InvalidIdentityData(ref message)
                if message.starts_with("Failed to purchase document")
                    && message.contains("BroadcastStateTransitionRequest")),
            "{error:?}"
        );

        let buyer_key = auth_key(1, SecurityLevel::CRITICAL, KeyType::ECDSA_SECP256K1);
        assert_eq!(*signer.0.lock().unwrap(), vec![buyer_key]);
    }

    /// A transfer of a document another identity owns is refused before
    /// any nonce is read or anything is signed: sent, it would only be a
    /// paid refusal.
    #[tokio::test]
    async fn should_refuse_to_transfer_a_document_another_identity_owns() {
        let contract = contract_requiring(SecurityLevel::HIGH);
        let wallet = trading_wallet(&contract, None).await;
        let signer = RecordingSigner::default();

        let error = wallet
            .identity()
            .transfer_document_with_signer(
                &owner(),
                &contract.id(),
                "note",
                &Identifier::from(NOTE_ID),
                &Identifier::from([0x77; 32]),
                &signer,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::InvalidParameter(ref message)
                if message.contains("is owned by")),
            "{error:?}"
        );
        assert!(signer.0.lock().unwrap().is_empty());
    }

    /// No document write names a key: the wallet signs with the first
    /// owner key the signer can sign with, so a wallet-derived identity (no
    /// key stored per public key) is not refused for want of a key Swift or
    /// Kotlin could see.
    #[tokio::test]
    async fn should_select_the_document_key_the_signer_can_sign_with() {
        let wallet = wallet_with_signing_keys().await;
        let select = |ids: &'static [u32]| {
            let wallet = Arc::clone(&wallet);
            async move {
                let note = note_type(SecurityLevel::HIGH);
                wallet
                    .identity()
                    .select_document_signing_key(
                        &owner(),
                        &note,
                        type_scope(&note),
                        &available(ids),
                    )
                    .await
                    .map(|key| key.id())
            }
        };
        assert_eq!(select(&[2]).await.unwrap(), 2);
        assert_eq!(select(&[1, 2]).await.unwrap(), 1);
    }

    /// A signer that can sign with none of the owner's eligible keys keeps
    /// the structured key-unavailable failure (FFI code 31), and is asked
    /// with the wallet manager lock released.
    #[tokio::test]
    async fn should_refuse_a_document_operation_when_the_signer_holds_no_eligible_key() {
        let wallet = wallet_with_signing_keys().await;
        let note = note_type(SecurityLevel::HIGH);
        let signer = lock_checking_signer(wallet.identity());
        let error = wallet
            .identity()
            .select_document_signing_key(&owner(), &note, type_scope(&note), &signer)
            .await
            .unwrap_err();
        assert_signer_key_unavailable(error);

        let stranger = Identifier::from([5; 32]);
        let error = wallet
            .identity()
            .select_document_signing_key(&stranger, &note, type_scope(&note), &available(&[1]))
            .await
            .unwrap_err();
        assert!(
            matches!(error, PlatformWalletError::IdentityNotFound(id) if id == stranger),
            "{error:?}"
        );
    }

    /// The document type's security requirement bounds the selection: a
    /// HIGH key the signer holds signs a HIGH type but never a CRITICAL one.
    #[tokio::test]
    async fn should_skip_document_keys_below_the_type_requirement() {
        let wallet = wallet_with_signing_keys().await;
        {
            let mut wm = wallet.identity().wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .identity_mut(&owner())
                .unwrap()
                .identity
                .add_public_key(
                    IdentityPublicKeyV0 {
                        id: 0,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::HIGH,
                        key_type: KeyType::ECDSA_SECP256K1,
                        data: vec![9; 33].into(),
                        ..Default::default()
                    }
                    .into(),
                );
        }
        let select = |requirement, ids: &'static [u32]| {
            let wallet = Arc::clone(&wallet);
            async move {
                let note = note_type(requirement);
                wallet
                    .identity()
                    .select_document_signing_key(
                        &owner(),
                        &note,
                        type_scope(&note),
                        &available(ids),
                    )
                    .await
                    .map(|key| key.id())
            }
        };
        assert_eq!(select(SecurityLevel::HIGH, &[0, 2]).await.unwrap(), 0);
        assert_eq!(select(SecurityLevel::CRITICAL, &[0, 2]).await.unwrap(), 2);
        assert_signer_key_unavailable(select(SecurityLevel::CRITICAL, &[0]).await.unwrap_err());
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

    #[tokio::test]
    async fn should_sign_a_document_write_for_an_identity_holding_only_hash160_keys() {
        use dpp::bls::native_bls::NativeBlsModule;
        use dpp::data_contracts::SystemDataContract;
        use dpp::document::DocumentV0;
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use dpp::system_data_contracts::load_system_data_contract;
        use dpp::util::hash::ripemd160_sha256;
        use dpp::version::PlatformVersion;
        use simple_signer::signer::SimpleSigner;

        // The default key layout of identities older Dash Evo Tool versions created: every
        // key a HASH160 of its public key
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();
        let keys: Vec<IdentityPublicKey> = [
            (0, SecurityLevel::MASTER),
            (1, SecurityLevel::CRITICAL),
            (2, SecurityLevel::HIGH),
        ]
        .into_iter()
        .map(|(id, security_level)| {
            let private_key = [id as u8 + 1; 32];
            let secret = dpp::secp256k1::SecretKey::from_secret_bytes(private_key)
                .expect("a valid secret key");
            let public_key = secret.public_key().serialize();
            let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id,
                purpose: Purpose::AUTHENTICATION,
                security_level,
                contract_bounds: None,
                key_type: KeyType::ECDSA_HASH160,
                read_only: false,
                data: BinaryData::new(ripemd160_sha256(&public_key).to_vec()),
                disabled_at: None,
            });
            signer.add_identity_public_key(key.clone(), private_key);
            key
        })
        .collect();
        let identity = identity(keys);

        let dashpay = load_system_data_contract(SystemDataContract::Dashpay, platform_version)
            .expect("the DashPay contract loads");
        let profile = dashpay
            .document_type_for_name("profile")
            .expect("the profile type");
        let key = document_signing_key(
            &identity,
            &signer,
            profile.security_level_requirement(),
            SigningScope::DocumentCreate {
                document_type: profile,
            },
        )
        .expect("the signer reaches the keys")
        .expect("a HASH160 key signs a document write");
        assert_eq!(key.id(), 2, "the HIGH key, as the profile type requires");

        let document = Document::V0(DocumentV0 {
            id: Identifier::from([7; 32]),
            owner_id: identity.id(),
            properties: BTreeMap::from([(
                "displayName".to_string(),
                Value::Text("Alice".to_string()),
            )]),
            ..Default::default()
        });
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            profile,
            [9; 32],
            key,
            1,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("the HASH160 key signs the batch");
        transition
            .verify_identity_signed_signature(key, &NativeBlsModule, platform_version)
            .expect("Platform verifies the signature by the HASH160 key");
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
