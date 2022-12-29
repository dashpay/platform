use dpp::{data_contract::extra::IndexProperty, util::json_schema::Index};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=IndexProperty)]
#[derive(Debug, Clone)]
pub struct IndexPropertyWasm {
    inner: IndexProperty,
}

impl From<IndexProperty> for IndexPropertyWasm {
    fn from(v: IndexProperty) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=IndexProperty)]
impl IndexPropertyWasm {
    #[wasm_bindgen(js_name=getName)]
    pub fn get_name(&self) -> String {
        self.inner.name.clone()
    }

    #[wasm_bindgen(js_name=isAscending)]
    pub fn is_ascending(&self) -> bool {
        self.inner.ascending
    }
}

#[wasm_bindgen(js_name=IndexDefinition)]
#[derive(Debug, Clone)]
pub struct IndexDefinitionWasm {
    inner: Index,
}

impl From<Index> for IndexDefinitionWasm {
    fn from(v: Index) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=IndexDefinition)]
impl IndexDefinitionWasm {
    #[wasm_bindgen(js_name=getName)]
    pub fn get_name(&self) -> String {
        self.inner.name.clone()
    }

    #[wasm_bindgen(js_name=getProperties)]
    pub fn get_properties(&self) -> Vec<JsValue> {
        self.inner
            .properties
            .iter()
            .map(|property| IndexPropertyWasm::from(property.clone()).into())
            .collect::<Vec<JsValue>>()
    }

    #[wasm_bindgen(js_name=isUnique)]
    pub fn is_unique(&self) -> bool {
        self.inner.unique
    }
}
