// use crate::data_contract::DataContract;
// use crate::document::document_transition::document_base_transition::JsonValue;
// use crate::identity::KeyID;
// use crate::serialization::PlatformDeserializable;
// use crate::serialization::PlatformSerializable;
// use crate::serialization::Signable;
// use crate::state_transition::{
//     StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
// };
// use crate::{ ProtocolError};
// use bincode::{config, Decode, Encode};
// use derive_more::From;
// use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
// use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
// use platform_value::{BinaryData, Identifier, Value};
// use serde::de::{MapAccess, Visitor};
// use serde::ser::SerializeMap;
// use serde::{Deserialize, Deserializer, Serialize, Serializer};
// use std::collections::BTreeMap;
// use std::convert::TryInto;
// use std::fmt;
// use platform_versioning::PlatformVersioned;
// use crate::state_transition::data_contract_update_transition::{DataContractUpdateTransition, DataContractUpdateTransitionV0};
//
//
// impl Serialize for DataContractUpdateTransition {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//         where
//             S: Serializer,
//     {
//         let mut state = serializer.serialize_map(None)?;
//
//         match *self {
//             DataContractUpdateTransition::V0(ref v0) => {
//                 state.serialize_entry("type", &StateTransitionType::DataContractUpdate)?;
//                 state.serialize_entry("version", &0u16)?;
//                 state.serialize_entry("dataContract", &v0.data_contract)?;
//                 state.serialize_entry("signaturePublicKeyId", &v0.signature_public_key_id)?;
//                 state.serialize_entry("signature", &v0.signature)?;
//             }
//         }
//
//         state.end()
//     }
// }
// struct DataContractUpdateTransitionVisitor;
//
// impl<'de> Visitor<'de> for DataContractUpdateTransitionVisitor {
//     type Value = DataContractUpdateTransition;
//
//     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//         formatter.write_str("a map representing a DataContractUpdateTransition")
//     }
//
//     fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//         where
//             A: MapAccess<'de>,
//     {
//         let mut version: Option<u16> = None;
//         let mut data_contract: Option<DataContract> = None;
//         let mut signature_public_key_id: Option<KeyID> = None;
//         let mut signature: Option<BinaryData> = None;
//
//         while let Some(key) = map.next_key()? {
//             match key {
//                 "version" => {
//                     version = Some(map.next_value()?);
//                 }
//                 "dataContract" => {
//                     data_contract = Some(map.next_value()?);
//                 }
//                 "signaturePublicKeyId" => {
//                     signature_public_key_id = Some(map.next_value()?);
//                 }
//                 "signature" => {
//                     signature = Some(map.next_value()?);
//                 }
//                 _ => {}
//             }
//         }
//
//         let version = version.ok_or_else(|| serde::de::Error::missing_field("version"))?;
//         let data_contract =
//             data_contract.ok_or_else(|| serde::de::Error::missing_field("dataContract"))?;
//         let signature_public_key_id = signature_public_key_id
//             .ok_or_else(|| serde::de::Error::missing_field("signaturePublicKeyId"))?;
//         let signature = signature.ok_or_else(|| serde::de::Error::missing_field("signature"))?;
//
//         match version {
//             0 => Ok(DataContractUpdateTransition::V0(
//                 DataContractUpdateTransitionV0 {
//                     data_contract,
//                     signature_public_key_id,
//                     signature,
//                 },
//             )),
//             _ => Err(serde::de::Error::unknown_variant(
//                 &format!("{}", version),
//                 &[],
//             )),
//         }
//     }
// }
//
// impl<'de> Deserialize<'de> for DataContractUpdateTransition {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where
//             D: Deserializer<'de>,
//     {
//         deserializer.deserialize_map(DataContractUpdateTransitionVisitor)
//     }
// }
