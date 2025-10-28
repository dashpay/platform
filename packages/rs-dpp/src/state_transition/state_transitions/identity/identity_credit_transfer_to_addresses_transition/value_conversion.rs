use std::collections::BTreeMap;

use platform_value::Value;

use crate::ProtocolError;

use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use crate::state_transition::state_transitions::identity_credit_transfer_to_addresses_transition::fields::*;
use crate::state_transition::StateTransitionValueConvert;

use crate::serialization::ValueConvertible;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl ValueConvertible<'_> for IdentityCreditTransferToAddressesTransition {}

impl StateTransitionValueConvert<'_> for IdentityCreditTransferToAddressesTransition {
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                let mut value = transition.to_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_canonical_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                let mut value = transition.to_canonical_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                let mut value = transition.to_canonical_cleaned_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                let mut value = transition.to_cleaned_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn from_object(
        mut raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let version: FeatureVersion = raw_object
            .remove_optional_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or({
                platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .contract_create_state_transition
                    .default_current_version
            });

        match version {
            0 => Ok(IdentityCreditTransferToAddressesTransitionV0::from_object(
                raw_object,
                platform_version,
            )?
            .into()),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditTransferToAddressesTransition version {n}"
            ))),
        }
    }

    fn from_value_map(
        mut raw_value_map: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let version: FeatureVersion = raw_value_map
            .remove_optional_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or({
                platform_version
                    .dpp
                    .state_transition_serialization_versions
                    .contract_create_state_transition
                    .default_current_version
            });

        match version {
            0 => Ok(
                IdentityCreditTransferToAddressesTransitionV0::from_value_map(
                    raw_value_map,
                    platform_version,
                )?
                .into(),
            ),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditTransferToAddressesTransition version {n}"
            ))),
        }
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        let version: u8 = value
            .get_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;

        match version {
            0 => IdentityCreditTransferToAddressesTransitionV0::clean_value(value),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditTransferToAddressesTransition version {n}"
            ))),
        }
    }
}
