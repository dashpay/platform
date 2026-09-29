//! Document state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for document operations like create, replace, delete, etc.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use crate::settings::PutSettingsInput;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;
use dash_sdk::platform::documents::transitions::DocumentDeleteTransitionBuilder;
use dash_sdk::platform::transition::purchase_document::PurchaseDocument;
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::transfer_document::TransferDocument;
use dash_sdk::platform::transition::update_price_of_document::UpdatePriceOfDocument;
use dash_sdk::platform::DataContract;
use js_sys::Reflect;
use std::sync::Arc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::error::WasmDppError;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::state_transitions::batch::action_fee_agreement::{
    DocumentActionFeeAgreementOptionsJs, DocumentActionFeeAgreementWasm,
};
use wasm_dpp2::state_transitions::batch::prefunded_voting_balance::PrefundedVotingBalanceWasm;
use wasm_dpp2::state_transitions::batch::token_payment_info::{
    TokenPaymentInfoOptionsJs, TokenPaymentInfoWasm,
};
use wasm_dpp2::utils::{
    get_class_type, try_from_options_mut, try_from_options_optional,
    try_from_options_optional_with, try_from_options_with, try_to_string, try_to_u64, IntoWasm,
};
use wasm_dpp2::IdentitySignerWasm;

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

/// The creation options `settings` carries, created when there are none: the document
/// transition builders read the action fee agreement and the contest fund from them.
fn creation_options_of(settings: &mut Option<PutSettings>) -> &mut StateTransitionCreationOptions {
    settings
        .get_or_insert_with(Default::default)
        .state_transition_creation_options
        .get_or_insert_with(Default::default)
}

/// Reads the `actionFeeAgreement` option, a `DocumentActionFeeAgreement` or the options to
/// construct one, into the creation options of `settings`. Left out, `settings` is unchanged.
fn apply_action_fee_agreement_option(
    options: &JsValue,
    settings: &mut Option<PutSettings>,
) -> Result<(), WasmSdkError> {
    let Some(agreement) = try_from_options_optional_with(options, "actionFeeAgreement", |v| {
        let refused = |what: &str| {
            WasmDppError::invalid_argument(format!(
                "actionFeeAgreement must be a DocumentActionFeeAgreement or its options, not {}",
                what
            ))
        };
        if !v.is_object() {
            return Err(refused("a primitive value"));
        }
        match get_class_type(v)?.as_str() {
            "DocumentActionFeeAgreement" => DocumentActionFeeAgreementWasm::try_from(v),
            // Another class has none of the option fields, and would read as an agreement to
            // pay nothing
            "" => DocumentActionFeeAgreementWasm::constructor(
                v.clone()
                    .unchecked_into::<DocumentActionFeeAgreementOptionsJs>(),
            ),
            other => Err(refused(&format!("an instance of {}", other))),
        }
    })?
    else {
        return Ok(());
    };
    creation_options_of(settings).action_fee_agreement = Some(agreement.into());
    Ok(())
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
   * The most, in credits, the document pays into the contest it joins when its
   * document type has a contested index. From protocol version 14 it is charged
   * the fund to join the contest, which doubles once the contest holds 250
   * contenders and again for every 50 more, and is refused, paid, when that is
   * more than this. The identity must hold what it states. Leave it out to state
   * the fund to join read just before the document is submitted. A document that
   * joins no contest ignores it.
   */
  contestFund?: bigint;

  /**
   * What the transition agrees to pay in action fees. Required from protocol
   * version 14 when the document type's `actionFees` charge a fee for this
   * action: without it Platform refuses the transition (40132). Name the
   * amounts the document type declares in the contract you showed the user, so
   * that a fee changed since is refused (40133) instead of paid. For a fee priced
   * by the fee multiplier, `feeMultiplier` names the multiplier you priced it
   * with and how far above it the executing epoch's may be (40134 otherwise);
   * a type that declares no `pricing` is priced by the fee multiplier. Ignored
   * when the action charges nothing. Refused before any request on an SDK
   * running a protocol version before 14, which has no place to carry it.
   */
  actionFeeAgreement?: DocumentActionFeeAgreement | DocumentActionFeeAgreementOptions;

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
    /// @param options - Creation options including document, identity key, and signer
    /// The id of a new document commits to the identity contract nonce of its
    /// create transition (protocol version 14), so it only exists once the
    /// document is put: the `id` of the document passed in is a placeholder
    /// until then, and is updated to the final id when this resolves.
    ///
    /// @returns Promise resolving to the confirmed Document as Platform
    ///          committed it — its final `id` and the consensus-populated system fields
    ///          (`$createdAt` and friends) included. For an indexOnly document type
    ///          the proof shows the document's entry at the proof's block, not that
    ///          this create wrote it: no stronger proof exists for such a document.
    ///          Keep THIS instance
    ///          when you later intend to delete an indexOnly document
    ///          whose type requires `$createdAt`: the delete carries the
    ///          document's values, and the pre-broadcast wrapper never
    ///          learns the block timestamp Platform assigned.
    #[wasm_bindgen(js_name = "documentCreate")]
    pub async fn document_create(
        &self,
        options: DocumentCreateOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options, "document")?;
        let document: Document = document_wasm.clone().into();

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

        // Extract settings from options, refusing a malformed one before the contract fetch
        let mut settings: Option<PutSettings> =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // The most the document pays into the contest it joins
        if let Some(contest_fund) = try_from_options_optional_with(&options, "contestFund", |v| {
            try_to_u64(v, "contestFund")
        })? {
            creation_options_of(&mut settings).contest_fund = Some(contest_fund);
        }
        apply_action_fee_agreement_option(&options, &mut settings)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Use PutDocument trait for creation, keeping the confirmed
        // document Platform returns — it carries the consensus-assigned
        // system fields the caller's pre-broadcast wrapper lacks.
        let confirmed_document = document
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

        // From protocol version 14 the id of a new document commits to the
        // identity contract nonce of its create transition, which is only
        // assigned while the document is being put: the id the caller's
        // document was built with is a placeholder. Hand the final id back to
        // that document too, so code that keeps using it (to replace,
        // transfer or delete what it just created) addresses the document
        // Platform stored. Best effort: the returned document is the
        // authoritative one, and a caller that freed its document meanwhile
        // is skipped. (A document still borrowed elsewhere would throw here
        // rather than no-op; nothing re-enters wasm during the await.)
        if let Ok(mut caller_document) =
            try_from_options_mut::<DocumentWasm>(&options, "document", "Document")
        {
            caller_document.inner_mut().set_id(confirmed_document.id());
        }

        Ok(DocumentWasm::new(
            confirmed_document,
            contract_id,
            document_type_name,
            Some(entropy_array),
        ))
    }
}

// ============================================================================
// Contest Fund To Join
// ============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Document")]
    pub type DocumentJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// The prefunded voting balance a create of `document` states to join the
    /// contest it enters: the contested index the document falls under and the
    /// fund to join that contest now, or `undefined` when the document joins no
    /// contest (its type has no contested index, or its values do not match one).
    ///
    /// `documentCreate` states this itself. A create transition built by hand
    /// passes it as `prefundedVotingBalance` to `new DocumentCreateTransition`:
    /// from protocol version 14 a contested create that states less than the
    /// fund to join is refused and still pays its fees. The fund doubles once the
    /// contest holds 250 contenders and again for every 50 more, so this reads
    /// the contenders with proved queries, one per 100 of them. From protocol
    /// version 14 a create may state more than this, as headroom against
    /// contenders joining before it lands: Platform charges it only the fund to
    /// join, but the identity must hold what it states. Before 14 it must state
    /// exactly this.
    ///
    /// @param document - The document to create; its contract is fetched (or
    ///                   read from the cache) to find the contested index
    /// @returns The index name and credits to state, or undefined
    #[wasm_bindgen(js_name = "getContestFundToJoin")]
    pub async fn get_contest_fund_to_join(
        &self,
        document: DocumentJs,
    ) -> Result<Option<PrefundedVotingBalanceWasm>, WasmSdkError> {
        // Cloned out of the caller's `Document` before the first await, so the
        // object is not borrowed while the contest is read
        let document = DocumentWasm::try_from(&JsValue::from(document))?;
        let contract_id: Identifier = document.data_contract_id().into();
        let data_contract = self.get_or_fetch_contract(contract_id).await?;
        let document_type_name = document.document_type_name();
        let document_type = data_contract
            .document_type_for_name(&document_type_name)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    document_type_name, e
                ))
            })?;
        let document: Document = document.into();

        let prefunded_voting_balance = self
            .inner_sdk()
            .prefunded_voting_balance_to_join(document_type, &document)
            .await?;
        Ok(prefunded_voting_balance.map(PrefundedVotingBalanceWasm::from))
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
   * Revision should be set to current revision + 1.
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
   * What the transition agrees to pay in action fees. Required from protocol
   * version 14 when the document type's `actionFees` charge a fee for this
   * action: without it Platform refuses the transition (40132). Name the
   * amounts the document type declares in the contract you showed the user, so
   * that a fee changed since is refused (40133) instead of paid. For a fee priced
   * by the fee multiplier, `feeMultiplier` names the multiplier you priced it
   * with and how far above it the executing epoch's may be (40134 otherwise);
   * a type that declares no `pricing` is priced by the fee multiplier. Ignored
   * when the action charges nothing. Refused before any request on an SDK
   * running a protocol version before 14, which has no place to carry it.
   */
  actionFeeAgreement?: DocumentActionFeeAgreement | DocumentActionFeeAgreementOptions;

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

        // Get metadata from document
        let contract_id: Identifier = document_wasm.data_contract_id().into();
        let document_type_name = document_wasm.document_type_name();

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Extract settings from options, refusing a malformed one before the contract fetch
        let mut settings: Option<PutSettings> =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        apply_action_fee_agreement_option(&options, &mut settings)?;
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

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
   * What the transition agrees to pay in action fees. Required from protocol
   * version 14 when the document type's `actionFees` charge a fee for this
   * action: without it Platform refuses the transition (40132). Name the
   * amounts the document type declares in the contract you showed the user, so
   * that a fee changed since is refused (40133) instead of paid. For a fee priced
   * by the fee multiplier, `feeMultiplier` names the multiplier you priced it
   * with and how far above it the executing epoch's may be (40134 otherwise);
   * a type that declares no `pricing` is priced by the fee multiplier. Ignored
   * when the action charges nothing. Refused before any request on an SDK
   * running a protocol version before 14, which has no place to carry it.
   */
  actionFeeAgreement?: DocumentActionFeeAgreement | DocumentActionFeeAgreementOptions;

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
    /// @returns Promise that resolves when the document is deleted. For an indexOnly
    ///          document type the proof shows the document's entry gone at the proof's
    ///          block, not that this delete removed it: no stronger proof exists for
    ///          such a document.
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

        // Check if it's a Document instance or a plain object with fields.
        // The full document is kept when provided — indexOnly document
        // types can ONLY be deleted from their values, so the id-only
        // plain-object form does not work for them.
        let (document_id, owner_id, contract_id, document_type_name, full_document): (
            Identifier,
            Identifier,
            Identifier,
            String,
            Option<Document>,
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
                Some(doc_inner),
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
                None,
            )
        };

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Extract settings from options, refusing a malformed one before the contract fetch
        let mut settings: Option<PutSettings> =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        apply_action_fee_agreement_option(&options, &mut settings)?;
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        let builder = delete_transition_builder(
            data_contract,
            document_type_name,
            DeleteTarget {
                document_id,
                owner_id,
                full_document,
            },
            token_payment_info,
            settings,
        );

        self.inner_sdk()
            .document_delete(builder, &identity_key, &signer)
            .await?;

        Ok(())
    }
}

/// Which document a delete removes: its id and owner, and the whole document when the
/// caller gave one (an indexOnly document is only deleted from its values).
struct DeleteTarget {
    document_id: Identifier,
    owner_id: Identifier,
    full_document: Option<Document>,
}

/// The delete transition builder for `target`. A provided document goes through
/// `from_document` so its values ride along (required for indexOnly document types).
fn delete_transition_builder(
    data_contract: DataContract,
    document_type_name: String,
    target: DeleteTarget,
    token_payment_info: Option<TokenPaymentInfo>,
    mut settings: Option<PutSettings>,
) -> DocumentDeleteTransitionBuilder {
    let builder = match &target.full_document {
        Some(full_document) => DocumentDeleteTransitionBuilder::from_document(
            Arc::new(data_contract),
            document_type_name,
            full_document,
        ),
        None => DocumentDeleteTransitionBuilder::new(
            Arc::new(data_contract),
            document_type_name,
            target.document_id,
            target.owner_id,
        ),
    };

    let builder = match token_payment_info {
        Some(token_payment_info) => builder.with_token_payment_info(token_payment_info),
        None => builder,
    };

    // Unlike the other document builders, the delete builder signs with its own user fee
    // increase and creation options (the action fee agreement, the signing options), and never
    // reads the ones in its settings
    let builder = match settings
        .as_ref()
        .and_then(|settings| settings.user_fee_increase)
    {
        Some(user_fee_increase) => builder.with_user_fee_increase(user_fee_increase),
        None => builder,
    };
    let builder = match settings
        .as_mut()
        .and_then(|settings| settings.state_transition_creation_options.take())
    {
        Some(creation_options) => builder.with_state_transition_creation_options(creation_options),
        None => builder,
    };

    match settings {
        Some(settings) => builder.with_settings(settings),
        None => builder,
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
   * What the transition agrees to pay in action fees. Required from protocol
   * version 14 when the document type's `actionFees` charge a fee for this
   * action: without it Platform refuses the transition (40132). Name the
   * amounts the document type declares in the contract you showed the user, so
   * that a fee changed since is refused (40133) instead of paid. For a fee priced
   * by the fee multiplier, `feeMultiplier` names the multiplier you priced it
   * with and how far above it the executing epoch's may be (40134 otherwise);
   * a type that declares no `pricing` is priced by the fee multiplier. Ignored
   * when the action charges nothing. Refused before any request on an SDK
   * running a protocol version before 14, which has no place to carry it.
   */
  actionFeeAgreement?: DocumentActionFeeAgreement | DocumentActionFeeAgreementOptions;

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

        // Extract settings from options, refusing a malformed one before the contract fetch
        let mut settings: Option<PutSettings> =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        apply_action_fee_agreement_option(&options, &mut settings)?;
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

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
   * What the transition agrees to pay in action fees. Required from protocol
   * version 14 when the document type's `actionFees` charge a fee for this
   * action: without it Platform refuses the transition (40132). Name the
   * amounts the document type declares in the contract you showed the user, so
   * that a fee changed since is refused (40133) instead of paid. For a fee priced
   * by the fee multiplier, `feeMultiplier` names the multiplier you priced it
   * with and how far above it the executing epoch's may be (40134 otherwise);
   * a type that declares no `pricing` is priced by the fee multiplier. Ignored
   * when the action charges nothing. Refused before any request on an SDK
   * running a protocol version before 14, which has no place to carry it.
   */
  actionFeeAgreement?: DocumentActionFeeAgreement | DocumentActionFeeAgreementOptions;

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

        // Extract settings from options, refusing a malformed one before the contract fetch
        let mut settings: Option<PutSettings> =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        apply_action_fee_agreement_option(&options, &mut settings)?;
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

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
   * What the transition agrees to pay in action fees. Required from protocol
   * version 14 when the document type's `actionFees` charge a fee for this
   * action: without it Platform refuses the transition (40132). Name the
   * amounts the document type declares in the contract you showed the user, so
   * that a fee changed since is refused (40133) instead of paid. For a fee priced
   * by the fee multiplier, `feeMultiplier` names the multiplier you priced it
   * with and how far above it the executing epoch's may be (40134 otherwise);
   * a type that declares no `pricing` is priced by the fee multiplier. Ignored
   * when the action charges nothing. Refused before any request on an SDK
   * running a protocol version before 14, which has no place to carry it.
   */
  actionFeeAgreement?: DocumentActionFeeAgreement | DocumentActionFeeAgreementOptions;

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

        // Extract settings from options, refusing a malformed one before the contract fetch
        let mut settings: Option<PutSettings> =
            try_from_options_optional::<PutSettingsInput>(&options, "settings")?.map(Into::into);
        apply_action_fee_agreement_option(&options, &mut settings)?;
        let token_payment_info = try_from_options_optional_token_payment_info(&options)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

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

/// Get an owned DocumentType from a DataContract
fn get_document_type(
    data_contract: &DataContract,
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
    use dash_sdk::dpp::data_contract::document_type::action_fees::agreement::v0::DocumentActionFeeAgreementV0;
    use dash_sdk::dpp::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;

    fn agreement() -> DocumentActionFeeAgreement {
        DocumentActionFeeAgreementV0 {
            owner: 80_000_000,
            moderators: 16_000_000,
            fee_multiplier: None,
        }
        .into()
    }

    fn builder_for(settings: Option<PutSettings>) -> DocumentDeleteTransitionBuilder {
        let contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("load the DPNS contract");
        delete_transition_builder(
            contract,
            "domain".to_string(),
            DeleteTarget {
                document_id: Identifier::new([1; 32]),
                owner_id: Identifier::new([2; 32]),
                full_document: None,
            },
            None,
            settings,
        )
    }

    /// The delete builder signs with its own creation options and never reads the ones in its
    /// settings, so an agreement left in the settings would be dropped from the transition.
    #[test]
    fn should_hand_the_delete_builder_the_action_fee_agreement_it_signs_with() {
        let mut settings = None;
        creation_options_of(&mut settings).action_fee_agreement = Some(agreement());
        creation_options_of(&mut settings)
            .signing_options
            .allow_signing_with_any_purpose = true;

        let builder = builder_for(settings);

        let creation_options = builder
            .state_transition_creation_options
            .expect("the builder carries the creation options");
        assert_eq!(creation_options.action_fee_agreement, Some(agreement()));
        assert!(
            creation_options
                .signing_options
                .allow_signing_with_any_purpose
        );
        let settings = builder.settings.expect("the other settings are kept");
        assert!(settings.state_transition_creation_options.is_none());
    }

    #[test]
    fn should_leave_the_delete_builder_without_creation_options_when_none_are_given() {
        let builder = builder_for(Some(PutSettings::default()));

        assert!(builder.state_transition_creation_options.is_none());
    }

    /// The delete builder signs with its own user fee increase, so the one in its settings
    /// would be dropped from the transition.
    #[test]
    fn should_hand_the_delete_builder_the_user_fee_increase_it_signs_with() {
        let builder = builder_for(Some(PutSettings {
            user_fee_increase: Some(3),
            ..Default::default()
        }));

        assert_eq!(builder.user_fee_increase, Some(3));
    }

    #[test]
    fn should_add_to_the_creation_options_the_settings_already_carry() {
        let mut settings = Some(PutSettings {
            user_fee_increase: Some(3),
            ..Default::default()
        });
        creation_options_of(&mut settings).contest_fund = Some(5);
        creation_options_of(&mut settings).action_fee_agreement = Some(agreement());

        let settings = settings.expect("settings are kept");
        assert_eq!(settings.user_fee_increase, Some(3));
        let creation_options = settings
            .state_transition_creation_options
            .expect("creation options are created once");
        assert_eq!(creation_options.contest_fund, Some(5));
        assert_eq!(creation_options.action_fee_agreement, Some(agreement()));
    }
}
