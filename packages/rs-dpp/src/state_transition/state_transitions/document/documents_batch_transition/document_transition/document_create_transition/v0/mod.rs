mod from_document;
pub mod v0_methods;

use bincode::{Decode, Encode};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueMapReplacementPathHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Bytes32, Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::string::ToString;

use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;

use crate::version::LATEST_PLATFORM_VERSION;
use crate::{data_contract::DataContract, errors::ProtocolError};

use crate::document::INITIAL_REVISION;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::{
    DocumentBaseTransitionV0, DocumentTransitionObjectLike,
};
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use derive_more::Display;

pub(self) mod property_names {
    pub const ENTROPY: &str = "$entropy";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

/// The Binary fields in [`DocumentCreateTransition`]
pub const BINARY_FIELDS: [&str; 1] = ["$entropy"];
/// The Identifier fields in [`DocumentCreateTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    fmt = "Base: {}, Entropy: {:?}, Created At: {:?}, Updated At: {:?}, Data: {:?}",
    "base",
    "entropy",
    "created_at",
    "updated_at",
    "data"
)]
pub struct DocumentCreateTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,

    /// Entropy used to create a Document ID.
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$entropy")
    )]
    pub entropy: [u8; 32],

    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$createdAt", skip_serializing_if = "Option::is_none")
    )]
    pub created_at: Option<TimestampMillis>,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$updatedAt", skip_serializing_if = "Option::is_none")
    )]
    pub updated_at: Option<TimestampMillis>,

    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub data: BTreeMap<String, Value>,
}
//
// impl DocumentCreateTransitionV0 {
//     pub fn get_revision(&self) -> Option<Revision> {
//         //todo: fix this
//         Some(INITIAL_REVISION)
//     }
//
//     pub(crate) fn to_document(&self, owner_id: Identifier) -> Result<Document, ProtocolError> {
//         let properties = self.data.clone().unwrap_or_default();
//         Ok(Document {
//             id: self.base.id,
//             owner_id,
//             properties,
//             created_at: self.created_at,
//             updated_at: self.updated_at,
//             revision: self.get_revision(),
//         })
//     }
//
//     pub(crate) fn to_extended_document(
//         &self,
//         owner_id: Identifier,
//     ) -> Result<ExtendedDocument, ProtocolError> {
//         Ok(ExtendedDocument {
//             feature_version: LATEST_PLATFORM_VERSION
//                 .extended_document
//                 .default_current_version,
//             document_type_name: self.base.document_type_name.clone(),
//             data_contract_id: self.base.data_contract_id,
//             document: self.to_document(owner_id)?,
//             data_contract: self.base.data_contract.clone(),
//             metadata: None,
//             entropy: Bytes32::new(self.entropy),
//         })
//     }
//
//     pub(crate) fn into_document(self, owner_id: Identifier) -> Result<Document, ProtocolError> {
//         let id = self.base.id;
//         let revision = self.get_revision();
//         let created_at = self.created_at;
//         let updated_at = self.updated_at;
//         let properties = self.data.unwrap_or_default();
//         Ok(Document {
//             id,
//             owner_id,
//             properties,
//             created_at,
//             updated_at,
//             revision,
//         })
//     }
// }

impl DocumentCreateTransitionV0 {
    #[cfg(feature = "state-transition-value-conversion")]
    pub(crate) fn from_value_map(
        mut map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        Ok(Self {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0::from_value_map_consume(
                &mut map,
                data_contract,
            )?),
            entropy: map
                .remove_hash256_bytes(property_names::ENTROPY)
                .map_err(ProtocolError::ValueError)?,
            created_at: map
                .remove_optional_integer(property_names::CREATED_AT)
                .map_err(ProtocolError::ValueError)?,
            updated_at: map
                .remove_optional_integer(property_names::UPDATED_AT)
                .map_err(ProtocolError::ValueError)?,
            data: map,
        })
    }

    #[cfg(feature = "state-transition-value-conversion")]
    pub(crate) fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut transition_base_map = self.base.to_value_map()?;
        transition_base_map.insert(
            property_names::ENTROPY.to_string(),
            Value::Bytes(self.entropy.to_vec()),
        );
        if let Some(created_at) = self.created_at {
            transition_base_map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            transition_base_map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }

        transition_base_map.extend(self.data.clone());

        Ok(transition_base_map)
    }
}

#[cfg(test)]
mod test {

    use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
    use platform_value::{platform_value, BinaryData, Identifier};
    use serde_json::json;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    fn data_contract_with_dynamic_properties() -> DataContract {
        // The following is equivalent to the data contract
        // {
        //     "protocolVersion" :0,
        //     "$id" : vec![0_u8;32],
        //     "$schema" : "schema",
        //     "version" : 0,
        //     "ownerId" : vec![0_u8;32],
        //     "documents" : {
        //         "test" : {
        //             "properties" : {
        //                 "alphaIdentifier" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                     "contentMediaType": "application/x.dash.dpp.identifier",
        //                 },
        //                 "alphaBinary" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                 }
        //             }
        //         }
        //     }
        // }
        let test_document_properties_alpha_identifier = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
        ]);
        let test_document_properties_alpha_binary = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
            (
                "contentMediaType",
                Value::Text("application/x.dash.dpp.identifier".to_string()),
            ),
        ]);
        let test_document_properties = Value::from([
            ("alphaIdentifier", test_document_properties_alpha_identifier),
            ("alphaBinary", test_document_properties_alpha_binary),
        ]);
        let test_document = Value::from([("properties", test_document_properties)]);
        let documents = Value::from([("test", test_document)]);
        Value::from([
            ("protocolVersion", Value::U32(1)),
            ("$id", Value::Identifier([0_u8; 32])),
            ("$schema", Value::Text("schema".to_string())),
            ("version", Value::U32(0)),
            ("ownerId", Value::Identifier([0_u8; 32])),
            ("documents", documents),
        ])
        .try_into()
        .unwrap()
    }

    #[test]
    #[cfg(feature = "state-transition-json-conversion")]
    fn convert_to_json_with_dynamic_binary_paths() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_binary = BinaryData::new(vec![10_u8; 32]);
        let alpha_identifier = Identifier::from([10_u8; 32]);
        let id = Identifier::from([11_u8; 32]);
        let data_contract_id = Identifier::from([13_u8; 32]);
        let entropy = Bytes32::new([14_u8; 32]);

        let raw_document = platform_value!({
            "$protocolVersion"  : 0u32,
            "$id" : id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "revision" : 1u32,
            "alphaBinary" : alpha_binary,
            "alphaIdentifier" : alpha_identifier,
            "$entropy" : entropy,
            "$action": 0u8,
        });

        let transition: DocumentCreateTransition =
            DocumentCreateTransition::from_object(raw_document, data_contract).unwrap();

        let json_transition = transition.to_json().expect("no errors");
        assert_eq!(json_transition["$id"], JsonValue::String(id.into()));
        assert_eq!(
            json_transition["$dataContractId"],
            JsonValue::String(data_contract_id.into())
        );
        assert_eq!(
            json_transition["alphaBinary"],
            JsonValue::String(alpha_binary.into())
        );
        assert_eq!(
            json_transition["alphaIdentifier"],
            JsonValue::String(alpha_identifier.into())
        );
        assert_eq!(
            json_transition["$entropy"],
            JsonValue::String(entropy.into())
        );
    }

    #[test]
    fn covert_to_object_from_json_value_with_dynamic_binary_paths() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_value = vec![10_u8; 32];
        let id = vec![11_u8; 32];
        let data_contract_id = vec![13_u8; 32];
        let entropy = vec![11_u8; 32];

        let raw_document = json!({
            "$protocolVersion"  : 0,
            "$id" : id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "revision" : 1,
            "alphaBinary" : alpha_value,
            "alphaIdentifier" : alpha_value,
            "$entropy" : entropy,
            "$action": 0 ,
        });

        let document: DocumentCreateTransition =
            DocumentCreateTransition::from_json_object(raw_document, data_contract).unwrap();

        let object_transition = document
            .to_object()
            .expect("no errors")
            .into_btree_string_map()
            .unwrap();
        assert_eq!(object_transition.get_identifier_bytes("$id").unwrap(), id);
        assert_eq!(
            object_transition
                .get_identifier_bytes("$dataContractId")
                .unwrap(),
            data_contract_id
        );
        assert_eq!(
            object_transition.get_bytes("alphaBinary").unwrap(),
            alpha_value
        );
        assert_eq!(
            object_transition
                .get_identifier_bytes("alphaIdentifier")
                .unwrap(),
            alpha_value
        );
    }
}
