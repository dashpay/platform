use crate::buffer::Buffer;
use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MaxIdentityPublicKeyLimitReachedError)]
pub struct MaxIdentityPublicKeyLimitReachedErrorWasm {
    max_items: usize,
    code: u32,
}

#[wasm_bindgen(js_class=MaxIdentityPublicKeyLimitReachedError)]
impl MaxIdentityPublicKeyLimitReachedErrorWasm {
    #[wasm_bindgen(js_name=getIdentityId)]
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
        Self {
            max_items, code
        }
    }
}
