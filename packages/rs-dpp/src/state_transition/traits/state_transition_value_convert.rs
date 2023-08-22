use crate::state_transition::{state_transition_helpers, StateTransitionFieldTypes};
use crate::ProtocolError;
use platform_value::{Value, ValueMapHelper};
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionValueConvert<'a>:
    Serialize + Deserialize<'a> + StateTransitionFieldTypes
{
    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        state_transition_helpers::to_object(self, skip_signature_paths)
    }

    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_canonical_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        let mut object = state_transition_helpers::to_object(self, skip_signature_paths)?;

        object.as_map_mut_ref().unwrap().sort_by_keys();
        Ok(object)
    }

    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        let mut object = state_transition_helpers::to_cleaned_object(self, skip_signature_paths)?;

        object.as_map_mut_ref().unwrap().sort_by_keys();
        Ok(object)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        self.to_object(skip_signature)
    }
    fn from_object(
        raw_object: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(raw_object).map_err(ProtocolError::ValueError)
    }

    fn from_value_map(
        raw_value_map: BTreeMap<String, Value>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(Value::Map(
            raw_value_map
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        ))
        .map_err(ProtocolError::ValueError)
    }
    fn clean_value(_value: &mut Value) -> Result<(), ProtocolError> {
        Ok(())
    }
}
