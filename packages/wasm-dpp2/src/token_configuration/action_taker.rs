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
    pub fn new(value: &JsValue) -> Result<ActionTakerWasm, JsValue> {
        let identifier = IdentifierWasm::try_from(value);

        if identifier.is_err() {
            let set_of_identifiers: Vec<Identifier> = Array::from(value)
                .to_vec()
                .iter()
                .map(|js_value: &JsValue| {
                    Identifier::from(IdentifierWasm::try_from(js_value).expect("err"))
                })
                .collect();

            Ok(ActionTakerWasm(ActionTaker::SpecifiedIdentities(
                BTreeSet::from_iter(set_of_identifiers),
            )))
        } else {
            Ok(ActionTakerWasm(ActionTaker::SingleIdentity(
                identifier?.into(),
            )))
        }
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
                let identifiers: Vec<IdentifierWasm> = value
                    .iter()
                    .map(|identifier: &Identifier| IdentifierWasm::from(identifier.clone()))
                    .collect();

                JsValue::from(identifiers)
            }
        }
    }

    #[wasm_bindgen(setter = "value")]
    pub fn set_value(&mut self, value: &JsValue) -> Result<(), JsValue> {
        self.0 = Self::new(value)?.0;

        Ok(())
    }
}
