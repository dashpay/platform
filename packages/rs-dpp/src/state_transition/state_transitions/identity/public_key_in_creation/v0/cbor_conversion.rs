use std::collections::BTreeMap;
use platform_value::Value;
use crate::ProtocolError;
use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::{CborConvert, FeatureVersioned, ValueConvert};
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::version::FeatureVersion;

impl Signable for IdentityPublicKeyInCreationV0 {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        todo!()
    }
}

impl PlatformSerializable for IdentityPublicKeyInCreationV0 {
    fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        todo!()
    }
}

impl CborConvert for IdentityPublicKeyInCreationV0 {

}