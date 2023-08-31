use crate::document::DocumentWasm;
use dpp::document::Document;
use itertools::Itertools;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Documents have mixed owner ids")]
pub struct MismatchOwnerIdsError {
    documents: Vec<DocumentWasm>,
}

#[wasm_bindgen]
impl MismatchOwnerIdsError {
    #[wasm_bindgen(constructor)]
    pub fn new(documents: Vec<JsValue>) -> MismatchOwnerIdsError {
        Self {
            documents: into_vec_of(&documents),
        }
    }

    #[wasm_bindgen(js_name=getDocuments)]
    pub fn get_documents(&self) -> Vec<JsValue> {
        to_vec_js(self.documents.clone())
    }
}

impl MismatchOwnerIdsError {
    pub fn from_documents(documents: Vec<Document>) -> MismatchOwnerIdsError {
        Self {
            documents: documents.into_iter().map(DocumentWasm::from).collect_vec(),
        }
    }
}
