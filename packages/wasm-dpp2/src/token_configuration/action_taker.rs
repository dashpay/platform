use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::group::action_taker::ActionTaker;
use dpp::prelude::Identifier;
use js_sys::Array;
use std::collections::BTreeSet;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "ActionTaker")]
pub struct ActionTakerWasm(ActionTaker);

impl From<ActionTaker> for ActionTakerWasm {
    fn from(action_taker: ActionTaker) -> Self {
        ActionTakerWasm(action_taker)
    }
}

impl From<ActionTakerWasm> for ActionTaker {
    fn from(action_taker: ActionTakerWasm) -> Self {
        action_taker.0
    }
}

#[wasm_bindgen(js_class = ActionTaker)]
impl ActionTakerWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ActionTaker".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ActionTaker".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(value: &JsValue) -> WasmDppResult<ActionTakerWasm> {
        if let Ok(identifier) = IdentifierWasm::try_from(value.clone()) {
            return Ok(ActionTakerWasm(ActionTaker::SingleIdentity(
                identifier.into(),
            )));
        }

        if !value.is_object() && !value.is_array() {
            return Err(WasmDppError::invalid_argument(
                "ActionTaker value must be an Identifier or array of Identifiers",
            ));
        }

        let array = Array::from(value);
        let mut identifiers = BTreeSet::new();

        for js_value in array.to_vec() {
            let identifier = IdentifierWasm::try_from(js_value)?;
            identifiers.insert(Identifier::from(identifier));
        }

        if identifiers.is_empty() {
            return Err(WasmDppError::invalid_argument(
                "ActionTaker array must contain at least one identifier",
            ));
        }

        Ok(ActionTakerWasm(ActionTaker::SpecifiedIdentities(
            identifiers,
        )))
    }

    #[wasm_bindgen(js_name = "getType")]
    pub fn get_type(&self) -> String {
        match &self.0 {
            ActionTaker::SpecifiedIdentities(_) => "SpecifiedIdentities".to_string(),
            ActionTaker::SingleIdentity(_) => "SingleIdentity".to_string(),
        }
    }

    #[wasm_bindgen(getter = "value")]
    pub fn get_value(&self) -> JsValue {
        match &self.0 {
            ActionTaker::SingleIdentity(value) => {
                JsValue::from(IdentifierWasm::from(value.clone()))
            }
            ActionTaker::SpecifiedIdentities(value) => {
                let array = Array::new();
                for identifier in value.iter() {
                    array.push(&IdentifierWasm::from(identifier.clone()).into());
                }
                array.into()
            }
        }
    }

    #[wasm_bindgen(setter = "value")]
    pub fn set_value(&mut self, value: &JsValue) -> WasmDppResult<()> {
        self.0 = Self::new(value)?.0;

        Ok(())
    }
}
