//! Document state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for document operations like create, replace, delete, etc.

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use crate::settings::extract_settings_from_options;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Getters};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dash_sdk::dpp::platform_value::{Identifier, Value as PlatformValue};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::IdentitySignerWasm;

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
   * The ID of the data contract.
   */
  dataContractId: Identifier;

  /**
   * The name of the document type within the contract.
   */
  documentType: string;

  /**
   * The identity ID of the document owner.
   */
  ownerId: Identifier;

  /**
   * The document data as a JSON object.
   * Should contain all required fields defined in the document type schema.
   */
  documentData: Record<string, unknown>;

  /**
   * 32 bytes of entropy for document ID generation.
   * Must be unique for each document.
   */
  entropy: Uint8Array;

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

/// Main input struct for document create options (serializable fields only).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentCreateOptionsInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    owner_id: IdentifierWasm,
}

fn deserialize_document_create_options(
    options: JsValue,
) -> Result<DocumentCreateOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "document create options",
    )
}

/// Result of creating a document.
#[wasm_bindgen(js_name = "DocumentCreateResult")]
pub struct DocumentCreateResultWasm {
    document_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = DocumentCreateResult)]
impl DocumentCreateResultWasm {
    /// The ID of the created document.
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> IdentifierWasm {
        self.document_id.clone()
    }
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
    /// @param options - Creation options including contract ID, document type, data, and signer
    /// @returns Promise resolving to DocumentCreateResult with the created document ID
    #[wasm_bindgen(js_name = "documentCreate")]
    pub async fn document_create(
        &self,
        options: DocumentCreateOptionsJs,
    ) -> Result<DocumentCreateResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_document_create_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let owner_id: Identifier = parsed.owner_id.into();

        // Extract document data from options
        let document_data_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("documentData"))
                .map_err(|_| WasmSdkError::invalid_argument("documentData is required"))?;

        if document_data_js.is_undefined() || document_data_js.is_null() {
            return Err(WasmSdkError::invalid_argument("documentData is required"));
        }

        let document_data_value: serde_json::Value =
            serde_wasm_bindgen::from_value(document_data_js)
                .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid document data: {}", e)))?;

        // Extract entropy from options
        let entropy_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("entropy"))
            .map_err(|_| WasmSdkError::invalid_argument("entropy is required"))?;

        let entropy_bytes: Vec<u8> = if let Some(arr) = entropy_js.dyn_ref::<js_sys::Uint8Array>() {
            arr.to_vec()
        } else {
            return Err(WasmSdkError::invalid_argument(
                "entropy must be a Uint8Array",
            ));
        };

        if entropy_bytes.len() != 32 {
            return Err(WasmSdkError::invalid_argument(
                "entropy must be exactly 32 bytes",
            ));
        }

        let mut entropy_array = [0u8; 32];
        entropy_array.copy_from_slice(&entropy_bytes);

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(self.inner_sdk(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Get document type
        let document_type_ref = data_contract
            .document_type_for_name(&parsed.document_type)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    parsed.document_type, e
                ))
            })?;

        // Convert JSON data to platform value
        let document_data_platform_value: PlatformValue = document_data_value.into();

        // Create the document
        let platform_version = self.inner_sdk().version();
        let document = document_type_ref
            .create_document_from_data(
                document_data_platform_value,
                owner_id,
                0, // block_time (will be set by platform)
                0, // core_block_height (will be set by platform)
                entropy_array,
                platform_version,
            )
            .map_err(|e| WasmSdkError::generic(format!("Failed to create document: {}", e)))?;

        let document_id = document.id();

        // Get identity contract nonce
        let identity_contract_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await?;

        // Create the state transition
        let state_transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            document_type_ref,
            entropy_array,
            &identity_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create document transition: {}", e))
        })?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transition: {}", e)))?;

        Ok(DocumentCreateResultWasm {
            document_id: document_id.into(),
        })
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
   * The ID of the data contract.
   */
  dataContractId: Identifier;

  /**
   * The name of the document type within the contract.
   */
  documentType: string;

  /**
   * The ID of the document to replace.
   */
  documentId: Identifier;

  /**
   * The identity ID of the document owner.
   */
  ownerId: Identifier;

  /**
   * The new document data as a JSON object.
   * Should contain all required fields defined in the document type schema.
   */
  documentData: Record<string, unknown>;

  /**
   * The current revision of the document.
   * The transition will use revision + 1.
   */
  revision: number;

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

/// Main input struct for document replace options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentReplaceOptionsInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    document_id: IdentifierWasm,
    owner_id: IdentifierWasm,
    revision: u64,
}

fn deserialize_document_replace_options(
    options: JsValue,
) -> Result<DocumentReplaceOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "document replace options",
    )
}

/// Result of replacing a document.
#[wasm_bindgen(js_name = "DocumentReplaceResult")]
pub struct DocumentReplaceResultWasm {
    document_id: IdentifierWasm,
    revision: u64,
}

#[wasm_bindgen(js_class = DocumentReplaceResult)]
impl DocumentReplaceResultWasm {
    /// The ID of the replaced document.
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> IdentifierWasm {
        self.document_id.clone()
    }

    /// The new revision of the document.
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> u64 {
        self.revision
    }
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
    /// @param options - Replace options including document ID, new data, revision, and signer
    /// @returns Promise resolving to DocumentReplaceResult with the updated document info
    #[wasm_bindgen(js_name = "documentReplace")]
    pub async fn document_replace(
        &self,
        options: DocumentReplaceOptionsJs,
    ) -> Result<DocumentReplaceResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_document_replace_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let document_id: Identifier = parsed.document_id.into();
        let owner_id: Identifier = parsed.owner_id.into();
        let new_revision = parsed.revision + 1;

        // Extract document data from options
        let document_data_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("documentData"))
                .map_err(|_| WasmSdkError::invalid_argument("documentData is required"))?;

        if document_data_js.is_undefined() || document_data_js.is_null() {
            return Err(WasmSdkError::invalid_argument("documentData is required"));
        }

        let document_data_value: serde_json::Value =
            serde_wasm_bindgen::from_value(document_data_js)
                .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid document data: {}", e)))?;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(self.inner_sdk(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Get document type
        let document_type_ref = data_contract
            .document_type_for_name(&parsed.document_type)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    parsed.document_type, e
                ))
            })?;

        // Convert JSON data to platform value
        let document_data_platform_value: PlatformValue = document_data_value.into();

        // Create the document with the new data
        let document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id,
            properties: document_data_platform_value
                .into_btree_string_map()
                .map_err(|e| {
                    WasmSdkError::serialization(format!("Failed to convert document data: {}", e))
                })?,
            revision: Some(new_revision),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        // Get identity contract nonce
        let identity_contract_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await?;

        // Create the state transition
        let state_transition = BatchTransition::new_document_replacement_transition_from_document(
            document,
            document_type_ref,
            &identity_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            self.inner_sdk().version(),
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create document replace transition: {}", e))
        })?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transition: {}", e)))?;

        Ok(DocumentReplaceResultWasm {
            document_id: document_id.into(),
            revision: new_revision,
        })
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
   * The ID of the data contract.
   */
  dataContractId: Identifier;

  /**
   * The name of the document type within the contract.
   */
  documentType: string;

  /**
   * The ID of the document to delete.
   */
  documentId: Identifier;

  /**
   * The identity ID of the document owner.
   */
  ownerId: Identifier;

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

/// Main input struct for document delete options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentDeleteOptionsInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    document_id: IdentifierWasm,
    owner_id: IdentifierWasm,
}

fn deserialize_document_delete_options(
    options: JsValue,
) -> Result<DocumentDeleteOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "document delete options",
    )
}

/// Result of deleting a document.
#[wasm_bindgen(js_name = "DocumentDeleteResult")]
pub struct DocumentDeleteResultWasm {
    document_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = DocumentDeleteResult)]
impl DocumentDeleteResultWasm {
    /// The ID of the deleted document.
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> IdentifierWasm {
        self.document_id.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Delete a document from Dash Platform.
    ///
    /// This method handles the complete document deletion flow:
    /// 1. Fetches the data contract and document from Platform
    /// 2. Creates and signs the document delete transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Delete options including document ID and signer
    /// @returns Promise resolving to DocumentDeleteResult confirming deletion
    #[wasm_bindgen(js_name = "documentDelete")]
    pub async fn document_delete(
        &self,
        options: DocumentDeleteOptionsJs,
    ) -> Result<DocumentDeleteResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_document_delete_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let document_id: Identifier = parsed.document_id.into();
        let owner_id: Identifier = parsed.owner_id.into();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(self.inner_sdk(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Get document type
        let document_type_ref = data_contract
            .document_type_for_name(&parsed.document_type)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    parsed.document_type, e
                ))
            })?;

        // Fetch the existing document to get its revision
        use dash_sdk::platform::DocumentQuery;

        let query = DocumentQuery::new_with_data_contract_id(
            self.inner_sdk(),
            contract_id,
            &parsed.document_type,
        )
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
        .with_document_id(&document_id);

        let existing_doc = dash_sdk::platform::Document::fetch(self.inner_sdk(), query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        let current_revision = existing_doc
            .revision()
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision is missing"))?;

        // Get identity contract nonce
        let identity_contract_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await?;

        // Create a minimal document for deletion
        let document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id,
            properties: Default::default(),
            revision: Some(current_revision),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        // Create the delete transition
        let state_transition = BatchTransition::new_document_deletion_transition_from_document(
            document,
            document_type_ref,
            &identity_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create delete transition: {}", e)))?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        let st: StateTransition = state_transition;
        st.broadcast(self.inner_sdk(), settings).await?;

        Ok(DocumentDeleteResultWasm {
            document_id: document_id.into(),
        })
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
   * The ID of the data contract.
   */
  dataContractId: Identifier;

  /**
   * The name of the document type within the contract.
   */
  documentType: string;

  /**
   * The ID of the document to transfer.
   */
  documentId: Identifier;

  /**
   * The current owner's identity ID.
   */
  ownerId: Identifier;

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

/// Main input struct for document transfer options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentTransferOptionsInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    document_id: IdentifierWasm,
    owner_id: IdentifierWasm,
    recipient_id: IdentifierWasm,
}

fn deserialize_document_transfer_options(
    options: JsValue,
) -> Result<DocumentTransferOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "document transfer options",
    )
}

/// Result of transferring a document.
#[wasm_bindgen(js_name = "DocumentTransferResult")]
pub struct DocumentTransferResultWasm {
    document_id: IdentifierWasm,
    new_owner_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = DocumentTransferResult)]
impl DocumentTransferResultWasm {
    /// The ID of the transferred document.
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> IdentifierWasm {
        self.document_id.clone()
    }

    /// The new owner's identity ID.
    #[wasm_bindgen(getter = "newOwnerId")]
    pub fn new_owner_id(&self) -> IdentifierWasm {
        self.new_owner_id.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Transfer a document to another identity.
    ///
    /// This method handles the complete document transfer flow:
    /// 1. Fetches the document from Platform
    /// 2. Creates and signs the document transfer transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Transfer options including document ID, recipient, and signer
    /// @returns Promise resolving to DocumentTransferResult with transfer info
    #[wasm_bindgen(js_name = "documentTransfer")]
    pub async fn document_transfer(
        &self,
        options: DocumentTransferOptionsJs,
    ) -> Result<DocumentTransferResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_document_transfer_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let document_id: Identifier = parsed.document_id.into();
        let owner_id: Identifier = parsed.owner_id.into();
        let recipient_id: Identifier = parsed.recipient_id.into();

        // Validate not transferring to self
        if owner_id == recipient_id {
            return Err(WasmSdkError::invalid_argument(
                "Cannot transfer document to yourself",
            ));
        }

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(self.inner_sdk(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Get document type
        let document_type_ref = data_contract
            .document_type_for_name(&parsed.document_type)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    parsed.document_type, e
                ))
            })?;

        // Fetch the existing document
        use dash_sdk::platform::DocumentQuery;

        let query = DocumentQuery::new_with_data_contract_id(
            self.inner_sdk(),
            contract_id,
            &parsed.document_type,
        )
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
        .with_document_id(&document_id);

        let existing_doc = dash_sdk::platform::Document::fetch(self.inner_sdk(), query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        let current_revision = existing_doc
            .revision()
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision is missing"))?;

        // Verify ownership
        if existing_doc.owner_id() != owner_id {
            return Err(WasmSdkError::invalid_argument(
                "Only the document owner can transfer it",
            ));
        }

        // Get identity contract nonce
        let identity_contract_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await?;

        // Create document with incremented revision for the transfer
        let transfer_document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id,
            properties: existing_doc.properties().clone(),
            revision: Some(current_revision + 1),
            created_at: existing_doc.created_at(),
            updated_at: existing_doc.updated_at(),
            transferred_at: existing_doc.transferred_at(),
            created_at_block_height: existing_doc.created_at_block_height(),
            updated_at_block_height: existing_doc.updated_at_block_height(),
            transferred_at_block_height: existing_doc.transferred_at_block_height(),
            created_at_core_block_height: existing_doc.created_at_core_block_height(),
            updated_at_core_block_height: existing_doc.updated_at_core_block_height(),
            transferred_at_core_block_height: existing_doc.transferred_at_core_block_height(),
            creator_id: existing_doc.creator_id(),
        });

        // Create the transfer transition
        let state_transition = BatchTransition::new_document_transfer_transition_from_document(
            transfer_document,
            document_type_ref,
            recipient_id,
            &identity_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create transfer transition: {}", e)))?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        let st: StateTransition = state_transition;
        st.broadcast(self.inner_sdk(), settings).await?;

        Ok(DocumentTransferResultWasm {
            document_id: document_id.into(),
            new_owner_id: recipient_id.into(),
        })
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
   * The ID of the data contract.
   */
  dataContractId: Identifier;

  /**
   * The name of the document type within the contract.
   */
  documentType: string;

  /**
   * The ID of the document to purchase.
   */
  documentId: Identifier;

  /**
   * The buyer's identity ID.
   */
  buyerId: Identifier;

  /**
   * The purchase price in credits.
   * Must match the document's listed price.
   */
  price: bigint | number;

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

/// Main input struct for document purchase options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentPurchaseOptionsInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    document_id: IdentifierWasm,
    buyer_id: IdentifierWasm,
    price: u64,
}

fn deserialize_document_purchase_options(
    options: JsValue,
) -> Result<DocumentPurchaseOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "document purchase options",
    )
}

/// Result of purchasing a document.
#[wasm_bindgen(js_name = "DocumentPurchaseResult")]
pub struct DocumentPurchaseResultWasm {
    document_id: IdentifierWasm,
    new_owner_id: IdentifierWasm,
    price_paid: u64,
}

#[wasm_bindgen(js_class = DocumentPurchaseResult)]
impl DocumentPurchaseResultWasm {
    /// The ID of the purchased document.
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> IdentifierWasm {
        self.document_id.clone()
    }

    /// The new owner's identity ID (the buyer).
    #[wasm_bindgen(getter = "newOwnerId")]
    pub fn new_owner_id(&self) -> IdentifierWasm {
        self.new_owner_id.clone()
    }

    /// The price that was paid in credits.
    #[wasm_bindgen(getter = "pricePaid")]
    pub fn price_paid(&self) -> js_sys::BigInt {
        js_sys::BigInt::from(self.price_paid)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Purchase a document that has a price set.
    ///
    /// This method handles the complete document purchase flow:
    /// 1. Fetches the document from Platform
    /// 2. Verifies the document has a matching price
    /// 3. Creates and signs the document purchase transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Purchase options including document ID, price, and signer
    /// @returns Promise resolving to DocumentPurchaseResult with purchase info
    #[wasm_bindgen(js_name = "documentPurchase")]
    pub async fn document_purchase(
        &self,
        options: DocumentPurchaseOptionsJs,
    ) -> Result<DocumentPurchaseResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_document_purchase_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let document_id: Identifier = parsed.document_id.into();
        let buyer_id: Identifier = parsed.buyer_id.into();
        let price = parsed.price;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(self.inner_sdk(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Get document type
        let document_type_ref = data_contract
            .document_type_for_name(&parsed.document_type)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    parsed.document_type, e
                ))
            })?;

        // Fetch the existing document
        use dash_sdk::platform::DocumentQuery;

        let query = DocumentQuery::new_with_data_contract_id(
            self.inner_sdk(),
            contract_id,
            &parsed.document_type,
        )
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
        .with_document_id(&document_id);

        let existing_doc = dash_sdk::platform::Document::fetch(self.inner_sdk(), query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        let current_revision = existing_doc
            .revision()
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision is missing"))?;

        // Verify the document has a price and it matches
        let listed_price = existing_doc
            .properties()
            .get_optional_integer::<u64>("$price")
            .map_err(|e| WasmSdkError::generic(format!("Failed to get document price: {}", e)))?
            .ok_or_else(|| WasmSdkError::not_found("Document is not for sale (no price set)"))?;

        if listed_price != price {
            return Err(WasmSdkError::invalid_argument(format!(
                "Price mismatch: document is listed for {} but purchase attempted with {}",
                listed_price, price
            )));
        }

        // Get identity contract nonce
        let identity_contract_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(buyer_id, contract_id, true, None)
            .await?;

        // Create document with incremented revision for the purchase
        let purchase_document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id: existing_doc.owner_id(),
            properties: existing_doc.properties().clone(),
            revision: Some(current_revision + 1),
            created_at: existing_doc.created_at(),
            updated_at: existing_doc.updated_at(),
            transferred_at: existing_doc.transferred_at(),
            created_at_block_height: existing_doc.created_at_block_height(),
            updated_at_block_height: existing_doc.updated_at_block_height(),
            transferred_at_block_height: existing_doc.transferred_at_block_height(),
            created_at_core_block_height: existing_doc.created_at_core_block_height(),
            updated_at_core_block_height: existing_doc.updated_at_core_block_height(),
            transferred_at_core_block_height: existing_doc.transferred_at_core_block_height(),
            creator_id: existing_doc.creator_id(),
        });

        // Create document purchase transition
        let state_transition = BatchTransition::new_document_purchase_transition_from_document(
            purchase_document,
            document_type_ref,
            buyer_id,
            price as Credits,
            &identity_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // No token payment info
            &signer,
            self.inner_sdk().version(),
            None, // Default options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create purchase transition: {}", e))
        })?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast purchase: {}", e)))?;

        Ok(DocumentPurchaseResultWasm {
            document_id: document_id.into(),
            new_owner_id: buyer_id.into(),
            price_paid: price,
        })
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
   * The ID of the data contract.
   */
  dataContractId: Identifier;

  /**
   * The name of the document type within the contract.
   */
  documentType: string;

  /**
   * The ID of the document.
   */
  documentId: Identifier;

  /**
   * The owner's identity ID.
   */
  ownerId: Identifier;

  /**
   * The price in credits.
   * Set to 0 to remove the price and make the document not for sale.
   */
  price: bigint | number;

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

/// Main input struct for document set price options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentSetPriceOptionsInput {
    data_contract_id: IdentifierWasm,
    document_type: String,
    document_id: IdentifierWasm,
    owner_id: IdentifierWasm,
    price: u64,
}

fn deserialize_document_set_price_options(
    options: JsValue,
) -> Result<DocumentSetPriceOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "document set price options",
    )
}

/// Result of setting a price on a document.
#[wasm_bindgen(js_name = "DocumentSetPriceResult")]
pub struct DocumentSetPriceResultWasm {
    document_id: IdentifierWasm,
    price: u64,
}

#[wasm_bindgen(js_class = DocumentSetPriceResult)]
impl DocumentSetPriceResultWasm {
    /// The ID of the document.
    #[wasm_bindgen(getter = "documentId")]
    pub fn document_id(&self) -> IdentifierWasm {
        self.document_id.clone()
    }

    /// The price that was set in credits.
    #[wasm_bindgen(getter)]
    pub fn price(&self) -> js_sys::BigInt {
        js_sys::BigInt::from(self.price)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Set a price on a document to enable purchases.
    ///
    /// This method handles the complete price setting flow:
    /// 1. Fetches the document from Platform
    /// 2. Verifies ownership
    /// 3. Creates and signs the price update transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Set price options including document ID, price, and signer
    /// @returns Promise resolving to DocumentSetPriceResult with price info
    #[wasm_bindgen(js_name = "documentSetPrice")]
    pub async fn document_set_price(
        &self,
        options: DocumentSetPriceOptionsJs,
    ) -> Result<DocumentSetPriceResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_document_set_price_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let document_id: Identifier = parsed.document_id.into();
        let owner_id: Identifier = parsed.owner_id.into();
        let price = parsed.price;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(self.inner_sdk(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Get document type
        let document_type_ref = data_contract
            .document_type_for_name(&parsed.document_type)
            .map_err(|e| {
                WasmSdkError::not_found(format!(
                    "Document type '{}' not found: {}",
                    parsed.document_type, e
                ))
            })?;

        // Fetch the existing document
        use dash_sdk::platform::DocumentQuery;

        let query = DocumentQuery::new_with_data_contract_id(
            self.inner_sdk(),
            contract_id,
            &parsed.document_type,
        )
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
        .with_document_id(&document_id);

        let existing_doc = dash_sdk::platform::Document::fetch(self.inner_sdk(), query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        let current_revision = existing_doc
            .revision()
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision is missing"))?;

        // Verify ownership
        if existing_doc.owner_id() != owner_id {
            return Err(WasmSdkError::invalid_argument(
                "Only the document owner can set its price",
            ));
        }

        // Get identity contract nonce
        let identity_contract_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await?;

        // Create document with incremented revision for the price update
        let price_update_document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id,
            properties: existing_doc.properties().clone(),
            revision: Some(current_revision + 1),
            created_at: existing_doc.created_at(),
            updated_at: existing_doc.updated_at(),
            transferred_at: existing_doc.transferred_at(),
            created_at_block_height: existing_doc.created_at_block_height(),
            updated_at_block_height: existing_doc.updated_at_block_height(),
            transferred_at_block_height: existing_doc.transferred_at_block_height(),
            created_at_core_block_height: existing_doc.created_at_core_block_height(),
            updated_at_core_block_height: existing_doc.updated_at_core_block_height(),
            transferred_at_core_block_height: existing_doc.transferred_at_core_block_height(),
            creator_id: existing_doc.creator_id(),
        });

        // Create the price update transition
        let state_transition = BatchTransition::new_document_update_price_transition_from_document(
            price_update_document,
            document_type_ref,
            price,
            &identity_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create price update transition: {}", e))
        })?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        let st: StateTransition = state_transition;
        st.broadcast(self.inner_sdk(), settings).await?;

        Ok(DocumentSetPriceResultWasm {
            document_id: document_id.into(),
            price,
        })
    }
}
