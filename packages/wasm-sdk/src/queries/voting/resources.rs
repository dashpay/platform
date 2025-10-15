use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::utils::{js_value_to_platform_value, js_values_to_platform_values};
use crate::WasmSdkError;
use dash_sdk::platform::FetchMany;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::ContestedResource;
use js_sys::Array;
use platform_value::{Identifier, Value};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "VotePollsByDocumentTypeQuery")]
pub struct VotePollsByDocumentTypeQueryWasm(VotePollsByDocumentTypeQuery);

impl VotePollsByDocumentTypeQueryWasm {
    pub(crate) fn into_inner(self) -> VotePollsByDocumentTypeQuery {
        self.0
    }
}

#[wasm_bindgen(js_name = "VotePollsByDocumentTypeQueryBuilder")]
pub struct VotePollsByDocumentTypeQueryBuilder {
    contract_id: Identifier,
    document_type_name: String,
    index_name: String,
    start_index_values: Vec<Value>,
    end_index_values: Vec<Value>,
    start_at_value: Option<(Value, bool)>,
    limit: Option<u16>,
    order_ascending: bool,
}

#[wasm_bindgen(js_class = VotePollsByDocumentTypeQueryBuilder)]
impl VotePollsByDocumentTypeQueryBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        data_contract_id: &str,
        document_type_name: &str,
        index_name: &str,
    ) -> Result<VotePollsByDocumentTypeQueryBuilder, WasmSdkError> {
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        Ok(Self {
            contract_id,
            document_type_name: document_type_name.to_string(),
            index_name: index_name.to_string(),
            start_index_values: Vec::new(),
            end_index_values: Vec::new(),
            start_at_value: None,
            limit: None,
            order_ascending: true,
        })
    }

    #[wasm_bindgen(js_name = "withStartIndexValues")]
    pub fn with_start_index_values(
        mut self,
        values: Vec<JsValue>,
    ) -> Result<VotePollsByDocumentTypeQueryBuilder, WasmSdkError> {
        self.start_index_values = js_values_to_platform_values(values)?;
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withEndIndexValues")]
    pub fn with_end_index_values(
        mut self,
        values: Vec<JsValue>,
    ) -> Result<VotePollsByDocumentTypeQueryBuilder, WasmSdkError> {
        self.end_index_values = js_values_to_platform_values(values)?;
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withStartAtValue")]
    pub fn with_start_at_value(
        mut self,
        value: JsValue,
        included: bool,
    ) -> Result<VotePollsByDocumentTypeQueryBuilder, WasmSdkError> {
        if value.is_null() || value.is_undefined() {
            self.start_at_value = None;
        } else {
            let platform_value = js_value_to_platform_value(value)?;
            self.start_at_value = Some((platform_value, included));
        }
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withLimit")]
    pub fn with_limit(
        mut self,
        limit: Option<u32>,
    ) -> Result<VotePollsByDocumentTypeQueryBuilder, WasmSdkError> {
        self.limit = match limit {
            Some(0) => None,
            Some(count) => {
                if count > u16::MAX as u32 {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "limit {} exceeds maximum of {}",
                        count,
                        u16::MAX
                    )));
                }
                Some(count as u16)
            }
            None => None,
        };
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withOrderAscending")]
    pub fn with_order_ascending(
        mut self,
        order_ascending: bool,
    ) -> VotePollsByDocumentTypeQueryBuilder {
        self.order_ascending = order_ascending;
        self
    }

    #[wasm_bindgen(js_name = "build")]
    pub fn build(self) -> VotePollsByDocumentTypeQueryWasm {
        let VotePollsByDocumentTypeQueryBuilder {
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit,
            order_ascending,
        } = self;

        VotePollsByDocumentTypeQueryWasm(VotePollsByDocumentTypeQuery {
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit,
            order_ascending,
        })
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResources")]
    pub async fn get_contested_resources(
        &self,
        query: VotePollsByDocumentTypeQueryWasm,
    ) -> Result<Array, WasmSdkError> {
        let contested_resources =
            ContestedResource::fetch_many(self.as_ref(), query.into_inner()).await?;

        let array = Array::new();
        for resource in contested_resources.0 {
            let js_value = serde_wasm_bindgen::to_value(&resource.0).map_err(|e| {
                WasmSdkError::serialization(format!(
                    "Failed to serialize contested resource value: {}",
                    e
                ))
            })?;
            array.push(&js_value);
        }

        Ok(array)
    }

    // Proof info versions for voting queries
    #[wasm_bindgen(js_name = "getContestedResourcesWithProofInfo")]
    pub async fn get_contested_resources_with_proof_info(
        &self,
        query: VotePollsByDocumentTypeQueryWasm,
    ) -> Result<JsValue, WasmSdkError> {
        let (contested_resources, metadata, proof) =
            ContestedResource::fetch_many_with_metadata_and_proof(
                self.as_ref(),
                query.into_inner(),
                None,
            )
            .await?;

        let resources: Vec<Value> = contested_resources
            .0
            .into_iter()
            .map(|resource| resource.0)
            .collect();

        let data = serde_wasm_bindgen::to_value(&resources).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize contested resources: {}", e))
        })?;

        let response = ProofMetadataResponseWasm::from_parts(data, metadata.into(), proof.into());

        Ok(JsValue::from(response))
    }
}
