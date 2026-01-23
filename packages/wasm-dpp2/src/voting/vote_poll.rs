use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::utils::{ToSerdeJSONExt, try_from_options, try_from_options_with};
use crate::{impl_try_from_js_value, impl_wasm_conversions, impl_wasm_type_info};
use dpp::bincode;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use js_sys::{Array, Object};
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
    contractId: IdentifierLike;
    documentTypeName: string;
    indexName: string;
    indexValues: any[];
}

/**
 * VotePoll serialized as a plain object.
 */
export interface VotePollObject {
    contractId: Uint8Array;
    documentTypeName: string;
    indexName: string;
    indexValues: any[];
}

/**
 * VotePoll serialized as JSON.
 */
export interface VotePollJSON {
    contractId: string;
    documentTypeName: string;
    indexName: string;
    indexValues: any[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "VotePollOptions")]
    pub type VotePollOptionsJs;

    #[wasm_bindgen(typescript_type = "VotePollObject")]
    pub type VotePollObjectJs;

    #[wasm_bindgen(typescript_type = "VotePollJSON")]
    pub type VotePollJSONJs;
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "VotePoll")]
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
    pub fn constructor(options: VotePollOptionsJs) -> WasmDppResult<VotePollWasm> {
        let options: JsValue = options.into();
        let object = Object::from(options.clone());

        // Extract contractId (required)
        let contract_id: IdentifierWasm = try_from_options(&object, "contractId")?;

        // Extract indexValues (required)
        let index_values = try_from_options_with(&object, "indexValues", |v| {
            let index_values_value = v.with_serde_to_platform_value()?;
            index_values_value
                .into_array()
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
        })?;

        // Extract simple fields via serde
        let opts: VotePollOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(VotePollWasm(VotePoll::ContestedDocumentResourceVotePoll(
            ContestedDocumentResourceVotePoll {
                contract_id: contract_id.into(),
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
    pub fn set_contract_id(&mut self, contract_id: IdentifierLikeJs) -> WasmDppResult<()> {
        let contract_id = contract_id.try_into()?;

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
    pub fn set_index_values(&mut self, index_values: js_sys::Array) -> WasmDppResult<()> {
        let js_value: JsValue = index_values.into();
        let values = js_value
            .with_serde_to_platform_value()?
            .into_array()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        self.0 = match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(mut poll) => {
                poll.index_values = values;

                VotePoll::ContestedDocumentResourceVotePoll(poll)
            }
        };

        Ok(())
    }
}

impl_try_from_js_value!(VotePollWasm, "VotePoll");
impl_wasm_conversions!(VotePollWasm, VotePoll, VotePollObjectJs, VotePollJSONJs);
impl_wasm_type_info!(VotePollWasm, VotePoll);
