use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueRemoveFromMapHelper, BTreeValueRemoveInnerValueFromMapHelper,
};
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::identity_update_transition::fields::*;
use crate::state_transition::identity_update_transition::v0::{
    get_list, remove_integer_list_or_default, IdentityUpdateTransitionV0,
};
use crate::state_transition::StateTransitionValueConvert;
use bincode::{config, Decode, Encode};

impl StateTransitionValueConvert for IdentityUpdateTransitionV0 {
    fn from_object(mut raw_object: Value) -> Result<IdentityUpdateTransitionV0, ProtocolError> {
        let signature = raw_object
            .get_binary_data(SIGNATURE)
            .map_err(ProtocolError::ValueError)?;
        let signature_public_key_id = raw_object
            .get_integer(SIGNATURE_PUBLIC_KEY_ID)
            .map_err(ProtocolError::ValueError)?;
        let identity_id = raw_object
            .get_identifier(IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?;

        let revision = raw_object
            .get_integer(REVISION)
            .map_err(ProtocolError::ValueError)?;
        let add_public_keys = get_list(&mut raw_object, property_names::ADD_PUBLIC_KEYS)?;
        let disable_public_keys =
            remove_integer_list_or_default(&mut raw_object, property_names::DISABLE_PUBLIC_KEYS)?;
        let public_keys_disabled_at = raw_object
            .remove_optional_integer(property_names::PUBLIC_KEYS_DISABLED_AT)
            .map_err(ProtocolError::ValueError)?;

        Ok(IdentityUpdateTransitionV0 {
            signature,
            signature_public_key_id,
            identity_id,
            revision,
            add_public_keys,
            disable_public_keys,
            public_keys_disabled_at,
            transition_type: StateTransitionType::IdentityUpdate,
        })
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }

    fn from_value_map(
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractCreateTransitionV0, ProtocolError> {
        todo()
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;
        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        let mut add_public_keys: Vec<Value> = vec![];
        for key in self.add_public_keys.iter() {
            add_public_keys.push(key.to_raw_object(skip_signature)?);
        }

        if !add_public_keys.is_empty() {
            value.insert_at_end(
                property_names::ADD_PUBLIC_KEYS.to_owned(),
                Value::Array(add_public_keys),
            )?;
        }

        Ok(value)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        if !self.add_public_keys.is_empty() {
            let mut add_public_keys: Vec<Value> = vec![];
            for key in self.add_public_keys.iter() {
                add_public_keys.push(key.to_raw_cleaned_object(skip_signature)?);
            }

            value.insert(
                property_names::ADD_PUBLIC_KEYS.to_owned(),
                Value::Array(add_public_keys),
            )?;
        }

        value.remove_optional_value_if_empty_array(property_names::ADD_PUBLIC_KEYS)?;

        value.remove_optional_value_if_empty_array(property_names::DISABLE_PUBLIC_KEYS)?;

        value.remove_optional_value_if_null(property_names::PUBLIC_KEYS_DISABLED_AT)?;

        Ok(value)
    }

    // Override to_canonical_cleaned_object to manage add_public_keys individually
    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        self.to_cleaned_object(skip_signature)
    }
}
