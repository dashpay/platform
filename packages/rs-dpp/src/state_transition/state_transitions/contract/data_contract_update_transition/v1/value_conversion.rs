use crate::state_transition::data_contract_update_transition::fields::*;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransitionV1, BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS,
};
use crate::state_transition::state_transitions::common_fields::property_names::{
    IDENTITY_CONTRACT_NONCE, USER_FEE_INCREASE,
};
use crate::state_transition::StateTransitionFieldTypes;
use crate::state_transition::StateTransitionValueConvert;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{IntegerReplacementType, ReplacementType, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

// Field names for V1 transition
const UPDATE_CONTRACT_SYSTEM_VERSION: &str = "updateContractSystemVersion";
const ID: &str = "id";
const OWNER_ID: &str = "ownerId";
const REVISION: &str = "revision";
const UPDATED_SCHEMA_DEFS: &str = "updatedSchemaDefs";
const NEW_SCHEMA_DEFS: &str = "newSchemaDefs";
const UPDATED_DOCUMENT_SCHEMAS: &str = "updatedDocumentSchemas";
const NEW_DOCUMENT_SCHEMAS: &str = "newDocumentSchemas";
const NEW_GROUPS: &str = "newGroups";
const NEW_TOKENS: &str = "newTokens";
const REMOVE_KEYWORDS: &str = "removeKeywords";
const ADD_KEYWORDS: &str = "addKeywords";
const UPDATE_DESCRIPTION: &str = "updateDescription";

impl StateTransitionValueConvert<'_> for DataContractUpdateTransitionV1 {
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            Self::signature_property_paths()
                .into_iter()
                .try_for_each(|path| {
                    object
                        .remove_values_matching_path(path)
                        .map_err(ProtocolError::ValueError)
                        .map(|_| ())
                })?;
        }
        Ok(object)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            Self::signature_property_paths()
                .into_iter()
                .try_for_each(|path| {
                    object
                        .remove_values_matching_path(path)
                        .map_err(ProtocolError::ValueError)
                        .map(|_| ())
                })?;
        }
        Ok(object)
    }

    fn from_object(
        mut raw_object: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractUpdateTransitionV1 {
            update_contract_system_version: raw_object
                .get_optional_integer(UPDATE_CONTRACT_SYSTEM_VERSION)
                .map_err(ProtocolError::ValueError)?,
            id: raw_object
                .remove_identifier(ID)
                .map_err(ProtocolError::ValueError)?,
            owner_id: raw_object
                .remove_identifier(OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            revision: raw_object
                .get_integer(REVISION)
                .map_err(ProtocolError::ValueError)?,
            updated_schema_defs: raw_object
                .remove(UPDATED_SCHEMA_DEFS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_schema_defs: raw_object
                .remove(NEW_SCHEMA_DEFS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            updated_document_schemas: raw_object
                .remove(UPDATED_DOCUMENT_SCHEMAS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_document_schemas: raw_object
                .remove(NEW_DOCUMENT_SCHEMAS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_groups: raw_object
                .remove(NEW_GROUPS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_tokens: raw_object
                .remove(NEW_TOKENS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            remove_keywords: raw_object
                .remove(REMOVE_KEYWORDS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            add_keywords: raw_object
                .remove(ADD_KEYWORDS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            update_description: raw_object
                .remove(UPDATE_DESCRIPTION)
                .ok()
                .map(platform_value::from_value)
                .transpose()?,
            identity_contract_nonce: raw_object.remove_integer(IDENTITY_CONTRACT_NONCE).map_err(
                |_| {
                    ProtocolError::DecodingError(
                        "identity contract nonce missing on data contract update state transition"
                            .to_string(),
                    )
                },
            )?,
            user_fee_increase: raw_object
                .get_optional_integer(USER_FEE_INCREASE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature: raw_object
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
        })
    }

    fn from_value_map(
        mut raw_value_map: BTreeMap<String, Value>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractUpdateTransitionV1 {
            update_contract_system_version: raw_value_map
                .remove_optional_integer(UPDATE_CONTRACT_SYSTEM_VERSION)
                .map_err(ProtocolError::ValueError)?,
            id: raw_value_map
                .remove_identifier(ID)
                .map_err(ProtocolError::ValueError)?,
            owner_id: raw_value_map
                .remove_identifier(OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            revision: raw_value_map
                .remove_integer(REVISION)
                .map_err(ProtocolError::ValueError)?,
            updated_schema_defs: raw_value_map
                .remove(UPDATED_SCHEMA_DEFS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_schema_defs: raw_value_map
                .remove(NEW_SCHEMA_DEFS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            updated_document_schemas: raw_value_map
                .remove(UPDATED_DOCUMENT_SCHEMAS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_document_schemas: raw_value_map
                .remove(NEW_DOCUMENT_SCHEMAS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_groups: raw_value_map
                .remove(NEW_GROUPS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            new_tokens: raw_value_map
                .remove(NEW_TOKENS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            remove_keywords: raw_value_map
                .remove(REMOVE_KEYWORDS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            add_keywords: raw_value_map
                .remove(ADD_KEYWORDS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            update_description: raw_value_map
                .remove(UPDATE_DESCRIPTION)
                .map(platform_value::from_value)
                .transpose()?,
            identity_contract_nonce: raw_value_map
                .remove_integer(IDENTITY_CONTRACT_NONCE)
                .map_err(|_| {
                    ProtocolError::DecodingError(
                        "identity contract nonce missing on data contract update state transition"
                            .to_string(),
                    )
                })?,
            user_fee_increase: raw_value_map
                .remove_optional_integer(USER_FEE_INCREASE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_value_map
                .remove_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature: raw_value_map
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
        })
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }
}
