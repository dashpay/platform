use crate::data_contract::document_type::array::ArrayItemType;
use crate::data_contract::document_type::{
    property_names, DocumentPropertyType, TypedArrayProperty,
};
use crate::data_contract::errors::DataContractError;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use std::collections::BTreeMap;

/// Generation 0: an array property that declares no `byteArray` is a typed
/// scalar array. Its `items` schema is read through
/// [`ArrayItemType::try_from_item_schema`], `minItems` and `maxItems` count
/// elements, and `uniqueItems` defaults to false. `contentMediaType` belongs
/// on the items and is refused on the array, as is `minItems` above
/// `maxItems`. Whether `maxItems` is declared, and stays under the system
/// limit, is checked at registration by the generation 3 driver under full
/// validation, like the other registration limits, so a stored contract is
/// never re-judged by a later cap.
#[inline(always)]
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

    let Some(items_value) = inner_properties.get(property_names::ITEMS) else {
        return Err(DataContractError::InvalidContractStructure(
            "an array property must be a byte array (byteArray: true) or declare an items schema"
                .to_string(),
        ));
    };
    if inner_properties.contains_key(property_names::CONTENT_MEDIA_TYPE) {
        return Err(DataContractError::InvalidContractStructure(
            "contentMediaType belongs on the items of a typed array, not on the array".to_string(),
        ));
    }
    let items = ArrayItemType::try_from(*items_value)?;
    let min_items: Option<u16> =
        inner_properties.get_optional_integer(property_names::MIN_ITEMS)?;
    let max_items: Option<u16> =
        inner_properties.get_optional_integer(property_names::MAX_ITEMS)?;
    if let (Some(min), Some(max)) = (min_items, max_items) {
        if min > max {
            return Err(DataContractError::InvalidContractStructure(format!(
                "typed array minItems {min} exceeds its maxItems {max}"
            )));
        }
    }
    let unique_items = inner_properties
        .get_optional_bool(property_names::UNIQUE_ITEMS)?
        .unwrap_or(false);

    Ok(Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
        items,
        min_items,
        max_items,
        unique_items,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn parse(schema: Value) -> Result<Option<DocumentPropertyType>, DataContractError> {
        let map = schema.to_btree_ref_string_map().unwrap();
        parse_typed_array_v0(&map)
    }

    #[test]
    fn should_leave_byte_arrays_and_scalars_to_the_scalar_parser() {
        assert_eq!(
            parse(platform_value!({ "type": "array", "byteArray": true, "maxItems": 32 })).unwrap(),
            None
        );
        assert_eq!(
            parse(platform_value!({ "type": "array", "byteArray": false })).unwrap(),
            None
        );
        assert_eq!(
            parse(platform_value!({ "type": "string", "maxLength": 3 })).unwrap(),
            None
        );
    }

    #[test]
    fn should_parse_a_typed_array_with_optional_bounds() {
        assert_eq!(
            parse(platform_value!({ "type": "array", "items": { "type": "integer" } })).unwrap(),
            Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
                items: ArrayItemType::Integer,
                min_items: None,
                max_items: None,
                unique_items: false,
            }))
        );
    }

    #[test]
    fn should_refuse_a_typed_array_without_items_with_content_media_type_or_with_min_over_max() {
        for (schema, fragment) in [
            (
                platform_value!({ "type": "array", "maxItems": 2 }),
                "items schema",
            ),
            (
                platform_value!({
                    "type": "array",
                    "maxItems": 2,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "items": { "type": "integer" }
                }),
                "contentMediaType",
            ),
            (
                platform_value!({
                    "type": "array",
                    "minItems": 3,
                    "maxItems": 2,
                    "items": { "type": "integer" }
                }),
                "minItems",
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
