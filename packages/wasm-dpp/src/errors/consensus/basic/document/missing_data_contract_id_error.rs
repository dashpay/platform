use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingDataContractIdError)]
pub struct MissingDataContractIdErrorWasm {
    code: u32,
}

impl MissingDataContractIdErrorWasm {
    pub fn new(code: u32) -> Self {
        MissingDataContractIdErrorWasm { code }
    }
}

#[wasm_bindgen(js_class=MissingDataContractIdError)]
impl MissingDataContractIdErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
