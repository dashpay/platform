//! Document state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for document operations like create, replace, delete, etc.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use crate::settings::extract_settings_from_options;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::platform::documents::transitions::DocumentDeleteTransitionBuilder;
use dash_sdk::platform::transition::purchase_document::PurchaseDocument;
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::transition::transfer_document::TransferDocument;
use dash_sdk::platform::transition::update_price_of_document::UpdatePriceOfDocument;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::utils::{get_class_type, try_to_u64, IntoWasm};
use wasm_dpp2::IdentitySignerWasm;

/// Extracts a string field from a JS options object.
fn extract_string_from_options(
    options: &JsValue,
    field_name: &str,
) -> Result<String, WasmSdkError> {
    let value = js_sys::Reflect::get(options, &JsValue::from_str(field_name))
        .map_err(|_| WasmSdkError::invalid_argument(format!("{} is required", field_name)))?;

    value
        .as_string()
        .ok_or_else(|| WasmSdkError::invalid_argument(format!("{} must be a string", field_name)))
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
   * Must include dataContractId, documentTypeName, and ownerId.
   * Entropy is optional - if not set, it will be auto-generated.
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
    /// @returns Promise that resolves when the document is created
    #[wasm_bindgen(js_name = "documentCreate")]
    pub async fn document_create(
        &self,
        options: DocumentCreateOptionsJs,
    ) -> Result<(), WasmSdkError> {
        let options_value: JsValue = options.into();

        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options_value, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.get_data_contract_id().into();
        let document_type_name = document_wasm.get_document_type_name();

        // Get entropy from document if set, otherwise let rs-sdk generate it
        let entropy = document_wasm.get_entropy().and_then(|e| {
            if e.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&e);
                Some(arr)
            } else {
                None
            }
        });

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use PutDocument trait for creation
        document
            .put_to_platform_and_wait_for_response(
                self.inner_sdk(),
                document_type,
                entropy,
                identity_key,
                None, // token_payment_info
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
        let options_value: JsValue = options.into();

        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options_value, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.get_data_contract_id().into();
        let document_type_name = document_wasm.get_document_type_name();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use PutDocument trait for replacement (revision > INITIAL_REVISION triggers replace)
        document
            .put_to_platform_and_wait_for_response(
                self.inner_sdk(),
                document_type,
                None, // entropy not needed for replace
                identity_key,
                None, // token_payment_info
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
        let options_value: JsValue = options.into();

        // Extract document field - can be either a Document instance or plain object
        let document_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("document"))
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
                doc.get_id().into(),
                doc_inner.owner_id(),
                doc.get_data_contract_id().into(),
                doc.get_document_type_name(),
            )
        } else {
            // It's a plain object - extract individual fields
            (
                IdentifierWasm::try_from_options(&document_js, "id")?.into(),
                IdentifierWasm::try_from_options(&document_js, "ownerId")?.into(),
                IdentifierWasm::try_from_options(&document_js, "dataContractId")?.into(),
                extract_string_from_options(&document_js, "documentTypeName")?,
            )
        };

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build and execute delete transition using DocumentDeleteTransitionBuilder
        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::new(data_contract),
            document_type_name,
            document_id,
            owner_id,
        );

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
        let options_value: JsValue = options.into();

        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options_value, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.get_data_contract_id().into();
        let owner_id: Identifier = document.owner_id();
        let document_type_name = document_wasm.get_document_type_name();

        // Extract recipient ID from options
        let recipient_id: Identifier =
            IdentifierWasm::try_from_options(&options_value, "recipientId")?.into();

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

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use TransferDocument trait
        document
            .transfer_document_to_identity_and_wait_for_response(
                recipient_id,
                self.inner_sdk(),
                document_type,
                identity_key,
                None, // token_payment_info
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
        let options_value: JsValue = options.into();

        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options_value, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.get_data_contract_id().into();
        let document_type_name = document_wasm.get_document_type_name();

        // Extract buyer ID from options
        let buyer_id: Identifier =
            IdentifierWasm::try_from_options(&options_value, "buyerId")?.into();

        // Extract price from options
        let price_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("price"))
            .map_err(|_| WasmSdkError::invalid_argument("price is required"))?;
        let price: Credits = try_to_u64(price_js).map_err(|e| {
            WasmSdkError::invalid_argument(format!("price must be a valid u64: {}", e))
        })?;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use PurchaseDocument trait
        document
            .purchase_document_and_wait_for_response(
                price,
                self.inner_sdk(),
                document_type,
                buyer_id,
                identity_key,
                None, // token_payment_info
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
        let options_value: JsValue = options.into();

        // Extract document from options
        let document_wasm = DocumentWasm::try_from_options(&options_value, "document")?;
        let document: Document = document_wasm.clone().into();

        // Get metadata from document
        let contract_id: Identifier = document_wasm.get_data_contract_id().into();
        let document_type_name = document_wasm.get_document_type_name();

        // Extract price from options
        let price_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("price"))
            .map_err(|_| WasmSdkError::invalid_argument("price is required"))?;
        let price: Credits = try_to_u64(price_js).map_err(|e| {
            WasmSdkError::invalid_argument(format!("price must be a valid u64: {}", e))
        })?;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Get document type (owned)
        let document_type = get_document_type(&data_contract, &document_type_name)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use UpdatePriceOfDocument trait
        document
            .update_price_of_document_and_wait_for_response(
                price,
                self.inner_sdk(),
                document_type,
                identity_key,
                None, // token_payment_info
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
