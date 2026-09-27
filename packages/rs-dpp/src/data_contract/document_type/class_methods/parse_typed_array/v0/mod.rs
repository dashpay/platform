use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use platform_version::version::PlatformVersion;

use crate::data_contract::document_type::array::{ArrayItemConstraints, TypedArrayProperty};
use crate::data_contract::document_type::class_methods::try_from_schema::apply_element_reference;
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
    platform_version: &PlatformVersion,
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

    let item_type = parse_element_type(items, options, platform_version)?;
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
        item_type: Box::new(item_type),
        item_constraints,
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
/// A `refersTo` on identifier elements is folded into the element type by
/// `apply_element_reference`, versioned with and calling the rules of
/// `apply_property_reference` that fold one into a scalar identifier, so an
/// element reference has the scalar's target types, keys and checks: the
/// element becomes `IdentifierWithReference(target)`. The one target refused
/// is `identityPublicKey`, in either form: its `keyIdProperty` names a single
/// sibling key id, and an `identityProperty` declaration sits on the key id
/// itself, neither of which can pair with many elements. The contract-level
/// checks of the declaration (the referenced document type, the
/// `propertyAgreement` sides and value kinds) need other contracts and run at
/// registration in drive-abci, which visits element references too.
fn parse_element_type(
    items: &Value,
    options: &DocumentPropertyTypeParsingOptions,
    platform_version: &PlatformVersion,
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
    let element_type = apply_element_reference(&items_map, element_type, platform_version)?;
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
        | DocumentPropertyType::IdentifierWithReference(_)
        | DocumentPropertyType::Boolean => Ok(element_type),
        other => Err(DataContractError::InvalidContractStructure(format!(
            "unsupported typed array element type: {}",
            other.name()
        ))),
    }
}

/// Whether an `enum` member, a `minimum` or a `maximum` is a value of the
/// element type: a string, an integer, a number (an integer counts) or a
/// boolean.
fn is_value_of(element_type: &DocumentPropertyType, value: &Value) -> bool {
    match element_type {
        DocumentPropertyType::String(_) => value.is_text(),
        DocumentPropertyType::U128
        | DocumentPropertyType::I128
        | DocumentPropertyType::U64
        | DocumentPropertyType::I64
        | DocumentPropertyType::U32
        | DocumentPropertyType::I32
        | DocumentPropertyType::U16
        | DocumentPropertyType::I16
        | DocumentPropertyType::U8
        | DocumentPropertyType::I8 => value.to_integer::<i128>().is_ok(),
        DocumentPropertyType::F64 => value.to_float().is_ok(),
        DocumentPropertyType::Boolean => value.as_bool().is_some(),
        _ => false,
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
    element_type: &DocumentPropertyType,
) -> Result<ArrayItemConstraints, DataContractError> {
    let items_map = items.to_btree_ref_string_map()?;
    let mut constraints = ArrayItemConstraints::default();
    let is_number = element_type.is_integer()
        || matches!(
            element_type,
            DocumentPropertyType::U128 | DocumentPropertyType::I128 | DocumentPropertyType::F64
        );

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
            element_type,
            DocumentPropertyType::ByteArray(_)
                | DocumentPropertyType::Identifier
                | DocumentPropertyType::IdentifierWithReference(_)
        ) {
            return Err(DataContractError::InvalidContractStructure(
                "enum is not supported on byte array or identifier elements of a typed array"
                    .to_string(),
            ));
        }
        if let Some(member) = members
            .iter()
            .find(|member| !is_value_of(element_type, member))
        {
            return Err(DataContractError::InvalidContractStructure(format!(
                "every enum member of a typed array's elements must be a {} value, found {}",
                element_type.name(),
                member
            )));
        }
        constraints.allowed_values = Some(members.clone());
    }

    if is_number {
        let read_bound = |keyword: &str| -> Result<Option<Value>, DataContractError> {
            let Some(bound) = items_map.get(keyword) else {
                return Ok(None);
            };
            if !is_value_of(element_type, bound) {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "the {keyword} of a typed array's elements must be a {} value, found {}",
                    element_type.name(),
                    bound
                )));
            }
            Ok(Some((*bound).clone()))
        };
        constraints.minimum = read_bound(property_names::MINIMUM)?;
        constraints.maximum = read_bound(property_names::MAXIMUM)?;
        if let (Some(minimum), Some(maximum)) = (&constraints.minimum, &constraints.maximum) {
            if minimum.to_float().ok() > maximum.to_float().ok() {
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
        parse_typed_array_v0(
            &map,
            &DocumentPropertyTypeParsingOptions::default(),
            PlatformVersion::latest(),
        )
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
        // Sized by its bounds, as a scalar integer is
        assert_eq!(*typed_array.item_type, DocumentPropertyType::U8);
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
                "must be a",
            ),
            (
                platform_value!({ "type": "boolean", "enum": [true, 0] }),
                "must be a boolean value",
            ),
            (
                platform_value!({ "type": "array", "byteArray": true, "enum": [[1, 2]] }),
                "not supported on byte array",
            ),
            // A number element's bound must be a number; an integer element's
            // bound is already read by the scalar parse that sizes it
            (
                platform_value!({ "type": "number", "minimum": "low" }),
                "minimum of a typed array's elements must be a",
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
                PlatformVersion::latest(),
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
