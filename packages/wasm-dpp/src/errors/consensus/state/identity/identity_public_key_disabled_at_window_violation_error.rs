use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::identity::identity_public_key_disabled_at_window_violation_error::IdentityPublicKeyDisabledAtWindowViolationError;
use dpp::consensus::ConsensusError;
use dpp::serialization::PlatformSerializable;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IdentityPublicKeyDisabledAtWindowViolationError)]
pub struct IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    inner: IdentityPublicKeyDisabledAtWindowViolationError,
}

impl From<&IdentityPublicKeyDisabledAtWindowViolationError>
    for IdentityPublicKeyDisabledAtWindowViolationErrorWasm
{
    fn from(e: &IdentityPublicKeyDisabledAtWindowViolationError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=IdentityPublicKeyDisabledAtWindowViolationError)]
impl IdentityPublicKeyDisabledAtWindowViolationErrorWasm {
    #[wasm_bindgen(js_name=getDisabledAt)]
    pub fn disabled_at(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        js_sys::Date::new(&JsValue::from_f64(self.inner.disabled_at() as f64))
    }

    #[wasm_bindgen(js_name=getTimeWindowStart)]
    pub fn time_window_start(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        js_sys::Date::new(&JsValue::from_f64(self.inner.time_window_start() as f64))
    }

    #[wasm_bindgen(js_name=getTimeWindowEnd)]
    pub fn time_window_end(&self) -> js_sys::Date {
        // TODO: Figure out how to match rust timestamps with JS timestamps
        js_sys::Date::new(&JsValue::from_f64(self.inner.time_window_end() as f64))
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
