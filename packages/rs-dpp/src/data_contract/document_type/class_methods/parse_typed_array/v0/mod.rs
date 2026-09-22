use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;

use crate::data_contract::document_type::array::{ArrayItemType, TypedArrayProperty};
use crate::data_contract::document_type::{property_names, DocumentPropertyType};
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

    let item_type = ArrayItemType::try_from(*items)?;

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
        item_type,
        min_items,
        max_items,
        unique_items: inner_properties
            .get_optional_bool(property_names::UNIQUE_ITEMS)?
            .unwrap_or_default(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn parse(schema: Value) -> Result<Option<DocumentPropertyType>, DataContractError> {
        let map = schema
            .to_btree_ref_string_map()
            .expect("the schema is a map");
        parse_typed_array_v0(&map)
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
                item_type: ArrayItemType::Integer,
                min_items: Some(1),
                max_items: 4,
                unique_items: false,
            }))
        );
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
