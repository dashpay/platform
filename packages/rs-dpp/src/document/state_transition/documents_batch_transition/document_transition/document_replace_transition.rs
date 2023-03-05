use platform_value::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::{ExtendedDocument, Revision};
use crate::{
    data_contract::DataContract,
    errors::ProtocolError,
    util::json_value::{JsonValueExt, ReplaceWith},
};

use super::{
    document_base_transition, document_base_transition::DocumentBaseTransition,
    merge_serde_json_values, Action, DocumentTransitionObjectLike,
};

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
    pub data: Option<JsonValue>,
}

impl DocumentReplaceTransition {
    pub(crate) fn to_document_for_dry_run(&self) -> Result<Document, ProtocolError> {
        let properties = self
            .data
            .as_ref()
            .map(|json_value| {
                let value: Value = json_value.clone().into();
                value.into_btree_map().map_err(ProtocolError::ValueError)
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Document {
            id: self.base.id.to_buffer(),
            owner_id: [0; 32], //0s are fine here
            properties,
            created_at: self.updated_at, // we can use the same time, as it can't be worse
            updated_at: self.updated_at,
            revision: Some(self.revision),
        })
    }

    pub(crate) fn replace_document(&self, document: &mut Document) -> Result<(), ProtocolError> {
        let properties = self
            .data
            .as_ref()
            .map(|json_value| {
                let value: Value = json_value.clone().into();
                value.into_btree_map().map_err(ProtocolError::ValueError)
            })
            .transpose()?
            .unwrap_or_default();
        document.revision = Some(self.revision);
        document.updated_at = self.updated_at;
        document.properties = properties;
        Ok(())
    }

    pub(crate) fn replace_extended_document(
        &self,
        document: &mut ExtendedDocument,
    ) -> Result<(), ProtocolError> {
        let properties = self
            .data
            .as_ref()
            .map(|json_value| {
                let value: Value = json_value.clone().into();
                value.into_btree_map().map_err(ProtocolError::ValueError)
            })
            .transpose()?
            .unwrap_or_default();
        document.document.revision = Some(self.revision);
        document.document.updated_at = self.updated_at;
        document.document.properties = properties;
        Ok(())
    }

    pub(crate) fn patch_document(self, document: &mut Document) -> Result<(), ProtocolError> {
        let properties = self
            .data
            .map(|json_value| {
                let value: Value = json_value.into();
                value.into_btree_map().map_err(ProtocolError::ValueError)
            })
            .transpose()?
            .unwrap_or_default();
        document.revision = Some(self.revision);
        document.updated_at = self.updated_at;
        document.properties.extend(properties);
        Ok(())
    }

    pub(crate) fn patch_extended_document(
        self,
        document: &mut ExtendedDocument,
    ) -> Result<(), ProtocolError> {
        let properties = self
            .data
            .map(|json_value| {
                let value: Value = json_value.into();
                value.into_btree_map().map_err(ProtocolError::ValueError)
            })
            .transpose()?
            .unwrap_or_default();
        document.document.revision = Some(self.revision);
        document.document.updated_at = self.updated_at;
        document.document.properties.extend(properties);
        Ok(())
    }
}

impl DocumentTransitionObjectLike for DocumentReplaceTransition {
    fn from_json_object(
        mut json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let document_type = json_value.get_string("$type")?;

        let (identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths(document_type)?;

        // Only dynamic binary paths are replaced with Bytes (no static ones)
        json_value.replace_binary_paths(binary_paths.into_iter(), ReplaceWith::Bytes)?;
        // Only dynamic identifiers are replaced with Bytes
        json_value.replace_identifier_paths(identifiers_paths, ReplaceWith::Bytes)?;
        let mut document: DocumentReplaceTransition = serde_json::from_value(json_value)?;

        document.base.action = Action::Replace;
        document.base.data_contract = data_contract;

        Ok(document)
    }

    fn from_raw_object(
        mut raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<DocumentReplaceTransition, ProtocolError> {
        // Only static identifiers are replaced, as the dynamic ones are stored as Arrays
        raw_transition.replace_identifier_paths(
            document_base_transition::IDENTIFIER_FIELDS,
            ReplaceWith::Base58,
        )?;

        let mut document: DocumentReplaceTransition = serde_json::from_value(raw_transition)?;
        document.base.action = Action::Replace;
        document.base.data_contract = data_contract;

        Ok(document)
    }

    fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        let transition_base_value = self.base.to_object()?;
        let mut transition_create_value = serde_json::to_value(self)?;

        merge_serde_json_values(&mut transition_create_value, transition_base_value)?;
        Ok(transition_create_value)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = serde_json::to_value(self)?;
        let (identifier_paths, binary_paths) = self
            .base
            .data_contract
            .get_identifiers_and_binary_paths(&self.base.document_type)?;

        value.replace_binary_paths(identifier_paths, ReplaceWith::Base58)?;
        value.replace_binary_paths(binary_paths, ReplaceWith::Base64)?;

        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn test_deserialize_serialize_to_json() {
        init();
        let transition_json = r#"{
					"$id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
					"$type": "note",
					"$action": 1,
					"$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
					"$revision" : 1,
					"message": "example_message_replace"
				}"#;

        let cdt: DocumentReplaceTransition =
            serde_json::from_str(transition_json).expect("no error");

        assert_eq!(cdt.base.action, Action::Replace);
        assert_eq!(cdt.base.document_type, "note");
        assert_eq!(cdt.revision, 1);
        assert_eq!(
            cdt.data.as_ref().unwrap()["message"],
            "example_message_replace"
        );

        let mut json_no_whitespace = transition_json.to_string();
        json_no_whitespace.retain(|v| !v.is_whitespace());

        assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
    }
}
