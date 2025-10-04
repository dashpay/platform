use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Default)]
#[wasm_bindgen(js_name = "GasFeesPaidBy")]
pub enum GasFeesPaidByWasm {
    #[default]
    DocumentOwner = 0,
    ContractOwner = 1,
    PreferContractOwner = 2,
}

impl TryFrom<JsValue> for GasFeesPaidByWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "documentowner" => Ok(GasFeesPaidByWasm::DocumentOwner),
                    "contractowner" => Ok(GasFeesPaidByWasm::ContractOwner),
                    "prefercontractowner" => Ok(GasFeesPaidByWasm::PreferContractOwner),
                    _ => Err(JsValue::from(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(GasFeesPaidByWasm::DocumentOwner),
                    1 => Ok(GasFeesPaidByWasm::ContractOwner),
                    2 => Ok(GasFeesPaidByWasm::PreferContractOwner),
                    _ => Err(JsValue::from(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<GasFeesPaidByWasm> for String {
    fn from(value: GasFeesPaidByWasm) -> Self {
        match value {
            GasFeesPaidByWasm::DocumentOwner => String::from("DocumentOwner"),
            GasFeesPaidByWasm::ContractOwner => String::from("ContractOwner"),
            GasFeesPaidByWasm::PreferContractOwner => String::from("PreferContractOwner"),
        }
    }
}

impl From<GasFeesPaidBy> for GasFeesPaidByWasm {
    fn from(value: GasFeesPaidBy) -> Self {
        match value {
            GasFeesPaidBy::DocumentOwner => GasFeesPaidByWasm::DocumentOwner,
            GasFeesPaidBy::ContractOwner => GasFeesPaidByWasm::ContractOwner,
            GasFeesPaidBy::PreferContractOwner => GasFeesPaidByWasm::PreferContractOwner,
        }
    }
}

impl From<GasFeesPaidByWasm> for GasFeesPaidBy {
    fn from(value: GasFeesPaidByWasm) -> Self {
        match value {
            GasFeesPaidByWasm::DocumentOwner => GasFeesPaidBy::DocumentOwner,
            GasFeesPaidByWasm::ContractOwner => GasFeesPaidBy::ContractOwner,
            GasFeesPaidByWasm::PreferContractOwner => GasFeesPaidBy::PreferContractOwner,
        }
    }
}
