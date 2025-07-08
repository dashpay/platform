use dpp::version::ProtocolVersion;
use std::ops::RangeInclusive;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = StateTransitionIsNotActiveError)]
pub struct StateTransitionIsNotActiveErrorWasm {
    state_transition_type: String,
    active_version_range_start: ProtocolVersion,
    active_version_range_end: ProtocolVersion,
    current_protocol_version: ProtocolVersion,
}

impl StateTransitionIsNotActiveErrorWasm {
    pub fn new(
        state_transition_type: String,
        active_version_range: RangeInclusive<ProtocolVersion>,
        current_protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            state_transition_type,
            active_version_range_start: *active_version_range.start(),
            active_version_range_end: *active_version_range.end(),
            current_protocol_version,
        }
    }
}

#[wasm_bindgen(js_class = StateTransitionIsNotActiveError)]
impl StateTransitionIsNotActiveErrorWasm {
    #[wasm_bindgen(js_name = getStateTransitionType)]
    pub fn get_state_transition_type(&self) -> String {
        self.state_transition_type.clone()
    }

    #[wasm_bindgen(js_name = getActiveVersionRangeStart)]
    pub fn get_active_version_range_start(&self) -> u32 {
        self.active_version_range_start
    }

    #[wasm_bindgen(js_name = getActiveVersionRangeEnd)]
    pub fn get_active_version_range_end(&self) -> u32 {
        self.active_version_range_end
    }

    #[wasm_bindgen(js_name = getCurrentProtocolVersion)]
    pub fn get_current_protocol_version(&self) -> u32 {
        self.current_protocol_version
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        let object = js_sys::Object::new();
        js_sys::Reflect::set(
            &object,
            &"stateTransitionType".into(),
            &self.state_transition_type.clone().into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &object,
            &"activeVersionRangeStart".into(),
            &JsValue::from(self.active_version_range_start),
        )
        .unwrap();
        js_sys::Reflect::set(
            &object,
            &"activeVersionRangeEnd".into(),
            &JsValue::from(self.active_version_range_end),
        )
        .unwrap();
        js_sys::Reflect::set(
            &object,
            &"currentProtocolVersion".into(),
            &JsValue::from(self.current_protocol_version),
        )
        .unwrap();
        object.into()
    }
}
