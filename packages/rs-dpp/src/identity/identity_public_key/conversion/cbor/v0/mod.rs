// use crate::version::PlatformVersion;
// use crate::ProtocolError;
// use ciborium::Value as CborValue;
//
// pub trait IdentityPublicKeyCborConversionMethodsV0 {
//     fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError>;
//     fn from_cbor_value(
//         cbor_value: &CborValue,
//         platform_version: &PlatformVersion,
//     ) -> Result<Self, ProtocolError>
//     where
//         Self: Sized;
//     fn to_cbor_value(&self) -> CborValue;
// }
