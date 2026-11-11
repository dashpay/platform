//! `immutable` declarations: the per-property immutability a mutable document
//! type carries from protocol version 14 onward.
//!
//! `immutable` lists top-level properties a replace may not change, add or
//! remove: by name, frozen at document creation, or as `{ property, when }`,
//! frozen for any replace its condition holds for (judged on the document the
//! replace writes, with the stored one read through `$old.`). Consensus
//! enforces both on every replace (code 40128). What this module adds is the
//! ability to *discover* the declarations, "which properties of this document
//! type can never change, and which are frozen under a condition?", without
//! hand-parsing the contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use crate::serialization::platform_value_to_object;
use crate::utils::define_own_property;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::platform_value::Value;
use js_sys::{Array, Object, Reflect};
use std::collections::BTreeSet;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TYPE_IMMUTABLE_PROPERTIES_TS: &'static str = r#"
/**
 * The immutability declarations of one document type.
 *
 * Mirrors the `immutable` keyword of the v3 document meta-schema, which is
 * active from protocol version 14: its entries naming a property, and its
 * `{ property, when }` entries. Both hold top-level property names only,
 * sorted: listing an object property freezes it whole, nested values
 * included.
 */
export type DocumentTypeImmutableProperties = {
  /**
   * Top-level properties frozen at document creation. A replace that
   * changes, adds or removes any of them is rejected with consensus code
   * 40128 (`DocumentImmutabilityErrorCode.DocumentImmutablePropertyChanged`).
   */
  immutable: string[];
  /**
   * Top-level properties frozen for any replace their condition holds for,
   * each with the condition as the contract declares it (the grammar of a
   * `propertyConstraints` rule). It is judged on the document the replace
   * writes, whose `$updatedAt` is the replace's block time, and reads the
   * stored document through `$old.` paths: `{ present: "$old.mood" }` lets an
   * absent `mood` be set once. A replace changing one while its condition
   * holds is rejected with code 40128 as well. No key is also in `immutable`.
   */
  immutableWhen: Record<string, unknown>;
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

/// The keyword and the keys of its conditional entries, as the v3 document
/// meta-schema spells them.
const IMMUTABLE: &str = "immutable";
const ENTRY_PROPERTY: &str = "property";
const ENTRY_WHEN: &str = "when";

/// The `when` of the entry of the `immutable` keyword in `schema` that lists
/// `property` with a condition, as declared.
fn declared_condition<'a>(schema: &'a Value, property: &str) -> Option<&'a Value> {
    let entries = schema
        .get_optional_value(IMMUTABLE)
        .ok()
        .flatten()?
        .as_array()?;
    entries.iter().find_map(|entry| {
        let entry = entry.as_map()?;
        let listed = entry.iter().any(|(key, value)| {
            key.as_text() == Some(ENTRY_PROPERTY) && value.as_text() == Some(property)
        });
        if !listed {
            return None;
        }
        entry
            .iter()
            .find(|(key, _)| key.as_text() == Some(ENTRY_WHEN))
            .map(|(_, when)| when)
    })
}

/// Build the `{ immutable, immutableWhen }` object for one document type.
/// Both come out in name order, which is the order the parsed declarations
/// keep and the order consensus reports the first offending property in. A
/// condition is the one the contract declares, for the properties the parsed
/// type freezes under a condition: a contract read at a version that predates
/// the keyword reports none.
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
    let conditions = Object::new();
    for property in document_type.immutable_field_conditions().keys() {
        let when = declared_condition(document_type.schema(), property).ok_or_else(|| {
            WasmDppError::generic(format!(
                "the condition freezing '{property}' of document type '{document_type_name}' is \
                 missing from its schema"
            ))
        })?;
        // A property named like an inherited accessor (`__proto__`) stays a key
        define_own_property(&conditions, property, platform_value_to_object(when)?)?;
    }
    set_field(
        &object,
        "immutableWhen",
        &conditions.into(),
        document_type_name,
    )?;
    Ok(object)
}
