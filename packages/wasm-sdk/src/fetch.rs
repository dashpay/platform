//! # Fetch Module
//!
//! This module provides a WASM-compatible way to fetch data from Platform.
//! It allows fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: A trait that defines how to fetch data from Platform in WASM environment.

use crate::dapi_client::{DapiClient, DapiClientConfig};
use crate::dpp::{DataContractWasm, IdentityWasm};
use crate::error::to_js_error;
use crate::sdk::WasmSdk;
use dpp::identity::Identity;
use dpp::prelude::DataContract;
// use dpp::document::Document; // Currently unused
use js_sys;
use platform_value::Identifier;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
// use wasm_drive_verify::document_verification::verify_document_proof; // Currently unused
use wasm_drive_verify::identity_verification::verify_full_identity_by_identity_id;

/// Options for fetch operations
#[wasm_bindgen]
#[derive(Clone, Debug, Default)]
pub struct FetchOptions {
    /// Number of retries for the request
    pub retries: Option<u32>,
    /// Timeout in milliseconds
    pub timeout: Option<u32>,
    /// Whether to request proof
    pub prove: Option<bool>,
}

#[wasm_bindgen]
impl FetchOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of retries
    #[wasm_bindgen(js_name = withRetries)]
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }

    /// Set the timeout in milliseconds
    #[wasm_bindgen(js_name = withTimeout)]
    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout = Some(timeout_ms);
        self
    }

    /// Set whether to request proof
    #[wasm_bindgen(js_name = withProve)]
    pub fn with_prove(mut self, prove: bool) -> Self {
        self.prove = Some(prove);
        self
    }
}

/// Fetch trait for retrieving data from Platform
#[allow(async_fn_in_trait)]
pub trait Fetch {
    /// Fetch an identity by ID
    async fn fetch_identity(
        &self,
        id: String,
        options: Option<FetchOptions>,
    ) -> Result<IdentityWasm, JsError>;

    /// Fetch a data contract by ID
    async fn fetch_data_contract(
        &self,
        id: String,
        options: Option<FetchOptions>,
    ) -> Result<DataContractWasm, JsError>;

    /// Fetch a document by ID
    async fn fetch_document(
        &self,
        id: String,
        contract_id: String,
        document_type: String,
        options: Option<FetchOptions>,
    ) -> Result<JsValue, JsError>;
}

/// Implementation of Fetch for WasmSdk
impl Fetch for WasmSdk {
    /// Fetch an identity by ID
    async fn fetch_identity(
        &self,
        id: String,
        options: Option<FetchOptions>,
    ) -> Result<IdentityWasm, JsError> {
        let options = options.unwrap_or_default();
        let prove = options.prove.unwrap_or(false);

        // Create DAPI client
        let client_config = DapiClientConfig::new(self.network());
        if let Some(timeout) = options.timeout {
            client_config.clone().set_timeout(timeout);
        }
        if let Some(retries) = options.retries {
            client_config.clone().set_retries(retries);
        }

        let client = DapiClient::new(client_config)?;

        // Fetch identity
        let response = client.get_identity(id.clone(), prove).await?;

        // Parse response
        if let Some(response_obj) = response.dyn_ref::<js_sys::Object>() {
            // Extract identity data
            let identity_value = js_sys::Reflect::get(response_obj, &"identity".into())
                .map_err(|_| JsError::new("Failed to get identity from response"))?;

            if identity_value.is_null() || identity_value.is_undefined() {
                return Err(JsError::new("Identity not found"));
            }

            // If we have proof, verify it
            if prove {
                let proof_value = js_sys::Reflect::get(response_obj, &"proof".into())
                    .map_err(|_| JsError::new("Failed to get proof from response"))?;

                if let Some(proof_str) = proof_value.as_string() {
                    use base64::Engine;
                    let proof_bytes = base64::engine::general_purpose::STANDARD
                        .decode(proof_str)
                        .map_err(|e| JsError::new(&format!("Failed to decode proof: {}", e)))?;

                    // Verify proof using wasm-drive-verify
                    let identifier = Identifier::from_string(
                        &id,
                        platform_value::string_encoding::Encoding::Base58,
                    )
                    .map_err(to_js_error)?;

                    let proof_array = js_sys::Uint8Array::from(&proof_bytes[..]);
                    let identity_id_bytes = identifier.to_buffer();
                    let identity_id_array = js_sys::Uint8Array::from(&identity_id_bytes[..]);

                    match verify_full_identity_by_identity_id(
                        &proof_array,
                        false,
                        &identity_id_array,
                        1,
                    ) {
                        Ok(result) => {
                            // The identity is returned as JsValue, we need to deserialize it
                            let identity_js = result.identity();
                            let identity_json = js_sys::JSON::stringify(&identity_js)
                                .map_err(|_| JsError::new("Failed to stringify verified identity"))?
                                .as_string()
                                .ok_or_else(|| JsError::new("Invalid identity JSON"))?;

                            let identity: Identity =
                                serde_json::from_str(&identity_json).map_err(|e| {
                                    JsError::new(&format!(
                                        "Failed to parse verified identity: {}",
                                        e
                                    ))
                                })?;

                            return Ok(IdentityWasm::from(identity));
                        }
                        Err(e) => {
                            return Err(JsError::new(&format!(
                                "Proof verification failed: {:?}",
                                e
                            )));
                        }
                    }
                }
            }

            // Convert identity from JS object to Identity
            let identity_json = js_sys::JSON::stringify(&identity_value)
                .map_err(|_| JsError::new("Failed to stringify identity"))?
                .as_string()
                .ok_or_else(|| JsError::new("Invalid identity JSON"))?;

            let identity: Identity = serde_json::from_str(&identity_json)
                .map_err(|e| JsError::new(&format!("Failed to parse identity: {}", e)))?;

            Ok(IdentityWasm::from(identity))
        } else {
            Err(JsError::new("Invalid response format"))
        }
    }

    /// Fetch a data contract by ID
    async fn fetch_data_contract(
        &self,
        id: String,
        options: Option<FetchOptions>,
    ) -> Result<DataContractWasm, JsError> {
        let options = options.unwrap_or_default();
        let prove = options.prove.unwrap_or(false);

        // Create DAPI client
        let client_config = DapiClientConfig::new(self.network());
        if let Some(timeout) = options.timeout {
            client_config.clone().set_timeout(timeout);
        }
        if let Some(retries) = options.retries {
            client_config.clone().set_retries(retries);
        }

        let client = DapiClient::new(client_config)?;

        // Fetch data contract
        let response = client.get_data_contract(id.clone(), prove).await?;

        // Parse response
        if let Some(response_obj) = response.dyn_ref::<js_sys::Object>() {
            // Extract data contract
            let contract_value = js_sys::Reflect::get(response_obj, &"dataContract".into())
                .map_err(|_| JsError::new("Failed to get data contract from response"))?;

            if contract_value.is_null() || contract_value.is_undefined() {
                return Err(JsError::new("Data contract not found"));
            }

            // Data contract proof verification is available in the verify module
            // using verify_data_contract_by_id(). However, automatic verification
            // during fetch would require handling the proof from the response.
            // The DAPI client currently returns JSON responses without proof data.

            // Convert data contract from JS object
            let contract_json = js_sys::JSON::stringify(&contract_value)
                .map_err(|_| JsError::new("Failed to stringify data contract"))?
                .as_string()
                .ok_or_else(|| JsError::new("Invalid data contract JSON"))?;

            let contract: DataContract = serde_json::from_str(&contract_json)
                .map_err(|e| JsError::new(&format!("Failed to parse data contract: {}", e)))?;

            Ok(DataContractWasm::from(contract))
        } else {
            Err(JsError::new("Invalid response format"))
        }
    }

    /// Fetch a document by ID
    async fn fetch_document(
        &self,
        id: String,
        contract_id: String,
        document_type: String,
        options: Option<FetchOptions>,
    ) -> Result<JsValue, JsError> {
        let options = options.unwrap_or_default();
        let prove = options.prove.unwrap_or(false);

        // Create DAPI client
        let client_config = DapiClientConfig::new(self.network());
        if let Some(timeout) = options.timeout {
            client_config.clone().set_timeout(timeout);
        }
        if let Some(retries) = options.retries {
            client_config.clone().set_retries(retries);
        }

        let client = DapiClient::new(client_config)?;

        // Create where clause to find document by ID
        let where_clause = serde_json::json!({
            "$id": id
        });

        // Fetch documents
        let response = client
            .get_documents(
                contract_id.clone(),
                document_type,
                serde_wasm_bindgen::to_value(&where_clause)?,
                JsValue::NULL,
                1,
                None,
                prove,
            )
            .await?;

        // Parse response
        if let Some(response_obj) = response.dyn_ref::<js_sys::Object>() {
            // Extract documents array
            let documents_value = js_sys::Reflect::get(response_obj, &"documents".into())
                .map_err(|_| JsError::new("Failed to get documents from response"))?;

            if let Some(documents_array) = documents_value.dyn_ref::<js_sys::Array>() {
                if documents_array.length() == 0 {
                    return Err(JsError::new("Document not found"));
                }

                let document_value = documents_array.get(0);

                // If we have proof, verify it
                if prove {
                    let proof_value = js_sys::Reflect::get(response_obj, &"proof".into())
                        .map_err(|_| JsError::new("Failed to get proof from response"))?;

                    if let Some(proof_str) = proof_value.as_string() {
                        use base64::Engine;
                        let _proof_bytes = base64::engine::general_purpose::STANDARD
                            .decode(proof_str)
                            .map_err(|e| JsError::new(&format!("Failed to decode proof: {}", e)))?;

                        // Document proof verification is now available!
                        // However, automatic verification during fetch would require:
                        // 1. First fetching the contract (if not cached)
                        // 2. Using it to verify the documents
                        //
                        // For now, users can manually verify using:
                        // - verifyDocumentsWithContract() when they have the contract
                        // - verifySingleDocument() for individual documents
                        //
                        // Automatic verification during fetch is left as a future enhancement
                        // to avoid circular dependencies and maintain flexibility
                    }
                }

                Ok(document_value)
            } else {
                Err(JsError::new("Invalid documents array in response"))
            }
        } else {
            Err(JsError::new("Invalid response format"))
        }
    }
}

/// Fetch an identity by ID
#[wasm_bindgen(js_name = fetchIdentity)]
pub async fn fetch_identity(
    sdk: &WasmSdk,
    identity_id: String,
    options: Option<FetchOptions>,
) -> Result<IdentityWasm, JsError> {
    sdk.fetch_identity(identity_id, options).await
}

/// Fetch a data contract by ID
#[wasm_bindgen(js_name = fetchDataContract)]
pub async fn fetch_data_contract(
    sdk: &WasmSdk,
    contract_id: String,
    options: Option<FetchOptions>,
) -> Result<DataContractWasm, JsError> {
    sdk.fetch_data_contract(contract_id, options).await
}

/// Fetch a document by ID
#[wasm_bindgen(js_name = fetchDocument)]
pub async fn fetch_document(
    sdk: &WasmSdk,
    document_id: String,
    contract_id: String,
    document_type: String,
    options: Option<FetchOptions>,
) -> Result<JsValue, JsError> {
    sdk.fetch_document(document_id, contract_id, document_type, options)
        .await
}

/// Fetch identity balance
#[wasm_bindgen(js_name = fetchIdentityBalance)]
pub async fn fetch_identity_balance(
    sdk: &WasmSdk,
    identity_id: String,
    options: Option<FetchOptions>,
) -> Result<u64, JsError> {
    let identity = sdk.fetch_identity(identity_id, options).await?;
    Ok(identity.balance() as u64)
}

/// Fetch identity nonce
#[wasm_bindgen(js_name = fetchIdentityNonce)]
pub async fn fetch_identity_nonce(
    sdk: &WasmSdk,
    _identity_id: String,
    _contract_id: String,
) -> Result<u64, JsError> {
    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let _client = DapiClient::new(client_config)?;

    // For now, use a mock implementation
    // In the future, this will use a specific DAPI method
    Ok(0)
}
