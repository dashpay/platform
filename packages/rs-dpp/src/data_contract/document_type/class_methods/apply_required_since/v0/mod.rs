use std::collections::BTreeMap;

use platform_value::Value;

use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;

/// Generation 0 parse rules: the keyword is admitted only on a top-level
/// property listed in `required`, and its value is a contract version of at
/// least 1 fitting in a u32.
pub(super) fn apply_required_since_v0(
    inner_properties: &BTreeMap<String, &Value>,
    is_required: bool,
    is_top_level: bool,
) -> Result<Option<u32>, DataContractError> {
    let Some(required_since_value) = inner_properties.get(property_names::REQUIRED_SINCE) else {
        return Ok(None);
    };

    if !is_top_level {
        return Err(DataContractError::InvalidContractStructure(
            "requiredSince is only allowed on top-level properties".to_string(),
        ));
    }

    if !is_required {
        return Err(DataContractError::InvalidContractStructure(
            "requiredSince is only allowed on properties listed in required".to_string(),
        ));
    }

    let required_since: u32 = required_since_value
        .to_integer()
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

    if required_since == 0 {
        return Err(DataContractError::InvalidContractStructure(
            "requiredSince must be a contract version of at least 1".to_string(),
        ));
    }

    Ok(Some(required_since))
}
