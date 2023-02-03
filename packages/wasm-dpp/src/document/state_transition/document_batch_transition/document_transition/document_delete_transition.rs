use dpp::{
    document::document_transition::{
        document_delete_transition, DocumentDeleteTransition, DocumentTransitionObjectLike,
    },
    util::json_value::JsonValueExt,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{identifier::IdentifierWrapper, lodash::lodash_set, utils::WithJsError};

#[wasm_bindgen(js_name=DocumentDeleteTransition)]
#[derive(Debug, Clone)]
pub struct DocumentDeleteTransitionWasm {
    inner: DocumentDeleteTransition,
}

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionWasm {
    fn from(v: DocumentDeleteTransition) -> Self {
        Self { inner: v }
    }
}

#[wasm_bindgen(js_class=DocumentDeleteTransition)]
impl DocumentDeleteTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let mut value = self.inner.to_object().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        for field in document_delete_transition::IDENTIFIER_FIELDS {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(field) {
                let id = IdentifierWrapper::new(bytes)?;
                lodash_set(&js_value, field, id.into());
            }
        }

        Ok(js_value)
    }
}
