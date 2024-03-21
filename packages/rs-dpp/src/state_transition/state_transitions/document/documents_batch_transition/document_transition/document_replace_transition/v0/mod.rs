mod from_document;
pub mod v0_methods;

use crate::prelude::{BlockHeight, CoreBlockHeight, Revision, TimestampMillis};
use bincode::{Decode, Encode};
use derive_more::Display;

use platform_value::{Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::block::block_info::BlockInfo;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
use crate::{document, ProtocolError};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;

mod property_names {
    pub const REVISION: &str = "$revision";
}

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(fmt = "Base: {}, Revision: {}, Data: {:?}", "base", "revision", "data")]
pub struct DocumentReplaceTransitionV0 {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$revision")
    )]
    pub revision: Revision,
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub data: BTreeMap<String, Value>,
}
//
// impl DocumentReplaceTransitionV0 {
//     pub(crate) fn to_document_for_dry_run(
//         &self,
//         owner_id: Identifier,
//     ) -> Result<Document, ProtocolError> {
//         let properties = self.data.clone().unwrap_or_default();
//         Ok(Document {
//             id: self.base.id,
//             owner_id,
//             properties,
//             created_at: self.updated_at, // we can use the same time, as it can't be worse
//             updated_at: self.updated_at,
//             revision: Some(self.revision),
//         })
//     }
//
//     pub(crate) fn to_extended_document_for_dry_run(
//         &self,
//         owner_id: Identifier,
//         platform_version
//     ) -> Result<ExtendedDocument, ProtocolError> {
//         Ok(ExtendedDocument {
//             feature_version: LATEST_PLATFORM_VERSION
//                 .extended_document
//                 .default_current_version,
//             document_type_name: self.base.document_type_name.clone(),
//             data_contract_id: self.base.data_contract_id,
//             document: self.to_document_for_dry_run(owner_id)?,
//             data_contract: self.base.data_contract.clone(),
//             metadata: None,
//             entropy: Bytes32::default(),
//         })
//     }
//
//     pub(crate) fn replace_document(&self, document: &mut Document) -> Result<(), ProtocolError> {
//         let properties = self.data.clone().unwrap_or_default();
//         document.revision = Some(self.revision);
//         document.updated_at = self.updated_at;
//         document.properties = properties;
//         Ok(())
//     }
//
//     pub(crate) fn replace_extended_document(
//         &self,
//         document: &mut ExtendedDocument,
//     ) -> Result<(), ProtocolError> {
//         let properties = self.data.clone().unwrap_or_default();
//         document.document.revision = Some(self.revision);
//         document.document.updated_at = self.updated_at;
//         document.document.properties = properties;
//         Ok(())
//     }
//
//     pub(crate) fn patch_document(self, document: &mut Document) -> Result<(), ProtocolError> {
//         let properties = self.data.clone().unwrap_or_default();
//         document.revision = Some(self.revision);
//         document.updated_at = self.updated_at;
//         document.properties.extend(properties);
//         Ok(())
//     }
//
//     pub(crate) fn patch_extended_document(
//         self,
//         document: &mut ExtendedDocument,
//     ) -> Result<(), ProtocolError> {
//         let properties = self.data.clone().unwrap_or_default();
//         document.document.revision = Some(self.revision);
//         document.document.updated_at = self.updated_at;
//         document.document.properties.extend(properties);
//         Ok(())
//     }
// }
//
// impl DocumentTransitionObjectLike for DocumentReplaceTransitionV0 {
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn from_json_object(
//         json_value: JsonValue,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError> {
//         let value: Value = json_value.into();
//         let mut map = value
//             .into_btree_string_map()
//             .map_err(ProtocolError::ValueError)?;
//
//         let document_type = map.get_str("$type")?;
//
//         let (identifiers_paths, binary_paths): (Vec<_>, Vec<_>) =
//             data_contract.get_identifiers_and_binary_paths_owned(document_type)?;
//
//         map.replace_at_paths(binary_paths.into_iter(), ReplacementType::BinaryBytes)?;
//
//         map.replace_at_paths(
//             identifiers_paths
//                 .into_iter()
//                 .chain(IDENTIFIER_FIELDS.iter().map(|a| a.to_string())),
//             ReplacementType::Identifier,
//         )?;
//         let document = Self::from_value_map(map, data_contract)?;
//
//         Ok(document)
//     }
//
//     fn from_object(
//         raw_transition: Value,
//         data_contract: DataContract,
//     ) -> Result<DocumentReplaceTransitionV0, ProtocolError> {
//         let map = raw_transition
//             .into_btree_string_map()
//             .map_err(ProtocolError::ValueError)?;
//         Self::from_value_map(map, data_contract)
//     }
//
//     fn from_value_map(
//         mut map: BTreeMap<String, Value>,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError>
//         where
//             Self: Sized,
//     {
//         Ok(DocumentReplaceTransitionV0 {
//             base: DocumentBaseTransition::from_value_map_consume(&mut map, data_contract)?,
//             revision: map
//                 .remove_integer(property_names::REVISION)
//                 .map_err(ProtocolError::ValueError)?,
//             updated_at: map
//                 .remove_optional_integer(property_names::UPDATED_AT)
//                 .map_err(ProtocolError::ValueError)?,
//             data: Some(map),
//         })
//     }
//
//     fn to_object(&self) -> Result<Value, ProtocolError> {
//         Ok(self.to_value_map()?.into())
//     }
//
//     fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
//         let mut transition_base_map = self.base.to_value_map()?;
//         transition_base_map.insert(
//             property_names::REVISION.to_string(),
//             Value::U64(self.revision),
//         );
//         if let Some(updated_at) = self.updated_at {
//             transition_base_map.insert(
//                 property_names::UPDATED_AT.to_string(),
//                 Value::U64(updated_at),
//             );
//         }
//         if let Some(properties) = self.data.clone() {
//             transition_base_map.extend(properties)
//         }
//         Ok(transition_base_map)
//     }
//
//     fn to_json(&self) -> Result<JsonValue, ProtocolError> {
//         self.to_cleaned_object()?
//             .try_into()
//             .map_err(ProtocolError::ValueError)
//     }
//
//     fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
//         Ok(self.to_value_map()?.into())
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
//                     "$action": 1,
//                     "$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
// 					"$id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
// 					"$revision" : 1,
// 					"$type": "note",
// 					"message": "example_message_replace"
// 				}"#;
//
//         let cdt: DocumentReplaceTransitionV0 =
//             serde_json::from_str(transition_json).expect("no error");
//
//         assert_eq!(cdt.base.document_type_name(), "note");
//         assert_eq!(cdt.revision, 1);
//         assert_eq!(
//             cdt.data.as_ref().unwrap().get_str("message").unwrap(),
//             "example_message_replace"
//         );
//
//         let mut json_no_whitespace = transition_json.to_string();
//         json_no_whitespace.retain(|v| !v.is_whitespace());
//
//         assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
//     }
// }

/// document from replace transition v0
pub trait DocumentFromReplaceTransitionV0 {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionV0` reference. This operation is typically used to replace or update an existing document with new information.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentReplaceTransitionV0` containing the new information for the document.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp representing when the original document was created. This is preserved during replacement.
    /// * `created_at_block_height` - An optional height of the block at which the original document was created. This is preserved during replacement.
    /// * `created_at_core_block_height` - An optional core block height at which the original document was created. This is preserved during replacement.
    /// * `block_info` - Information about the current block at the time of this replace transition.
    /// * `document_type` - A reference to the `DocumentTypeRef` indicating the type of the document being replaced.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform under which the document is being replaced.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful replacement, returns a new `Document` object populated with the provided data. Returns a `ProtocolError` if the replacement fails due to validation errors or other issues.
    ///
    /// # Errors
    ///
    /// This function may return a `ProtocolError` if validation fails, required fields are missing, or if there are mismatches between field types and the schema defined in the data contract.
    fn try_from_replace_transition_v0(
        value: &DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionV0` instance. This function is similar to `try_from_replace_transition_v0` but consumes the `DocumentReplaceTransitionV0` instance, making it suitable for scenarios where the transition is owned and should not be reused after document creation.
    ///
    /// # Arguments
    ///
    /// * `value` - An owned `DocumentReplaceTransitionV0` instance containing the new information for the document.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp representing when the original document was created. This is preserved during replacement.
    /// * `created_at_block_height` - An optional height of the block at which the original document was created. This is preserved during replacement.
    /// * `created_at_core_block_height` - An optional core block height at which the original document was created. This is preserved during replacement.
    /// * `block_info` - Information about the current block at the time of this replace transition.
    /// * `document_type` - A reference to the `DocumentTypeRef` indicating the type of the document being replaced.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform under which the document is being replaced.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful replacement, returns a new `Document` object populated with the provided data. Returns a `ProtocolError` if the replacement fails due to validation errors or other issues.
    ///
    /// # Errors
    ///
    /// This function may return a `ProtocolError` for the same reasons as `try_from_replace_transition_v0`, including validation failures, missing required fields, or schema mismatches.
    fn try_from_owned_replace_transition_v0(
        value: DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransitionV0 for Document {
    fn try_from_replace_transition_v0(
        value: &DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            data,
        } = value;

        let id = base.id();

        let requires_updated_at = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT);

        let requires_updated_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_BLOCK_HEIGHT);

        let requires_updated_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_CORE_BLOCK_HEIGHT);

        let updated_at = if requires_updated_at {
            Some(block_info.time_ms)
        } else {
            None
        };

        let updated_at_block_height = if requires_updated_at_block_height {
            Some(block_info.height)
        } else {
            None
        };

        let updated_at_core_block_height = if requires_updated_at_core_block_height {
            Some(block_info.core_height)
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
                properties: data.clone(),
                revision: Some(*revision),
                created_at,
                updated_at,
                created_at_block_height,
                updated_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_owned_replace_transition_v0(
        value: DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            data,
        } = value;

        let id = base.id();

        let requires_updated_at = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT);

        let requires_updated_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_BLOCK_HEIGHT);

        let requires_updated_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_CORE_BLOCK_HEIGHT);

        let updated_at = if requires_updated_at {
            Some(block_info.time_ms)
        } else {
            None
        };

        let updated_at_block_height = if requires_updated_at_block_height {
            Some(block_info.height)
        } else {
            None
        };

        let updated_at_core_block_height = if requires_updated_at_core_block_height {
            Some(block_info.core_height)
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
                revision: Some(revision),
                created_at,
                updated_at,
                created_at_block_height,
                updated_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
