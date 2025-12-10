use crate::group::token_event::TokenEventWasm;
use dpp::group::action_event::GroupActionEvent;
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
}
