use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use itertools::Itertools;
use platform_value::{Identifier, Value};

// If another document type (like V1) ever were to exist we would need to implement serialize_value_for_key_v0 again

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn deserialize_value_for_key_v0(
        &self,
        key: &str,
        value: &[u8],
    ) -> Result<Value, ProtocolError> {
        match key {
            "$ownerId" | "$id" => {
                let bytes = Identifier::from_bytes(value)?;
                Ok(Value::Identifier(bytes.to_buffer()))
            }
            "$createdAt" | "$updatedAt" | "$transferredAt" => Ok(Value::U64(
                DocumentPropertyType::decode_date_timestamp(value).ok_or(
                    ProtocolError::DataContractError(DataContractError::FieldRequirementUnmet(
                        "value must be 8 bytes long".to_string(),
                    )),
                )?,
            )),
            "$createdAtBlockHeight" | "$updatedAtBlockHeight" | "$transferredAtBlockHeight" => {
                Ok(Value::U64(DocumentPropertyType::decode_u64(value).ok_or(
                    ProtocolError::DataContractError(DataContractError::FieldRequirementUnmet(
                        "value must be 8 bytes long".to_string(),
                    )),
                )?))
            }
            "$createdAtCoreBlockHeight"
            | "$updatedAtCoreBlockHeight"
            | "$transferredAtCoreBlockHeight" => {
                Ok(Value::U32(DocumentPropertyType::decode_u32(value).ok_or(
                    ProtocolError::DataContractError(DataContractError::FieldRequirementUnmet(
                        "value must be 4 bytes long".to_string(),
                    )),
                )?))
            }
            _ => {
                let property = self.flattened_properties.get(key).ok_or_else(|| {
                    DataContractError::DocumentTypeFieldNotFound(format!("expected contract to have field: {key}, contract fields are {} on document type {}", self.flattened_properties.keys().join(" | "), self.name))
                })?;
                property.property_type.decode_value_for_tree_keys(value)
            }
        }
    }
}
