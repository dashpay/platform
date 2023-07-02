use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::serialization_traits::PlatformSerializable;
use platform_serialization::PlatformSignable;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{Convertible, data_contract::DataContract, identity::KeyID, NonConsensusError, prelude::Identifier, ProtocolError, state_transition::{
    StateTransitionFieldTypes, StateTransitionLike,
    StateTransitionType,
}};

use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use crate::state_transition::abstract_state_transition::StateTransitionValueConvert;
use crate::state_transition::data_contract_create_transition::{DataContractCreateTransition, DataContractCreateTransitionV0};
use crate::state_transition::data_contract_create_transition::fields::*;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};


impl StateTransitionValueConvert for DataContractCreateTransitionV0 {
    fn from_raw_object(raw_object: Value) -> Result<Self, ProtocolError> {
        let mut state_transition = Self::default();

        let mut transition_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        if let Some(keys_value_array) = transition_map
            .remove_optional_inner_value_array::<Vec<_>>(PUBLIC_KEYS)
            .map_err(ProtocolError::ValueError)?
        {
            let keys = keys_value_array
                .into_iter()
                .map(|val| val.try_into().map_err(ProtocolError::ValueError))
                .collect::<Result<Vec<IdentityPublicKeyInCreation>, ProtocolError>>()?;
            state_transition.set_public_keys(keys);
        }

        if let Some(proof) = transition_map.get(property_names::ASSET_LOCK_PROOF) {
            state_transition.set_asset_lock_proof(AssetLockProof::try_from(proof)?)?;
        }

        if let Some(signature) =
            transition_map.get_optional_binary_data(property_names::SIGNATURE)?
        {
            state_transition.set_signature(signature);
        }

        state_transition.protocol_version =
            transition_map.get_integer(property_names::PROTOCOL_VERSION)?;

        Ok(state_transition)
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

        let mut public_keys: Vec<Value> = vec![];
        for key in self.public_keys.iter() {
            public_keys.push(key.to_raw_object(skip_signature)?);
        }

        value.insert(
            property_names::PUBLIC_KEYS.to_owned(),
            Value::Array(public_keys),
        )?;

        Ok(value)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        let mut public_keys: Vec<Value> = vec![];
        for key in self.public_keys.iter() {
            public_keys.push(key.to_raw_cleaned_object(skip_signature)?);
        }

        value.insert(
            property_names::PUBLIC_KEYS.to_owned(),
            Value::Array(public_keys),
        )?;

        Ok(value)
    }
}