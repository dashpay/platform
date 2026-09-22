use std::collections::BTreeMap;

use platform_value::Value;
use platform_version::version::PlatformVersion;

use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::errors::DataContractError;

mod v0;

/// Parses a typed array property: `type: "array"` with an `items` element
/// schema in place of `byteArray`, into [`DocumentPropertyType::TypedArray`].
///
/// Returns `None` for every other property, a byte array included, which the
/// caller leaves to `DocumentPropertyType::try_from_value_map`.
///
/// Versioned on `parse_typed_array` in the platform version's document type
/// schema versions. `None` selects the behavior of the versions that predate
/// typed arrays: nothing is parsed here, so `try_from_value_map` refuses an
/// array that is not a byte array, exactly as those versions always did.
pub(crate) fn parse_typed_array(
    inner_properties: &BTreeMap<String, &Value>,
    platform_version: &PlatformVersion,
) -> Result<Option<DocumentPropertyType>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .parse_typed_array
    {
        None => Ok(None),
        Some(0) => v0::parse_typed_array_v0(inner_properties),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "parse_typed_array version {version} is not supported"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::array::{
        ArrayItemConstraints, ArrayItemType, TypedArrayProperty,
    };
    use platform_value::platform_value;

    #[test]
    fn should_parse_a_typed_array_from_protocol_version_14_and_leave_it_alone_before() {
        let schema = platform_value!({
            "type": "array",
            "minItems": 1,
            "maxItems": 8,
            "uniqueItems": true,
            "items": { "type": "string", "maxLength": 16 }
        });
        let map = schema
            .to_btree_ref_string_map()
            .expect("the schema is a map");

        assert_eq!(
            parse_typed_array(&map, PlatformVersion::latest()).expect("parses"),
            Some(DocumentPropertyType::TypedArray(TypedArrayProperty {
                item_type: ArrayItemType::String(None, Some(16)),
                item_constraints: ArrayItemConstraints::default(),
                min_items: Some(1),
                max_items: 8,
                unique_items: true,
            }))
        );
        // Protocol version 13 leaves the property to the scalar parser
        let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13 exists");
        assert_eq!(
            parse_typed_array(&map, platform_version_13).expect("parses"),
            None
        );
    }
}
