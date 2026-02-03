use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{IntegerReplacementType, ReplacementType, Value};

use crate::ProtocolError;

use platform_version::version::PlatformVersion;
use crate::state_transition::{StateTransitionFieldTypes, StateTransitionValueConvert};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;
use crate::state_transition::data_contract_create_transition::fields::*;
use crate::state_transition::state_transitions::common_fields::property_names::USER_FEE_INCREASE;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

// Field names for V1 transition
const CONTRACT_SYSTEM_VERSION: &str = "contractSystemVersion";
const OWNER_ID: &str = "ownerId";
const CONFIG: &str = "config";
const SCHEMA_DEFS: &str = "schemaDefs";
const DOCUMENT_SCHEMAS: &str = "documentSchemas";
const GROUPS: &str = "groups";
const TOKENS: &str = "tokens";
const KEYWORDS: &str = "keywords";
const DESCRIPTION: &str = "description";

impl StateTransitionValueConvert<'_> for DataContractCreateTransitionV1 {
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
        Ok(DataContractCreateTransitionV1 {
            contract_system_version: raw_object
                .get_integer(CONTRACT_SYSTEM_VERSION)
                .map_err(ProtocolError::ValueError)?,
            owner_id: raw_object
                .remove_identifier(OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            config: platform_value::from_value(raw_object.remove(CONFIG).map_err(|_| {
                ProtocolError::DecodingError("config missing on state transition".to_string())
            })?)?,
            schema_defs: raw_object
                .remove(SCHEMA_DEFS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?,
            document_schemas: platform_value::from_value(
                raw_object.remove(DOCUMENT_SCHEMAS).map_err(|_| {
                    ProtocolError::DecodingError(
                        "document_schemas missing on state transition".to_string(),
                    )
                })?,
            )?,
            groups: raw_object
                .remove(GROUPS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            tokens: raw_object
                .remove(TOKENS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            keywords: raw_object
                .remove(KEYWORDS)
                .ok()
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            description: raw_object
                .remove(DESCRIPTION)
                .ok()
                .map(platform_value::from_value)
                .transpose()?,
            identity_nonce: raw_object
                .get_optional_integer(IDENTITY_NONCE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
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
        Ok(DataContractCreateTransitionV1 {
            contract_system_version: raw_value_map
                .remove_integer(CONTRACT_SYSTEM_VERSION)
                .map_err(ProtocolError::ValueError)?,
            owner_id: raw_value_map
                .remove_identifier(OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            config: platform_value::from_value(raw_value_map.remove(CONFIG).ok_or(
                ProtocolError::DecodingError("config missing on state transition".to_string()),
            )?)?,
            schema_defs: raw_value_map
                .remove(SCHEMA_DEFS)
                .map(platform_value::from_value)
                .transpose()?,
            document_schemas: platform_value::from_value(
                raw_value_map
                    .remove(DOCUMENT_SCHEMAS)
                    .ok_or(ProtocolError::DecodingError(
                        "document_schemas missing on state transition".to_string(),
                    ))?,
            )?,
            groups: raw_value_map
                .remove(GROUPS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            tokens: raw_value_map
                .remove(TOKENS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            keywords: raw_value_map
                .remove(KEYWORDS)
                .map(platform_value::from_value)
                .transpose()?
                .unwrap_or_default(),
            description: raw_value_map
                .remove(DESCRIPTION)
                .map(platform_value::from_value)
                .transpose()?,
            identity_nonce: raw_value_map
                .remove_optional_integer(IDENTITY_NONCE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
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
