use crate::error::WasmDppError;
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "BatchType")]
pub enum BatchTypeWasm {
    Create,
    Replace,
    Delete,
    Transfer,
    Purchase,
    UpdatePrice,
    IgnoreWhileBumpingRevision,
}

impl TryFrom<JsValue> for BatchTypeWasm {
    type Error = WasmDppError;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "create" => Ok(BatchTypeWasm::Create),
                    "replace" => Ok(BatchTypeWasm::Replace),
                    "delete" => Ok(BatchTypeWasm::Delete),
                    "transfer" => Ok(BatchTypeWasm::Transfer),
                    "purchase" => Ok(BatchTypeWasm::Purchase),
                    "updateprice" => Ok(BatchTypeWasm::UpdatePrice),
                    "ignorewhilebumpingrevision" => Ok(BatchTypeWasm::IgnoreWhileBumpingRevision),
                    _ => Err(WasmDppError::invalid_argument(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(WasmDppError::invalid_argument(
                    "cannot read value from enum",
                )),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(BatchTypeWasm::Create),
                    1 => Ok(BatchTypeWasm::Replace),
                    2 => Ok(BatchTypeWasm::Delete),
                    3 => Ok(BatchTypeWasm::Transfer),
                    4 => Ok(BatchTypeWasm::Purchase),
                    5 => Ok(BatchTypeWasm::UpdatePrice),
                    6 => Ok(BatchTypeWasm::IgnoreWhileBumpingRevision),
                    _ => Err(WasmDppError::invalid_argument(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<BatchTypeWasm> for String {
    fn from(value: BatchTypeWasm) -> Self {
        match value {
            BatchTypeWasm::Create => String::from("create"),
            BatchTypeWasm::Replace => String::from("replace"),
            BatchTypeWasm::Delete => String::from("delete"),
            BatchTypeWasm::Transfer => String::from("transfer"),
            BatchTypeWasm::Purchase => String::from("purchase"),
            BatchTypeWasm::UpdatePrice => String::from("updatePrice"),
            BatchTypeWasm::IgnoreWhileBumpingRevision => String::from("ignoreWhileBumpingRevision"),
        }
    }
}

impl From<DocumentTransitionActionType> for BatchTypeWasm {
    fn from(action_type: DocumentTransitionActionType) -> Self {
        match action_type {
            DocumentTransitionActionType::Create => BatchTypeWasm::Create,
            DocumentTransitionActionType::Replace => BatchTypeWasm::Replace,
            DocumentTransitionActionType::Delete => BatchTypeWasm::Delete,
            DocumentTransitionActionType::Transfer => BatchTypeWasm::Transfer,
            DocumentTransitionActionType::Purchase => BatchTypeWasm::Purchase,
            DocumentTransitionActionType::UpdatePrice => BatchTypeWasm::UpdatePrice,
            DocumentTransitionActionType::IgnoreWhileBumpingRevision => {
                BatchTypeWasm::IgnoreWhileBumpingRevision
            }
        }
    }
}
