use platform_value::{IntegerReplacementType, ReplacementType, Value};

use crate::{state_transition::StateTransitionFieldTypes, ProtocolError};

use crate::state_transition::identity_update_transition::fields::*;
use crate::state_transition::identity_update_transition::v0::{
    remove_integer_list_or_default, IdentityUpdateTransitionV0,
};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionValueConvert;

use crate::state_transition::state_transitions::common_fields::property_names::{
    NONCE, USER_FEE_INCREASE,
};
use platform_version::version::PlatformVersion;

impl<'a> StateTransitionValueConvert<'a> for IdentityUpdateTransitionV0 {
    fn from_object(
        mut raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
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
        let nonce = raw_object
            .get_integer(NONCE)
            .map_err(ProtocolError::ValueError)?;
        let user_fee_increase = raw_object
            .get_optional_integer(USER_FEE_INCREASE)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();
        let add_public_keys = raw_object
            .remove_optional_array(property_names::ADD_PUBLIC_KEYS)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default()
            .into_iter()
            .map(|value| IdentityPublicKeyInCreation::from_object(value, platform_version))
            .collect::<Result<Vec<_>, ProtocolError>>()?;
        let disable_public_keys =
            remove_integer_list_or_default(&mut raw_object, property_names::DISABLE_PUBLIC_KEYS)?;

        Ok(IdentityUpdateTransitionV0 {
            signature,
            signature_public_key_id,
            identity_id,
            revision,
            nonce,
            add_public_keys,
            disable_public_keys,
            user_fee_increase,
        })
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
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
            add_public_keys.push(key.to_object(skip_signature)?);
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
                add_public_keys.push(key.to_cleaned_object(skip_signature)?);
            }

            value.insert(
                property_names::ADD_PUBLIC_KEYS.to_owned(),
                Value::Array(add_public_keys),
            )?;
        }

        value.remove_optional_value_if_empty_array(property_names::ADD_PUBLIC_KEYS)?;

        value.remove_optional_value_if_empty_array(property_names::DISABLE_PUBLIC_KEYS)?;

        Ok(value)
    }

    // Override to_canonical_cleaned_object to manage add_public_keys individually
    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        self.to_cleaned_object(skip_signature)
    }
}
