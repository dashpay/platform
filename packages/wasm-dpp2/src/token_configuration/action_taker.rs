use crate::identifier::IdentifierWASM;
use dpp::group::action_taker::ActionTaker;
use dpp::prelude::Identifier;
use js_sys::Array;
use std::collections::BTreeSet;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "ActionTakerWASM")]
pub struct ActionTakerWASM(ActionTaker);

impl From<ActionTaker> for ActionTakerWASM {
    fn from(action_taker: ActionTaker) -> Self {
        ActionTakerWASM(action_taker)
    }
}

impl From<ActionTakerWASM> for ActionTaker {
    fn from(action_taker: ActionTakerWASM) -> Self {
        action_taker.0
    }
}

#[wasm_bindgen]
impl ActionTakerWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "ActionTakerWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "ActionTakerWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(value: &JsValue) -> Result<ActionTakerWASM, JsValue> {
        let identifier = IdentifierWASM::try_from(value);

        if identifier.is_err() {
            let set_of_identifiers: Vec<Identifier> = Array::from(value)
                .to_vec()
                .iter()
                .map(|js_value: &JsValue| {
                    Identifier::from(IdentifierWASM::try_from(js_value).expect("err"))
                })
                .collect();

            Ok(ActionTakerWASM(ActionTaker::SpecifiedIdentities(
                BTreeSet::from_iter(set_of_identifiers),
            )))
        } else {
            Ok(ActionTakerWASM(ActionTaker::SingleIdentity(
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
                JsValue::from(IdentifierWASM::from(value.clone()))
            }
            ActionTaker::SpecifiedIdentities(value) => {
                let identifiers: Vec<IdentifierWASM> = value
                    .iter()
                    .map(|identifier: &Identifier| IdentifierWASM::from(identifier.clone()))
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
