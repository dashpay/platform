use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::DataContract;
use crate::state_transition::data_contract_update_transition::fields::*;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransitionV0, BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS,
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
use platform_version::TryIntoPlatformVersioned;
use std::collections::BTreeMap;

impl StateTransitionValueConvert<'_> for DataContractUpdateTransitionV0 {
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
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractUpdateTransitionV0 {
            identity_contract_nonce: raw_object.remove_integer(IDENTITY_CONTRACT_NONCE).map_err(
                |_| {
                    ProtocolError::DecodingError(
                        "identity contract nonce missing on data contract update state transition"
                            .to_string(),
                    )
                },
            )?,
            signature: raw_object
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_value(
                raw_object.remove(DATA_CONTRACT).map_err(|_| {
                    ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    )
                })?,
                true,
                platform_version,
            )?
            .try_into_platform_versioned(platform_version)?,
            user_fee_increase: raw_object
                .get_optional_integer(USER_FEE_INCREASE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
        })
    }

    fn from_value_map(
        mut raw_value_map: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractUpdateTransitionV0 {
            identity_contract_nonce: raw_value_map
                .remove_integer(IDENTITY_CONTRACT_NONCE)
                .map_err(|_| {
                    ProtocolError::DecodingError(
                        "identity contract nonce missing on data contract update state transition"
                            .to_string(),
                    )
                })?,
            signature: raw_value_map
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_value_map
                .remove_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_value(
                raw_value_map
                    .remove(DATA_CONTRACT)
                    .ok_or(ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    ))?,
                false,
                platform_version,
            )?
            .try_into_platform_versioned(platform_version)?,
            user_fee_increase: raw_value_map
                .remove_optional_integer(USER_FEE_INCREASE)
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
