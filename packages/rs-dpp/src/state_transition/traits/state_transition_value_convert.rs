use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::state_transition_helpers;
use crate::ProtocolError;
use platform_value::{Value, ValueMapHelper};
use serde::Serialize;
use std::collections::BTreeMap;

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionValueConvert: Serialize {
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
    fn from_object(raw_object: Value) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn from_value_map(
        raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn clean_value(value: &mut Value) -> Result<(), ProtocolError>;
}

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait ValueConvert: Serialize {
    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_canonical_object(&self) -> Result<Value, ProtocolError> {
        let mut object = self.to_object()?;
        object.as_map_mut_ref().unwrap().sort_by_keys();
        Ok(object)
    }

    /// Returns the [`platform_value::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_canonical_cleaned_object(&self) -> Result<Value, ProtocolError> {
        let mut object = self.to_cleaned_object()?;
        object.as_map_mut_ref().unwrap().sort_by_keys();
        Ok(object)
    }

    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        self.to_object()
    }
    fn from_object(raw_object: Value) -> Result<Self, ProtocolError>
        where
            Self: Sized;
    fn from_value_map(
        raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<Self, ProtocolError>
        where
            Self: Sized;
    fn clean_value(value: &mut Value) -> Result<(), ProtocolError>;
}
