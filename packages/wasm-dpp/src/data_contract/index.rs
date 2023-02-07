use dpp::{data_contract::document_type::IndexProperty, util::json_schema::Index};
use serde::{ser::SerializeMap, Serialize, Serializer};
use wasm_bindgen::prelude::*;

use crate::with_js_error;

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

/// Wrapper structure to serialize `Index` the way js-dpp expects it.
#[derive(Debug)]
struct IndexSerializeJs<'a>(&'a Index);

impl Serialize for IndexSerializeJs<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("name", &self.0.name)?;
        if self.0.unique {
            map.serialize_entry("unique", &true)?;
        }
        map.serialize_entry(
            "properties",
            &self
                .0
                .properties
                .iter()
                .map(IndexPropertySerializeJs)
                .collect::<Vec<_>>(),
        )?;

        map.end()
    }
}

/// Wrapper structure to serialize `InderProperty` the way js-dpp expects it.
#[derive(Debug)]
struct IndexPropertySerializeJs<'a>(&'a IndexProperty);

impl Serialize for IndexPropertySerializeJs<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        let order_str = if self.0.ascending { "asc" } else { "desc" };
        map.serialize_entry(&self.0.name, order_str)?;

        map.end()
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

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let object = with_js_error!(IndexSerializeJs(&self.inner).serialize(&serializer))?;
        Ok(object)
    }
}
