use std::collections::BTreeMap;

use platform_value::{IntegerReplacementType, ReplacementType, Value};

use crate::{prelude::Identifier, state_transition::StateTransitionFieldTypes, ProtocolError};

use crate::address_funds::{AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::KeyOfTypeNonce;

use crate::state_transition::identity_topup_from_addresses_transition::fields::*;
use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use crate::state_transition::StateTransitionValueConvert;

use platform_version::version::PlatformVersion;

impl StateTransitionValueConvert<'_> for IdentityTopUpFromAddressesTransitionV0 {
    fn from_object(
        raw_object: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let identity_id = Identifier::from(
            raw_object
                .get_hash256(IDENTITY_ID)
                .map_err(ProtocolError::ValueError)?,
        );

        let inputs: BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)> =
            if let Some(inputs_value) = raw_object.get_optional_value(INPUTS).map_err(ProtocolError::ValueError)? {
                platform_value::from_value(inputs_value.clone())?
            } else {
                BTreeMap::new()
            };

        let user_fee_increase = raw_object
            .get_optional_integer(USER_FEE_INCREASE)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();

        let input_witnesses: Vec<AddressWitness> =
            if let Some(witnesses_value) = raw_object.get_optional_value(INPUT_WITNESSES).map_err(ProtocolError::ValueError)? {
                platform_value::from_value(witnesses_value.clone())?
            } else {
                vec![]
            };

        Ok(IdentityTopUpFromAddressesTransitionV0 {
            inputs,
            identity_id,
            user_fee_increase,
            input_witnesses,
        })
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

        Ok(value)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
    }
}
