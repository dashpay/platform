use crate::identity::state_transition::properties::PROPERTY_PROTOCOL_VERSION;
use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::util::cbor_serializer;
use crate::ProtocolError;
use serde::Serialize;

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionCborConvert: Serialize + Signable + PlatformSerializable {
    // Returns the cbor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut value = self.to_canonical_cleaned_object(skip_signature)?;
        let protocol_version = value.remove_integer(PROPERTY_PROTOCOL_VERSION)?;

        cbor_serializer::serializable_value_to_cbor(&value, Some(protocol_version))
    }
}
