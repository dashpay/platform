use crate::error::WasmDppError;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "VoteStateResultType")]
#[allow(non_camel_case_types)]
#[derive(Default, Clone)]
pub enum VoteStateResultTypeWasm {
    #[default]
    Documents = 0,
    VoteTally = 1,
    DocumentsAndVoteTally = 2,
}

impl TryFrom<JsValue> for VoteStateResultTypeWasm {
    type Error = WasmDppError;

    fn try_from(value: JsValue) -> Result<VoteStateResultTypeWasm, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "documents" => Ok(VoteStateResultTypeWasm::Documents),
                    "votetally" => Ok(VoteStateResultTypeWasm::VoteTally),
                    "documentsandvotetally" => Ok(VoteStateResultTypeWasm::DocumentsAndVoteTally),
                    _ => Err(WasmDppError::invalid_argument("unknown result type")),
                },
            },
            false => match value.as_f64() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(VoteStateResultTypeWasm::Documents),
                    1 => Ok(VoteStateResultTypeWasm::VoteTally),
                    2 => Ok(VoteStateResultTypeWasm::DocumentsAndVoteTally),
                    _ => Err(WasmDppError::invalid_argument("unknown action type")),
                },
            },
        }
    }
}

impl From<VoteStateResultTypeWasm> for String {
    fn from(result_type: VoteStateResultTypeWasm) -> Self {
        match result_type {
            VoteStateResultTypeWasm::Documents => String::from("Documents"),
            VoteStateResultTypeWasm::VoteTally => String::from("VoteTally"),
            VoteStateResultTypeWasm::DocumentsAndVoteTally => String::from("DocumentsAndVoteTally"),
        }
    }
}
