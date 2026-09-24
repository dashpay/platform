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

use crate::data_contract::document_type_reference::reference_target_to_js;
use crate::error::{WasmDppError, WasmDppResult};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::array::{ArrayItemConstraints, TypedArrayProperty};
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
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
 * characters, `minItems` / `maxItems` a byte array element's bytes,
 * `minimum` / `maximum` an integer or number element's range, and `enum`
 * the values an element must be one of, in declared order. A bound is
 * absent when the schema omits it.
 */
export type DocumentTypedArrayItem =
  | { type: 'integer'; minimum?: number; maximum?: number; enum?: number[] }
  | { type: 'number'; minimum?: number; maximum?: number; enum?: number[] }
  | { type: 'boolean'; enum?: boolean[] }
  | { type: 'string'; minLength?: number; maxLength?: number; enum?: string[] }
  | { type: 'byteArray'; minItems?: number; maxItems?: number }
  | {
      type: 'identifier';
      /**
       * The `refersTo` declaration every element carries, when the `items`
       * schema declares one: consensus checks each element as a single
       * reference when a document is created or replaced, and a write error
       * names the failing element by its list path (`"reasons[2]"`). Never
       * `identityPublicKey`; a reference expression, which each element must
       * meet on its own. The same declaration is listed by
       * `documentTypeReferences` at the path `"<path>[]"`.
       */
      refersTo?: DocumentPropertyReferenceTarget;
    };

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
  /** The most elements a document may hold; every typed array declares it. */
  maxItems: number;
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

/// A scalar element constraint as a JS value: strings, numbers and
/// booleans, which are the only member kinds the parser admits.
fn scalar_to_js(value: &Value) -> Option<JsValue> {
    if let Some(text) = value.as_text() {
        return Some(JsValue::from_str(text));
    }
    if let Some(flag) = value.as_bool() {
        return Some(JsValue::from_bool(flag));
    }
    value.to_float().ok().map(JsValue::from_f64)
}

/// Build the flat, internally-tagged JS object for one element type.
/// `declaring_contract_id` resolves an element reference's absent
/// `contractId`, as `documentTypeReferences` does.
fn item_to_js(
    item_type: &DocumentPropertyType,
    constraints: &ArrayItemConstraints,
    declaring_contract_id: Identifier,
    path: &str,
) -> WasmDppResult<JsValue> {
    let object = Object::new();
    let kind = match item_type {
        DocumentPropertyType::U8
        | DocumentPropertyType::I8
        | DocumentPropertyType::U16
        | DocumentPropertyType::I16
        | DocumentPropertyType::U32
        | DocumentPropertyType::I32
        | DocumentPropertyType::U64
        | DocumentPropertyType::I64
        | DocumentPropertyType::U128
        | DocumentPropertyType::I128 => "integer",
        // No items schema parses to a date; reported as the number it
        // decodes to rather than failing the whole collection
        DocumentPropertyType::F64 | DocumentPropertyType::Date => "number",
        DocumentPropertyType::Boolean => "boolean",
        DocumentPropertyType::String(_) => "string",
        DocumentPropertyType::ByteArray(_) => "byteArray",
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            "identifier"
        }
        other => {
            return Err(WasmDppError::generic(format!(
                "the typed array declared at '{path}' has a {} element, which is not a scalar",
                other.name()
            )));
        }
    };
    set_field(&object, "type", &JsValue::from_str(kind), path)?;

    match item_type {
        DocumentPropertyType::String(sizes) => {
            set_bound(&object, "minLength", sizes.min_length, path)?;
            set_bound(&object, "maxLength", sizes.max_length, path)?;
        }
        DocumentPropertyType::ByteArray(sizes) => {
            set_bound(&object, "minItems", sizes.min_size, path)?;
            set_bound(&object, "maxItems", sizes.max_size, path)?;
        }
        DocumentPropertyType::IdentifierWithReference(target) => {
            set_field(
                &object,
                "refersTo",
                &reference_target_to_js(target, declaring_contract_id, path)?,
                path,
            )?;
        }
        _ => {}
    }

    // The element constraints the parser reads, absent when undeclared
    if let Some(minimum) = constraints.minimum.as_ref().and_then(scalar_to_js) {
        set_field(&object, "minimum", &minimum, path)?;
    }
    if let Some(maximum) = constraints.maximum.as_ref().and_then(scalar_to_js) {
        set_field(&object, "maximum", &maximum, path)?;
    }
    if let Some(allowed_values) = &constraints.allowed_values {
        let members = Array::new();
        for member in allowed_values.iter().filter_map(scalar_to_js) {
            members.push(&member);
        }
        set_field(&object, "enum", &members, path)?;
    }

    Ok(object.into())
}

/// Build the JS object for one typed array property.
fn typed_array_to_js(
    path: &str,
    typed_array: &TypedArrayProperty,
    declaring_contract_id: Identifier,
) -> WasmDppResult<JsValue> {
    let object = Object::new();
    set_field(&object, "path", &JsValue::from_str(path), path)?;
    set_field(
        &object,
        "items",
        &item_to_js(
            &typed_array.item_type,
            &typed_array.item_constraints,
            declaring_contract_id,
            path,
        )?,
        path,
    )?;
    set_bound(&object, "minItems", typed_array.min_items, path)?;
    set_field(
        &object,
        "maxItems",
        &JsValue::from_f64(f64::from(typed_array.max_items)),
        path,
    )?;
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
    declaring_contract_id: Identifier,
) -> WasmDppResult<Array> {
    let typed_arrays = Array::new();

    for (path, property) in document_type.flattened_properties() {
        if let DocumentPropertyType::TypedArray(typed_array) = &property.property_type {
            typed_arrays.push(&typed_array_to_js(
                path,
                typed_array,
                declaring_contract_id,
            )?);
        }
    }

    Ok(typed_arrays)
}
