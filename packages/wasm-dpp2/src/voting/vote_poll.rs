use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::ToSerdeJSONExt;
use crate::{impl_try_from_options, impl_wasm_conversions, impl_wasm_type_info};
use dpp::bincode;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use js_sys::{Array, Object, Reflect};
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollOptions {
    document_type_name: String,
    index_name: String,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface VotePollOptions {
    contractId: Identifier | Uint8Array | string;
    documentTypeName: string;
    indexName: string;
    indexValues: any[];
}
"#;

#[derive(Clone)]
#[wasm_bindgen(js_name = VotePoll)]
pub struct VotePollWasm(VotePoll);

impl From<VotePoll> for VotePollWasm {
    fn from(poll: VotePoll) -> Self {
        VotePollWasm(poll)
    }
}

impl From<VotePollWasm> for VotePoll {
    fn from(poll: VotePollWasm) -> Self {
        poll.0
    }
}

#[wasm_bindgen(js_class = VotePoll)]
impl VotePollWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(unchecked_param_type = "VotePollOptions")] options: JsValue,
    ) -> WasmDppResult<VotePollWasm> {
        let object = Object::from(options.clone());

        // Extract contractId (required)
        let js_contract_id = Reflect::get(&object, &JsValue::from_str("contractId"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing contractId: {:?}", e)))?;
        let contract_id = IdentifierWasm::try_from(&js_contract_id)?.into();

        // Extract indexValues (required)
        let js_index_values = Reflect::get(&object, &JsValue::from_str("indexValues"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing indexValues: {:?}", e)))?;
        let index_values_value = js_index_values.with_serde_to_platform_value()?;
        let index_values = index_values_value
            .into_array()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        // Extract simple fields via serde
        let opts: VotePollOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(VotePollWasm(VotePoll::ContestedDocumentResourceVotePoll(
            ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name: opts.document_type_name,
                index_name: opts.index_name,
                index_values,
            },
        )))
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.0.to_string()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn contract_id(&self) -> IdentifierWasm {
        match &self.0 {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => {
                IdentifierWasm::from(poll.contract_id)
            }
        }
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> String {
        match &self.0 {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => poll.document_type_name.clone(),
        }
    }

    #[wasm_bindgen(getter = "indexName")]
    pub fn index_name(&self) -> String {
        match &self.0 {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => poll.index_name.clone(),
        }
    }

    #[wasm_bindgen(getter = "indexValues")]
    pub fn index_values(&self) -> WasmDppResult<Array> {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        match &self.0 {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => {
                let encoded: WasmDppResult<Vec<Vec<u8>>> = poll
                    .index_values
                    .iter()
                    .map(|value| {
                        bincode::encode_to_vec(value, config)
                            .map_err(|err| WasmDppError::serialization(err.to_string()))
                    })
                    .collect();

                let js_array = Array::new();

                for bytes in encoded? {
                    js_array.push(&JsValue::from(bytes));
                }

                Ok(js_array)
            }
        }
    }

    #[wasm_bindgen(setter = "contractId")]
    pub fn set_contract_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        let contract_id = IdentifierWasm::try_from(js_contract_id)?.into();

        self.0 = match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(mut poll) => {
                poll.contract_id = contract_id;

                VotePoll::ContestedDocumentResourceVotePoll(poll)
            }
        };

        Ok(())
    }

    #[wasm_bindgen(setter = "documentTypeName")]
    pub fn set_document_type_name(&mut self, document_type_name: String) {
        self.0 = match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(mut poll) => {
                poll.document_type_name = document_type_name;

                VotePoll::ContestedDocumentResourceVotePoll(poll)
            }
        }
    }

    #[wasm_bindgen(setter = "indexName")]
    pub fn set_index_name(&mut self, index_name: String) {
        self.0 = match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(mut poll) => {
                poll.index_name = index_name;

                VotePoll::ContestedDocumentResourceVotePoll(poll)
            }
        };
    }

    #[wasm_bindgen(setter = "indexValues")]
    pub fn set_index_values(&mut self, js_index_values: JsValue) -> WasmDppResult<()> {
        let index_values = js_index_values
            .with_serde_to_platform_value()?
            .into_array()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        self.0 = match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(mut poll) => {
                poll.index_values = index_values;

                VotePoll::ContestedDocumentResourceVotePoll(poll)
            }
        };

        Ok(())
    }
}

impl_try_from_options!(VotePollWasm, "VotePoll");
impl_wasm_conversions!(VotePollWasm, VotePoll);
impl_wasm_type_info!(VotePollWasm, VotePoll);
