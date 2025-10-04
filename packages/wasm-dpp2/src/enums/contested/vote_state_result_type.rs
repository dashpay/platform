use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "VoteStateResultTypeWASM")]
#[allow(non_camel_case_types)]
#[derive(Default, Clone)]
pub enum VoteStateResultTypeWASM {
    #[default]
    Documents = 0,
    VoteTally = 1,
    DocumentsAndVoteTally = 2,
}

impl TryFrom<JsValue> for VoteStateResultTypeWASM {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<VoteStateResultTypeWASM, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "documents" => Ok(VoteStateResultTypeWASM::Documents),
                    "votetally" => Ok(VoteStateResultTypeWASM::VoteTally),
                    "documentsandvotetally" => Ok(VoteStateResultTypeWASM::DocumentsAndVoteTally),
                    _ => Err(JsValue::from("unknown result type")),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(VoteStateResultTypeWASM::Documents),
                    1 => Ok(VoteStateResultTypeWASM::VoteTally),
                    2 => Ok(VoteStateResultTypeWASM::DocumentsAndVoteTally),
                    _ => Err(JsValue::from("unknown action type")),
                },
            },
        }
    }
}

impl From<VoteStateResultTypeWASM> for String {
    fn from(result_type: VoteStateResultTypeWASM) -> Self {
        match result_type {
            VoteStateResultTypeWASM::Documents => String::from("Documents"),
            VoteStateResultTypeWASM::VoteTally => String::from("VoteTally"),
            VoteStateResultTypeWASM::DocumentsAndVoteTally => String::from("DocumentsAndVoteTally"),
        }
    }
}
