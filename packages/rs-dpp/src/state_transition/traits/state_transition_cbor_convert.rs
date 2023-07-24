use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::util::cbor_serializer;
use crate::ProtocolError;
use serde::Serialize;
use crate::state_transition::{StateTransitionValueConvert, FeatureVersioned, ValueConvert};

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionCborConvert: Serialize + Signable + PlatformSerializable + StateTransitionValueConvert + FeatureVersioned {
    // Returns the cbor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut value = self.to_canonical_cleaned_object(skip_signature)?;

        cbor_serializer::serializable_value_to_cbor(&value, Some(self.feature_version() as u32))
    }
}

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait CborConvert: Serialize + Signable + PlatformSerializable + ValueConvert + FeatureVersioned {
    // Returns the cbor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut value = self.to_canonical_cleaned_object()?;

        cbor_serializer::serializable_value_to_cbor(&value, Some(self.feature_version() as u32))
    }
}

