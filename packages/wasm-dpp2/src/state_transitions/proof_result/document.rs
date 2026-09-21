//! `VerifiedDocuments` proof-result wrapper.

use super::helpers::{js_obj, json_safe_credits, read_credits_property, read_map_property};
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use js_sys::{BigInt, Map};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "VerifiedDocuments")]
#[derive(Clone)]
pub struct VerifiedDocumentsWasm {
    pub(super) documents: Map, // Map<string(base58), DocumentWasm | undefined>
    /// The credit balance of the batch's owner after the write, read from the
    /// same state as the documents: a snapshot at the proof's block.
    pub(super) owner_balance: u64,
}

#[wasm_bindgen(js_class = VerifiedDocuments)]
impl VerifiedDocumentsWasm {
    #[wasm_bindgen(getter)]
    pub fn documents(&self) -> Map {
        self.documents.clone()
    }

    /// The credit balance of the batch's owner after the write, as a `BigInt`.
    #[wasm_bindgen(getter, js_name = ownerBalance)]
    pub fn owner_balance(&self) -> JsValue {
        BigInt::from(self.owner_balance).into()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[
            ("documents", self.documents.clone().into()),
            ("ownerBalance", BigInt::from(self.owner_balance).into()),
        ])
    }

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a
    /// plain object so its entries survive serialisation (otherwise
    /// `JSON.stringify({documents: <Map>})` produces `{"documents":{}}`), and
    /// the balance is a number while exact, else a decimal string.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            (
                "documents",
                crate::serialization::conversions::normalize_js_value_for_json(
                    &self.documents.clone().into(),
                )?,
            ),
            ("ownerBalance", json_safe_credits(self.owner_balance)),
        ]))
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedDocumentsWasm> {
        let documents = read_map_property(&value, "documents")?;
        let owner_balance = read_credits_property(&value, "ownerBalance")?;
        Ok(VerifiedDocumentsWasm {
            documents,
            owner_balance,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedDocumentsWasm> {
        Self::from_object(value)
    }
}

impl_wasm_type_info!(VerifiedDocumentsWasm, VerifiedDocuments);
