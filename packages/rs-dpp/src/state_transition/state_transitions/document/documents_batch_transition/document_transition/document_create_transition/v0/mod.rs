mod from_document;
pub mod v0_methods;

use bincode::{Decode, Encode};

#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use std::string::ToString;

use crate::identity::TimestampMillis;

use crate::{data_contract::DataContract, errors::ProtocolError};

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::document::{Document, DocumentV0};
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use derive_more::Display;
use platform_version::version::PlatformVersion;

#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::documents_batch_transition;

mod property_names {
    pub const ENTROPY: &str = "$entropy";
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
#[display(fmt = "Base: {}, Entropy: {:?}, Data: {:?}", "base", "entropy", "data")]
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
        let identity_contract_nonce = map
            .remove_integer(documents_batch_transition::document_base_transition::property_names::IDENTITY_CONTRACT_NONCE)
            .map_err(ProtocolError::ValueError)?;
        Ok(Self {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0::from_value_map_consume(
                &mut map,
                data_contract,
                identity_contract_nonce,
            )?),
            entropy: map
                .remove_hash256_bytes(property_names::ENTROPY)
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

        transition_base_map.extend(self.data.clone());

        Ok(transition_base_map)
    }
}

/// documents from create transition v0
pub trait DocumentFromCreateTransitionV0 {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentCreateTransition` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_create_transition_v0(
        v0: DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_time: TimestampMillis,
        requires_created_at: bool,
        requires_updated_at: bool,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentCreateTransitionActionV0` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_create_transition_v0(
        v0: &DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_time: TimestampMillis,
        requires_created_at: bool,
        requires_updated_at: bool,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromCreateTransitionV0 for Document {
    fn try_from_owned_create_transition_v0(
        v0: DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_time: TimestampMillis,
        requires_created_at: bool,
        requires_updated_at: bool,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let DocumentCreateTransitionV0 { base, data, .. } = v0;

        match base {
            DocumentBaseTransition::V0(base_v0) => {
                let DocumentBaseTransitionV0 {
                    id,
                    document_type_name,
                    ..
                } = base_v0;

                let document_type =
                    data_contract.document_type_for_name(document_type_name.as_str())?;

                let created_at = if requires_created_at {
                    Some(block_time)
                } else {
                    None
                };
                let updated_at = if requires_updated_at {
                    Some(block_time)
                } else {
                    None
                };

                match platform_version
                    .dpp
                    .document_versions
                    .document_structure_version
                {
                    0 => Ok(DocumentV0 {
                        id,
                        owner_id,
                        properties: data,
                        revision: document_type.initial_revision(),
                        created_at,
                        updated_at,
                    }
                    .into()),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "Document::try_from_create_transition_v0".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    fn try_from_create_transition_v0(
        v0: &DocumentCreateTransitionV0,
        owner_id: Identifier,
        block_time: TimestampMillis,
        requires_created_at: bool,
        requires_updated_at: bool,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let DocumentCreateTransitionV0 { base, data, .. } = v0;

        match base {
            DocumentBaseTransition::V0(base_v0) => {
                let DocumentBaseTransitionV0 {
                    id,
                    document_type_name,
                    ..
                } = base_v0;

                let document_type =
                    data_contract.document_type_for_name(document_type_name.as_str())?;

                let created_at = if requires_created_at {
                    Some(block_time)
                } else {
                    None
                };
                let updated_at = if requires_updated_at {
                    Some(block_time)
                } else {
                    None
                };

                match platform_version
                    .dpp
                    .document_versions
                    .document_structure_version
                {
                    0 => Ok(DocumentV0 {
                        id: *id,
                        owner_id,
                        properties: data.clone(),
                        revision: document_type.initial_revision(),
                        created_at,
                        updated_at,
                    }
                    .into()),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "Document::try_from_owned_create_transition_v0".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::data_contract::data_contract::DataContractV0;
    use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
    use platform_value::btreemap_extensions::BTreeValueMapHelper;
    use platform_value::{platform_value, BinaryData, Bytes32, Identifier};
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use serde_json::json;

    use super::*;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use serde_json::Value as JsonValue;

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
            ("position", Value::U64(0)),
        ]);
        let test_document_properties_alpha_binary = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
            ("minItems", Value::U64(32)),
            ("maxItems", Value::U64(32)),
            (
                "contentMediaType",
                Value::Text("application/x.dash.dpp.identifier".to_string()),
            ),
            ("position", Value::U64(1)),
        ]);
        let test_document_properties = Value::from([
            ("alphaIdentifier", test_document_properties_alpha_identifier),
            ("alphaBinary", test_document_properties_alpha_binary),
        ]);
        let test_document = Value::from([
            ("type", Value::Text("object".to_string())),
            ("properties", test_document_properties),
            ("additionalProperties", Value::Bool(false)),
        ]);
        let documents = Value::from([("test", test_document)]);
        DataContract::V0(
            DataContractV0::from_value(
                Value::from([
                    ("$id", Value::Identifier([0_u8; 32])),
                    ("id", Value::Identifier([0_u8; 32])),
                    ("$schema", Value::Text("schema".to_string())),
                    ("$format_version", Value::Text("0".to_string())),
                    ("version", Value::U32(0)),
                    ("documentSchemas", documents),
                    ("ownerId", Value::Identifier([0_u8; 32])),
                ]),
                true,
                LATEST_PLATFORM_VERSION,
            )
            .unwrap(),
        )
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
            "$version"  : "0".to_string(),
            "$id" : id,
            "id" : id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "$identityContractNonce": 0u64,
            "revision" : 1u32,
            "alphaBinary" : alpha_binary,
            "alphaIdentifier" : alpha_identifier,
            "$entropy" : entropy,
            "$action": 0u8,
        });

        let transition: DocumentCreateTransition =
            DocumentCreateTransition::from_object(raw_document, data_contract).unwrap();

        let json_transition = transition.to_json().expect("no errors");
        assert_eq!(json_transition["V0"]["$id"], JsonValue::String(id.into()));
        assert_eq!(
            json_transition["V0"]["$dataContractId"],
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
    fn convert_to_object_from_json_value_with_dynamic_binary_paths() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_value = vec![10_u8; 32];
        let id = vec![11_u8; 32];
        let data_contract_id = vec![13_u8; 32];
        let entropy = vec![11_u8; 32];

        let raw_document = json!({
            "$protocolVersion"  : 0,
            "$version"  : "0",
            "$id" : id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "$identityContractNonce": 0u64,
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

        let v0 = object_transition.get("V0").expect("to get V0");
        let right_id = Identifier::from_bytes(&id).unwrap();
        let right_data_contract_id = Identifier::from_bytes(&data_contract_id).unwrap();

        assert_eq!(v0["$id"], Value::Identifier(right_id.into_buffer()));
        assert_eq!(
            v0["$dataContractId"],
            Value::Identifier(right_data_contract_id.into_buffer())
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
