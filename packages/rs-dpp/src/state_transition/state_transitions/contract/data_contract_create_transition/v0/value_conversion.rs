use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use crate::state_transition::{StateTransitionFieldTypes, StateTransitionValueConvert};
use crate::state_transition::data_contract_create_transition::{DataContractCreateTransition, DataContractCreateTransitionV0};
use crate::state_transition::data_contract_create_transition::fields::*;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

impl StateTransitionValueConvert for DataContractCreateTransitionV0 {
    fn from_object(mut raw_object: Value) -> Result<DataContractCreateTransitionV0, ProtocolError> {
        Ok(DataContractCreateTransitionV0 {
            signature: raw_object
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            entropy: raw_object
                .remove_optional_bytes_32(ENTROPY)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_object(raw_object.remove(DATA_CONTRACT).map_err(
                |_| {
                    ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    )
                },
            )?)?,
            ..Default::default()
        })
    }

    fn from_value_map(
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractCreateTransitionV0, ProtocolError> {
        Ok(DataContractCreateTransitionV0 {
            signature: raw_data_contract_create_transition
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_data_contract_create_transition
                .remove_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            entropy: raw_data_contract_create_transition
                .remove_optional_bytes_32(ENTROPY)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_object(
                raw_data_contract_create_transition
                    .remove(DATA_CONTRACT)
                    .ok_or(ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    ))?,
            )?,
            ..Default::default()
        })
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }

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
        object.insert(String::from(DATA_CONTRACT), self.data_contract.to_object()?)?;
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
        object.insert(
            String::from(DATA_CONTRACT),
            self.data_contract.to_cleaned_object()?,
        )?;
        Ok(object)
    }
}
