use bincode::{Decode, Encode};

use std::convert::TryInto;

use derive_more::From;

use platform_value::Value;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::ProtocolError;
use crate::{identity::SecurityLevel, state_transition::StateTransitionFieldTypes};

pub use self::document_transition::{
    document_base_transition, document_create_transition,
    document_create_transition::DocumentCreateTransition, document_delete_transition,
    document_delete_transition::DocumentDeleteTransition, document_replace_transition,
    document_replace_transition::DocumentReplaceTransition,
};

use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::PlatformVersioned;

pub mod accessors;
pub mod document_transition;
pub mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_like;
mod v0;
#[cfg(feature = "validation")]
mod validation;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::state_transition::data_contract_update_transition::{
    SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
};

use crate::state_transition::documents_batch_transition::fields::property_names;

use crate::identity::state_transition::OptionallyAssetLockProved;
pub use v0::*;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.documents_batch_state_transition"
)]
pub enum DocumentsBatchTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(DocumentsBatchTransitionV0),
}

//
// impl Default for DocumentsBatchTransition {
//     fn default() -> Self {
//         match LATEST_PLATFORM_VERSION
//             .state_transitions
//             .documents_batch_state_transition
//             .default_current_version
//         {
//             0 => DocumentsBatchTransitionV0::default().into(),
//             _ => DocumentsBatchTransitionV0::default().into(), //for now
//         }
//     }
// }
//
// impl DocumentsBatchTransition {
//     #[cfg(feature = "state-transition-json-conversion")]
//     pub fn from_json_object(
//         json_value: JsonValue,
//         data_contracts: Vec<DataContract>,
//     ) -> Result<Self, ProtocolError> {
//         let mut json_value = json_value;
//
//         let maybe_signature = json_value.get_string(property_names::SIGNATURE).ok();
//         let signature = if let Some(signature) = maybe_signature {
//             Some(BinaryData(
//                 BASE64_STANDARD.decode(signature).context("signature exists but isn't valid base64")?,
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
//     pub fn from_object_with_contracts(
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
//     pub fn from_value_map(
//         mut map: BTreeMap<String, Value>,
//         data_contracts: Vec<DataContract>,
//     ) -> Result<Self, ProtocolError> {
//         let mut batch_transitions = DocumentsBatchTransition {
//             feature_version: map
//                 .get_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
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
//                 let data_contract_id =
//                     raw_transition_map.get_hash256_bytes(property_names::DATA_CONTRACT_ID)?;
//                 let document_type = raw_transition_map.get_str(property_names::DOCUMENT_TYPE)?;
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
//     pub fn transitions(&self) -> &Vec<DocumentTransition> {
//         &self.transitions
//     }
//
//     pub fn transitions_slice(&self) -> &[DocumentTransition] {
//         self.transitions.as_slice()
//     }
//
//     pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
//         value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
//         value.replace_integer_type_at_paths(U16_FIELDS, IntegerReplacementType::U16)?;
//         Ok(())
//     }
// }
//
//
// impl DocumentsBatchTransition {
//     fn to_value(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         Ok(self.to_value_map(skip_signature)?.into())
//     }
//
//     fn to_value_map(&self, skip_signature: bool) -> Result<BTreeMap<String, Value>, ProtocolError> {
//         let mut map = BTreeMap::new();
//         map.insert(
//             property_names::STATE_TRANSITION_PROTOCOL_VERSION.to_string(),
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
// }

impl StateTransitionFieldTypes for DocumentsBatchTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::OWNER_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }
    //
    // fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
    //     self.to_object(skip_signature)
    //         .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    // }
    //
    // fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
    //     let mut object: Value = platform_value::to_value(self)?;
    //     if skip_signature {
    //         for path in Self::signature_property_paths() {
    //             let _ = object.remove_values_matching_path(path);
    //         }
    //     }
    //     let mut transitions = vec![];
    //     for transition in self.transitions.iter() {
    //         transitions.push(transition.to_object()?)
    //     }
    //     object.insert(
    //         String::from(property_names::TRANSITIONS),
    //         Value::Array(transitions),
    //     )?;
    //
    //     Ok(object)
    // }
    //
    // #[cfg(feature = "state-transition-cbor-conversion")]
    // fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
    //     let mut result_buf = self.feature_version.encode_var_vec();
    //     let value: CborValue = self.to_object(skip_signature)?.try_into()?;
    //
    //     let map = CborValue::serialized(&value)
    //         .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
    //
    //     let mut canonical_map: CborCanonicalMap = map.try_into()?;
    //     canonical_map.remove(property_names::STATE_TRANSITION_PROTOCOL_VERSION);
    //
    //     // Replace binary fields individually for every transition using respective data contract
    //     if let Some(CborValue::Array(ref mut transitions)) =
    //         canonical_map.get_mut(&CborValue::Text(property_names::TRANSITIONS.to_string()))
    //     {
    //         for (i, cbor_transition) in transitions.iter_mut().enumerate() {
    //             let transition = self
    //                 .transitions
    //                 .get(i)
    //                 .context(format!("transition with index {} doesn't exist", i))?;
    //
    //             let (identifier_properties, binary_properties) = transition
    //                 .base()
    //                 .data_contract
    //                 .get_identifiers_and_binary_paths(
    //                     &self.transitions[i].base().document_type_name,
    //                 )?;
    //
    //             if transition.get_updated_at().is_none() {
    //                 cbor_transition.remove("$updatedAt");
    //             }
    //
    //             cbor_transition.replace_paths(
    //                 identifier_properties
    //                     .into_iter()
    //                     .chain(binary_properties)
    //                     .chain(document_base_transition::IDENTIFIER_FIELDS)
    //                     .chain(document_create_transition::BINARY_FIELDS),
    //                 FieldType::ArrayInt,
    //                 FieldType::Bytes,
    //             );
    //         }
    //     }
    //
    //     canonical_map.replace_paths(
    //         Self::binary_property_paths()
    //             .into_iter()
    //             .chain(Self::identifiers_property_paths()),
    //         FieldType::ArrayInt,
    //         FieldType::Bytes,
    //     );
    //
    //     if !skip_signature {
    //         if self.signature.is_none() {
    //             canonical_map.insert(property_names::SIGNATURE, CborValue::Null)
    //         }
    //         if self.signature_public_key_id.is_none() {
    //             canonical_map.insert(property_names::SIGNATURE_PUBLIC_KEY_ID, CborValue::Null)
    //         }
    //     }
    //
    //     canonical_map.sort_canonical();
    //
    //     let mut buffer = canonical_map
    //         .to_bytes()
    //         .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
    //     result_buf.append(&mut buffer);
    //
    //     Ok(result_buf)
    // }
    //
    // fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
    //     let mut object: Value = platform_value::to_value(self)?;
    //     if skip_signature {
    //         for path in Self::signature_property_paths() {
    //             let _ = object.remove_values_matching_path(path);
    //         }
    //     }
    //     let mut transitions = vec![];
    //     for transition in self.transitions.iter() {
    //         transitions.push(transition.to_cleaned_object()?)
    //     }
    //     object.insert(
    //         String::from(property_names::TRANSITIONS),
    //         Value::Array(transitions),
    //     )?;
    //
    //     Ok(object)
    // }
}

// TODO: Make a DocumentType method
pub fn get_security_level_requirement(v: &Value, default: SecurityLevel) -> SecurityLevel {
    let maybe_security_level: Option<u64> = v
        .get_optional_integer(property_names::SECURITY_LEVEL_REQUIREMENT)
        // TODO: Data Contract must already valid so there is no chance that this will fail
        .expect("document schema must be a map");

    match maybe_security_level {
        Some(some_level) => (some_level as u8).try_into().unwrap_or(default),
        None => default,
    }
}
//
// #[cfg(test)]
// mod test {
//     use itertools::Itertools;
//     use std::sync::Arc;
//
//     use platform_value::Bytes32;
//     use serde_json::json;
//
//     use crate::tests::fixtures::get_extended_documents_fixture;
//     use crate::{
//         document::{
//             document_factory::DocumentFactory,
//             fetch_and_validate_data_contract::DataContractFetcherAndValidator,
//         },
//         state_repository::MockStateRepositoryLike,
//         tests::fixtures::{
//             get_data_contract_fixture, get_document_transitions_fixture,
//             get_document_validator_fixture,
//         },
//     };
//
//     use super::{document_transition::Action, *};
//
//     #[test]
//     fn should_return_highest_sec_level_for_all_transitions() {
//         let mut data_contract = get_data_contract_fixture(None).data_contract;
//         data_contract
//             .documents
//             .get_mut("niceDocument")
//             .unwrap()
//             .insert(
//                 property_names::SECURITY_LEVEL_REQUIREMENT.to_string(),
//                 json!(SecurityLevel::MEDIUM),
//             )
//             .unwrap();
//         data_contract
//             .documents
//             .get_mut("prettyDocument")
//             .unwrap()
//             .insert(
//                 property_names::SECURITY_LEVEL_REQUIREMENT.to_string(),
//                 json!(SecurityLevel::MASTER),
//             )
//             .unwrap();
//
//         // 0 is niceDocument,
//         // 1 and 2 are pretty documents,
//         // 3 and 4 are indexed documents that do not have security level specified
//         let documents = get_extended_documents_fixture(data_contract).unwrap();
//         let medium_security_document = documents.get(0).unwrap();
//         let master_security_document = documents.get(1).unwrap();
//         let no_security_level_document = documents.get(3).unwrap();
//
//         let document_factory = DocumentFactory::new(
//             1,
//             get_document_validator_fixture(),
//             DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
//         );
//
//         let batch_transition = document_factory
//             .create_state_transition(vec![(
//                 Action::Create,
//                 vec![medium_security_document.to_owned()],
//             )])
//             .expect("batch transition should be created");
//
//         assert!(batch_transition
//             .get_security_level_requirement()
//             .iter()
//             .contains(&SecurityLevel::MEDIUM));
//
//         let batch_transition = document_factory
//             .create_state_transition(vec![(
//                 Action::Create,
//                 vec![
//                     medium_security_document.to_owned(),
//                     master_security_document.to_owned(),
//                 ],
//             )])
//             .expect("batch transition should be created");
//
//         assert!(batch_transition
//             .get_security_level_requirement()
//             .iter()
//             .contains(&SecurityLevel::MASTER));
//
//         let batch_transition = document_factory
//             .create_state_transition(vec![(
//                 Action::Create,
//                 vec![no_security_level_document.to_owned()],
//             )])
//             .expect("batch transition should be created");
//
//         assert!(batch_transition
//             .get_security_level_requirement()
//             .iter()
//             .contains(&SecurityLevel::HIGH));
//     }
//
//     #[test]
//     fn should_convert_to_batch_transition_to_the_buffer() {
//         let transition_id_base58 = "6o8UfoeE2s7dTkxxyPCixuxe8TM5DtCGHTMummUN6t5M";
//         let expected_bytes_hex ="01a5647479706501676f776e657249645820a858bdc49c968148cd12648ee048d34003e9da3fbf2cbc62c31bb4c717bf690d697369676e6174757265f76b7472616e736974696f6e7381a7632469645820561b9b2e90b7c0ca355f729777b45bc646a18f5426a9462f0333c766135a3120646e616d656543757469656524747970656c6e696365446f63756d656e746724616374696f6e006824656e74726f707958202cdbaeda81c14765ba48432ff5cc900a7cacd4538b817fc71f38907aaa7023746a246372656174656441741b000001853a3602876f2464617461436f6e74726163744964582049aea5df2124a51d5d8dcf466e238fbc77fd72601be69daeb6dba75e8d26b30c747369676e61747572655075626c69634b65794964f7" ;
//         let data_contract_id_base58 = "5xdDqypFMPfvF6UdWxefCGvRFyxgkPZCAK6TS4pvvw6T";
//         let owner_id_base58 = "CL9ydpdxP4kQniGx6z5JUL8K72gnwcemKT2aJmh7sdwJ";
//         let entropy_base64 = "LNuu2oHBR2W6SEMv9cyQCnys1FOLgX/HHziQeqpwI3Q=";
//
//         let transition_id =
//             Identifier::from_string(transition_id_base58, Encoding::Base58).unwrap();
//         let expected_bytes = hex::decode(expected_bytes_hex).unwrap();
//         let data_contract_id =
//             Identifier::from_string(data_contract_id_base58, Encoding::Base58).unwrap();
//         let owner_id = Identifier::from_string(owner_id_base58, Encoding::Base58).unwrap();
//         let entropy_bytes: [u8; 32] = BASE64_STANDARD.decode(entropy_base64).unwrap().try_into().unwrap();
//
//         let mut data_contract = get_data_contract_fixture(Some(owner_id)).data_contract;
//         data_contract.id = data_contract_id;
//
//         let documents = get_extended_documents_fixture(data_contract.clone()).unwrap();
//         let mut document = documents.first().unwrap().to_owned();
//         document.entropy = Bytes32::new(entropy_bytes);
//
//         let transitions = get_document_transitions_fixture([(DocumentTransitionActionType::Create, vec![document])]);
//         let mut transition = transitions.first().unwrap().to_owned();
//         if let DocumentTransition::Create(ref mut t) = transition {
//             t.created_at = Some(1671718896263);
//             t.base.id = transition_id;
//         }
//
//         let mut map = BTreeMap::new();
//         map.insert(
//             "ownerId".to_string(),
//             Value::Identifier(owner_id.to_buffer()),
//         );
//         map.insert(
//             "transitions".to_string(),
//             Value::Array(vec![transition.to_object().unwrap()]),
//         );
//
//         let state_transition = DocumentsBatchTransition::from_value_map(map, vec![data_contract])
//             .expect("transition should be created");
//
//         let bytes = state_transition.to_cbor_buffer(false).unwrap();
//
//         assert_eq!(hex::encode(expected_bytes), hex::encode(bytes));
//     }
// }
impl OptionallyAssetLockProved for DocumentsBatchTransition {}
