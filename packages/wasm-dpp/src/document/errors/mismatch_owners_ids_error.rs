use crate::document::extended_document::ExtendedDocumentWasm;
use dpp::document::ExtendedDocument;
use itertools::Itertools;
use thiserror::Error;

use super::*;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Documents have mixed owner ids")]
pub struct MismatchOwnerIdsError {
    documents: Vec<ExtendedDocumentWasm>,
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
    pub fn from_documents(documents: Vec<ExtendedDocument>) -> MismatchOwnerIdsError {
        Self {
            documents: documents
                .into_iter()
                .map(ExtendedDocumentWasm::from)
                .collect_vec(),
        }
    }
}
