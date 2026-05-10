//! Document state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for document operations like create, replace, delete, etc.
//!
//! # Two-Phase API (Prepare + Execute)
//!
//! In addition to the all-in-one methods (`documentCreate`, `documentReplace`, `documentDelete`),
//! this module provides `prepare_*` variants that build and sign a `StateTransition` without
//! broadcasting it. This enables idempotent retry patterns:
//!
//! 1. Call `prepareDocumentCreate()` to get a signed `StateTransition`
//! 2. Cache `stateTransition.toBytes()` for retry safety
//! 3. Call `broadcastStateTransition(st)` + `waitForResponse(st)`
//! 4. On timeout, deserialize cached bytes and rebroadcast the **identical** ST
//!
//! This avoids the duplicate state transition problem that occurs when retrying
//! the all-in-one methods after a timeout (which would create a new ST with a new nonce).
//!
//! ## Nonce consumption
//!
//! Every successful `prepareDocument*` call resolves the next identity-contract nonce
//! for this SDK instance and advances its local nonce cache. If that cache is empty
//! or stale, the SDK may first fetch the current nonce from Platform. The signed state
//! transition embeds that nonce, but Platform state is not mutated until the transition
//! is actually broadcast and processed.
//!
//! Only call `prepareDocument*` when you intend to broadcast the returned transition
//! (or persist the bytes and retry broadcasting that exact transition). Discarding a
//! prepared transition leaves this SDK instance's local nonce cache ahead until it is
//! refreshed, but it does not reserve or consume the nonce remotely on Platform. If
//! you need a "dry run" with no local nonce-cache side effects in this SDK instance,
//! do not use the prepare API.
//!
//! ### Pre-broadcast failures
//!
//! If a `prepareDocument*` call fails *before* the transition is broadcast (build,
//! sign, or local structure validation error), the bumped identity-contract nonce is
//! conditionally rolled back via rs-sdk's
//! [`Sdk::rollback_identity_contract_nonce`](dash_sdk::Sdk::rollback_identity_contract_nonce).
//! The rollback only adjusts the cache entry if it still equals the nonce allocated
//! by the failed attempt, so it does not clobber concurrent allocations. This makes
//! these errors safe to retry: a follow-up `prepareDocument*` call will reuse the
//! freed nonce instead of skipping it. Broadcast failures (which happen *after*
//! `prepareDocument*` returns) intentionally do **not** roll back, because the
//! network may have already observed the nonce.
//!
//! ## One-shot document revision rules
//!
//! `documentCreate()` now accepts only documents whose revision is unset or
//! `INITIAL_REVISION`, and `documentReplace()` now accepts only documents whose
//! revision is greater than `INITIAL_REVISION`.
//!
//! Earlier wasm-sdk behavior could silently route invalid revisions to the other
//! transition type. That implicit routing is no longer performed: invalid
//! revisions now fail with `InvalidArgument` instead.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use crate::settings::PutSettingsInput;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::document::{Document, DocumentV0Getters, INITIAL_REVISION};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;
use dash_sdk::platform::documents::transitions::DocumentDeleteTransitionBuilder;
use dash_sdk::platform::transition::purchase_document::PurchaseDocument;
use dash_sdk::platform::transition::put_document::{
    build_signed_document_create_transition, build_signed_document_replace_transition, PutDocument,
};
use dash_sdk::platform::transition::transfer_document::TransferDocument;
use dash_sdk::platform::transition::update_price_of_document::UpdatePriceOfDocument;
use dash_sdk::platform::transition::validation::ensure_valid_state_transition_structure;
use js_sys::Reflect;
use std::sync::Arc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::state_transitions::batch::token_payment_info::{
    TokenPaymentInfoOptionsJs, TokenPaymentInfoWasm,
};
use wasm_dpp2::utils::{
    get_class_type, try_from_options_optional, try_from_options_with, try_to_string, try_to_u64,
    IntoWasm,
};
use wasm_dpp2::IdentitySignerWasm;
use wasm_dpp2::StateTransitionWasm;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_PAYMENT_INFO_TS: &str = r#"
/**
 * Token-based payment metadata for document actions that require token cost agreement.
 */
export interface DocumentTokenPaymentInfo {
  /**
   * Optional external token contract ID.
   * If omitted, the token is expected to come from the current document contract.
   */
  paymentTokenContractId?: IdentifierLike;

  /**
   * Token position within the token contract.
   */
  tokenContractPosition: number;

  /**
   * Optional minimum token amount the payer agrees to spend.
   */
  minimumTokenCost?: bigint;

  /**
   * Optional maximum token amount the payer agrees to spend.
   */
  maximumTokenCost?: bigint;

  /**
   * Which party covers gas fees for the document action.
   */
  gasFeesPaidBy?: GasFeesPaidByLike;
}
"#;

fn try_from_options_optional_token_payment_info(
    options: &JsValue,
) -> Result<Option<TokenPaymentInfo>, WasmSdkError> {
    let token_payment_info_value = Reflect::get(options, &JsValue::from_str("tokenPaymentInfo"))
        .map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "Failed to read tokenPaymentInfo option: {:?}",
                err
            ))
        })?;

    if token_payment_info_value.is_null() || token_payment_info_value.is_undefined() {
        return Ok(None);
    }

    let token_payment_info = TokenPaymentInfoWasm::constructor(
        token_payment_info_value.unchecked_into::<TokenPaymentInfoOptionsJs>(),
    )
    .map_err(|err| WasmSdkError::invalid_argument(err.to_string()))?;

    Ok(Some(token_payment_info.into()))
}

// ============================================================================
// Document Create
// ============================================================================

/// TypeScript interface for document create options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_CREATE_OPTIONS_TS: &'static str = r#"
/**
 * Options for creating a new document on Dash Platform.
 */
export interface DocumentCreateOptions {
  /**
   * The document to create.
   * Use `new Document(...)` or `Document.fromJSON(...)` to construct it.
   * Must include dataContractId, documentTypeName, ownerId, and entropy.
   * Revision must be omitted or set to 1 (INITIAL_REVISION).
   * Other revisions are rejected with InvalidArgument instead of being routed
   * to documentReplace().
   */
  document: Document;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the owner identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional token payment agreement for document types with tokenCost.create.
   */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentCreateOptions")]
    pub type DocumentCreateOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Create a new document on Dash Platform.
    ///
    /// This method handles the complete document creation flow:
    /// 1. Fetches the data contract from Platform
    /// 2. Validates the document data against the document type schema
    /// 3. Creates and signs the document create transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// The document revision must be unset or `INITIAL_REVISION`. Revisions
    /// greater than `INITIAL_REVISION` now return `InvalidArgument` instead of
    /// being routed to `documentReplace()`.
    ///
    /// @param options - Creation options including document, identity key, and signer
    /// @returns Promise that resolves when the document is created
    #[wasm_bindgen(js_name = "documentCreate")]
    pub async fn document_create(
        &self,
        options: DocumentCreateOptionsJs,
    ) -> Result<(), WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        ensure_document_create_revision(document.revision(), "documentReplace")?;

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Get entropy from document
        let entropy = document_wasm.entropy().ok_or_else(|| {
            WasmSdkError::invalid_argument("Document must have entropy set for creation")
        })?;

        if entropy.len() != 32 {
            return Err(WasmSdkError::invalid_argument(
                "Document entropy must be exactly 32 bytes",
            ));
        }

        let mut entropy_array = [0u8; 32];
        entropy_array.copy_from_slice(&entropy);

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Use PutDocument trait for creation
        document
            .put_to_platform_and_wait_for_response(
                self.inner_sdk(),
                document_type,
                Some(entropy_array),
                identity_key,
                token_payment_info,
                &signer,
                settings,
            )
            .await?;

        Ok(())
    }
}

// ============================================================================
// Document Replace
// ============================================================================

/// TypeScript interface for document replace options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_REPLACE_OPTIONS_TS: &'static str = r#"
/**
 * Options for replacing an existing document on Dash Platform.
 */
export interface DocumentReplaceOptions {
  /**
   * The document with updated data.
   * Must have the same ID as the existing document.
   * Revision must be set to current revision + 1 and therefore be greater than
   * 1 (INITIAL_REVISION). Missing, 0, or 1 revisions are rejected with
   * InvalidArgument instead of being routed to documentCreate().
   */
  document: Document;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the owner identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional token payment agreement for document types with tokenCost.replace.
   */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentReplaceOptions")]
    pub type DocumentReplaceOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Replace an existing document on Dash Platform.
    ///
    /// This method handles the complete document replacement flow:
    /// 1. Fetches the data contract from Platform
    /// 2. Validates the new document data against the document type schema
    /// 3. Creates and signs the document replace transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// The document revision must be greater than `INITIAL_REVISION`. Missing
    /// or initial revisions now return `InvalidArgument` instead of being
    /// routed to `documentCreate()`.
    ///
    /// @param options - Replace options including document, identity key, and signer
    /// @returns Promise that resolves when the document is replaced
    #[wasm_bindgen(js_name = "documentReplace")]
    pub async fn document_replace(
        &self,
        options: DocumentReplaceOptionsJs,
    ) -> Result<(), WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        ensure_document_replace_revision(document.revision(), "documentCreate")?;

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Use PutDocument trait for replacement (revision > INITIAL_REVISION triggers replace)
        document
            .put_to_platform_and_wait_for_response(
                self.inner_sdk(),
                document_type,
                None, // entropy not needed for replace
                identity_key,
                token_payment_info,
                &signer,
                settings,
            )
            .await?;

        Ok(())
    }
}

// ============================================================================
// Document Delete
// ============================================================================

/// TypeScript interface for document delete options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_DELETE_OPTIONS_TS: &'static str = r#"
/**
 * Options for deleting a document from Dash Platform.
 */
export interface DocumentDeleteOptions {
  /**
   * The document to delete - either a Document instance or an object with identifiers.
   *
   * @example
   * // Using a Document instance
   * { document: myDocument, ... }
   *
   * // Using individual fields
   * { document: { id: "...", ownerId: "...", dataContractId: "...", documentTypeName: "note" }, ... }
   */
  document: Document | {
    id: IdentifierLike;
    ownerId: IdentifierLike;
    dataContractId: IdentifierLike;
    documentTypeName: string;
  };

  /**
   * The identity public key to use for signing the transition.
   * Get this from the owner identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional token payment agreement for document types with tokenCost.delete.
   */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentDeleteOptions")]
    pub type DocumentDeleteOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Delete a document from Dash Platform.
    ///
    /// This method handles the complete document deletion flow:
    /// 1. Fetches the data contract from Platform
    /// 2. Creates and signs the document delete transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Delete options including document (or document identifiers), identity key, and signer
    /// @returns Promise that resolves when the document is deleted
    #[wasm_bindgen(js_name = "documentDelete")]
    pub async fn document_delete(
        &self,
        options: DocumentDeleteOptionsJs,
    ) -> Result<(), WasmSdkError> {
        // Extract document field - can be either a Document instance or plain object
        let document_js = js_sys::Reflect::get(&options, &JsValue::from_str("document"))
            .map_err(|_| WasmSdkError::invalid_argument("document is required"))?;

        if document_js.is_undefined() || document_js.is_null() {
            return Err(WasmSdkError::invalid_argument("document is required"));
        }

        // Check if it's a Document instance or a plain object with fields
        let (document_id, owner_id, contract_id, document_type_name): (
            Identifier,
            Identifier,
            Identifier,
            String,
        ) = if get_class_type(&document_js).ok().as_deref() == Some("Document") {
            // It's a Document instance - extract fields from it
            let doc: DocumentWasm = document_js
                .to_wasm::<DocumentWasm>("Document")
                .map(|boxed| (*boxed).clone())?;
            let doc_inner: Document = doc.clone().into();
            (
                doc.id().into(),
                doc_inner.owner_id(),
                doc.data_contract_id().into(),
                doc.document_type_name(),
            )
        } else {
            // It's a plain object - extract individual fields
            (
                IdentifierWasm::try_from_options(&document_js, "id")?.into(),
                IdentifierWasm::try_from_options(&document_js, "ownerId")?.into(),
                IdentifierWasm::try_from_options(&document_js, "dataContractId")?.into(),
                try_from_options_with(&document_js, "documentTypeName", |v| {
                    try_to_string(v, "documentTypeName")
                })?,
            )
        };

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Build and execute delete transition using DocumentDeleteTransitionBuilder
        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::new(data_contract),
            document_type_name,
            document_id,
            owner_id,
        );

        let builder = if let Some(token_payment_info) = token_payment_info {
            builder.with_token_payment_info(token_payment_info)
        } else {
            builder
        };

        let builder = if let Some(s) = settings {
            builder.with_settings(s)
        } else {
            builder
        };

        self.inner_sdk()
            .document_delete(builder, &identity_key, &signer)
            .await?;

        Ok(())
    }
}

// ============================================================================
// Prepare Document Create (Two-Phase API)
// ============================================================================

/// TypeScript interface for prepare document create options
#[wasm_bindgen(typescript_custom_section)]
const PREPARE_DOCUMENT_CREATE_OPTIONS_TS: &'static str = r#"
/**
 * Options for preparing a document creation state transition without broadcasting.
 *
 * Use this for idempotent retry patterns:
 * 1. Call `prepareDocumentCreate()` to get a signed `StateTransition`
 * 2. Cache `stateTransition.toBytes()` for retry safety
 * 3. Call `broadcastStateTransition(st)` + `waitForResponse(st)`
 * 4. On timeout, deserialize cached bytes and rebroadcast the **identical** ST
 */
export interface PrepareDocumentCreateOptions {
  /** The document to create. */
  document: Document;
  /** The identity public key to use for signing. */
  identityKey: IdentityPublicKey;
  /** Signer containing the private key for the identity key. */
  signer: IdentitySigner;
  /** Optional token payment agreement for document types with tokenCost.create. */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;
  /** Optional settings (retries, timeouts, userFeeIncrease). */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PrepareDocumentCreateOptions")]
    pub type PrepareDocumentCreateOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Prepare a document creation state transition without broadcasting.
    ///
    /// This method handles nonce management, ST construction, and signing, but does
    /// **not** broadcast or wait for a response. The returned `StateTransition` can be:
    ///
    /// - Serialized with `toBytes()` and cached for retry safety
    /// - Broadcast with `broadcastStateTransition(st)`
    /// - Awaited with `waitForResponse(st)`
    ///
    /// This is the "prepare" half of the two-phase API. Use it when you need
    /// idempotent retry behavior — on timeout, you can rebroadcast the exact same
    /// signed transition instead of creating a new one with a new nonce.
    ///
    /// **Nonce consumption:** A successful call advances this SDK instance's local
    /// identity-contract nonce cache and embeds that nonce in the signed transition.
    /// Platform state is not mutated until broadcast/processing. Only call this when
    /// you intend to broadcast / persist-and-retry the returned transition. See module
    /// docs for details.
    ///
    /// @param options - Creation options including document, identity key, and signer
    /// @returns The signed StateTransition ready for broadcasting
    #[wasm_bindgen(js_name = "prepareDocumentCreate")]
    pub async fn prepare_document_create(
        &self,
        options: PrepareDocumentCreateOptionsJs,
    ) -> Result<StateTransitionWasm, WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        ensure_document_create_revision(document.revision(), "prepareDocumentReplace")?;

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Get entropy from document
        let entropy = document_wasm.entropy().ok_or_else(|| {
            WasmSdkError::invalid_argument("Document must have entropy set for creation")
        })?;

        if entropy.len() != 32 {
            return Err(WasmSdkError::invalid_argument(
                "Document entropy must be exactly 32 bytes",
            ));
        }

        let mut entropy_array = [0u8; 32];
        entropy_array.copy_from_slice(&entropy);

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Build, sign, and structurally validate the state transition without
        // broadcasting it. Local pre-broadcast failures are rolled back inside
        // rs-sdk so the identity-contract nonce cache cannot advance past a
        // nonce the network never observed.
        let state_transition = build_signed_document_create_transition(
            self.inner_sdk(),
            &document,
            &document_type,
            entropy_array,
            &identity_key,
            token_payment_info,
            &signer,
            settings,
        )
        .await?;

        Ok(state_transition.into())
    }
}

// ============================================================================
// Prepare Document Replace (Two-Phase API)
// ============================================================================

/// TypeScript interface for prepare document replace options
#[wasm_bindgen(typescript_custom_section)]
const PREPARE_DOCUMENT_REPLACE_OPTIONS_TS: &'static str = r#"
/**
 * Options for preparing a document replace state transition without broadcasting.
 *
 * Use this for idempotent retry patterns. See `prepareDocumentCreate` for the full pattern.
 */
export interface PrepareDocumentReplaceOptions {
  /** The document with updated data (same ID, incremented revision). */
  document: Document;
  /** The identity public key to use for signing. */
  identityKey: IdentityPublicKey;
  /** Signer containing the private key for the identity key. */
  signer: IdentitySigner;
  /** Optional token payment agreement for document types with tokenCost.replace. */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;
  /** Optional settings (retries, timeouts, userFeeIncrease). */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PrepareDocumentReplaceOptions")]
    pub type PrepareDocumentReplaceOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Prepare a document replace state transition without broadcasting.
    ///
    /// This method handles nonce management, ST construction, and signing, but does
    /// **not** broadcast or wait for a response. See `prepareDocumentCreate` for
    /// the full two-phase usage pattern.
    ///
    /// **Nonce consumption:** A successful call advances this SDK instance's local
    /// identity-contract nonce cache and embeds that nonce in the signed transition.
    /// Platform state is not mutated until broadcast/processing. Only call this when
    /// you intend to broadcast / persist-and-retry the returned transition. See module
    /// docs for details.
    ///
    /// @param options - Replace options including document, identity key, and signer
    /// @returns The signed StateTransition ready for broadcasting
    #[wasm_bindgen(js_name = "prepareDocumentReplace")]
    pub async fn prepare_document_replace(
        &self,
        options: PrepareDocumentReplaceOptionsJs,
    ) -> Result<StateTransitionWasm, WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        ensure_document_replace_revision(document.revision(), "prepareDocumentCreate")?;

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Build, sign, and structurally validate the state transition without
        // broadcasting it. Local pre-broadcast failures are rolled back inside
        // rs-sdk so the identity-contract nonce cache cannot advance past a
        // nonce the network never observed.
        let state_transition = build_signed_document_replace_transition(
            self.inner_sdk(),
            &document,
            &document_type,
            &identity_key,
            token_payment_info,
            &signer,
            settings,
        )
        .await?;

        Ok(state_transition.into())
    }
}

// ============================================================================
// Prepare Document Delete (Two-Phase API)
// ============================================================================

/// TypeScript interface for prepare document delete options
#[wasm_bindgen(typescript_custom_section)]
const PREPARE_DOCUMENT_DELETE_OPTIONS_TS: &'static str = r#"
/**
 * Options for preparing a document delete state transition without broadcasting.
 *
 * Use this for idempotent retry patterns. See `prepareDocumentCreate` for the full pattern.
 */
export interface PrepareDocumentDeleteOptions {
  /**
   * The document to delete — either a Document instance or an object with identifiers.
   */
  document: Document | {
    id: IdentifierLike;
    ownerId: IdentifierLike;
    dataContractId: IdentifierLike;
    documentTypeName: string;
  };
  /** The identity public key to use for signing. */
  identityKey: IdentityPublicKey;
  /** Signer containing the private key for the identity key. */
  signer: IdentitySigner;
  /** Optional token payment agreement for document types with tokenCost.delete. */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;
  /** Optional settings (retries, timeouts, userFeeIncrease). */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PrepareDocumentDeleteOptions")]
    pub type PrepareDocumentDeleteOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Prepare a document delete state transition without broadcasting.
    ///
    /// This method handles nonce management, ST construction, and signing, but does
    /// **not** broadcast or wait for a response. See `prepareDocumentCreate` for
    /// the full two-phase usage pattern.
    ///
    /// **Nonce consumption:** A successful call advances this SDK instance's local
    /// identity-contract nonce cache and embeds that nonce in the signed transition.
    /// Platform state is not mutated until broadcast/processing. Only call this when
    /// you intend to broadcast / persist-and-retry the returned transition. See module
    /// docs for details.
    ///
    /// @param options - Delete options including document identifiers, identity key, and signer
    /// @returns The signed StateTransition ready for broadcasting
    #[wasm_bindgen(js_name = "prepareDocumentDelete")]
    pub async fn prepare_document_delete(
        &self,
        options: PrepareDocumentDeleteOptionsJs,
    ) -> Result<StateTransitionWasm, WasmSdkError> {
        // Extract document field - can be either a Document instance or plain object
        let document_js = js_sys::Reflect::get(&options, &JsValue::from_str("document"))
            .map_err(|_| WasmSdkError::invalid_argument("document is required"))?;

        if document_js.is_undefined() || document_js.is_null() {
            return Err(WasmSdkError::invalid_argument("document is required"));
        }

        // Check if it's a Document instance or a plain object with fields
        let (document_id, owner_id, contract_id, document_type_name): (
            Identifier,
            Identifier,
            Identifier,
            String,
        ) = if get_class_type(&document_js).ok().as_deref() == Some("Document") {
            let doc: DocumentWasm = document_js
                .to_wasm::<DocumentWasm>("Document")
                .map(|boxed| (*boxed).clone())?;
            let doc_inner: Document = doc.clone().into();
            (
                doc.id().into(),
                doc_inner.owner_id(),
                doc.data_contract_id().into(),
                doc.document_type_name(),
            )
        } else {
            (
                IdentifierWasm::try_from_options(&document_js, "id")?.into(),
                IdentifierWasm::try_from_options(&document_js, "ownerId")?.into(),
                IdentifierWasm::try_from_options(&document_js, "dataContractId")?.into(),
                try_from_options_with(&document_js, "documentTypeName", |v| {
                    try_to_string(v, "documentTypeName")
                })?,
            )
        };

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Build the delete transition using the builder's sign method (which does NOT broadcast)
        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::new(data_contract),
            document_type_name,
            document_id,
            owner_id,
        );

        let builder = if let Some(token_payment_info) = token_payment_info {
            builder.with_token_payment_info(token_payment_info)
        } else {
            builder
        };

        let builder = if let Some(s) = settings {
            builder.with_settings(s)
        } else {
            builder
        };

        // Pre-allocate the identity-contract nonce so that any pre-broadcast
        // failure (sign or local structure validation) can be rolled back via
        // rs-sdk's `rollback_identity_contract_nonce`. The rollback is
        // conditional: it only adjusts the cache entry if it still equals the
        // nonce allocated here, so it does not clobber concurrent allocations.
        let allocated_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(owner_id, contract_id, true, settings)
            .await?;

        let state_transition = match builder
            .sign_with_nonce(
                allocated_nonce,
                &identity_key,
                &signer,
                self.inner_sdk().version(),
            )
            .await
        {
            Ok(st) => st,
            Err(err) => {
                self.inner_sdk()
                    .rollback_identity_contract_nonce(owner_id, contract_id, allocated_nonce)
                    .await;
                return Err(err.into());
            }
        };

        // Validate structure before handing the ST back, mirroring rs-sdk's
        // pre-broadcast check. For document Batch transitions this currently
        // ends up as a no-op because DPP returns UnsupportedFeatureError until
        // that structure validation is implemented there.
        if let Err(err) =
            ensure_valid_state_transition_structure(&state_transition, self.inner_sdk().version())
        {
            self.inner_sdk()
                .rollback_identity_contract_nonce(owner_id, contract_id, allocated_nonce)
                .await;
            return Err(err.into());
        }

        Ok(state_transition.into())
    }
}

// ============================================================================
// Document Transfer
// ============================================================================

/// TypeScript interface for document transfer options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TRANSFER_OPTIONS_TS: &'static str = r#"
/**
 * Options for transferring a document to another identity.
 */
export interface DocumentTransferOptions {
  /**
   * The document to transfer.
   * Must include id, ownerId, dataContractId, documentTypeName, and revision.
   */
  document: Document;

  /**
   * The new owner's identity ID.
   */
  recipientId: Identifier;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the owner identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional token payment agreement for document types with tokenCost.transfer.
   */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentTransferOptions")]
    pub type DocumentTransferOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Transfer a document to another identity.
    ///
    /// This method handles the complete document transfer flow:
    /// 1. Fetches the data contract from Platform
    /// 2. Creates and signs the document transfer transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Transfer options including document, recipient, and signer
    /// @returns Promise that resolves when the document is transferred
    #[wasm_bindgen(js_name = "documentTransfer")]
    pub async fn document_transfer(
        &self,
        options: DocumentTransferOptionsJs,
    ) -> Result<(), WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let owner_id: Identifier = document.owner_id();
        let document_type_name = document_wasm.document_type_name();

        // Extract recipient ID from options
        let recipient_id: Identifier =
            IdentifierWasm::try_from_options(&options, "recipientId")?.into();

        // Validate not transferring to self
        if owner_id == recipient_id {
            return Err(WasmSdkError::invalid_argument(
                "Cannot transfer document to yourself",
            ));
        }

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Use TransferDocument trait
        document
            .transfer_document_to_identity_and_wait_for_response(
                recipient_id,
                self.inner_sdk(),
                document_type,
                identity_key,
                token_payment_info,
                &signer,
                settings,
            )
            .await?;

        Ok(())
    }
}

// ============================================================================
// Document Purchase
// ============================================================================

/// TypeScript interface for document purchase options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PURCHASE_OPTIONS_TS: &'static str = r#"
/**
 * Options for purchasing a document that has a price set.
 */
export interface DocumentPurchaseOptions {
  /**
   * The document to purchase.
   * Must include id, ownerId, dataContractId, documentTypeName, and revision.
   */
  document: Document;

  /**
   * The buyer's identity ID.
   */
  buyerId: Identifier;

  /**
   * The purchase price in credits.
   * Must match the document's listed price.
   */
  price: bigint;

  /**
   * The public key to use for signing the transition.
   * Get this from the buyer identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional token payment agreement for document types with tokenCost.purchase.
   */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentPurchaseOptions")]
    pub type DocumentPurchaseOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Purchase a document that has a price set.
    ///
    /// This method handles the complete document purchase flow:
    /// 1. Fetches the data contract from Platform
    /// 2. Creates and signs the document purchase transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Purchase options including document, buyer ID, price, and signer
    /// @returns Promise that resolves when the purchase is complete
    #[wasm_bindgen(js_name = "documentPurchase")]
    pub async fn document_purchase(
        &self,
        options: DocumentPurchaseOptionsJs,
    ) -> Result<(), WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Extract buyer ID from options
        let buyer_id: Identifier = IdentifierWasm::try_from_options(&options, "buyerId")?.into();

        // Extract price from options
        let price: Credits = try_from_options_with(&options, "price", |v| try_to_u64(v, "price"))?;

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Use PurchaseDocument trait
        document
            .purchase_document_and_wait_for_response(
                price,
                self.inner_sdk(),
                document_type,
                buyer_id,
                identity_key,
                token_payment_info,
                &signer,
                settings,
            )
            .await?;

        Ok(())
    }
}

// ============================================================================
// Document Set Price
// ============================================================================

/// TypeScript interface for document set price options
#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_SET_PRICE_OPTIONS_TS: &'static str = r#"
/**
 * Options for setting a price on a document to enable purchases.
 */
export interface DocumentSetPriceOptions {
  /**
   * The document to set a price on.
   * Must include id, ownerId, dataContractId, documentTypeName, and revision.
   */
  document: Document;

  /**
   * The price in credits.
   * Set to 0 to remove the price and make the document not for sale.
   */
  price: bigint;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the owner identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional token payment agreement for document types with tokenCost.update_price.
   */
  tokenPaymentInfo?: DocumentTokenPaymentInfo;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentSetPriceOptions")]
    pub type DocumentSetPriceOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Set a price on a document to enable purchases.
    ///
    /// This method handles the complete price setting flow:
    /// 1. Fetches the data contract from Platform
    /// 2. Creates and signs the price update transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Set price options including document, price, and signer
    /// @returns Promise that resolves when the price is set
    #[wasm_bindgen(js_name = "documentSetPrice")]
    pub async fn document_set_price(
        &self,
        options: DocumentSetPriceOptionsJs,
    ) -> Result<(), WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Extract price from options
        let price: Credits = try_from_options_with(&options, "price", |v| try_to_u64(v, "price"))?;

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Use UpdatePriceOfDocument trait
        document
            .update_price_of_document_and_wait_for_response(
                price,
                self.inner_sdk(),
                document_type,
                identity_key,
                token_payment_info,
                &signer,
                settings,
            )
            .await?;

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn ensure_document_create_revision(
    revision: Option<u64>,
    replace_api_name: &str,
) -> Result<(), WasmSdkError> {
    if let Some(revision) = revision {
        if revision != INITIAL_REVISION {
            return Err(WasmSdkError::invalid_argument(format!(
                "Document revision is {} but create requires revision to be unset or {}. Use {} for existing documents.",
                revision, INITIAL_REVISION, replace_api_name,
            )));
        }
    }

    Ok(())
}

fn ensure_document_replace_revision(
    revision: Option<u64>,
    create_api_name: &str,
) -> Result<(), WasmSdkError> {
    let revision = revision.ok_or_else(|| {
        WasmSdkError::invalid_argument(format!(
            "Document must have a revision set for replace. Use {} for new documents.",
            create_api_name,
        ))
    })?;

    if revision <= INITIAL_REVISION {
        return Err(WasmSdkError::invalid_argument(format!(
            "Document revision is {} but replace requires revision > {}. Use {} for new documents.",
            revision, INITIAL_REVISION, create_api_name,
        )));
    }

    Ok(())
}

/// Get an owned DocumentType from a DataContract
fn get_document_type(
    data_contract: &dash_sdk::platform::DataContract,
    document_type_name: &str,
) -> Result<DocumentType, WasmSdkError> {
    data_contract
        .document_type_cloned_for_name(document_type_name)
        .map_err(|e| {
            WasmSdkError::not_found(format!(
                "Document type '{}' not found: {}",
                document_type_name, e
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dash_sdk::dpp::state_transition::StateTransition;
    use dash_sdk::dpp::version::PlatformVersion;

    #[test]
    fn create_revision_guard_accepts_none_and_initial_revision() {
        assert!(ensure_document_create_revision(None, "prepareDocumentReplace").is_ok());
        assert!(
            ensure_document_create_revision(Some(INITIAL_REVISION), "prepareDocumentReplace")
                .is_ok()
        );
    }

    #[test]
    fn create_revision_guard_rejects_non_initial_revision() {
        let err = ensure_document_create_revision(Some(0), "prepareDocumentReplace")
            .expect_err("revision 0 should fail");
        assert!(err.to_string().contains("prepareDocumentReplace"));
        assert!(err.to_string().contains("create requires revision"));
    }

    #[test]
    fn replace_revision_guard_accepts_only_greater_than_initial_revision() {
        assert!(ensure_document_replace_revision(
            Some(INITIAL_REVISION + 1),
            "prepareDocumentCreate"
        )
        .is_ok());
    }

    #[test]
    fn replace_revision_guard_rejects_missing_or_initial_revision() {
        let missing = ensure_document_replace_revision(None, "prepareDocumentCreate")
            .expect_err("missing revision should fail");
        assert!(missing.to_string().contains("prepareDocumentCreate"));

        let initial =
            ensure_document_replace_revision(Some(INITIAL_REVISION), "prepareDocumentCreate")
                .expect_err("initial revision should fail");
        assert!(initial.to_string().contains("prepareDocumentCreate"));
        assert!(initial.to_string().contains("replace requires revision"));
    }

    /// Regression test for the UnsupportedFeatureError pass-through path.
    ///
    /// DPP's `validate_structure` implementation returns `UnsupportedFeatureError`
    /// for identity-based state transitions (see rs-dpp `state_transition/mod.rs`
    /// `StateTransitionStructureValidation` impl). rs-sdk intentionally allows
    /// these through before broadcasting; the prepare APIs delegate to that
    /// shared helper, so we sanity-check the same behavior here against the
    /// public API to guard against regressions if the helper relocates.
    #[test]
    fn validate_accepts_unsupported_feature_errors() {
        let version = PlatformVersion::latest();
        let st: StateTransition = IdentityCreditTransferTransition::default_versioned(version)
            .expect("default versioned ICT transition")
            .into();
        assert!(
            ensure_valid_state_transition_structure(&st, version).is_ok(),
            "identity-based STs should pass through via UnsupportedFeatureError"
        );
    }
}
