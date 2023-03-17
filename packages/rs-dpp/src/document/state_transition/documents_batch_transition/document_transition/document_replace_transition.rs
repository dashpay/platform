use platform_value::btreemap_extensions::BTreeValueMapReplacementPathHelper;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::data_contract::document_type::document_type::PROTOCOL_VERSION;
use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::Identifier;
use crate::prelude::{ExtendedDocument, Revision};
use crate::{data_contract::DataContract, errors::ProtocolError};

use super::{document_base_transition::DocumentBaseTransition, DocumentTransitionObjectLike};

pub(self) mod property_names {
    pub const REVISION: &str = "$revision";
    pub const UPDATED_AT: &str = "$updatedAt";
}

/// Identifier fields in [`DocumentReplaceTransition`]
pub use super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReplaceTransition {
    #[serde(flatten)]
    pub base: DocumentBaseTransition,
    #[serde(rename = "$revision")]
    pub revision: Revision,
    #[serde(skip_serializing_if = "Option::is_none", rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<BTreeMap<String, Value>>,
}

impl DocumentReplaceTransition {
    pub(crate) fn to_document_for_dry_run(
        &self,
        owner_id: Identifier,
    ) -> Result<Document, ProtocolError> {
        let properties = self.data.clone().unwrap_or_default();
        Ok(Document {
            id: self.base.id.to_buffer(),
            owner_id: owner_id.to_buffer(),
            properties,
            created_at: self.updated_at, // we can use the same time, as it can't be worse
            updated_at: self.updated_at,
            revision: Some(self.revision),
        })
    }

    pub(crate) fn to_extended_document_for_dry_run(
        &self,
        owner_id: Identifier,
    ) -> Result<ExtendedDocument, ProtocolError> {
        Ok(ExtendedDocument {
            protocol_version: PROTOCOL_VERSION,
            document_type_name: self.base.document_type_name.clone(),
            data_contract_id: self.base.data_contract_id,
            document: self.to_document_for_dry_run(owner_id)?,
            data_contract: self.base.data_contract.clone(),
            metadata: None,
            entropy: [0; 32],
        })
    }

    pub(crate) fn replace_document(&self, document: &mut Document) -> Result<(), ProtocolError> {
        let properties = self.data.clone().unwrap_or_default();
        document.revision = Some(self.revision);
        document.updated_at = self.updated_at;
        document.properties = properties;
        Ok(())
    }

    pub(crate) fn replace_extended_document(
        &self,
        document: &mut ExtendedDocument,
    ) -> Result<(), ProtocolError> {
        let properties = self.data.clone().unwrap_or_default();
        document.document.revision = Some(self.revision);
        document.document.updated_at = self.updated_at;
        document.document.properties = properties;
        Ok(())
    }

    pub(crate) fn patch_document(self, document: &mut Document) -> Result<(), ProtocolError> {
        let properties = self.data.clone().unwrap_or_default();
        document.revision = Some(self.revision);
        document.updated_at = self.updated_at;
        document.properties.extend(properties);
        Ok(())
    }

    pub(crate) fn patch_extended_document(
        self,
        document: &mut ExtendedDocument,
    ) -> Result<(), ProtocolError> {
        let properties = self.data.clone().unwrap_or_default();
        document.document.revision = Some(self.revision);
        document.document.updated_at = self.updated_at;
        document.document.properties.extend(properties);
        Ok(())
    }
}

impl DocumentTransitionObjectLike for DocumentReplaceTransition {
    fn from_json_object(
        json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let value: Value = json_value.into();
        let mut map = value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let document_type = map.get_str("$type")?;

        let (identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths_owned(document_type)?;

        map.replace_at_paths(binary_paths.into_iter(), ReplacementType::BinaryBytes)?;

        map.replace_at_paths(
            identifiers_paths
                .into_iter()
                .chain(IDENTIFIER_FIELDS.iter().map(|a| a.to_string())),
            ReplacementType::Identifier,
        )?;
        let document = Self::from_value_map(map, data_contract)?;

        Ok(document)
    }

    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<DocumentReplaceTransition, ProtocolError> {
        let map = raw_transition
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Self::from_value_map(map, data_contract)
    }

    fn from_value_map(
        mut map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        Ok(DocumentReplaceTransition {
            base: DocumentBaseTransition::from_value_map_consume(&mut map, data_contract)?,
            revision: map
                .remove_integer(property_names::REVISION)
                .map_err(ProtocolError::ValueError)?,
            updated_at: map
                .remove_optional_integer(property_names::UPDATED_AT)
                .map_err(ProtocolError::ValueError)?,
            data: Some(map),
        })
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }

    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut transition_base_map = self.base.to_value_map()?;
        transition_base_map.insert(
            property_names::REVISION.to_string(),
            Value::U64(self.revision),
        );
        if let Some(updated_at) = self.updated_at {
            transition_base_map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(properties) = self.data.clone() {
            transition_base_map.extend(properties)
        }
        Ok(transition_base_map)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::document::document_transition::Action;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn test_deserialize_serialize_to_json() {
        init();
        let transition_json = r#"{
                    "$action": 1,
                    "$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
					"$id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
					"$revision" : 1,
					"$type": "note",
					"message": "example_message_replace"
				}"#;

        let cdt: DocumentReplaceTransition =
            serde_json::from_str(transition_json).expect("no error");

        assert_eq!(cdt.base.action, Action::Replace);
        assert_eq!(cdt.base.document_type_name, "note");
        assert_eq!(cdt.revision, 1);
        assert_eq!(
            cdt.data.as_ref().unwrap().get_str("message").unwrap(),
            "example_message_replace"
        );

        let mut json_no_whitespace = transition_json.to_string();
        json_no_whitespace.retain(|v| !v.is_whitespace());

        assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
    }
}
