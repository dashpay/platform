use crate::buffer::Buffer;
use dpp::identifier::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityPublicKeyDisabledAtWindowViolationError)]
pub struct IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    disabled_at: u64,
    time_window_start: u64,
    time_window_end: u64,
    code: u32,
}

#[wasm_bindgen(js_class=IdentityPublicKeyDisabledAtWindowViolationError)]
impl IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    #[wasm_bindgen(js_name=getDisabledAt)]
    pub fn disabled_at(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        let timestamp = JsValue::from(self.disabled_at as u32);
        js_sys::Date::new(&timestamp)
    }

    #[wasm_bindgen(js_name=getTimeWindowStart)]
    pub fn time_window_start(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        let timestamp = JsValue::from(self.time_window_start as u32);
        js_sys::Date::new(&timestamp)
    }

    #[wasm_bindgen(js_name=getTimeWindowEnd)]
    pub fn time_window_end(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        let timestamp = JsValue::from(self.time_window_end as u32);
        js_sys::Date::new(&timestamp)
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    pub fn new(
        disabled_at: u64,
        time_window_start: u64,
        time_window_end: u64,
        code: u32
    ) -> Self {
        Self {
            disabled_at,
            time_window_start,
            time_window_end,
            code,
        }
    }
}
