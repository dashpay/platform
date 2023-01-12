use dpp::{
    document::document_transition::{
        document_create_transition, DocumentCreateTransition, DocumentTransitionObjectLike,
    },
    util::json_value::JsonValueExt,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer, identifier::IdentifierWrapper, lodash::lodash_set, utils::WithJsError,
};

#[wasm_bindgen(js_name=DocumentCreateTransition)]
#[derive(Debug, Clone)]
pub struct DocumentCreateTransitionWasm {
    inner: DocumentCreateTransition,
}

impl From<DocumentCreateTransition> for DocumentCreateTransitionWasm {
    fn from(v: DocumentCreateTransition) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=DocumentCreateTransition)]
impl DocumentCreateTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let mut value = self.inner.to_object().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        let (identifiers_paths, binary_paths) = self
            .inner
            .base
            .data_contract
            .get_identifiers_and_binary_paths(&self.inner.base.document_type);

        for path in identifiers_paths
            .into_iter()
            .chain(document_create_transition::IDENTIFIER_FIELDS)
        {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                let id = IdentifierWrapper::new(bytes)?;
                lodash_set(&js_value, path, id.into());
            }
        }

        for path in binary_paths
            .into_iter()
            .chain(document_create_transition::BINARY_FIELDS)
        {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                let buffer = Buffer::from_bytes(&bytes);
                lodash_set(&js_value, path, buffer.into());
            }
        }

        Ok(js_value)
    }
}
