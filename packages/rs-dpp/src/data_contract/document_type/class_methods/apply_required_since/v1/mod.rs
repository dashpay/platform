use std::collections::BTreeMap;

use platform_value::Value;

use crate::data_contract::document_type::property_names;
use crate::data_contract::errors::DataContractError;

/// Generation 1 parse rules (protocol version 15): generation 0 at every depth. The keyword is
/// admitted on a property listed as required, a top-level one in the type's `required` or an
/// object's member in that object's `required` or the `required` beside its `$ref`, and its
/// value is a contract version of at least 1 fitting in a u32.
///
/// A member with the keyword is stored like an optional member at every contract version, with
/// a presence byte (`DocumentProperty::always_required`), so documents written before it was
/// required still decode; the document validator requires it of every document written from
/// then on, as its object's `required` names it.
pub(super) fn apply_required_since_v1(
    inner_properties: &BTreeMap<String, &Value>,
    is_required: bool,
) -> Result<Option<u32>, DataContractError> {
    let Some(required_since_value) = inner_properties.get(property_names::REQUIRED_SINCE) else {
        return Ok(None);
    };

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
