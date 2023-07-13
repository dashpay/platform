use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::util::cbor_serializer;
use crate::ProtocolError;
use ciborium::{Value as CborValue, Value};

pub trait IdentityPublicKeyCborConversionMethodsV0 {
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError>;
    fn from_cbor_value(cbor_value: &CborValue) -> Result<Self, ProtocolError>;
    fn to_cbor_value(&self) -> CborValue;
}
