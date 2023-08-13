use crate::state_transition::{FeatureVersioned, StateTransitionValueConvert};
use crate::util::cbor_serializer;
use crate::ProtocolError;

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionCborConvert<'a>:
    StateTransitionValueConvert<'a> + FeatureVersioned
{
    // Returns the cbor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut value = self.to_canonical_cleaned_object(skip_signature)?;

        cbor_serializer::serializable_value_to_cbor(&value, Some(self.feature_version() as u32))
    }
}
