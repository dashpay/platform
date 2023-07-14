use std::collections::BTreeMap;
use std::format;
use ciborium::{Value as CborValue};
use crate::ProtocolError;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;

pub trait IdentityCborConversionMethodsV0 {
    /// Converts the identity to a cbor buffer
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError>;
    fn from_cbor(identity_cbor: &[u8]) -> Result<Self, ProtocolError>;
}