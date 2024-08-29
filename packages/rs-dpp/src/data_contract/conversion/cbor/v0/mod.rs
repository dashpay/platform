use crate::util::cbor_value::CborCanonicalMap;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Identifier;

pub trait DataContractCborConversionMethodsV0 {
    fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn from_cbor(
        cbor_bytes: impl AsRef<[u8]>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn to_cbor(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError>;
    fn to_cbor_canonical_map(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<CborCanonicalMap, ProtocolError>;
}
