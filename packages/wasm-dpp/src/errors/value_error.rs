use dpp::platform_value::Error as PlatformValueError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=PlatformValueError, inspectable)]
#[derive(Debug)]
pub struct PlatformValueErrorWasm {
    message: String,
}

impl From<PlatformValueError> for PlatformValueErrorWasm {
    fn from(e: PlatformValueError) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl From<&PlatformValueError> for PlatformValueErrorWasm {
    fn from(e: &PlatformValueError) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl PlatformValueErrorWasm {
    pub fn new(e: PlatformValueError) -> Self {
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
}
