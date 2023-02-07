use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MaxIdentityPublicKeyLimitReachedError)]
pub struct MaxIdentityPublicKeyLimitReachedErrorWasm {
    // TODO we can't use usize otherwise it might be a big int in JS if count is too high
    max_items: usize,
    code: u32,
}

#[wasm_bindgen(js_class=MaxIdentityPublicKeyLimitReachedError)]
impl MaxIdentityPublicKeyLimitReachedErrorWasm {
    #[wasm_bindgen(js_name=getMaxItems)]
    pub fn max_items(&self) -> usize {
        self.max_items
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl MaxIdentityPublicKeyLimitReachedErrorWasm {
    pub fn new(max_items: usize, code: u32) -> Self {
        Self { max_items, code }
    }
}
