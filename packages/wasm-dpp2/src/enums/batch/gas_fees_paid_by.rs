use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Default)]
#[wasm_bindgen(js_name = "GasFeesPaidBy")]
pub enum GasFeesPaidByWASM {
    #[default]
    DocumentOwner = 0,
    ContractOwner = 1,
    PreferContractOwner = 2,
}

impl TryFrom<JsValue> for GasFeesPaidByWASM {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_string() {
            true => match value.as_string() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val.to_lowercase().as_str() {
                    "documentowner" => Ok(GasFeesPaidByWASM::DocumentOwner),
                    "contractowner" => Ok(GasFeesPaidByWASM::ContractOwner),
                    "prefercontractowner" => Ok(GasFeesPaidByWASM::PreferContractOwner),
                    _ => Err(JsValue::from(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
            false => match value.as_f64() {
                None => Err(JsValue::from("cannot read value from enum")),
                Some(enum_val) => match enum_val as u8 {
                    0 => Ok(GasFeesPaidByWASM::DocumentOwner),
                    1 => Ok(GasFeesPaidByWASM::ContractOwner),
                    2 => Ok(GasFeesPaidByWASM::PreferContractOwner),
                    _ => Err(JsValue::from(format!(
                        "unknown batch type value: {}",
                        enum_val
                    ))),
                },
            },
        }
    }
}

impl From<GasFeesPaidByWASM> for String {
    fn from(value: GasFeesPaidByWASM) -> Self {
        match value {
            GasFeesPaidByWASM::DocumentOwner => String::from("DocumentOwner"),
            GasFeesPaidByWASM::ContractOwner => String::from("ContractOwner"),
            GasFeesPaidByWASM::PreferContractOwner => String::from("PreferContractOwner"),
        }
    }
}

impl From<GasFeesPaidBy> for GasFeesPaidByWASM {
    fn from(value: GasFeesPaidBy) -> Self {
        match value {
            GasFeesPaidBy::DocumentOwner => GasFeesPaidByWASM::DocumentOwner,
            GasFeesPaidBy::ContractOwner => GasFeesPaidByWASM::ContractOwner,
            GasFeesPaidBy::PreferContractOwner => GasFeesPaidByWASM::PreferContractOwner,
        }
    }
}

impl From<GasFeesPaidByWASM> for GasFeesPaidBy {
    fn from(value: GasFeesPaidByWASM) -> Self {
        match value {
            GasFeesPaidByWASM::DocumentOwner => GasFeesPaidBy::DocumentOwner,
            GasFeesPaidByWASM::ContractOwner => GasFeesPaidBy::ContractOwner,
            GasFeesPaidByWASM::PreferContractOwner => GasFeesPaidBy::PreferContractOwner,
        }
    }
}
