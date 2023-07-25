use anyhow::Context;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::collections::BTreeMap;

use platform_value::btreemap_extensions::{
    BTreeValueMapReplacementPathHelper, BTreeValueRemoveFromMapHelper,
};
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::documents_batch_transition::fields::property_names::TRANSITIONS;
use crate::state_transition::documents_batch_transition::fields::*;
use crate::state_transition::documents_batch_transition::{
    document_base_transition, document_create_transition, DocumentsBatchTransitionV0,
};
use crate::state_transition::{StateTransitionFieldTypes, StateTransitionValueConvert};
use bincode::{config, Decode, Encode};
//
// impl StateTransitionValueConvert for DocumentsBatchTransitionV0 {
//     fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         Ok(self.to_value_map(skip_signature)?.into())
//     }
//
//     fn to_value_map(&self, skip_signature: bool) -> Result<BTreeMap<String, Value>, ProtocolError> {
//         let mut map = BTreeMap::new();
//         map.insert(
//             property_names::PROTOCOL_VERSION.to_string(),
//             Value::U16(self.feature_version),
//         );
//         map.insert(
//             property_names::TRANSITION_TYPE.to_string(),
//             Value::U8(self.transition_type as u8),
//         );
//         map.insert(
//             property_names::OWNER_ID.to_string(),
//             Value::Identifier(self.owner_id.to_buffer()),
//         );
//
//         if !skip_signature {
//             if let Some(signature) = self.signature.as_ref() {
//                 map.insert(
//                     property_names::SIGNATURE.to_string(),
//                     Value::Bytes(signature.to_vec()),
//                 );
//             }
//             if let Some(signature_key_id) = self.signature_public_key_id {
//                 map.insert(
//                     property_names::SIGNATURE.to_string(),
//                     Value::U32(signature_key_id),
//                 );
//             }
//         }
//         let mut transitions = vec![];
//         for transition in self.transitions.iter() {
//             transitions.push(transition.to_object()?)
//         }
//         map.insert(
//             property_names::TRANSITIONS.to_string(),
//             Value::Array(transitions),
//         );
//
//         Ok(map)
//     }
//
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn from_json_object(
//         json_value: JsonValue,
//         data_contracts: Vec<DataContract>,
//     ) -> Result<Self, ProtocolError> {
//         let mut json_value = json_value;
//
//         let maybe_signature = json_value.get_string(property_names::SIGNATURE).ok();
//         let signature = if let Some(signature) = maybe_signature {
//             Some(BinaryData(
//                 base64::decode(signature).context("signature exists but isn't valid base64")?,
//             ))
//         } else {
//             None
//         };
//
//         let mut batch_transitions = DocumentsBatchTransition {
//             feature_version: json_value
//                 .get_u64(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
//                 // js-dpp allows `protocolVersion` to be undefined
//                 .unwrap_or(LATEST_VERSION as u64) as u16,
//             signature,
//             signature_public_key_id: json_value
//                 .get_u64(property_names::SIGNATURE_PUBLIC_KEY_ID)
//                 .ok()
//                 .map(|v| v as KeyID),
//             owner_id: Identifier::from_string(
//                 json_value.get_string(property_names::OWNER_ID)?,
//                 Encoding::Base58,
//             )?,
//             ..Default::default()
//         };
//
//         let mut document_transitions: Vec<DocumentTransition> = vec![];
//         let maybe_transitions = json_value.remove(property_names::TRANSITIONS);
//         if let Ok(JsonValue::Array(json_transitions)) = maybe_transitions {
//             let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
//                 .into_iter()
//                 .map(|dc| (dc.id.as_bytes().to_vec(), dc))
//                 .collect();
//
//             for json_transition in json_transitions {
//                 let id = Identifier::from_string(
//                     json_transition.get_string(property_names::DATA_CONTRACT_ID)?,
//                     Encoding::Base58,
//                 )?;
//                 let data_contract =
//                     data_contracts_map
//                         .get(&id.as_bytes().to_vec())
//                         .ok_or_else(|| {
//                             anyhow!(
//                                 "Data Contract doesn't exists for Transition: {:?}",
//                                 json_transition
//                             )
//                         })?;
//                 let document_transition =
//                     DocumentTransition::from_json_object(json_transition, data_contract.clone())?;
//                 document_transitions.push(document_transition);
//             }
//         }
//
//         batch_transitions.transitions = document_transitions;
//         Ok(batch_transitions)
//     }
//
//     /// creates the instance of [`DocumentsBatchTransition`] from raw object
//     fn from_object_with_contracts(
//         raw_object: Value,
//         data_contracts: Vec<DataContract>,
//     ) -> Result<Self, ProtocolError> {
//         let map = raw_object
//             .into_btree_string_map()
//             .map_err(ProtocolError::ValueError)?;
//         Self::from_value_map(map, data_contracts)
//     }
//
//     /// creates the instance of [`DocumentsBatchTransition`] from a value map
//     fn from_value_map(
//         mut map: BTreeMap<String, Value>,
//         data_contracts: Vec<DataContract>,
//     ) -> Result<Self, ProtocolError> {
//         let mut batch_transitions = DocumentsBatchTransition {
//             feature_version: map
//                 .get_integer(property_names::PROTOCOL_VERSION)
//                 // js-dpp allows `protocolVersion` to be undefined
//                 .unwrap_or(LATEST_VERSION as u64) as u16,
//             signature: map
//                 .get_optional_binary_data(property_names::SIGNATURE)
//                 .map_err(ProtocolError::ValueError)?,
//             signature_public_key_id: map
//                 .get_optional_integer(property_names::SIGNATURE_PUBLIC_KEY_ID)
//                 .map_err(ProtocolError::ValueError)?,
//             owner_id: Identifier::from(
//                 map.get_hash256_bytes(property_names::OWNER_ID)
//                     .map_err(ProtocolError::ValueError)?,
//             ),
//             ..Default::default()
//         };
//
//         let mut document_transitions: Vec<DocumentTransition> = vec![];
//         let maybe_transitions = map.remove(property_names::TRANSITIONS);
//         if let Some(Value::Array(raw_transitions)) = maybe_transitions {
//             let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
//                 .into_iter()
//                 .map(|dc| (dc.id.as_bytes().to_vec(), dc))
//                 .collect();
//
//             for raw_transition in raw_transitions {
//                 let mut raw_transition_map = raw_transition
//                     .into_btree_string_map()
//                     .map_err(ProtocolError::ValueError)?;
//                 let data_contract_id = raw_transition_map
//                     .get_hash256_bytes(property_names::DATA_CONTRACT_ID)?;
//                 let document_type =
//                     raw_transition_map.get_str(property_names::DOCUMENT_TYPE)?;
//                 let data_contract = data_contracts_map
//                     .get(data_contract_id.as_slice())
//                     .ok_or_else(|| {
//                         anyhow!(
//                             "Data Contract doesn't exists for Transition: {:?}",
//                             raw_transition_map
//                         )
//                     })?;
//
//                 //Because we don't know how the json came in we need to sanitize it
//                 let (identifiers, binary_paths): (Vec<_>, Vec<_>) =
//                     data_contract.get_identifiers_and_binary_paths_owned(document_type)?;
//
//                 raw_transition_map
//                     .replace_at_paths(
//                         identifiers.into_iter().chain(
//                             document_base_transition::IDENTIFIER_FIELDS
//                                 .iter()
//                                 .map(|a| a.to_string()),
//                         ),
//                         ReplacementType::Identifier,
//                     )
//                     .map_err(ProtocolError::ValueError)?;
//                 raw_transition_map
//                     .replace_at_paths(
//                         binary_paths.into_iter().chain(
//                             document_create_transition::BINARY_FIELDS
//                                 .iter()
//                                 .map(|a| a.to_string()),
//                         ),
//                         ReplacementType::BinaryBytes,
//                     )
//                     .map_err(ProtocolError::ValueError)?;
//
//                 let document_transition =
//                     DocumentTransition::from_value_map(raw_transition_map, data_contract.clone())?;
//                 document_transitions.push(document_transition);
//             }
//         }
//
//         batch_transitions.transitions = document_transitions;
//         Ok(batch_transitions)
//     }
//
//     fn from_object(mut raw_object: Value) -> Result<DataContractCreateTransitionV0, ProtocolError> {
//         Ok(DataContractCreateTransitionV0 {
//             signature: raw_object
//                 .remove_optional_binary_data(SIGNATURE)
//                 .map_err(ProtocolError::ValueError)?
//                 .unwrap_or_default(),
//             signature_public_key_id: raw_object
//                 .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
//                 .map_err(ProtocolError::ValueError)?
//                 .unwrap_or_default(),
//             entropy: raw_object
//                 .remove_optional_bytes_32(ENTROPY)
//                 .map_err(ProtocolError::ValueError)?
//                 .unwrap_or_default(),
//             data_contract: DataContract::from_object(raw_object.remove(DATA_CONTRACT).map_err(
//                 |_| {
//                     ProtocolError::DecodingError(
//                         "data contract missing on state transition".to_string(),
//                     )
//                 },
//             )?)?,
//             ..Default::default()
//         })
//     }
//
//     fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
//         value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
//         value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
//         Ok(())
//     }
//
//     fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         let mut object: Value = platform_value::to_value(self)?;
//         if skip_signature {
//             for path in Self::signature_property_paths() {
//                 let _ = object.remove_values_matching_path(path);
//             }
//         }
//         let mut transitions = vec![];
//         for transition in self.transitions.iter() {
//             transitions.push(transition.to_object()?)
//         }
//         object.insert(String::from(TRANSITIONS), Value::Array(transitions))?;
//
//         Ok(object)
//     }
//
//     fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         let mut object: Value = platform_value::to_value(self)?;
//         if skip_signature {
//             for path in Self::signature_property_paths() {
//                 let _ = object.remove_values_matching_path(path);
//             }
//         }
//         let mut transitions = vec![];
//         for transition in self.transitions.iter() {
//             transitions.push(transition.to_cleaned_object()?)
//         }
//         object.insert(String::from(TRANSITIONS), Value::Array(transitions))?;
//
//         Ok(object)
//     }
// }
