use dpp::consensus::basic::value_error::ValueError;
use wasm_bindgen::prelude::*;

// TODO Rename to ProtocolValueError

#[wasm_bindgen(js_name=PlatformValueError, inspectable)]
#[derive(Debug)]
pub struct PlatformValueErrorWasm {
    message: String,
}

impl From<ValueError> for PlatformValueErrorWasm {
    fn from(e: ValueError) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl From<&ValueError> for PlatformValueErrorWasm {
    fn from(e: &ValueError) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl PlatformValueErrorWasm {
    pub fn new(e: ValueError) -> Self {
        PlatformValueErrorWasm {
            message: e.to_string(),
        }
    }
}

#[wasm_bindgen(js_class=PlatformValueError)]
impl PlatformValueErrorWasm {
    #[wasm_bindgen(js_name=getMessage)]
    pub fn get_message(&self) -> String {
        self.message.clone()
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string(&self) -> String {
        format!("PlatformValueError: {}", self.message)
    }
}
