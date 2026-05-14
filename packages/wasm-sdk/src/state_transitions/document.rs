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
//!
//! ## Document id ↔ entropy invariant (create paths)
//!
//! `documentCreate()` / `prepareDocumentCreate()` now require that the
//! document's `id` matches the id derived from
//! `(dataContractId, ownerId, documentTypeName, entropy)` via the v0 document
//! id derivation. Mismatches are rejected with `InvalidArgument` **before**
//! any identity-contract nonce is allocated, so failed calls do not advance
//! the local nonce cache.
//!
//! **Migration / compatibility note:** earlier wasm-sdk behavior accepted any
//! `id` and silently embedded the document under whatever id the caller had
//! set. If you previously built a `Document` with a hand-picked id and a
//! separate entropy value, you must now either let the `Document`
//! constructor derive both together (its default behavior) or call
//! `Document.generateId(...)` with the same entropy you intend to use.
//!
//! ## Trusted wasm-dpp2 producers: `identityKey` and `signer`
//!
//! Both the prepare and one-shot APIs deliberately accept `identityKey`
//! and `signer` only as their real wasm-dpp2 class instances and read
//! them through `IdentityPublicKeyWasm::try_from_options` /
//! `IdentitySignerWasm::try_from_options`. Those conversions trust the
//! wasm-bindgen `__wbg_ptr` field on the value.
//!
//! This is an intentional carve-out from the structural-extraction
//! hardening applied to `document` / `paymentTokenContractId` / etc.,
//! because:
//!
//! * `IdentitySigner` is an opaque handle that owns key material plus
//!   the in-Rust signing-callback state. There is no public JS field
//!   surface to reconstruct it from — by design, callers must not have
//!   direct access to the raw private key bytes. Any "structural"
//!   extraction would have to invent a new producer API for the signer,
//!   which would be a strictly weaker security boundary than just
//!   refusing non-wasm-dpp2 inputs.
//! * `IdentityPublicKey` is conceptually copyable, but it is only ever
//!   produced inside the SDK (e.g. fetched from Platform, parsed from a
//!   contract, derived from the signer) and immediately handed back to
//!   the SDK for the same call. It never crosses an untrusted boundary
//!   in the way `document` does (which callers freely build from
//!   JSON/object literals).
//!
//! Callers must therefore pass wasm-dpp2 class instances produced by
//! this SDK (or wasm-dpp2 directly). Passing a forged JS object with a
//! spoofed `__wbg_ptr` is out of scope of this API's safety guarantees;
//! the structural extraction applied elsewhere closes that hole only on
//! the inputs that legitimately accept plain-object shapes.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use crate::settings::PutSettingsInput;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Getters, INITIAL_REVISION};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;
use dash_sdk::platform::documents::transitions::{
    build_signed_document_delete_transition, DocumentDeleteTransitionBuilder,
};
use dash_sdk::platform::transition::purchase_document::PurchaseDocument;
use dash_sdk::platform::transition::put_document::{
    build_signed_document_create_transition, build_signed_document_replace_transition,
    derive_document_id_from_parts, ensure_revision_for_create, ensure_revision_for_replace,
    PutDocument,
};
use dash_sdk::platform::transition::transfer_document::TransferDocument;
use dash_sdk::platform::transition::update_price_of_document::UpdatePriceOfDocument;
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
    get_class_type, try_from_options_optional, try_from_options_optional_with,
    try_from_options_with, try_to_fixed_bytes, try_to_string, try_to_u32, try_to_u64, JsValueExt,
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

    // We support two input shapes for `tokenPaymentInfo`:
    //
    // 1. A plain `DocumentTokenPaymentInfo` options bag (no `__type`
    //    marker) — parsed via the public constructor.
    // 2. An existing wasm-dpp2 `TokenPaymentInfo` class instance produced
    //    by `new TokenPaymentInfo(...)` (whose `__type` getter returns
    //    `"TokenPaymentInfo"`) — copied through its public getters and then
    //    parsed via the public constructor.
    //
    // Avoid `TokenPaymentInfoWasm::try_from(&value)` here: that path reads
    // the wasm-bindgen `__wbg_ptr` field, and this API accepts untrusted JS
    // values. A forged object can spoof the public `__type` getter/string,
    // but it cannot force us to dereference an arbitrary wasm pointer when
    // we only copy public fields into a fresh options bag.
    // Both shapes funnel through a fresh options bag built from safe,
    // structural reads. In particular, `paymentTokenContractId` is read
    // via `extract_optional_identifier_property` and re-attached as a
    // raw byte array, never forwarded as the original `JsValue`. That
    // closes the same `__wbg_ptr`-trust hole `extract_identifier_property`
    // protects against: a forged object can spoof a public `__type` /
    // `toString` / `toJSON` surface, but it cannot smuggle an arbitrary
    // wasm pointer into the eventual `TokenPaymentInfoWasm::constructor`
    // parse because we only ever forward extracted bytes.
    let class_type = get_class_type(&token_payment_info_value)
        .map_err(|err| WasmSdkError::invalid_argument(err.to_string()))?;
    let options = match class_type.as_str() {
        "TokenPaymentInfo" | "" => {
            token_payment_info_options_from_public_fields(&token_payment_info_value)?
        }
        // Plain object path: no `__type` getter is set up, so
        // `get_class_type` returns `Ok("")` (empty string default in
        // `JsValue::as_string().unwrap_or_default()`). Treat the empty
        // string the same as "no class marker present". Any other
        // class marker is rejected below.
        other => {
            return Err(WasmSdkError::invalid_argument(format!(
                "tokenPaymentInfo must be a plain DocumentTokenPaymentInfo options object \
                 or a TokenPaymentInfo instance, got class '{other}'"
            )));
        }
    };
    let token_payment_info = TokenPaymentInfoWasm::constructor(options)
        .map_err(|err| WasmSdkError::invalid_argument(err.to_string()))?;

    Ok(Some(token_payment_info.into()))
}

fn token_payment_info_options_from_public_fields(
    value: &JsValue,
) -> Result<TokenPaymentInfoOptionsJs, WasmSdkError> {
    let options = js_sys::Object::new();

    // `paymentTokenContractId` is identifier-shaped and accepts class
    // instances, byte arrays, or base58/hex strings. Run it through the
    // safe structural extractor so a forged `Identifier`-shaped JS object
    // cannot smuggle a raw `__wbg_ptr` through to the constructor parse.
    if let Some(payment_token_contract_id) =
        extract_optional_identifier_property(value, "paymentTokenContractId")?
    {
        let buffer: [u8; 32] = payment_token_contract_id.to_buffer();
        let bytes = js_sys::Uint8Array::from(&buffer[..]);
        Reflect::set(
            &options,
            &JsValue::from_str("paymentTokenContractId"),
            &bytes.into(),
        )
        .map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "failed to copy tokenPaymentInfo.paymentTokenContractId: {}",
                err.error_message()
            ))
        })?;
    }

    // The remaining fields are primitives / enums — copying them
    // structurally is safe because none of them feed into a
    // `__wbg_ptr`-trusting conversion path.
    for field in [
        "tokenContractPosition",
        "minimumTokenCost",
        "maximumTokenCost",
        "gasFeesPaidBy",
    ] {
        let field_value = Reflect::get(value, &JsValue::from_str(field)).map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "failed to read tokenPaymentInfo.{field}: {}",
                err.error_message()
            ))
        })?;
        if !field_value.is_undefined() {
            Reflect::set(&options, &JsValue::from_str(field), &field_value).map_err(|err| {
                WasmSdkError::invalid_argument(format!(
                    "failed to copy tokenPaymentInfo.{field}: {}",
                    err.error_message()
                ))
            })?;
        }
    }

    Ok(options.unchecked_into::<TokenPaymentInfoOptionsJs>())
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
   *
   * **Migration note (id ↔ entropy invariant):** `document.id` must match
   * the id derived from `(dataContractId, ownerId, documentTypeName, entropy)`
   * via the v0 document-id derivation. Mismatches are rejected with
   * `InvalidArgument` before any identity-contract nonce is allocated. The
   * `Document` constructor derives both together by default; if you set the
   * id or entropy explicitly, keep them consistent.
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
        // Extract document via public/structural JS fields only — never
        // the wasm-bindgen pointer. See `extract_prepare_document` for
        // the security rationale; the one-shot create path applies the
        // same hardening as the prepare path.
        let document_wasm = extract_prepare_document(&options)?;
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

        // Reject id-vs-entropy mismatches *before* fetching the contract.
        // The same invariant is independently enforced by the strict rs-sdk
        // helper as the security boundary; this just saves a round trip on
        // caller mistakes.
        ensure_document_id_matches_entropy_fast(
            document.id(),
            contract_id,
            document.owner_id(),
            &document_type_name,
            &entropy_array,
            self.inner_sdk().version(),
        )?;

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;
        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);

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
        // Extract document via public/structural JS fields only — never
        // the wasm-bindgen pointer. See `extract_prepare_document` for
        // the security rationale; the one-shot replace path applies the
        // same hardening as the prepare path.
        let document_wasm = extract_prepare_document(&options)?;
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

        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;
        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);

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
        // Extract the four delete identifiers via public/structural JS
        // fields only — never the wasm-bindgen pointer. See
        // `extract_delete_identifiers` for the security rationale.
        let (document_id, owner_id, contract_id, document_type_name) =
            extract_delete_identifiers(&options)?;

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;
        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);

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
  /**
   * The document to create.
   *
   * **Migration note (id ↔ entropy invariant):** `document.id` must match
   * the id derived from `(dataContractId, ownerId, documentTypeName, entropy)`
   * via the v0 document-id derivation. Mismatches are rejected with
   * `InvalidArgument` before any identity-contract nonce is allocated, so
   * failed calls do not advance the local nonce cache. The `Document`
   * constructor derives both together by default; if you set the id or
   * entropy explicitly, keep them consistent.
   */
  document: Document;
  /**
   * The identity public key to use for signing.
   *
   * Must be a wasm-dpp2 `IdentityPublicKey` instance produced by this SDK
   * (e.g. fetched from Platform or derived from the signer). This field
   * is read via the trusted wasm-bindgen handle and is **not** validated
   * structurally — see the module-level "Trusted wasm-dpp2 producers"
   * note for why this carve-out exists.
   */
  identityKey: IdentityPublicKey;
  /**
   * Signer containing the private key for the identity key.
   *
   * Must be a wasm-dpp2 `IdentitySigner` instance. `IdentitySigner` is
   * an opaque handle that owns private key material and signing-callback
   * state, so it has no structural JS-field shape that could be
   * safely reconstructed; this input is therefore intentionally trusted
   * via its wasm-bindgen handle. See the module-level "Trusted wasm-dpp2
   * producers" note for the full rationale.
   */
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
        // Extract document via public/structural JS fields only — never the
        // wasm-bindgen pointer. See `extract_prepare_document` for the
        // security rationale.
        let document_wasm = extract_prepare_document(&options)?;
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

        // Reject id-vs-entropy mismatches *before* fetching the contract.
        // The same invariant is independently enforced by the strict rs-sdk
        // helper as the security boundary; this just saves a round trip on
        // caller mistakes.
        ensure_document_id_matches_entropy_fast(
            document.id(),
            contract_id,
            document.owner_id(),
            &document_type_name,
            &entropy_array,
            self.inner_sdk().version(),
        )?;

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;
        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);

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
  /**
   * The identity public key to use for signing.
   *
   * Must be a wasm-dpp2 `IdentityPublicKey` instance produced by this SDK
   * (e.g. fetched from Platform or derived from the signer). This field
   * is read via the trusted wasm-bindgen handle and is **not** validated
   * structurally — see the module-level "Trusted wasm-dpp2 producers"
   * note for why this carve-out exists.
   */
  identityKey: IdentityPublicKey;
  /**
   * Signer containing the private key for the identity key.
   *
   * Must be a wasm-dpp2 `IdentitySigner` instance. `IdentitySigner` is
   * an opaque handle that owns private key material and signing-callback
   * state, so it has no structural JS-field shape that could be
   * safely reconstructed; this input is therefore intentionally trusted
   * via its wasm-bindgen handle. See the module-level "Trusted wasm-dpp2
   * producers" note for the full rationale.
   */
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
        // Extract document via public/structural JS fields only — never the
        // wasm-bindgen pointer. See `extract_prepare_document` for the
        // security rationale.
        let document_wasm = extract_prepare_document(&options)?;
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

        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;
        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);

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
  /**
   * The identity public key to use for signing.
   *
   * Must be a wasm-dpp2 `IdentityPublicKey` instance produced by this SDK
   * (e.g. fetched from Platform or derived from the signer). This field
   * is read via the trusted wasm-bindgen handle and is **not** validated
   * structurally — see the module-level "Trusted wasm-dpp2 producers"
   * note for why this carve-out exists.
   */
  identityKey: IdentityPublicKey;
  /**
   * Signer containing the private key for the identity key.
   *
   * Must be a wasm-dpp2 `IdentitySigner` instance. `IdentitySigner` is
   * an opaque handle that owns private key material and signing-callback
   * state, so it has no structural JS-field shape that could be
   * safely reconstructed; this input is therefore intentionally trusted
   * via its wasm-bindgen handle. See the module-level "Trusted wasm-dpp2
   * producers" note for the full rationale.
   */
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
        // Extract the four delete identifiers via public/structural JS
        // fields only — never the wasm-bindgen pointer. See
        // `extract_delete_identifiers` for the security rationale.
        let (document_id, owner_id, contract_id, document_type_name) =
            extract_delete_identifiers(&options)?;

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;
        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Extract settings from options
        let settings =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);

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

        // Delegate the nonce-allocate / sign / structure-validate / rollback
        // sequence to rs-sdk's shared helper so wasm-sdk and FFI share the
        // single implementation.
        let state_transition = build_signed_document_delete_transition(
            self.inner_sdk(),
            &builder,
            &identity_key,
            &signer,
        )
        .await?;

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
        // Extract document via public/structural JS fields only — never
        // the wasm-bindgen pointer. See `extract_prepare_document` for
        // the security rationale.
        let document_wasm = extract_prepare_document(&options)?;
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
        // Extract document via public/structural JS fields only — never
        // the wasm-bindgen pointer. See `extract_prepare_document` for
        // the security rationale.
        let document_wasm = extract_prepare_document(&options)?;
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
        // Extract document via public/structural JS fields only — never
        // the wasm-bindgen pointer. See `extract_prepare_document` for
        // the security rationale.
        let document_wasm = extract_prepare_document(&options)?;
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

/// Extract an identifier-shaped property from a JS object using only
/// public/structural JS reads, never the wasm-bindgen `__wbg_ptr` field.
///
/// `IdentifierWasm::try_from(&JsValue)` (and therefore
/// `IdentifierWasm::try_from_options`) falls through to
/// `to_wasm::<IdentifierWasm>("Identifier")` whenever the input "looks like"
/// an `Identifier` class instance, which dereferences the wasm-bindgen
/// `__wbg_ptr` field as a Rust pointer. A forged JS object can advertise
/// `__type === "Identifier"` while handing us an arbitrary numeric
/// `__wbg_ptr`, so trusting that pointer on untrusted inputs is unsound —
/// the same rationale that motivates `extract_prepare_document` /
/// `extract_delete_identifiers`.
///
/// Accepted public shapes:
///   * a Base58 / hex string (via `IdentifierWasm::try_from(&str)`),
///   * a `Uint8Array` or `Array` of bytes (via `IdentifierWasm::try_from(&[u8])`),
///   * an `Identifier` class instance — read via its **public** JS methods
///     (`toBytes()`, then `toString()` / `toJSON()`), never via `__wbg_ptr`.
fn extract_identifier_property(
    container: &JsValue,
    property_name: &str,
) -> Result<Identifier, WasmSdkError> {
    let value = Reflect::get(container, &JsValue::from_str(property_name)).map_err(|err| {
        WasmSdkError::invalid_argument(format!(
            "failed to read '{}' from options: {:?}",
            property_name, err
        ))
    })?;

    if value.is_undefined() || value.is_null() {
        return Err(WasmSdkError::invalid_argument(format!(
            "'{}' is required",
            property_name
        )));
    }

    // String: Base58 or hex.
    if let Some(s) = value.as_string() {
        return identifier_from_str(&s, property_name);
    }

    // Uint8Array / typed-array byte source.
    if value.is_instance_of::<js_sys::Uint8Array>() {
        let bytes = js_sys::Uint8Array::from(value.clone()).to_vec();
        return identifier_from_bytes(&bytes, property_name);
    }

    // Plain JS array of byte-like numbers.
    if js_sys::Array::is_array(&value) {
        let bytes = js_sys::Uint8Array::from(value.clone()).to_vec();
        return identifier_from_bytes(&bytes, property_name);
    }

    // Class-instance case: read via *public* JS methods only. We deliberately
    // do not look at `__type` / `__wbg_ptr`. `toBytes()` is not present on
    // `Object.prototype`, so it serves as a clean discriminator for the
    // real `Identifier` class instance shape; we fall back to
    // `toString()` / `toJSON()` (both yield the base58 form on real
    // `Identifier` instances).
    if value.is_object() {
        if let Some(bytes) = call_no_arg_method_returning_uint8array(&value, "toBytes") {
            return identifier_from_bytes(&bytes, property_name);
        }
        if let Some(s) = call_no_arg_method_returning_string(&value, "toString") {
            // Skip the unhelpful default `Object.prototype.toString` result
            // ("[object Object]"), which is never a valid identifier anyway
            // but produces a confusing parse error if we let it through.
            if !s.starts_with('[') {
                return identifier_from_str(&s, property_name);
            }
        }
        if let Some(s) = call_no_arg_method_returning_string(&value, "toJSON") {
            return identifier_from_str(&s, property_name);
        }
    }

    Err(WasmSdkError::invalid_argument(format!(
        "'{}' must be an Identifier, Uint8Array, array, or base58/hex string",
        property_name
    )))
}

/// Same structural extraction as [`extract_identifier_property`], but
/// returns `Ok(None)` if the property is missing/`undefined`/`null`
/// instead of erroring. Use for genuinely optional identifier fields like
/// `creatorId` and `paymentTokenContractId` so a missing value is not
/// conflated with a malformed one.
fn extract_optional_identifier_property(
    container: &JsValue,
    property_name: &str,
) -> Result<Option<Identifier>, WasmSdkError> {
    let value = Reflect::get(container, &JsValue::from_str(property_name)).map_err(|err| {
        WasmSdkError::invalid_argument(format!(
            "failed to read '{}' from options: {:?}",
            property_name, err
        ))
    })?;

    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }

    extract_identifier_property(container, property_name).map(Some)
}

fn identifier_from_str(s: &str, property_name: &str) -> Result<Identifier, WasmSdkError> {
    IdentifierWasm::try_from(s)
        .map(Into::into)
        .map_err(|err| WasmSdkError::invalid_argument(format!("'{}': {}", property_name, err)))
}

fn identifier_from_bytes(bytes: &[u8], property_name: &str) -> Result<Identifier, WasmSdkError> {
    IdentifierWasm::try_from(bytes)
        .map(Into::into)
        .map_err(|err| WasmSdkError::invalid_argument(format!("'{}': {}", property_name, err)))
}

fn call_no_arg_method_returning_uint8array(obj: &JsValue, method: &str) -> Option<Vec<u8>> {
    let func = Reflect::get(obj, &JsValue::from_str(method)).ok()?;
    if !func.is_function() {
        return None;
    }
    let func: js_sys::Function = func.unchecked_into();
    let ret = func.call0(obj).ok()?;
    if ret.is_undefined() || ret.is_null() {
        return None;
    }
    ret.dyn_into::<js_sys::Uint8Array>()
        .ok()
        .map(|arr| arr.to_vec())
}

fn call_no_arg_method_returning_string(obj: &JsValue, method: &str) -> Option<String> {
    let func = Reflect::get(obj, &JsValue::from_str(method)).ok()?;
    if !func.is_function() {
        return None;
    }
    let func: js_sys::Function = func.unchecked_into();
    func.call0(obj).ok()?.as_string()
}

/// Extract the four identifiers needed to build a document delete transition
/// (`id`, `ownerId`, `dataContractId`, `documentTypeName`) from the `document`
/// field of a delete-options object.
///
/// We accept *both* a `wasm-dpp2` `Document` class instance and a plain
/// `{ id, ownerId, dataContractId, documentTypeName }` options bag — but the
/// extraction itself reads only the public JS getters/fields on the value.
/// We deliberately do **not** call `DocumentWasm::try_from` /
/// `to_wasm::<DocumentWasm>("Document")` here: those paths read the
/// wasm-bindgen `__wbg_ptr` field and dereference it as a Rust pointer. A
/// forged JS object can advertise `__type === "Document"` but still hand us
/// an arbitrary numeric `__wbg_ptr`, so trusting the pointer on untrusted
/// inputs is unsound. Using only `id`, `ownerId`, `dataContractId`, and
/// `documentTypeName` (which `Document` exposes as ordinary
/// `#[wasm_bindgen(getter = ...)]` accessors) keeps the read structural and
/// safe for both shapes.
fn extract_delete_identifiers(
    options: &JsValue,
) -> Result<(Identifier, Identifier, Identifier, String), WasmSdkError> {
    let document_js = js_sys::Reflect::get(options, &JsValue::from_str("document"))
        .map_err(|_| WasmSdkError::invalid_argument("document is required"))?;

    if document_js.is_undefined() || document_js.is_null() {
        return Err(WasmSdkError::invalid_argument("document is required"));
    }

    let document_id = extract_identifier_property(&document_js, "id")?;
    let owner_id = extract_identifier_property(&document_js, "ownerId")?;
    let contract_id = extract_identifier_property(&document_js, "dataContractId")?;
    let document_type_name = try_from_options_with(&document_js, "documentTypeName", |v| {
        try_to_string(v, "documentTypeName")
    })?;

    Ok((document_id, owner_id, contract_id, document_type_name))
}

/// Extract a `DocumentWasm` from the `document` field of a prepare-options
/// object.
///
/// `prepareDocumentCreate` / `prepareDocumentReplace` accept either:
///   * a `wasm-dpp2` `Document` class instance (whose getters return
///     `IdentifierWasm`, `bigint`, `Uint8Array`, …), or
///   * a plain `{ id, ownerId, dataContractId, documentTypeName, properties,
///     revision?, entropy?, createdAt?, … }` options bag.
///
/// # Structural-only extraction
///
/// Both shapes are read through the **public JS surface only**
/// (`Reflect::get`, public getters, `toBytes()` / `toString()`,
/// [`wasm_dpp2::serialization::js_value_to_platform_value`]); the
/// wasm-bindgen `__wbg_ptr` field is never dereferenced. This keeps the
/// hardening from spoofed `__type` + `__wbg_ptr` attacks consistent
/// across the two accepted input shapes, including the typed
/// `properties` payload: `js_value_to_platform_value` preserves
/// `Uint8Array` (32-byte → `Value::Identifier`, otherwise
/// `Value::Bytes`) and `BigInt` (→ `Value::U64` / `Value::I64`) coming
/// back from the public `Document.properties` getter, which itself uses
/// [`wasm_dpp2::serialization::platform_value_to_object`] to serialize
/// the inner Rust map. We deliberately do **not** call
/// `DocumentWasm::try_from(&JsValue)` /
/// `to_wasm::<DocumentWasm>("Document")` here: those paths trust the
/// caller-supplied `__wbg_ptr` numeric value as a Rust pointer, which is
/// unsound on public JS input — the same rationale that motivates
/// `extract_identifier_property` / `extract_delete_identifiers` and
/// shares the threat model documented at the top of this file.
fn extract_prepare_document(options: &JsValue) -> Result<DocumentWasm, WasmSdkError> {
    let document_js = Reflect::get(options, &JsValue::from_str("document"))
        .map_err(|_| WasmSdkError::invalid_argument("document is required"))?;

    if document_js.is_undefined() || document_js.is_null() {
        return Err(WasmSdkError::invalid_argument("document is required"));
    }

    let id = extract_identifier_property(&document_js, "id")?;
    let owner_id = extract_identifier_property(&document_js, "ownerId")?;
    let contract_id = extract_identifier_property(&document_js, "dataContractId")?;
    let document_type_name = try_from_options_with(&document_js, "documentTypeName", |v| {
        try_to_string(v, "documentTypeName")
    })?;
    // Rebuild the typed `BTreeMap<String, Value>` from the public
    // `Document.properties` getter (or a plain JS options bag) without
    // touching `__wbg_ptr`. `js_value_to_platform_value` preserves
    // `Uint8Array` as `Value::Identifier` (32-byte) / `Value::Bytes` and
    // `BigInt` as `Value::U64` / `Value::I64`, matching what
    // `platform_value_to_object` produces on the getter side.
    let properties = try_from_options_with(&document_js, "properties", |v| {
        let pv = wasm_dpp2::serialization::js_value_to_platform_value(v)?;
        pv.into_btree_string_map()
            .map_err(|err| wasm_dpp2::error::WasmDppError::invalid_argument(err.to_string()))
    })?;

    let revision =
        try_from_options_optional_with(&document_js, "revision", |v| try_to_u64(v, "revision"))?;

    let entropy = try_from_options_optional_with(&document_js, "entropy", |v| {
        try_to_fixed_bytes::<32>(v.clone(), "entropy")
    })?;

    let created_at =
        try_from_options_optional_with(&document_js, "createdAt", |v| try_to_u64(v, "createdAt"))?;
    let updated_at =
        try_from_options_optional_with(&document_js, "updatedAt", |v| try_to_u64(v, "updatedAt"))?;
    let transferred_at = try_from_options_optional_with(&document_js, "transferredAt", |v| {
        try_to_u64(v, "transferredAt")
    })?;
    let created_at_block_height =
        try_from_options_optional_with(&document_js, "createdAtBlockHeight", |v| {
            try_to_u64(v, "createdAtBlockHeight")
        })?;
    let updated_at_block_height =
        try_from_options_optional_with(&document_js, "updatedAtBlockHeight", |v| {
            try_to_u64(v, "updatedAtBlockHeight")
        })?;
    let transferred_at_block_height =
        try_from_options_optional_with(&document_js, "transferredAtBlockHeight", |v| {
            try_to_u64(v, "transferredAtBlockHeight")
        })?;
    let created_at_core_block_height =
        try_from_options_optional_with(&document_js, "createdAtCoreBlockHeight", |v| {
            try_to_u32(v, "createdAtCoreBlockHeight")
        })?;
    let updated_at_core_block_height =
        try_from_options_optional_with(&document_js, "updatedAtCoreBlockHeight", |v| {
            try_to_u32(v, "updatedAtCoreBlockHeight")
        })?;
    let transferred_at_core_block_height =
        try_from_options_optional_with(&document_js, "transferredAtCoreBlockHeight", |v| {
            try_to_u32(v, "transferredAtCoreBlockHeight")
        })?;

    // `creatorId` is optional and read structurally. On a real
    // `wasm-dpp2` `Document` instance it comes from the public
    // `creatorId` getter; on plain-object inputs callers can pass it
    // explicitly. Missing / null / undefined yields `None`, matching the
    // `DocumentWasm` constructor's default.
    let creator_id = extract_optional_identifier_property(&document_js, "creatorId")?;

    let document = Document::V0(DocumentV0 {
        id,
        owner_id,
        properties,
        revision,
        created_at,
        updated_at,
        transferred_at,
        created_at_block_height,
        updated_at_block_height,
        transferred_at_block_height,
        created_at_core_block_height,
        updated_at_core_block_height,
        transferred_at_core_block_height,
        creator_id,
    });

    Ok(DocumentWasm::new(
        document,
        contract_id,
        document_type_name,
        entropy,
    ))
}

/// Fast-fail verification that `document.id` matches the
/// platform-version-dispatched document-id derivation for
/// `(contract_id, owner_id, document_type_name, entropy)`.
///
/// This is the same invariant the strict create helper enforces, but lifted
/// out of the rs-sdk path so wasm-sdk `documentCreate` /
/// `prepareDocumentCreate` can reject mismatches **before** the contract is
/// fetched from Platform (or read from local cache), saving a round trip on
/// caller mistakes. The rs-sdk helper still enforces this independently as
/// the security boundary; this is purely an early reject.
///
/// Dispatch goes through the shared rs-sdk
/// [`derive_document_id_from_parts`] helper so the
/// `DocumentMethodVersions::derive_document_id` match lives in exactly
/// one place: an unknown future version surfaces here as an
/// `InvalidArgument` instead of silently using the v0 formula. The
/// rs-sdk strict create helper performs the same dispatch independently.
fn ensure_document_id_matches_entropy_fast(
    document_id: Identifier,
    contract_id: Identifier,
    owner_id: Identifier,
    document_type_name: &str,
    entropy: &[u8; 32],
    platform_version: &dash_sdk::dpp::version::PlatformVersion,
) -> Result<(), WasmSdkError> {
    let expected = derive_document_id_from_parts(
        &contract_id,
        &owner_id,
        document_type_name,
        entropy,
        platform_version,
    )
    .map_err(|err| {
        WasmSdkError::invalid_argument(format!(
            "{err}; the wasm-sdk fast id-vs-entropy check could not dispatch \
             the derive_document_id version for this platform version. \
             Upgrade wasm-sdk to a build that supports this platform version."
        ))
    })?;
    if document_id != expected {
        return Err(WasmSdkError::invalid_argument(format!(
            "document.id does not match the platform-version-dispatched \
             document-id derivation \
             (dataContractId, ownerId, documentTypeName, entropy); \
             expected {expected}, got {document_id}. \
             The Document constructor derives both together by default; if you set the \
             id or entropy explicitly, keep them consistent."
        )));
    }
    Ok(())
}

/// Wasm-side revision guard for the document **create** path.
///
/// Delegates the accept/reject decision to the rs-sdk
/// [`ensure_revision_for_create`] helper so wasm-sdk and rs-sdk cannot
/// drift on which revisions are valid on the create path. On rejection,
/// this function constructs wasm-specific error messages — a dedicated
/// revision-0 wording (since `Some(0)` is invalid for *both* create and
/// replace, pointing at the sibling API would mislead) and an API-name
/// hint that names the wasm-sdk replace entry point.
fn ensure_document_create_revision(
    revision: Option<u64>,
    replace_api_name: &str,
) -> Result<(), WasmSdkError> {
    if ensure_revision_for_create(revision).is_ok() {
        return Ok(());
    }
    match revision {
        // `Some(0)` is invalid for *both* create and replace, so do not
        // point users at the sibling API — they would just see the same
        // rejection from `ensure_document_replace_revision`. Emit a
        // dedicated message that makes the always-invalid value explicit.
        Some(0) => Err(WasmSdkError::invalid_argument(format!(
            "Document revision is 0 but revision 0 is invalid for both create and replace. \
             Use unset or {} (INITIAL_REVISION) for create, or > {} for replace.",
            INITIAL_REVISION, INITIAL_REVISION,
        ))),
        Some(rev) => Err(WasmSdkError::invalid_argument(format!(
            "Document revision is {} but create requires revision to be unset or {}. Use {} for existing documents.",
            rev, INITIAL_REVISION, replace_api_name,
        ))),
        // `ensure_revision_for_create` accepts `None`, so the rs-sdk
        // helper would have short-circuited above for this case.
        None => unreachable!(
            "ensure_revision_for_create accepts None; wasm guard reached an unreachable arm"
        ),
    }
}

/// Wasm-side revision guard for the document **replace** path.
///
/// Delegates the accept/reject decision to the rs-sdk
/// [`ensure_revision_for_replace`] helper so wasm-sdk and rs-sdk cannot
/// drift on which revisions are valid on the replace path. On rejection,
/// this function constructs wasm-specific error messages — a dedicated
/// revision-0 wording (since `Some(0)` is invalid for *both* create and
/// replace, pointing at the sibling API would mislead) and an API-name
/// hint that names the wasm-sdk create entry point.
fn ensure_document_replace_revision(
    revision: Option<u64>,
    create_api_name: &str,
) -> Result<(), WasmSdkError> {
    if ensure_revision_for_replace(revision).is_ok() {
        return Ok(());
    }
    match revision {
        // `Some(0)` is invalid for *both* create and replace, so do not
        // point users at the sibling API — they would just see the same
        // rejection from `ensure_document_create_revision`. Emit a
        // dedicated message that makes the always-invalid value explicit.
        Some(0) => Err(WasmSdkError::invalid_argument(format!(
            "Document revision is 0 but revision 0 is invalid for both create and replace. \
             Use unset or {} (INITIAL_REVISION) for create, or > {} for replace.",
            INITIAL_REVISION, INITIAL_REVISION,
        ))),
        Some(rev) => Err(WasmSdkError::invalid_argument(format!(
            "Document revision is {} but replace requires revision > {}. Use {} for new documents.",
            rev, INITIAL_REVISION, create_api_name,
        ))),
        None => Err(WasmSdkError::invalid_argument(format!(
            "Document must have a revision set for replace. Use {} for new documents.",
            create_api_name,
        ))),
    }
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
    use dash_sdk::platform::transition::validation::ensure_valid_state_transition_structure;

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
        let err = ensure_document_create_revision(Some(2), "prepareDocumentReplace")
            .expect_err("revision > INITIAL_REVISION should fail");
        assert!(err.to_string().contains("prepareDocumentReplace"));
        assert!(err.to_string().contains("create requires revision"));
    }

    /// Revision `Some(0)` is invalid for *both* create and replace. The
    /// rejection message must therefore not point users at the sibling
    /// API (which would also reject), it must say revision 0 is invalid
    /// for both paths.
    #[test]
    fn create_revision_guard_rejects_zero_with_dedicated_message() {
        let err = ensure_document_create_revision(Some(0), "prepareDocumentReplace")
            .expect_err("revision 0 should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("revision 0 is invalid for both create and replace"),
            "expected dedicated revision-0 message, got: {msg}"
        );
        assert!(
            !msg.contains("prepareDocumentReplace"),
            "revision-0 message must not point users at the sibling API which also rejects: {msg}"
        );
    }

    /// Revision `Some(0)` is invalid for *both* create and replace. The
    /// rejection message must therefore not point users at the sibling
    /// API (which would also reject), it must say revision 0 is invalid
    /// for both paths.
    #[test]
    fn replace_revision_guard_rejects_zero_with_dedicated_message() {
        let err = ensure_document_replace_revision(Some(0), "prepareDocumentCreate")
            .expect_err("revision 0 should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("revision 0 is invalid for both create and replace"),
            "expected dedicated revision-0 message, got: {msg}"
        );
        assert!(
            !msg.contains("prepareDocumentCreate"),
            "revision-0 message must not point users at the sibling API which also rejects: {msg}"
        );
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

    /// `ensure_document_id_matches_entropy_fast` must produce the same
    /// derivation as `Document::generate_document_id_v0`, so a matching id
    /// passes and a non-matching id is rejected with a clear message. This
    /// lets `documentCreate` / `prepareDocumentCreate` reject caller
    /// mistakes before fetching the contract.
    #[test]
    fn fast_id_matches_entropy_accepts_matching_id_and_rejects_mismatch() {
        let contract_id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
        let entropy = [3u8; 32];
        let document_type_name = "note";
        let expected = Document::generate_document_id_v0(
            &contract_id,
            &owner_id,
            document_type_name,
            entropy.as_slice(),
        );

        assert!(ensure_document_id_matches_entropy_fast(
            expected,
            contract_id,
            owner_id,
            document_type_name,
            &entropy,
            PlatformVersion::latest(),
        )
        .is_ok());

        let bogus = Identifier::from([0xAB; 32]);
        assert_ne!(bogus, expected, "test precondition");
        let err = ensure_document_id_matches_entropy_fast(
            bogus,
            contract_id,
            owner_id,
            document_type_name,
            &entropy,
            PlatformVersion::latest(),
        )
        .expect_err("mismatch must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("does not match"),
            "expected id-mismatch message, got: {msg}"
        );
    }

    /// Unknown `derive_document_id` versions must be rejected by the wasm
    /// fast check instead of silently falling back to the v0 formula, so
    /// the wasm fast pre-check tracks the same dispatch table as the
    /// rs-sdk `derive_document_id_from_parts` helper. Synthesizes a
    /// bumped version constant rather than depending on a future
    /// platform version landing.
    #[test]
    fn fast_id_matches_entropy_rejects_unknown_derive_version() {
        let contract_id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);
        let entropy = [3u8; 32];
        let document_type_name = "note";
        // Any document id will do — the dispatcher must error before
        // any equality check happens.
        let document_id = Identifier::from([0xAB; 32]);

        let mut bumped = PlatformVersion::latest().clone();
        bumped
            .dpp
            .document_versions
            .document_method_versions
            .derive_document_id = 99;

        let err = ensure_document_id_matches_entropy_fast(
            document_id,
            contract_id,
            owner_id,
            document_type_name,
            &entropy,
            &bumped,
        )
        .expect_err("unknown derive_document_id version must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("derive_document_id"),
            "expected dispatch-error message mentioning derive_document_id, got: {msg}"
        );
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
