//! `distinctFrom` declarations: the identifier properties a document type
//! requires to differ from another value of the same document, from
//! protocol version 14 onward.
//!
//! `distinctFrom` annotates an identifier property with what its value must
//! differ from: the document's `$ownerId`, or another identifier property of
//! the same document type. Consensus compares the two on every create and
//! replace and refuses an equal pair (basic code 10419); a declaration whose
//! named property is absent from the document passes. What this module adds
//! is the ability to *discover* the declarations, "which properties of this
//! document type must differ, and from what?", without hand-parsing the
//! contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PROPERTY_DISTINCT_FROM_TS: &'static str = r#"
/**
 * A single `distinctFrom` declaration on a document type.
 *
 * Mirrors the `distinctFrom` keyword of the v3 document meta-schema, which is
 * active from protocol version 14. The field name is the schema keyword's
 * own, so what `contract.toJSON()` shows under `distinctFrom` and what these
 * accessors return line up key for key.
 */
export type DocumentPropertyDistinctFrom = {
  /**
   * Dotted path of the declaring property within the document type, for
   * example `"delegateId"`, or `"meta.reviewerId"` for a nested one. An
   * identifier property, or a typed array of identifiers whose `items`
   * carry the declaration, in which case every element is bound. This is the same string consensus reports in the `property` field
   * of `DocumentPropertyNotDistinctError` (code 10419).
   */
  path: string;
  /**
   * What the property's value must differ from: `"$ownerId"` for the
   * document's owner, or the dotted path of another identifier property of
   * the same document type. Consensus refuses a create or replace whose two
   * values are equal; when the named property is absent from the document
   * there is nothing to differ from, so the write passes.
   */
  distinctFrom: string;
};
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Array<DocumentPropertyDistinctFrom>")]
    pub type DocumentPropertyDistinctFromArrayJs;

    #[wasm_bindgen(typescript_type = "Map<string, Array<DocumentPropertyDistinctFrom>>")]
    pub type DocumentPropertyDistinctFromMapJs;
}

/// `Reflect::set` with the collection-getter error convention the `tokens`
/// and `groups` getters on `DataContract` already use.
fn set_field(target: &Object, key: &str, value: &JsValue, path: &str) -> WasmDppResult<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(|_| {
        WasmDppError::generic(format!(
            "unable to serialize the `{key}` field of the distinctFrom declaration at '{path}'"
        ))
    })?;
    Ok(())
}

/// Collect every `distinctFrom` declaration of one document type, in schema
/// property order.
///
/// Walks `flattened_properties` rather than `properties` because that is
/// what the consensus check walks, and because its error `property` is
/// built from its dotted key. Using the nested map would miss nested
/// declarations entirely.
pub(crate) fn distinct_from_for_document_type(
    document_type: DocumentTypeRef<'_>,
) -> WasmDppResult<Array> {
    let declarations = Array::new();

    for (path, property) in document_type.flattened_properties() {
        let Some(distinct_from) = &property.distinct_from else {
            continue;
        };
        let object = Object::new();
        set_field(&object, "path", &JsValue::from_str(path), path)?;
        set_field(
            &object,
            "distinctFrom",
            &JsValue::from_str(distinct_from.as_str()),
            path,
        )?;
        declarations.push(&object);
    }

    Ok(declarations)
}
