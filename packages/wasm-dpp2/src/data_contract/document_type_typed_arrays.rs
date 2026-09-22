//! Typed array properties: the lists of scalars a document type may declare
//! from protocol version 14 onward.
//!
//! A typed array is `type: "array"` with an `items` schema naming what every
//! element is, where a byte array declares `byteArray: true` instead. It is
//! stored inline in the document as an element count followed by the
//! elements, and it cannot be indexed. What this module adds is the ability
//! to *discover* the declarations, "which properties of this document type
//! are lists, and of what?", without hand-parsing the contract's raw JSON
//! schema.

use crate::error::{WasmDppError, WasmDppResult};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::array::{ArrayItemType, TypedArrayProperty};
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TYPED_ARRAY_PROPERTY_TS: &'static str = r#"
/**
 * What every element of a typed array is, parsed from its `items` schema.
 *
 * `type` names the element kind as DPP parses it: `byteArray` is an
 * `items` schema with `byteArray: true`, and `identifier` one that also
 * carries the identifier `contentMediaType`. The bound names are the schema
 * keywords' own: `minLength` / `maxLength` count a string element's
 * characters, `minItems` / `maxItems` a byte array element's bytes. A bound
 * is absent when the schema omits it.
 */
export type DocumentTypedArrayItem =
  | { type: 'integer' }
  | { type: 'number' }
  | { type: 'boolean' }
  | { type: 'string'; minLength?: number; maxLength?: number }
  | { type: 'byteArray'; minItems?: number; maxItems?: number }
  | { type: 'identifier' };

/**
 * A single typed array property of a document type.
 *
 * Mirrors the typed array form of the v3 document meta-schema, which is
 * active from protocol version 14. The field names are the schema
 * keywords' own, so what `contract.toJSON()` shows and what these accessors
 * return line up key for key.
 */
export type DocumentTypedArrayProperty = {
  /**
   * Dotted path of the property within the document type, for example
   * `"reasons"`, or `"team.members"` for one nested in an object.
   */
  path: string;
  /** What every element is. */
  items: DocumentTypedArrayItem;
  /** The fewest elements a document may hold; absent when not declared. */
  minItems?: number;
  /**
   * The most elements a document may hold. Contract registration requires
   * it, so it is only absent on a contract parsed without validation.
   */
  maxItems?: number;
  /** Whether a document repeating an element is refused. */
  uniqueItems: boolean;
};
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Array<DocumentTypedArrayProperty>")]
    pub type DocumentTypedArrayPropertyArrayJs;

    #[wasm_bindgen(typescript_type = "Map<string, Array<DocumentTypedArrayProperty>>")]
    pub type DocumentTypedArrayPropertyMapJs;
}

/// `Reflect::set` with the collection-getter error convention the `tokens`
/// and `groups` getters on `DataContract` already use.
fn set_field(target: &Object, key: &str, value: &JsValue, path: &str) -> WasmDppResult<()> {
    Reflect::set(target, &JsValue::from_str(key), value).map_err(|_| {
        WasmDppError::generic(format!(
            "unable to serialize the `{key}` field of the typed array declared at '{path}'"
        ))
    })?;
    Ok(())
}

/// Set a bound only when the schema declares it, matching the schema's own
/// omission.
fn set_bound<T: Into<f64>>(
    target: &Object,
    key: &str,
    bound: Option<T>,
    path: &str,
) -> WasmDppResult<()> {
    match bound {
        Some(bound) => set_field(target, key, &JsValue::from_f64(bound.into()), path),
        None => Ok(()),
    }
}

/// Build the flat, internally-tagged JS object for one element type.
fn item_to_js(item_type: &ArrayItemType, path: &str) -> WasmDppResult<JsValue> {
    let object = Object::new();
    let kind = match item_type {
        ArrayItemType::Integer => "integer",
        ArrayItemType::Number => "number",
        ArrayItemType::Boolean => "boolean",
        ArrayItemType::String(..) => "string",
        ArrayItemType::ByteArray(..) => "byteArray",
        ArrayItemType::Identifier => "identifier",
        // No items schema parses to a date; reported as the number it
        // decodes to rather than failing the whole collection
        ArrayItemType::Date => "number",
    };
    set_field(&object, "type", &JsValue::from_str(kind), path)?;

    // Parsed from u16 schema values, so exact as JS numbers
    let as_f64 = |bound: &Option<usize>| bound.map(|bound| bound as f64);
    match item_type {
        ArrayItemType::String(min_length, max_length) => {
            set_bound(&object, "minLength", as_f64(min_length), path)?;
            set_bound(&object, "maxLength", as_f64(max_length), path)?;
        }
        ArrayItemType::ByteArray(min_size, max_size) => {
            set_bound(&object, "minItems", as_f64(min_size), path)?;
            set_bound(&object, "maxItems", as_f64(max_size), path)?;
        }
        ArrayItemType::Integer
        | ArrayItemType::Number
        | ArrayItemType::Boolean
        | ArrayItemType::Identifier
        | ArrayItemType::Date => {}
    }

    Ok(object.into())
}

/// Build the JS object for one typed array property.
fn typed_array_to_js(path: &str, typed_array: &TypedArrayProperty) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;
    set_field(
        &object,
        "items",
        &item_to_js(&typed_array.item_type, path)?,
        path,
    )?;
    set_bound(&object, "minItems", typed_array.min_items, path)?;
    set_bound(&object, "maxItems", typed_array.max_items, path)?;
    set_field(
        &object,
        "uniqueItems",
        &JsValue::from_bool(typed_array.unique_items),
        path,
    )?;
    Ok(object.into())
}

/// Collect every typed array property of one document type, in schema
/// property order.
///
/// Walks `flattened_properties`, which reaches a typed array nested in an
/// object property and names it by its dotted path.
pub(crate) fn typed_arrays_for_document_type(
    document_type: DocumentTypeRef<'_>,
) -> WasmDppResult<Array> {
    let typed_arrays = Array::new();

    for (path, property) in document_type.flattened_properties() {
        if let DocumentPropertyType::TypedArray(typed_array) = &property.property_type {
            typed_arrays.push(&typed_array_to_js(path, typed_array)?);
        }
    }

    Ok(typed_arrays)
}
