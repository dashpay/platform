use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;

use crate::data_contract::document_type::array::{
    ArrayItemConstraints, ArrayItemType, TypedArrayProperty,
};
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
    let item_constraints = parse_item_constraints(items, &item_type)?;

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
        item_constraints,
        min_items,
        max_items,
        unique_items: inner_properties
            .get_optional_bool(property_names::UNIQUE_ITEMS)?
            .unwrap_or_default(),
    })))
}

/// Whether an `enum` member is a value of the element type: a string, an
/// integer, a number (an integer counts) or a boolean.
fn is_member_of(item_type: &ArrayItemType, member: &Value) -> bool {
    match item_type {
        ArrayItemType::String(_, _) => member.is_text(),
        ArrayItemType::Integer => member.to_integer::<i64>().is_ok(),
        ArrayItemType::Number => member.to_float().is_ok(),
        ArrayItemType::Boolean => member.as_bool().is_some(),
        ArrayItemType::ByteArray(_, _) | ArrayItemType::Identifier | ArrayItemType::Date => false,
    }
}

/// The `enum`, `minimum` and `maximum` of the `items` schema. These are the
/// shape of the declaration, so they hold on every parse: an `enum` has at
/// least one member, every member is of the element type and a byte array or
/// identifier element takes none; `minimum` and `maximum` belong to integer
/// and number elements, are read as that type, and `minimum` never exceeds
/// `maximum`. The meta-schema states the same rules for the validating path.
fn parse_item_constraints(
    items: &Value,
    item_type: &ArrayItemType,
) -> Result<ArrayItemConstraints, DataContractError> {
    let items_map = items.to_btree_ref_string_map()?;
    let mut constraints = ArrayItemConstraints::default();

    if let Some(members) = items_map.get(property_names::ENUM) {
        let Some(members) = members.as_array() else {
            return Err(DataContractError::InvalidContractStructure(
                "the enum of a typed array's elements must be a list of values".to_string(),
            ));
        };
        if members.is_empty() {
            return Err(DataContractError::InvalidContractStructure(
                "the enum of a typed array's elements must hold at least one value".to_string(),
            ));
        }
        if matches!(
            item_type,
            ArrayItemType::ByteArray(_, _) | ArrayItemType::Identifier | ArrayItemType::Date
        ) {
            return Err(DataContractError::InvalidContractStructure(
                "enum is not supported on byte array or identifier elements of a typed array"
                    .to_string(),
            ));
        }
        if let Some(member) = members
            .iter()
            .find(|member| !is_member_of(item_type, member))
        {
            return Err(DataContractError::InvalidContractStructure(format!(
                "every enum member of a typed array's elements must be a {} value, found {}",
                item_type.name(),
                member
            )));
        }
        constraints.allowed_values = Some(members.clone());
    }

    if matches!(item_type, ArrayItemType::Integer | ArrayItemType::Number) {
        let read_bound = |keyword: &str| -> Result<Option<Value>, DataContractError> {
            let Some(bound) = items_map.get(keyword) else {
                return Ok(None);
            };
            if !is_member_of(item_type, bound) {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "the {keyword} of a typed array's elements must be a {} value, found {}",
                    item_type.name(),
                    bound
                )));
            }
            Ok(Some((*bound).clone()))
        };
        constraints.minimum = read_bound(property_names::MINIMUM)?;
        constraints.maximum = read_bound(property_names::MAXIMUM)?;
        if let (Some(minimum), Some(maximum)) = (&constraints.minimum, &constraints.maximum) {
            let min_exceeds_max = match item_type {
                ArrayItemType::Integer => {
                    minimum.to_integer::<i64>().ok() > maximum.to_integer::<i64>().ok()
                }
                _ => minimum.to_float().ok() > maximum.to_float().ok(),
            };
            if min_exceeds_max {
                return Err(DataContractError::InvalidContractStructure(
                    "the minimum of a typed array's elements may not exceed their maximum: no \
                     document could hold the list"
                        .to_string(),
                ));
            }
        }
    }

    Ok(constraints)
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
                item_constraints: ArrayItemConstraints::default(),
                min_items: Some(1),
                max_items: 4,
                unique_items: false,
            }))
        );
    }

    #[test]
    fn should_parse_the_enum_minimum_and_maximum_of_the_elements() {
        let parsed = parse(platform_value!({
            "type": "array",
            "maxItems": 4,
            "items": { "type": "integer", "minimum": 1, "maximum": 10, "enum": [1, 5, 10] }
        }))
        .expect("parses");
        let Some(DocumentPropertyType::TypedArray(typed_array)) = parsed else {
            panic!("expected a typed array, got {parsed:?}");
        };
        assert_eq!(typed_array.item_type, ArrayItemType::Integer);
        // The bounds keep the schema's own value kinds; compare as integers
        let as_integer = |value: &Value| value.to_integer::<i64>().expect("an integer");
        let constraints = &typed_array.item_constraints;
        assert_eq!(
            constraints
                .allowed_values
                .as_ref()
                .map(|members| members.iter().map(as_integer).collect::<Vec<_>>()),
            Some(vec![1, 5, 10])
        );
        assert_eq!(constraints.minimum.as_ref().map(as_integer), Some(1));
        assert_eq!(constraints.maximum.as_ref().map(as_integer), Some(10));
    }

    #[test]
    fn should_refuse_element_constraints_that_no_element_could_satisfy() {
        for (items, fragment) in [
            (
                platform_value!({ "type": "string", "enum": [] }),
                "at least one value",
            ),
            (
                platform_value!({ "type": "string", "enum": ["a", 1] }),
                "must be a string value",
            ),
            (
                platform_value!({ "type": "integer", "enum": [1, "b"] }),
                "must be a integer value",
            ),
            (
                platform_value!({ "type": "boolean", "enum": [true, 0] }),
                "must be a boolean value",
            ),
            (
                platform_value!({ "type": "array", "byteArray": true, "enum": [[1, 2]] }),
                "not supported on byte array",
            ),
            (
                platform_value!({ "type": "integer", "minimum": "low" }),
                "minimum of a typed array's elements must be a integer",
            ),
            (
                platform_value!({ "type": "number", "minimum": 2.5, "maximum": 1 }),
                "may not exceed their maximum",
            ),
        ] {
            let error = parse(platform_value!({
                "type": "array",
                "maxItems": 4,
                "items": items.clone()
            }))
            .expect_err("should be refused")
            .to_string();
            assert!(
                error.contains(fragment),
                "{items:?}: expected {fragment:?}, got {error}"
            );
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
