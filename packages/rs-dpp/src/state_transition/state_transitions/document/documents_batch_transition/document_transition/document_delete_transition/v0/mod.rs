use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::{data_contract::DataContract, errors::ProtocolError};
use bincode::{Decode, Encode};
use derive_more::Display;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode, PartialEq, Display)]
#[display(fmt = "Base: {}", "base")]
pub struct DocumentDeleteTransitionV0 {
    #[serde(flatten)]
    pub base: DocumentBaseTransition,
}
//
// impl DocumentTransitionObjectLike for DocumentDeleteTransitionV0 {
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn from_json_object(
//         json_value: JsonValue,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError> {
//         let mut document: DocumentDeleteTransitionV0 = serde_json::from_value(json_value)?;
//         document.base.data_contract = data_contract;
//
//         Ok(document)
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn from_object(
//         raw_transition: Value,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError> {
//         let base = DocumentBaseTransition::from_object(raw_transition, data_contract)?;
//
//         Ok(DocumentDeleteTransitionV0 { base })
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn from_value_map(
//         mut map: BTreeMap<String, Value>,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError>
//         where
//             Self: Sized,
//     {
//         let base = DocumentBaseTransition::from_value_map_consume(&mut map, data_contract)?;
//
//         Ok(DocumentDeleteTransitionV0 { base })
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_object(&self) -> Result<Value, ProtocolError> {
//         self.base.to_object()
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
//         self.base.to_value_map()
//     }
//
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn to_json(&self) -> Result<JsonValue, ProtocolError> {
//         self.base.to_json()
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
//         self.base.to_cleaned_object()
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0Methods;
//     use super::*;
//
//     fn init() {
//         let _ = env_logger::builder()
//             .filter_level(log::LevelFilter::Debug)
//             .try_init();
//     }
//
//     #[test]
//     fn test_deserialize_serialize_to_json() {
//         init();
//         let transition_json = r#"{
//                     "$action": 3,
//                     "$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
// 					"$id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
// 					"$type": "note"
// 				}"#;
//
//         let cdt: DocumentDeleteTransitionV0 =
//             serde_json::from_str(transition_json).expect("no error");
//
//         assert_eq!(cdt.base.document_type_name(), "note");
//
//         let mut json_no_whitespace = transition_json.to_string();
//         json_no_whitespace.retain(|v| !v.is_whitespace());
//
//         assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
//     }
// }
