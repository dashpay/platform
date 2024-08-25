use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::document_timestamp_window_violation_error::DocumentTimestampWindowViolationError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentTimestampWindowViolationError)]
pub struct DocumentTimestampWindowViolationErrorWasm {
    inner: DocumentTimestampWindowViolationError,
}

impl From<&DocumentTimestampWindowViolationError> for DocumentTimestampWindowViolationErrorWasm {
    fn from(e: &DocumentTimestampWindowViolationError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DocumentTimestampWindowViolationError)]
impl DocumentTimestampWindowViolationErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getTimestampName)]
    pub fn timestamp_name(&self) -> String {
        self.inner.timestamp_name()
    }

    #[wasm_bindgen(js_name=getTimestamp)]
    pub fn timestamp(&self) -> js_sys::Date {
        let date = js_sys::Date::new_0();
        date.set_time(*self.inner.timestamp() as f64);
        date
    }

    #[wasm_bindgen(js_name=getTimeWindowStart)]
    pub fn time_window_start(&self) -> js_sys::Date {
        let date = js_sys::Date::new_0();
        date.set_time(*self.inner.time_window_start() as f64);
        date
    }

    #[wasm_bindgen(js_name=getTimeWindowEnd)]
    pub fn time_window_end(&self) -> js_sys::Date {
        let date = js_sys::Date::new_0();
        date.set_time(*self.inner.time_window_end() as f64);
        date
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
