use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;

use crate::data_contract::document_type::array::TypedArrayProperty;
use crate::data_contract::document_type::{
    property_names, DocumentPropertyType, DocumentPropertyTypeParsingOptions,
};
use crate::data_contract::errors::DataContractError;

/// Generation 0 parse rules: an array property that does not declare
/// `byteArray` is a typed array, whose `items` must be one scalar element
/// schema. `minItems` and `maxItems` count elements and fit a u16; `maxItems`
/// is required and `minItems` may not exceed it; `contentMediaType` belongs on
/// the items; `uniqueItems` defaults to false. These are the shape of the
/// declaration, so they hold on every parse; the cap on `maxItems` is a
/// registration limit, checked by the generation 3 driver under full
/// validation.
pub(super) fn parse_typed_array_v0(
    inner_properties: &BTreeMap<String, &Value>,
    options: &DocumentPropertyTypeParsingOptions,
) -> Result<Option<DocumentPropertyType>, DataContractError> {
    let is_array = inner_properties
        .get(property_names::TYPE)
        .and_then(|type_value| type_value.as_text())
        == Some("array");

    // A byte array keeps its scalar parse, and so does `byteArray: false`,
    // which that parse refuses
    if !is_array || inner_properties.contains_key(property_names::BYTE_ARRAY) {
        return Ok(None);
    }

    let Some(items) = inner_properties.get(property_names::ITEMS) else {
        return Err(DataContractError::InvalidContractStructure(
            "an array property must declare either byteArray: true or the items schema of its \
             elements"
                .to_string(),
        ));
    };

    if inner_properties.contains_key(property_names::CONTENT_MEDIA_TYPE) {
        return Err(DataContractError::InvalidContractStructure(
            "contentMediaType belongs on the items of a typed array, not on the array".to_string(),
        ));
    }

    let item_type = parse_element_type(items, options)?;

    // Fee estimation sizes the inline list by its bound
    let Some(max_items) = inner_properties.get_optional_integer(property_names::MAX_ITEMS)? else {
        return Err(DataContractError::InvalidContractStructure(
            "a typed array must declare maxItems: its inline encoding is sized by it".to_string(),
        ));
    };
    let min_items: Option<u16> =
        inner_properties.get_optional_integer(property_names::MIN_ITEMS)?;
    if min_items.is_some_and(|min_items| min_items > max_items) {
        return Err(DataContractError::InvalidContractStructure(format!(
            "a typed array's minItems may not exceed its maxItems of {max_items}: no document \
             could hold the list"
        )));
    }

    Ok(Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
        item_type: Box::new(item_type),
        min_items,
        max_items,
        unique_items: inner_properties
            .get_optional_bool(property_names::UNIQUE_ITEMS)?
            .unwrap_or_default(),
    })))
}

/// The element type of a typed array: its `items` schema parsed exactly as a
/// scalar property schema is, so an integer element takes the width its
/// bounds give it and a byte array element with the identifier media type is
/// an identifier. Objects and arrays of arrays are refused.
///
/// `refersTo` is refused for now. A reference on identifier elements would be
/// read from this same map and folded into the element type, as
/// `apply_property_reference` folds one into a scalar identifier.
fn parse_element_type(
    items: &Value,
    options: &DocumentPropertyTypeParsingOptions,
) -> Result<DocumentPropertyType, DataContractError> {
    // The tuple form (`items: [..]`) and boolean schemas are not one element
    // schema
    let items_map = items.to_btree_ref_string_map().map_err(|_| {
        DataContractError::InvalidContractStructure(
            "the items of a typed array must be one element schema (an object)".to_string(),
        )
    })?;
    if items_map.contains_key(property_names::REF) {
        return Err(DataContractError::InvalidContractStructure(
            "the items of a typed array must be an inline element schema, not a $ref".to_string(),
        ));
    }
    if items_map.contains_key(property_names::REFERS_TO) {
        return Err(DataContractError::InvalidContractStructure(
            "refersTo is not supported on the elements of a typed array".to_string(),
        ));
    }
    match items_map
        .get(property_names::TYPE)
        .and_then(|type_value| type_value.as_text())
    {
        Some("object") => {
            return Err(DataContractError::InvalidContractStructure(
                "arrays of objects are not supported: the elements of a typed array must be \
                 scalars (integer, number, string, boolean, byte array or identifier)"
                    .to_string(),
            ))
        }
        Some("array") if !items_map.contains_key(property_names::BYTE_ARRAY) => {
            return Err(DataContractError::InvalidContractStructure(
                "arrays of arrays are not supported: an element of a typed array may be a byte \
                 array (byteArray: true) or an identifier, but not another array"
                    .to_string(),
            ))
        }
        _ => {}
    }

    let element_type = DocumentPropertyType::try_from_value_map(&items_map, options)?;
    match element_type {
        DocumentPropertyType::U128
        | DocumentPropertyType::I128
        | DocumentPropertyType::U64
        | DocumentPropertyType::I64
        | DocumentPropertyType::U32
        | DocumentPropertyType::I32
        | DocumentPropertyType::U16
        | DocumentPropertyType::I16
        | DocumentPropertyType::U8
        | DocumentPropertyType::I8
        | DocumentPropertyType::F64
        | DocumentPropertyType::String(_)
        | DocumentPropertyType::ByteArray(_)
        | DocumentPropertyType::Identifier
        | DocumentPropertyType::Boolean => Ok(element_type),
        other => Err(DataContractError::InvalidContractStructure(format!(
            "unsupported typed array element type: {}",
            other.name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn parse(schema: Value) -> Result<Option<DocumentPropertyType>, DataContractError> {
        let map = schema
            .to_btree_ref_string_map()
            .expect("the schema is a map");
        parse_typed_array_v0(&map, &DocumentPropertyTypeParsingOptions::default())
    }

    #[test]
    fn should_leave_byte_arrays_and_scalars_to_the_scalar_parser() {
        for schema in [
            platform_value!({ "type": "array", "byteArray": true, "maxItems": 32 }),
            // The scalar parse refuses it, as it always did
            platform_value!({ "type": "array", "byteArray": false }),
            platform_value!({ "type": "string", "maxLength": 3 }),
        ] {
            assert_eq!(parse(schema.clone()).expect("parses"), None, "{schema:?}");
        }
    }

    #[test]
    fn should_parse_a_typed_array_with_its_bounds() {
        assert_eq!(
            parse(platform_value!({
                "type": "array",
                "minItems": 1,
                "maxItems": 4,
                "items": { "type": "integer" }
            }))
            .expect("parses"),
            Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
                item_type: Box::new(DocumentPropertyType::I64),
                min_items: Some(1),
                max_items: 4,
                unique_items: false,
            }))
        );
    }

    /// An element is parsed by the scalar parser, so it takes the type a
    /// scalar property of the same schema takes: an integer sized by its
    /// bounds (when the contract sizes integers), an identifier from the
    /// identifier media type.
    #[test]
    fn should_type_an_element_as_a_scalar_property_of_its_schema() {
        for (items, sized_integer_types, expected) in [
            (
                platform_value!({ "type": "integer", "minimum": 0, "maximum": 100 }),
                true,
                DocumentPropertyType::U8,
            ),
            (
                platform_value!({ "type": "integer", "minimum": -1000, "maximum": 1000 }),
                true,
                DocumentPropertyType::I16,
            ),
            // A contract that does not size integers keeps them at 64 bits
            (
                platform_value!({ "type": "integer", "minimum": 0, "maximum": 100 }),
                false,
                DocumentPropertyType::I64,
            ),
            (
                platform_value!({
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier"
                }),
                true,
                DocumentPropertyType::Identifier,
            ),
        ] {
            let schema =
                platform_value!({ "type": "array", "maxItems": 4, "items": items.clone() });
            let map = schema
                .to_btree_ref_string_map()
                .expect("the schema is a map");
            let parsed = parse_typed_array_v0(
                &map,
                &DocumentPropertyTypeParsingOptions {
                    sized_integer_types,
                },
            )
            .expect("parses");
            let Some(DocumentPropertyType::TypedArray(typed_array)) = parsed else {
                panic!("{items:?} should parse to a typed array");
            };
            assert_eq!(*typed_array.item_type, expected, "{items:?}");
        }
    }

    #[test]
    fn should_refuse_a_typed_array_missing_items_or_max_items_or_with_a_misplaced_bound() {
        for (schema, fragment) in [
            (
                platform_value!({ "type": "array", "maxItems": 2 }),
                "items schema",
            ),
            (
                platform_value!({ "type": "array", "items": { "type": "integer" } }),
                "must declare maxItems",
            ),
            (
                platform_value!({
                    "type": "array",
                    "maxItems": 2,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "items": { "type": "integer" }
                }),
                "contentMediaType belongs on the items",
            ),
            (
                platform_value!({
                    "type": "array",
                    "minItems": 3,
                    "maxItems": 2,
                    "items": { "type": "integer" }
                }),
                "minItems may not exceed its maxItems",
            ),
        ] {
            let error = parse(schema.clone())
                .expect_err("should be refused")
                .to_string();
            assert!(
                error.contains(fragment),
                "{schema:?}: expected {fragment:?}, got {error}"
            );
        }
    }
}
