use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, Value};
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
    RegisterDpnsNameInput,
};
use dash_sdk::platform::{documents::document_query::DocumentQuery, Fetch, FetchMany, Identity};
use drive::query::{WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::ContextProvider;
use js_sys::Array;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::identity::IdentityWasm;
use wasm_dpp2::utils::{try_from_options_with, try_to_string};
use wasm_dpp2::IdentitySignerWasm;

#[wasm_bindgen(js_name = "RegisterDpnsNameResult")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDpnsNameResultWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "preorderDocumentId")]
    pub preorder_document_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "domainDocumentId")]
    pub domain_document_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "fullDomainName")]
    pub full_domain_name: String,
}

#[wasm_bindgen(js_name = "DpnsUsernameInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DpnsUsernameInfoWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub username: String,
    #[wasm_bindgen(getter_with_clone, js_name = "identityId")]
    pub identity_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "documentId")]
    pub document_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = DpnsUsernameInfo)]
impl DpnsUsernameInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(username: String, identity_id: IdentifierWasm, document_id: IdentifierWasm) -> Self {
        Self {
            username,
            identity_id,
            document_id,
        }
    }
}

impl_wasm_serde_conversions!(RegisterDpnsNameResultWasm, RegisterDpnsNameResult);
impl_wasm_serde_conversions!(DpnsUsernameInfoWasm, DpnsUsernameInfo);

const DEFAULT_DPNS_USERNAMES_LIMIT: u32 = 10;

fn resolve_dpns_usernames_limit(limit: Option<u32>) -> u32 {
    match limit {
        Some(0) | None => DEFAULT_DPNS_USERNAMES_LIMIT,
        Some(value) => value,
    }
}

fn usernames_from_documents(documents_result: Documents) -> Array {
    let usernames_array = Array::new();
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            let properties = doc.properties();
            if let (Some(Value::Text(label)), Some(Value::Text(parent_domain))) = (
                properties.get("label"),
                properties.get("normalizedParentDomainName"),
            ) {
                let username = format!("{}.{}", label, parent_domain);
                usernames_array.push(&JsValue::from(username));
            }
        }
    }
    usernames_array
}

// ============================================================================
// DPNS Register Name
// ============================================================================

/// TypeScript interface for DPNS name registration options
#[wasm_bindgen(typescript_custom_section)]
const DPNS_REGISTER_NAME_OPTIONS_TS: &'static str = r#"
/**
 * Options for registering a DPNS username on Dash Platform.
 */
export interface DpnsRegisterNameOptions {
  /**
   * The username label to register (without the .dash suffix).
   * Must be a valid DPNS username (3-63 characters, alphanumeric and hyphens).
   */
  label: string;

  /**
   * The identity that will own the username.
   * Fetch the identity first using `getIdentity()`.
   */
  identity: Identity;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional callback called after the preorder document is submitted.
   * Receives the preorder Document object.
   */
  preorderCallback?: (preorderDocument: Document) => void;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DpnsRegisterNameOptions")]
    pub type DpnsRegisterNameOptionsJs;
}

// TS definition for DpnsUsernamesQuery used by getDpnsUsernames*
#[wasm_bindgen(typescript_custom_section)]
const DPNS_USERNAMES_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving DPNS usernames.
 */
export interface DpnsUsernamesQuery {
  /**
   * Identity to fetch usernames for.
   */
  identityId: IdentifierLike;

  /**
   * Maximum number of usernames to return. Use 0 for default.
   * @default 10
   */
  limit?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DpnsUsernamesQuery")]
    pub type DpnsUsernamesQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DpnsUsernamesQueryInput {
    identity_id: IdentifierWasm,
    #[serde(default)]
    limit: Option<u32>,
}

struct DpnsUsernamesQueryParsed {
    identity_id: Identifier,
    limit: Option<u32>,
}

fn parse_dpns_usernames_query(
    query: DpnsUsernamesQueryJs,
) -> Result<DpnsUsernamesQueryParsed, WasmSdkError> {
    let input: DpnsUsernamesQueryInput =
        deserialize_required_query(query, "Query object is required", "DPNS usernames query")?;

    Ok(DpnsUsernamesQueryParsed {
        identity_id: input.identity_id.into(),
        limit: input.limit,
    })
}

/// Extracts an optional JS function from options.
fn extract_callback_from_options(
    options: &JsValue,
    field_name: &str,
) -> Result<Option<js_sys::Function>, WasmSdkError> {
    let value = js_sys::Reflect::get(options, &JsValue::from_str(field_name))
        .map_err(|_| WasmSdkError::invalid_argument(format!("Failed to get {}", field_name)))?;

    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }

    let func = value.dyn_into::<js_sys::Function>().map_err(|_| {
        WasmSdkError::invalid_argument(format!("{} must be a function", field_name))
    })?;

    Ok(Some(func))
}

/// DPNS contract ID constant
const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
/// DPNS domain document type
const DPNS_DOCUMENT_TYPE: &str = "domain";

impl WasmSdk {
    /// Get DPNS contract, checking context provider cache first, then fetching if needed.
    async fn get_dpns_contract(&self) -> Result<Arc<DataContract>, WasmSdkError> {
        let contract_id =
            Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        // First check if the contract is available in the context provider
        if let Some(context_provider) = self.as_ref().context_provider() {
            if let Ok(Some(contract)) =
                context_provider.get_data_contract(&contract_id, self.as_ref().version())
            {
                return Ok(contract);
            }
        }

        // If not in context, fetch from platform
        let contract = DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::generic("DPNS contract not found"))?;
        Ok(Arc::new(contract))
    }

    async fn fetch_dpns_usernames(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<Array, WasmSdkError> {
        // Use rs-sdk method with resolved limit for consistency with proof path
        let resolved_limit = resolve_dpns_usernames_limit(limit);
        let usernames = self
            .as_ref()
            .get_dpns_usernames_by_identity(identity_id, Some(resolved_limit))
            .await?;

        let array = Array::new();
        for username in usernames {
            array.push(&JsValue::from_str(&username.full_name));
        }
        Ok(array)
    }

    async fn fetch_dpns_usernames_with_proof(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // For proof queries, we still need to use document query directly
        // as rs-sdk doesn't have a with_proof variant
        let dpns_contract = self.get_dpns_contract().await?;

        let query = DocumentQuery {
            data_contract: dpns_contract,
            document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
            where_clauses: vec![WhereClause {
                field: "records.identity".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(identity_id.to_buffer()),
            }],
            order_by_clauses: vec![],
            limit: resolve_dpns_usernames_limit(limit),
            start: None,
        };

        let (documents_result, metadata, proof) =
            Document::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;
        let usernames_array = usernames_from_documents(documents_result);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            usernames_array,
            metadata,
            proof,
        ))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "dpnsConvertToHomographSafe")]
    pub fn dpns_convert_to_homograph_safe(input: &str) -> String {
        convert_to_homograph_safe_chars(input)
    }

    #[wasm_bindgen(js_name = "dpnsIsValidUsername")]
    pub fn dpns_is_valid_username(label: &str) -> bool {
        is_valid_username(label)
    }

    #[wasm_bindgen(js_name = "dpnsIsContestedUsername")]
    pub fn dpns_is_contested_username(label: &str) -> bool {
        is_contested_username(label)
    }

    /// Register a DPNS username on Dash Platform.
    ///
    /// This method handles the complete DPNS registration flow:
    /// 1. Creates and submits a preorder document
    /// 2. Waits for preorder confirmation
    /// 3. Creates and submits the domain document
    /// 4. Returns the result with both document IDs
    ///
    /// @param options - Registration options including label, identity, key, and signer
    /// @returns Promise that resolves to the registration result
    #[wasm_bindgen(js_name = "dpnsRegisterName")]
    pub async fn dpns_register_name(
        &self,
        options: DpnsRegisterNameOptionsJs,
    ) -> Result<RegisterDpnsNameResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Extract label from options
        let label: String =
            try_from_options_with(&options_value, "label", |v| try_to_string(v, "label"))?;

        // Extract identity from options
        let identity: Identity = IdentityWasm::try_from_options(&options_value, "identity")?.into();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_public_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional preorder callback
        let preorder_callback = extract_callback_from_options(&options_value, "preorderCallback")?;

        // Set up the callback if provided
        thread_local! {
            static PREORDER_CALLBACK: std::cell::RefCell<Option<js_sys::Function>>
                = const { std::cell::RefCell::new(None) };
        }

        if let Some(ref js_callback) = preorder_callback {
            PREORDER_CALLBACK.with(|cb| {
                *cb.borrow_mut() = Some(js_callback.clone());
            });
        }

        // Get DPNS contract ID for the callback
        let dpns_contract_id = Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).unwrap();

        let callback_box = if preorder_callback.is_some() {
            Some(Box::new(move |doc: &Document| {
                PREORDER_CALLBACK.with(|cb| {
                    if let Some(js_callback) = cb.borrow().as_ref() {
                        // Convert to DocumentWasm with DPNS metadata
                        let doc_wasm = DocumentWasm::new(
                            doc.clone(),
                            dpns_contract_id,
                            "preorder".to_string(),
                            None,
                        );
                        let _ = js_callback.call1(&JsValue::NULL, &JsValue::from(doc_wasm));
                    }
                });
            }) as Box<dyn FnOnce(&Document) + Send>)
        } else {
            None
        };

        let input = RegisterDpnsNameInput {
            label,
            identity,
            identity_public_key,
            signer,
            preorder_callback: callback_box,
        };

        let result = self.as_ref().register_dpns_name(input).await?;

        // Clean up callback
        PREORDER_CALLBACK.with(|cb| {
            *cb.borrow_mut() = None;
        });

        Ok(RegisterDpnsNameResultWasm {
            preorder_document_id: IdentifierWasm::from(result.preorder_document.id()),
            domain_document_id: IdentifierWasm::from(result.domain_document.id()),
            full_domain_name: result.full_domain_name,
        })
    }

    #[wasm_bindgen(js_name = "dpnsIsNameAvailable")]
    pub async fn dpns_is_name_available(&self, label: &str) -> Result<bool, WasmSdkError> {
        self.as_ref()
            .is_dpns_name_available(label)
            .await
            .map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(js_name = "dpnsResolveName")]
    pub async fn dpns_resolve_name(&self, name: &str) -> Result<Option<String>, WasmSdkError> {
        let result = self.as_ref().resolve_dpns_name(name).await?;

        Ok(result.map(|identity_id| identity_id.to_string(Encoding::Base58)))
    }

    #[wasm_bindgen(js_name = "getDpnsUsernameByName")]
    pub async fn get_dpns_username_by_name(
        &self,
        username: &str,
    ) -> Result<Option<DpnsUsernameInfoWasm>, WasmSdkError> {
        let parts: Vec<&str> = username.split('.').collect();
        if parts.len() != 2 {
            return Err(WasmSdkError::invalid_argument(
                "Invalid username format. Expected format: label.domain",
            ));
        }
        let label = parts[0];
        let domain = parts[1];

        let dpns_contract = self.get_dpns_contract().await?;
        let mut query = DocumentQuery::new(dpns_contract, DPNS_DOCUMENT_TYPE)?;

        query = query.with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(label.to_lowercase()),
        });

        query = query.with_where(WhereClause {
            field: "normalizedParentDomainName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(domain.to_lowercase()),
        });

        let documents = Document::fetch_many(self.as_ref(), query).await?;

        if let Some((_, Some(document))) = documents.into_iter().next() {
            Ok(Some(DpnsUsernameInfoWasm {
                username: username.to_string(),
                identity_id: IdentifierWasm::from(document.owner_id()),
                document_id: IdentifierWasm::from(document.id()),
            }))
        } else {
            Ok(None)
        }
    }

    #[wasm_bindgen(
        js_name = "getDpnsUsernameByNameWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<DpnsUsernameInfo | null>"
    )]
    pub async fn get_dpns_username_by_name_with_proof_info(
        &self,
        username: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let parts: Vec<&str> = username.split('.').collect();
        if parts.len() != 2 {
            return Err(WasmSdkError::invalid_argument(
                "Invalid username format. Expected format: label.domain",
            ));
        }
        let label = parts[0];
        let domain = parts[1];

        let dpns_contract = self.get_dpns_contract().await?;
        let mut query = DocumentQuery::new(dpns_contract, DPNS_DOCUMENT_TYPE)?;

        query = query.with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(label.to_lowercase()),
        });

        query = query.with_where(WhereClause {
            field: "normalizedParentDomainName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(domain.to_lowercase()),
        });

        let (documents, metadata, proof) =
            Document::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let data = if let Some((_, Some(document))) = documents.into_iter().next() {
            let result = DpnsUsernameInfoWasm {
                username: username.to_string(),
                identity_id: IdentifierWasm::from(document.owner_id()),
                document_id: IdentifierWasm::from(document.id()),
            };

            // Use json_compatible() to ensure objects become plain JS objects (not Maps)
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            result.serialize(&serializer).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize username info: {}", e))
            })?
        } else {
            JsValue::NULL
        };

        let response = ProofMetadataResponseWasm::from_sdk_parts(data, metadata, proof);

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getDpnsUsernames", unchecked_return_type = "Array<string>")]
    pub async fn get_dpns_usernames(
        &self,
        query: DpnsUsernamesQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let params = parse_dpns_usernames_query(query)?;
        self.fetch_dpns_usernames(params.identity_id, params.limit)
            .await
    }

    #[wasm_bindgen(js_name = "getDpnsUsername")]
    pub async fn get_dpns_username(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        identity_id: IdentifierLikeJs,
    ) -> Result<Option<String>, WasmSdkError> {
        let identity_id_parsed = IdentifierWasm::try_from(identity_id)?.into();

        let array = self
            .fetch_dpns_usernames(identity_id_parsed, Some(1))
            .await?;

        if array.length() == 0 {
            return Ok(None);
        }

        array
            .get(0)
            .as_string()
            .map(Some)
            .ok_or_else(|| WasmSdkError::generic("DPNS username is not a string"))
    }

    #[wasm_bindgen(
        js_name = "getDpnsUsernamesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<string>>"
    )]
    pub async fn get_dpns_usernames_with_proof_info(
        &self,
        query: DpnsUsernamesQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_dpns_usernames_query(query)?;
        self.fetch_dpns_usernames_with_proof(params.identity_id, params.limit)
            .await
    }

    #[wasm_bindgen(
        js_name = "getDpnsUsernameWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<string | null>"
    )]
    pub async fn get_dpns_username_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")]
        identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let identity_id_parsed = IdentifierWasm::try_from(identity_id)?.into();

        let mut response = self
            .fetch_dpns_usernames_with_proof(identity_id_parsed, Some(1))
            .await?;

        let usernames = js_sys::Array::from(&response.data());
        let username = if usernames.length() > 0 {
            usernames.get(0)
        } else {
            JsValue::NULL
        };

        response.set_data(username);

        Ok(response)
    }
}
