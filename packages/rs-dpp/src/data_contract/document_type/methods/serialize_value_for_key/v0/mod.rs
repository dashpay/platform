use crate::data_contract::document_type::v0::{DocumentTypeV0, DEFAULT_HASH_SIZE, MAX_INDEX_SIZE};
use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use itertools::Itertools;
use platform_value::Value;

// If another document type (like V1) ever were to exist we would need to implement serialize_value_for_key_v0 again

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn serialize_value_for_key_v0(
        &self,
        key: &str,
        value: &Value,
    ) -> Result<Vec<u8>, ProtocolError> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = value
                    .to_identifier_bytes()
                    .map_err(ProtocolError::ValueError)?;
                if bytes.len() != DEFAULT_HASH_SIZE {
                    Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet(
                            "expected system value to be 32 bytes long".to_string(),
                        ),
                    ))
                } else {
                    Ok(bytes)
                }
            }
            "$createdAt" | "$updatedAt" => DocumentPropertyType::encode_date_timestamp(
                value.to_integer().map_err(ProtocolError::ValueError)?,
            ),
            _ => {
                let property = self.flattened_properties.get(key).ok_or_else(|| {
                    DataContractError::DocumentTypeFieldNotFound(format!("expected contract to have field: {key}, contract fields are {} on document type {}", self.flattened_properties.keys().join(" | "), self.name))
                })?;
                let bytes = property.property_type.encode_value_for_tree_keys(value)?;
                if bytes.len() > MAX_INDEX_SIZE {
                    Err(ProtocolError::DataContractError(
                        DataContractError::FieldRequirementUnmet(
                            "value must be less than 256 bytes long".to_string(),
                        ),
                    ))
                } else {
                    Ok(bytes)
                }
            }
        }
    }
}
