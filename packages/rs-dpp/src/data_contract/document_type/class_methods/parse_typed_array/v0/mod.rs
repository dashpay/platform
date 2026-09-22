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
