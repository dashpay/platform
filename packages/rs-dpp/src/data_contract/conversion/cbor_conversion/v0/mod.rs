use std::collections::BTreeMap;
use std::format;
use ciborium::Value as CborValue;
use serde_json::Value;
use platform_value::{Identifier, Value};
use crate::data_contract::{DataContract, property_names};
use crate::ProtocolError;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::{cbor_serializer, deserializer};
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::version::PlatformVersion;

pub trait DataContractCborConversionMethodsV0 {
    fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>;
    fn from_cbor(
        cbor_bytes: impl AsRef<[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>;
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError>;
    /// Returns Data Contract as a Buffer
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError>;
    fn to_cbor_canonical_map(&self) -> Result<CborCanonicalMap, ProtocolError>;
}