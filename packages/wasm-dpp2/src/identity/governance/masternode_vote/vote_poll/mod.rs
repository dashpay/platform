use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::utils::ToSerdeJSONExt;
use dpp::bincode;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use js_sys::Array;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "VotePoll".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "VotePoll".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_contract_id: &JsValue,
        document_type_name: String,
        index_name: String,
        js_index_values: JsValue,
    ) -> WasmDppResult<VotePollWasm> {
        let contract_id = IdentifierWasm::try_from(js_contract_id)?;

        let index_values_value = js_index_values.with_serde_to_platform_value()?;
        let index_values = index_values_value
            .into_array()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(VotePollWasm(VotePoll::ContestedDocumentResourceVotePoll(
            ContestedDocumentResourceVotePoll {
                contract_id: contract_id.into(),
                document_type_name,
                index_name,
                index_values,
            },
        )))
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn contract_id(&self) -> IdentifierWasm {
        match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => poll.contract_id.into(),
        }
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> String {
        match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => poll.document_type_name.into(),
        }
    }

    #[wasm_bindgen(getter = "indexName")]
    pub fn index_name(&self) -> String {
        match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => poll.index_name.into(),
        }
    }

    #[wasm_bindgen(getter = "indexValues")]
    pub fn index_values(&self) -> WasmDppResult<Array> {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        match self.0.clone() {
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
    pub fn set_contract_id(&mut self, js_contract_id: &JsValue) -> WasmDppResult<()> {
        let contract_id = IdentifierWasm::try_from(js_contract_id)?;

        self.0 = match self.0.clone() {
            VotePoll::ContestedDocumentResourceVotePoll(mut poll) => {
                poll.contract_id = contract_id.into();

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
