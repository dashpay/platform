use crate::error::{WasmDppError, WasmDppResult};
use crate::group::token_event::TokenEventWasm;
use dpp::group::action_event::GroupActionEvent;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "GroupActionEvent")]
pub struct GroupActionEventWasm(GroupActionEvent);

impl From<GroupActionEvent> for GroupActionEventWasm {
    fn from(event: GroupActionEvent) -> Self {
        GroupActionEventWasm(event)
    }
}

impl From<GroupActionEventWasm> for GroupActionEvent {
    fn from(event: GroupActionEventWasm) -> Self {
        event.0
    }
}

#[wasm_bindgen(js_class = GroupActionEvent)]
impl GroupActionEventWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "GroupActionEvent".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name(&self) -> String {
        "GroupActionEvent".to_string()
    }

    #[wasm_bindgen(getter = "variant")]
    pub fn variant(&self) -> String {
        match &self.0 {
            GroupActionEvent::TokenEvent(_) => "TokenEvent".to_string(),
        }
    }

    #[wasm_bindgen(js_name = "tokenEvent")]
    pub fn token_event(&self) -> TokenEventWasm {
        match &self.0 {
            GroupActionEvent::TokenEvent(event) => TokenEventWasm::from(event.clone()),
        }
    }

    #[wasm_bindgen(js_name = "eventName")]
    pub fn event_name(&self) -> String {
        self.0.event_name()
    }

    #[wasm_bindgen(js_name = "publicNote")]
    pub fn public_note(&self) -> Option<String> {
        self.0.public_note().map(|note| note.to_string())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"variant".into(), &self.variant().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(&obj, &"eventName".into(), &self.event_name().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        match self.public_note() {
            Some(note) => {
                Reflect::set(&obj, &"publicNote".into(), &note.into())
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
            }
            None => {
                Reflect::set(&obj, &"publicNote".into(), &JsValue::NULL)
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
            }
        }
        match &self.0 {
            GroupActionEvent::TokenEvent(event) => {
                let token_event_wasm = TokenEventWasm::from(event.clone());
                Reflect::set(&obj, &"tokenEvent".into(), &token_event_wasm.to_json()?)
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
            }
        }
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"variant".into(), &self.variant().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        Reflect::set(&obj, &"eventName".into(), &self.event_name().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        match self.public_note() {
            Some(note) => {
                Reflect::set(&obj, &"publicNote".into(), &note.into())
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
            }
            None => {
                Reflect::set(&obj, &"publicNote".into(), &JsValue::NULL)
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
            }
        }
        match &self.0 {
            GroupActionEvent::TokenEvent(event) => {
                let token_event_wasm = TokenEventWasm::from(event.clone());
                Reflect::set(&obj, &"tokenEvent".into(), &token_event_wasm.to_object()?)
                    .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
            }
        }
        Ok(obj.into())
    }
}
