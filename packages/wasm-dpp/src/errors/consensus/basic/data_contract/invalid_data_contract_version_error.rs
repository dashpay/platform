use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDataContractVersionError)]
pub struct InvalidDataContractVersionErrorWasm {
    expected_version: u32,
    version: u32,
    code: u32,
}

impl InvalidDataContractVersionErrorWasm {
    pub fn new(expected_version: u32, version: u32, code: u32) -> Self {
        InvalidDataContractVersionErrorWasm {
            expected_version,
            version,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidDataContractVersionError)]
impl InvalidDataContractVersionErrorWasm {
    #[wasm_bindgen(js_name=getExpectedVersion)]
    pub fn get_expected_version(&self) -> u32 {
        self.expected_version
    }

    #[wasm_bindgen(js_name=getVersion)]
    pub fn get_version(&self) -> u32 {
        self.version
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
