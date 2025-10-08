use crate::error::WasmSdkError;
use crate::queries::{ProofInfoWasm, ProofMetadataResponse, ResponseMetadataWasm};
use crate::sdk::WasmSdk;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, Value};
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
    RegisterDpnsNameInput,
};
use dash_sdk::platform::{documents::document_query::DocumentQuery, Fetch, FetchMany, Identity};
use drive::query::{WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use js_sys::Array;
use serde::{Deserialize, Serialize};
use simple_signer::SingleKeySigner;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDpnsNameResult {
    pub preorder_document_id: String,
    pub domain_document_id: String,
    pub full_domain_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DpnsUsernameInfo {
    username: String,
    identity_id: String,
    document_id: String,
}

#[wasm_bindgen(js_name = "DpnsUsernamesProofResponse")]
#[derive(Clone)]
pub struct DpnsUsernamesProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub usernames: Array,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "DpnsUsernameProofResponse")]
#[derive(Clone)]
pub struct DpnsUsernameProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub username: JsValue,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
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

    #[wasm_bindgen(js_name = "dpnsRegisterName")]
    pub async fn dpns_register_name(
        &self,
        label: &str,
        identity_id: &str,
        public_key_id: u32,
        private_key_wif: &str,
        preorder_callback: Option<js_sys::Function>,
    ) -> Result<JsValue, WasmSdkError> {
        let identity_id_parsed = Identifier::from_string(identity_id, Encoding::Base58)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let identity = Identity::fetch(self.as_ref(), identity_id_parsed)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        let signer = SingleKeySigner::new(private_key_wif).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid private key WIF: {}", e))
        })?;

        let identity_public_key = identity
            .get_public_key_by_id(public_key_id)
            .ok_or_else(|| {
                WasmSdkError::not_found(format!("Public key with ID {} not found", public_key_id))
            })?
            .clone();

        thread_local! {
            static PREORDER_CALLBACK: std::cell::RefCell<Option<js_sys::Function>> = const { std::cell::RefCell::new(None) };
        }

        if let Some(ref js_callback) = preorder_callback {
            PREORDER_CALLBACK.with(|cb| {
                *cb.borrow_mut() = Some(js_callback.clone());
            });
        }

        let callback_box = if preorder_callback.is_some() {
            Some(Box::new(move |doc: &Document| {
                PREORDER_CALLBACK.with(|cb| {
                    if let Some(js_callback) = cb.borrow().as_ref() {
                        let preorder_info = serde_json::json!({
                            "documentId": doc.id().to_string(Encoding::Base58),
                            "ownerId": doc.owner_id().to_string(Encoding::Base58),
                            "revision": doc.revision().unwrap_or(0),
                            "createdAt": doc.created_at(),
                            "createdAtBlockHeight": doc.created_at_block_height(),
                            "createdAtCoreBlockHeight": doc.created_at_core_block_height(),
                            "message": "Preorder document submitted successfully",
                        });

                        if let Ok(js_value) = serde_wasm_bindgen::to_value(&preorder_info) {
                            let _ = js_callback.call1(&JsValue::NULL, &js_value);
                        }
                    }
                });
            }) as Box<dyn FnOnce(&Document) + Send>)
        } else {
            None
        };

        let input = RegisterDpnsNameInput {
            label: label.to_string(),
            identity,
            identity_public_key,
            signer,
            preorder_callback: callback_box,
        };

        let result = self.as_ref().register_dpns_name(input).await?;

        PREORDER_CALLBACK.with(|cb| {
            *cb.borrow_mut() = None;
        });

        let js_result = RegisterDpnsNameResult {
            preorder_document_id: result.preorder_document.id().to_string(Encoding::Base58),
            domain_document_id: result.domain_document.id().to_string(Encoding::Base58),
            full_domain_name: result.full_domain_name,
        };

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        js_result
            .serialize(&serializer)
            .map_err(|e| WasmSdkError::serialization(format!("Failed to serialize result: {}", e)))
    }

    #[wasm_bindgen(js_name = "dpnsIsNameAvailable")]
    pub async fn dpns_is_name_available(&self, label: &str) -> Result<bool, WasmSdkError> {
        self.as_ref()
            .is_dpns_name_available(label)
            .await
            .map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(js_name = "dpnsResolveName")]
    pub async fn dpns_resolve_name(&self, name: &str) -> Result<JsValue, WasmSdkError> {
        let result = self.as_ref().resolve_dpns_name(name).await?;

        match result {
            Some(identity_id) => Ok(JsValue::from_str(&identity_id.to_string(Encoding::Base58))),
            None => Ok(JsValue::NULL),
        }
    }

    #[wasm_bindgen(js_name = "getDpnsUsernameByName")]
    pub async fn get_dpns_username_by_name(&self, username: &str) -> Result<JsValue, WasmSdkError> {
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        const DPNS_DOCUMENT_TYPE: &str = "domain";

        let parts: Vec<&str> = username.split('.').collect();
        if parts.len() != 2 {
            return Err(WasmSdkError::invalid_argument(
                "Invalid username format. Expected format: label.domain",
            ));
        }
        let label = parts[0];
        let domain = parts[1];

        let contract_id =
            Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        let mut query = DocumentQuery::new_with_data_contract_id(
            self.as_ref(),
            contract_id,
            DPNS_DOCUMENT_TYPE,
        )
        .await?;

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
            let result = DpnsUsernameInfo {
                username: username.to_string(),
                identity_id: document.owner_id().to_string(Encoding::Base58),
                document_id: document.id().to_string(Encoding::Base58),
            };

            serde_wasm_bindgen::to_value(&result).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
            })
        } else {
            Err(WasmSdkError::not_found(format!(
                "Username '{}' not found",
                username
            )))
        }
    }

    #[wasm_bindgen(js_name = "getDpnsUsernameByNameWithProofInfo")]
    pub async fn get_dpns_username_by_name_with_proof_info(
        &self,
        username: &str,
    ) -> Result<JsValue, WasmSdkError> {
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        const DPNS_DOCUMENT_TYPE: &str = "domain";

        let parts: Vec<&str> = username.split('.').collect();
        if parts.len() != 2 {
            return Err(WasmSdkError::invalid_argument(
                "Invalid username format. Expected format: label.domain",
            ));
        }
        let label = parts[0];
        let domain = parts[1];

        let contract_id =
            Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        let mut query = DocumentQuery::new_with_data_contract_id(
            self.as_ref(),
            contract_id,
            DPNS_DOCUMENT_TYPE,
        )
        .await?;

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

        if let Some((_, Some(document))) = documents.into_iter().next() {
            let result = DpnsUsernameInfo {
                username: username.to_string(),
                identity_id: document.owner_id().to_string(Encoding::Base58),
                document_id: document.id().to_string(Encoding::Base58),
            };

            let response = ProofMetadataResponse {
                data: result,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response.serialize(&serializer).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
            })
        } else {
            Err(WasmSdkError::not_found(format!(
                "Username '{}' not found",
                username
            )))
        }
    }

    #[wasm_bindgen(js_name = "getDpnsUsernames")]
    pub async fn get_dpns_usernames(
        &self,
        identity_id: &str,
        limit: Option<u32>,
    ) -> Result<JsValue, WasmSdkError> {
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        const DPNS_DOCUMENT_TYPE: &str = "domain";

        let identity_id_parsed = Identifier::from_string(identity_id, Encoding::Base58)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let contract_id =
            Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        let mut query = DocumentQuery::new_with_data_contract_id(
            self.as_ref(),
            contract_id,
            DPNS_DOCUMENT_TYPE,
        )
        .await?;

        let where_clause = WhereClause {
            field: "records.identity".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(identity_id_parsed.to_buffer()),
        };

        query = query.with_where(where_clause);
        query.limit = limit.unwrap_or(10);

        let documents_result: Documents = Document::fetch_many(self.as_ref(), query).await?;

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

        Ok(usernames_array.into())
    }

    #[wasm_bindgen(js_name = "getDpnsUsername")]
    pub async fn get_dpns_username(&self, identity_id: &str) -> Result<JsValue, WasmSdkError> {
        let result = self.get_dpns_usernames(identity_id, Some(1)).await?;
        let array = Array::from(&result);

        if array.length() > 0 {
            Ok(array.get(0))
        } else {
            Ok(JsValue::NULL)
        }
    }

    #[wasm_bindgen(js_name = "getDpnsUsernamesWithProofInfo")]
    pub async fn get_dpns_usernames_with_proof_info(
        &self,
        identity_id: &str,
        limit: Option<u32>,
    ) -> Result<DpnsUsernamesProofResponseWasm, WasmSdkError> {
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        const DPNS_DOCUMENT_TYPE: &str = "domain";

        let identity_id_parsed = Identifier::from_string(identity_id, Encoding::Base58)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        let contract_id =
            Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        let mut query = DocumentQuery::new_with_data_contract_id(
            self.as_ref(),
            contract_id,
            DPNS_DOCUMENT_TYPE,
        )
        .await?;

        let where_clause = WhereClause {
            field: "records.identity".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(identity_id_parsed.to_buffer()),
        };

        query = query.with_where(where_clause);
        query.limit = limit.unwrap_or(10);

        let (documents_result, metadata, proof) =
            Document::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

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

        Ok(DpnsUsernamesProofResponseWasm {
            usernames: usernames_array,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    #[wasm_bindgen(js_name = "getDpnsUsernameWithProofInfo")]
    pub async fn get_dpns_username_with_proof_info(
        &self,
        identity_id: &str,
    ) -> Result<DpnsUsernameProofResponseWasm, WasmSdkError> {
        let DpnsUsernamesProofResponseWasm {
            usernames,
            metadata,
            proof,
        } = self
            .get_dpns_usernames_with_proof_info(identity_id, Some(1))
            .await?;

        let username = if usernames.length() > 0 {
            usernames.get(0)
        } else {
            JsValue::NULL
        };

        Ok(DpnsUsernameProofResponseWasm {
            username,
            metadata,
            proof,
        })
    }
}
