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
