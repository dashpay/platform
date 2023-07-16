use crate::version::PlatformVersion;
use crate::ProtocolError;

pub trait IdentityCborConversionMethodsV0 {
    /// Converts the identity to a cbor buffer
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError>;
    fn from_cbor(
        identity_cbor: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
