// mod v0;
// use crate::identity::IdentityPublicKey;
// use crate::version::PlatformVersion;
// use crate::ProtocolError;
// use ciborium::Value as CborValue;
// pub use v0::*;
//
// impl IdentityPublicKeyCborConversionMethodsV0 for IdentityPublicKey {
//     fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
//         match self {
//             IdentityPublicKey::V0(v0) => v0.to_cbor_buffer(),
//         }
//     }
//
//     fn from_cbor_value(
//         cbor_value: &CborValue,
//         platform_version: &PlatformVersion,
//     ) -> Result<Self, ProtocolError> {
//         match platform_version
//             .dpp
//             .identity_versions
//             .identity_key_structure_version
//         {
//             0 => IdentityPublicKey::from_cbor_value(cbor_value, platform_version),
//             version => Err(ProtocolError::UnknownVersionMismatch {
//                 method: "IdentityPublicKey::from_cbor_value".to_string(),
//                 known_versions: vec![0],
//                 received: version,
//             }),
//         }
//     }
//
//     fn to_cbor_value(&self) -> CborValue {
//         match self {
//             IdentityPublicKey::V0(v0) => v0.to_cbor_value(),
//         }
//     }
// }
//
// impl Into<CborValue> for &IdentityPublicKey {
//     fn into(self) -> CborValue {
//         self.to_cbor_value()
//     }
// }
