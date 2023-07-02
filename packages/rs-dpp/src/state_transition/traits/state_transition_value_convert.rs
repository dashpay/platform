use std::collections::BTreeMap;
use serde::Serialize;
use platform_value::{Value, ValueMapHelper};
use crate::ProtocolError;
use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::state_transition_helpers;

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionValueConvert: Serialize + Signable + PlatformSerializable {
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
    fn from_raw_object(
        raw_object: Value,
    ) -> Result<Self, ProtocolError>;
    fn from_value_map(
        raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<Self, ProtocolError>;
    fn clean_value(value: &mut Value) -> Result<(), ProtocolError>;
}
