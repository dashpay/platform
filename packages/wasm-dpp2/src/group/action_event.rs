use crate::group::token_event::TokenEventWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use dpp::group::action_event::GroupActionEvent;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * GroupActionEvent serialized as a plain object.
 */
export interface GroupActionEventObject {
    variant: GroupActionEventVariant;
    tokenEvent?: TokenEventObject;
}

/**
 * GroupActionEvent serialized as JSON.
 */
export interface GroupActionEventJSON {
    variant: number;
    tokenEvent?: TokenEventJSON;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupActionEventObject")]
    pub type GroupActionEventObjectJs;

    #[wasm_bindgen(typescript_type = "GroupActionEventJSON")]
    pub type GroupActionEventJSONJs;
}

/// TypeScript enum for GroupActionEvent variants
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupActionEventVariant {
    TokenEvent = 0,
}

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
    #[wasm_bindgen(getter = "variant")]
    pub fn variant(&self) -> GroupActionEventVariant {
        match &self.0 {
            GroupActionEvent::TokenEvent(_) => GroupActionEventVariant::TokenEvent,
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

impl_wasm_conversions!(
    GroupActionEventWasm,
    GroupActionEvent,
    GroupActionEventObjectJs,
    GroupActionEventJSONJs
);
impl_wasm_type_info!(GroupActionEventWasm, GroupActionEvent);
