use std::collections::BTreeMap;
use std::convert::TryFrom;

use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueRemoveInnerValueFromMapHelper,
};
use platform_value::{IntegerReplacementType, ReplacementType, Value};

use crate::{
    state_transition::{StateTransitionFieldTypes, StateTransitionLike},
    ProtocolError,
};

use crate::fee::Credits;
use crate::identity::KeyOfType;
use crate::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use crate::state_transition::identity_create_from_addresses_transition::fields::*;
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionValueConvert;

use platform_version::version::PlatformVersion;

impl StateTransitionValueConvert<'_> for IdentityCreateFromAddressesTransitionV0 {
    fn from_object(
        raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut state_transition = Self::default();

        let mut transition_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        // Parse public keys
        if let Some(keys_value_array) = transition_map
            .remove_optional_inner_value_array::<Vec<_>>(PUBLIC_KEYS)
            .map_err(ProtocolError::ValueError)?
        {
            let keys = keys_value_array
                .into_iter()
                .map(|val| IdentityPublicKeyInCreation::from_object(val, platform_version))
                .collect::<Result<Vec<IdentityPublicKeyInCreation>, ProtocolError>>()?;
            state_transition.set_public_keys(keys);
        }

        // Parse inputs
        if let Some(inputs_value) = transition_map.remove(INPUTS) {
            let inputs: Vec<KeyOfType> = platform_value::from_value(inputs_value)?;
            state_transition.set_inputs(inputs);
        }

        // Parse outputs
        if let Some(outputs_value) = transition_map.remove(OUTPUTS) {
            let outputs: BTreeMap<KeyOfType, Credits> = platform_value::from_value(outputs_value)?;
            state_transition.set_outputs(outputs);
        }

        // Parse user fee increase
        if let Some(user_fee_increase) = transition_map.get_u16(USER_FEE_INCREASE)? {
            state_transition.user_fee_increase = user_fee_increase;
        }

        // Parse input signatures
        if let Some(signatures_value) = transition_map
            .remove_optional_inner_value_array::<Vec<_>>(INPUT_SIGNATURES)
            .map_err(ProtocolError::ValueError)?
        {
            let signatures = signatures_value
                .into_iter()
                .map(|val| platform_value::from_value(val))
                .collect::<Result<Vec<_>, _>>()?;
            state_transition.input_signatures = signatures;
        }

        Ok(state_transition)
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }

    fn from_value_map(
        raw_value_map: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let value: Value = raw_value_map.into();
        Self::from_object(value, platform_version)
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
            public_keys.push(key.to_object(skip_signature)?);
        }

        value.insert(PUBLIC_KEYS.to_owned(), Value::Array(public_keys))?;

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
            public_keys.push(key.to_cleaned_object(skip_signature)?);
        }

        value.insert(PUBLIC_KEYS.to_owned(), Value::Array(public_keys))?;

        Ok(value)
    }
}
