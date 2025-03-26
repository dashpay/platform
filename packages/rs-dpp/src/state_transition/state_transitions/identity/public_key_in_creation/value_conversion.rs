use crate::serialization::ValueConvertible;
use crate::state_transition::batch_transition::fields::property_names::STATE_TRANSITION_PROTOCOL_VERSION;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionValueConvert;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;
use platform_version::version::{FeatureVersion, PlatformVersion};
use std::collections::BTreeMap;

impl ValueConvertible<'_> for IdentityPublicKeyInCreation {}

impl StateTransitionValueConvert<'_> for IdentityPublicKeyInCreation {
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKeyInCreation::V0(public_key) => {
                let mut value = public_key.to_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_canonical_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKeyInCreation::V0(public_key) => {
                let mut value = public_key.to_canonical_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKeyInCreation::V0(public_key) => {
                let mut value = public_key.to_canonical_cleaned_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityPublicKeyInCreation::V0(public_key) => {
                let mut value = public_key.to_cleaned_object(skip_signature)?;
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
            0 => Ok(
                IdentityPublicKeyInCreationV0::from_object(raw_object, platform_version)?.into(),
            ),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityPublicKeyInCreation version {n}"
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
                    .identity_public_key_in_creation
                    .default_current_version
            });

        match version {
            0 => Ok(IdentityPublicKeyInCreationV0::from_value_map(
                raw_value_map,
                platform_version,
            )?
            .into()),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityPublicKeyInCreation version {n}"
            ))),
        }
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        let version: u8 = value
            .get_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;

        match version {
            0 => IdentityPublicKeyInCreationV0::clean_value(value),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityPublicKeyInCreation version {n}"
            ))),
        }
    }
}
