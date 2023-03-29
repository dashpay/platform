use std::collections::HashMap;

use dpp::state_transition::fee::Refunds;
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

use crate::{identifier::IdentifierWrapper, utils::Inner};

#[derive(Clone)]
#[wasm_bindgen(js_name=Refunds)]
pub struct RefundsWasm(Refunds);

#[wasm_bindgen(js_class=Refunds)]
impl RefundsWasm {
    #[wasm_bindgen(getter)]
    pub fn identifier(&self) -> IdentifierWrapper {
        self.0.identifier.into()
    }

    #[wasm_bindgen(getter)]
    pub fn credits_per_epoch(&self) -> js_sys::Map {
        convert_hashmap_to_jsmap(&self.0.credits_per_epoch)
    }
}

impl From<Refunds> for RefundsWasm {
    fn from(value: Refunds) -> Self {
        RefundsWasm(value)
    }
}
impl From<&Refunds> for RefundsWasm {
    fn from(value: &Refunds) -> Self {
        RefundsWasm(value.clone())
    }
}

impl Inner for RefundsWasm {
    type InnerItem = Refunds;

    fn into_inner(self) -> Self::InnerItem {
        self.0
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.0
    }
}

pub fn convert_hashmap_to_jsmap(map: &HashMap<String, u64>) -> js_sys::Map {
    let mut js_map = js_sys::Map::new();
    for (key, value) in map {
        js_map.set(&JsValue::from_str(&key), &BigInt::from(*value));
    }
    js_map
}
