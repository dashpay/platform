use crate::error::WasmDppError;
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
    type Error = WasmDppError;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if let Some(enum_val) = value.as_string() {
            return match enum_val.to_lowercase().as_str() {
                "documentowner" => Ok(GasFeesPaidByWasm::DocumentOwner),
                "contractowner" => Ok(GasFeesPaidByWasm::ContractOwner),
                "prefercontractowner" => Ok(GasFeesPaidByWasm::PreferContractOwner),
                _ => Err(WasmDppError::invalid_argument(format!(
                    "unknown batch type value: {}",
                    enum_val
                ))),
            };
        }

        if let Some(enum_val) = value.as_f64() {
            return match enum_val as u8 {
                0 => Ok(GasFeesPaidByWasm::DocumentOwner),
                1 => Ok(GasFeesPaidByWasm::ContractOwner),
                2 => Ok(GasFeesPaidByWasm::PreferContractOwner),
                _ => Err(WasmDppError::invalid_argument(format!(
                    "unknown batch type value: {}",
                    enum_val
                ))),
            };
        }

        Err(WasmDppError::invalid_argument(
            "cannot read value from gas fees enum",
        ))
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
