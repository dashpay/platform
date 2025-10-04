use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "BatchType")]
pub enum BatchTypeWASM {
    Create,
    Replace,
    Delete,
    Transfer,
    Purchase,
    UpdatePrice,
    IgnoreWhileBumpingRevision,
}

impl TryFrom<JsValue> for BatchTypeWASM {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "create" => Ok(BatchTypeWASM::Create),
                    "replace" => Ok(BatchTypeWASM::Replace),
                    "delete" => Ok(BatchTypeWASM::Delete),
                    "transfer" => Ok(BatchTypeWASM::Transfer),
                    "purchase" => Ok(BatchTypeWASM::Purchase),
                    "updateprice" => Ok(BatchTypeWASM::UpdatePrice),
                    "ignorewhilebumpingrevision" => Ok(BatchTypeWASM::IgnoreWhileBumpingRevision),
                    _ => Err(JsValue::from(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(BatchTypeWASM::Create),
                    1 => Ok(BatchTypeWASM::Replace),
                    2 => Ok(BatchTypeWASM::Delete),
                    3 => Ok(BatchTypeWASM::Transfer),
                    4 => Ok(BatchTypeWASM::Purchase),
                    5 => Ok(BatchTypeWASM::UpdatePrice),
                    6 => Ok(BatchTypeWASM::IgnoreWhileBumpingRevision),
                    _ => Err(JsValue::from(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<BatchTypeWASM> for String {
    fn from(value: BatchTypeWASM) -> Self {
        match value {
            BatchTypeWASM::Create => String::from("create"),
            BatchTypeWASM::Replace => String::from("replace"),
            BatchTypeWASM::Delete => String::from("delete"),
            BatchTypeWASM::Transfer => String::from("transfer"),
            BatchTypeWASM::Purchase => String::from("purchase"),
            BatchTypeWASM::UpdatePrice => String::from("updatePrice"),
            BatchTypeWASM::IgnoreWhileBumpingRevision => String::from("ignoreWhileBumpingRevision"),
        }
    }
}

impl From<DocumentTransitionActionType> for BatchTypeWASM {
    fn from(action_type: DocumentTransitionActionType) -> Self {
        match action_type {
            DocumentTransitionActionType::Create => BatchTypeWASM::Create,
            DocumentTransitionActionType::Replace => BatchTypeWASM::Replace,
            DocumentTransitionActionType::Delete => BatchTypeWASM::Delete,
            DocumentTransitionActionType::Transfer => BatchTypeWASM::Transfer,
            DocumentTransitionActionType::Purchase => BatchTypeWASM::Purchase,
            DocumentTransitionActionType::UpdatePrice => BatchTypeWASM::UpdatePrice,
            DocumentTransitionActionType::IgnoreWhileBumpingRevision => {
                BatchTypeWASM::IgnoreWhileBumpingRevision
            }
        }
    }
}
