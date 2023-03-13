use dpp::prelude::TimestampMillis;
use wasm_bindgen::prelude::*;

use crate::utils::timestamp_millis_to_js_date;

#[wasm_bindgen(js_name=IdentityPublicKeyDisabledAtWindowViolationError)]
pub struct IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    disabled_at: TimestampMillis,
    time_window_start: TimestampMillis,
    time_window_end: TimestampMillis,
    code: u32,
}

#[wasm_bindgen(js_class=IdentityPublicKeyDisabledAtWindowViolationError)]
impl IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    #[wasm_bindgen(js_name=getDisabledAt)]
    pub fn disabled_at(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        timestamp_millis_to_js_date(self.disabled_at)
    }

    #[wasm_bindgen(js_name=getTimeWindowStart)]
    pub fn time_window_start(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        timestamp_millis_to_js_date(self.time_window_start)
    }

    #[wasm_bindgen(js_name=getTimeWindowEnd)]
    pub fn time_window_end(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        timestamp_millis_to_js_date(self.time_window_end)
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    pub fn new(
        disabled_at: TimestampMillis,
        time_window_start: TimestampMillis,
        time_window_end: TimestampMillis,
        code: u32,
    ) -> Self {
        Self {
            disabled_at,
            time_window_start,
            time_window_end,
            code,
        }
    }
}
