use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::ProtocolError;
use ciborium::Value as CborValue;
use std::collections::BTreeMap;
use std::format;

pub trait IdentityCborConversionMethodsV0 {
    /// Converts the identity to a cbor buffer
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError>;
    fn from_cbor(identity_cbor: &[u8]) -> Result<Self, ProtocolError>;
}
