use std::collections::BTreeMap;

use platform_value::Value;
use platform_version::version::PlatformVersion;

use crate::data_contract::errors::DataContractError;

mod v0;

/// Parses the `requiredSince` keyword: the contract version from which the
/// property is required. Only meaningful on top-level required properties —
/// the document wire format encodes a required property without a presence
/// flag, so requiredness that varies by contract version must be resolvable
/// per property from the current schema alone (see the per-document contract
/// version stamp in document serialization format 3).
///
/// Versioned on `apply_required_since` in the platform version's document
/// type schema versions. `None` selects the behavior of the versions that
/// predate the keyword: it is ignored entirely, so their parses stay
/// byte-for-byte identical to what they always produced.
pub(crate) fn apply_required_since(
    inner_properties: &BTreeMap<String, &Value>,
    is_required: bool,
    is_top_level: bool,
    platform_version: &PlatformVersion,
) -> Result<Option<u32>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_required_since
    {
        None => Ok(None),
        Some(0) => v0::apply_required_since_v0(inner_properties, is_required, is_top_level),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_required_since version {version} is not supported"
        ))),
    }
}
