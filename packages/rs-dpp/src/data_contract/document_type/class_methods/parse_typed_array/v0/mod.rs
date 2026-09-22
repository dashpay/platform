use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;

use crate::data_contract::document_type::array::{ArrayItemType, TypedArrayProperty};
use crate::data_contract::document_type::{property_names, DocumentPropertyType};
use crate::data_contract::errors::DataContractError;

/// Generation 0 parse rules: an array property that does not declare
/// `byteArray` is a typed array, whose `items` must be one scalar element
/// schema. `minItems` and `maxItems` count elements and fit a u16;
/// `uniqueItems` defaults to false.
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

    Ok(Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
        item_type: ArrayItemType::try_from(*items)?,
        min_items: inner_properties.get_optional_integer(property_names::MIN_ITEMS)?,
        max_items: inner_properties.get_optional_integer(property_names::MAX_ITEMS)?,
        unique_items: inner_properties
            .get_optional_bool(property_names::UNIQUE_ITEMS)?
            .unwrap_or_default(),
    })))
}
