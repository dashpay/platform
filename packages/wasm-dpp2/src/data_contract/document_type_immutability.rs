//! `immutable` / `immutableAllowSetting` / `immutableAfter` declarations: the
//! per-property immutability a mutable document type carries from protocol
//! version 14 onward.
//!
//! `immutable` lists the top-level properties frozen at document creation,
//! `immutableAllowSetting` the subset of them a replace may still set while
//! the stored document has no value for them (frozen from then on), and
//! `immutableAfter` the properties a replace may change only for so many
//! seconds after the document's `$createdAt`. Consensus enforces them on
//! every replace (codes 40128 and 40143). What this module
//! adds is the ability to *discover* the declarations, "which properties of
//! this document type can never change, and which may still be set once?",
//! without hand-parsing the contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::define_own_property;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use js_sys::{Array, Object, Reflect};
use std::collections::BTreeSet;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TYPE_IMMUTABLE_PROPERTIES_TS: &'static str = r#"
/**
 * The immutability declarations of one document type.
 *
 * Mirrors the `immutable`, `immutableAllowSetting` and `immutableAfter`
 * keywords of the v3 document meta-schema, which are active from protocol
 * version 14. The field names are the schema keywords' own, so what
 * `contract.toJSON()` shows and what these accessors return line up key for
 * key. All three are sorted by property name and hold top-level property
 * names only: listing an object property freezes it whole, nested values
 * included.
 */
export type DocumentTypeImmutableProperties = {
  /**
   * Top-level properties frozen at document creation. A replace that
   * changes, adds or removes any of them is rejected with consensus code
   * 40128 (`DocumentImmutabilityErrorCode.DocumentImmutablePropertyChanged`),
   * except for the one transition `immutableAllowSetting` permits.
   */
  immutable: string[];
  /**
   * The subset of `immutable` a replace may still set while the stored
   * document has no value for it. Once present the property is frozen like
   * the rest. Always a subset of `immutable`.
   */
  immutableAllowSetting: string[];
  /**
   * Top-level properties a replace may change only for a while after the
   * document is created, each mapped to that window in seconds. Once block
   * time is past the document's `$createdAt` plus the window, a replace that
   * changes, adds or removes the property is rejected with consensus code
   * 40143 (`DocumentImmutabilityErrorCode.DocumentPropertyEditWindowElapsed`).
   * No key is also in `immutable`.
   */
  immutableAfter: Record<string, number>;
};
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentTypeImmutableProperties")]
    pub type DocumentTypeImmutablePropertiesJs;

    #[wasm_bindgen(typescript_type = "Map<string, DocumentTypeImmutableProperties>")]
    pub type DocumentTypeImmutablePropertiesMapJs;
}

/// `Reflect::set` with the collection-getter error convention the `tokens`
/// and `groups` getters on `DataContract` already use.
fn set_field(
    target: &Object,
    key: &str,
    value: &JsValue,
    document_type_name: &str,
) -> WasmDppResult<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(|_| {
        WasmDppError::generic(format!(
            "unable to serialize the `{key}` field of the immutability declarations of document \
             type '{document_type_name}'"
        ))
    })?;
    Ok(())
}

fn names_to_array(names: &BTreeSet<String>) -> Array {
    names.iter().map(|name| JsValue::from_str(name)).collect()
}

/// Build the `{ immutable, immutableAllowSetting, immutableAfter }` object
/// for one document type. Each comes out in name order, which is the order
/// the sets and the map keep and the order consensus reports the first
/// offending property in.
pub(crate) fn immutable_properties_for_document_type(
    document_type: DocumentTypeRef<'_>,
    document_type_name: &str,
) -> WasmDppResult<Object> {
    let object = Object::new();
    set_field(
        &object,
        "immutable",
        &names_to_array(document_type.immutable_fields()).into(),
        document_type_name,
    )?;
    set_field(
        &object,
        "immutableAllowSetting",
        &names_to_array(document_type.immutable_fields_allow_setting()).into(),
        document_type_name,
    )?;
    let windows = Object::new();
    for (property, seconds) in document_type.immutable_after_seconds() {
        // A property named like an inherited accessor (`__proto__`) stays a key
        define_own_property(&windows, property, JsValue::from(*seconds))?;
    }
    set_field(
        &object,
        "immutableAfter",
        &windows.into(),
        document_type_name,
    )?;
    Ok(object)
}
