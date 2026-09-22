//! Typed scalar array properties: the `type: array` properties declared by an
//! `items` schema instead of `byteArray: true`, which a contract may carry
//! from protocol version 14 onward.
//!
//! Such a property is a list of one scalar type stored inline in the document
//! as a count followed by its elements; consensus enforces the schema's
//! `minItems` / `maxItems` / `uniqueItems` and the item bounds when the
//! document is written. What this module adds is the ability to *discover*
//! the declarations, "which properties of this document type are lists, and
//! of what?", without hand-parsing the contract's raw JSON schema.

use crate::error::{WasmDppError, WasmDppResult};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::array::ArrayItemType;
use dpp::data_contract::document_type::{
    DocumentPropertyType, DocumentTypeRef, TypedArrayProperty,
};
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TYPED_ARRAY_PROPERTY_TS: &'static str = r#"
/**
 * The item type of a typed scalar array property, as the contract parser
 * reads the array's `items` schema.
 *
 * `type` is the parsed kind, not the raw schema `type`: a byte array item
 * (`type: 'array'` with `byteArray: true` in the schema) whose
 * `contentMediaType` is the identifier media type is an `identifier`, and
 * any other byte array item is a `byteArray` with its `minItems` /
 * `maxItems` byte bounds. A string item carries its `minLength` /
 * `maxLength`. A bound is absent when the schema declares none. Every other
 * bound the item schema may declare (`minimum`, `pattern`, ...) stays in the
 * raw schema, which `contract.toJSON()` still shows.
 */
export type DocumentPropertyArrayItemType =
  | { type: 'integer' }
  | { type: 'number' }
  | { type: 'string'; minLength?: number; maxLength?: number }
  | { type: 'byteArray'; minItems?: number; maxItems?: number }
  | { type: 'identifier' }
  | { type: 'boolean' }
  | { type: 'date' };

/**
 * A typed scalar array property of a document type: `type: array` declared
 * by an `items` schema (protocol version 14). Its value is a list of one
 * scalar type, stored inline in the document as a count followed by the
 * elements, and carried by a document as a plain array of the item type's
 * values. The field names are the schema keywords' own, so what
 * `contract.toJSON()` shows under the property and what these accessors
 * return line up key for key.
 */
export type DocumentTypedArrayProperty = {
  /**
   * Dotted path of the property within the document type, for example
   * `"reasons"`, or `"meta.tags"` for a nested one.
   */
  path: string;
  /** The parsed `items` schema. */
  items: DocumentPropertyArrayItemType;
  /**
   * Fewest elements a document may carry. Absent when the schema declares
   * none, which consensus reads as zero.
   */
  minItems?: number;
  /**
   * Most elements a document may carry. Contract registration requires it,
   * so it is only absent on a contract parsed without validation.
   */
  maxItems?: number;
  /** Whether consensus refuses a repeated element. */
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
            "unable to serialize the `{key}` field of the typed array property at '{path}'"
        ))
    })?;
    Ok(())
}

/// Sets a size bound when the schema declares one; an undeclared bound is
/// absent, matching the schema's own omission.
fn set_optional_size(
    target: &Object,
    key: &str,
    size: Option<usize>,
    path: &str,
) -> WasmDppResult<()> {
    if let Some(size) = size {
        set_field(target, key, &JsValue::from_f64(size as f64), path)?;
    }
    Ok(())
}

/// The internally-tagged JS object for one item type.
fn item_type_to_js(item_type: &ArrayItemType, path: &str) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "type", &JsValue::from_str(item_type.name()), path)?;
    match item_type {
        ArrayItemType::String(min_length, max_length) => {
            set_optional_size(&object, "minLength", *min_length, path)?;
            set_optional_size(&object, "maxLength", *max_length, path)?;
        }
        ArrayItemType::ByteArray(min_size, max_size) => {
            set_optional_size(&object, "minItems", *min_size, path)?;
            set_optional_size(&object, "maxItems", *max_size, path)?;
        }
        ArrayItemType::Integer
        | ArrayItemType::Number
        | ArrayItemType::Identifier
        | ArrayItemType::Boolean
        | ArrayItemType::Date => {}
    }
    Ok(object.into())
}

/// Build the flat JS object for one typed array property.
fn typed_array_to_js(path: &str, array: &TypedArrayProperty) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;
    set_field(
        &object,
        "items",
        &item_type_to_js(&array.items, path)?,
        path,
    )?;
    if let Some(min_items) = array.min_items {
        set_field(
            &object,
            "minItems",
            &JsValue::from_f64(f64::from(min_items)),
            path,
        )?;
    }
    if let Some(max_items) = array.max_items {
        set_field(
            &object,
            "maxItems",
            &JsValue::from_f64(f64::from(max_items)),
            path,
        )?;
    }
    set_field(
        &object,
        "uniqueItems",
        &JsValue::from_bool(array.unique_items),
        path,
    )?;
    Ok(object.into())
}

/// Collect every typed array property of one document type, in schema
/// property order.
///
/// Walks `flattened_properties`, as the reference accessor does, so a nested
/// array is reported under its dotted path.
pub(crate) fn typed_array_properties_for_document_type(
    document_type: DocumentTypeRef<'_>,
) -> WasmDppResult<Array> {
    let arrays = Array::new();

    for (path, property) in document_type.flattened_properties() {
        if let DocumentPropertyType::TypedArray(array) = &property.property_type {
            arrays.push(&typed_array_to_js(path, array)?);
        }
    }

    Ok(arrays)
}
