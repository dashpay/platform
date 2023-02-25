use std::convert::TryInto;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use platform_value::Value;

use crate::{
    data_contract::DataContract, document::document_transition::Action, errors::ProtocolError,
    util::json_value::JsonValueExt, util::json_value::ReplaceWith,
};
use crate::document::{Document, DocumentsBatchTransition};
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::util::serializer::value_to_cbor;

use super::INITIAL_REVISION;
use super::{
    document_base_transition, document_base_transition::DocumentBaseTransition,
    merge_serde_json_values, DocumentTransitionObjectLike,
};

/// The Binary fields in [`DocumentCreateTransition`]
pub const BINARY_FIELDS: [&str; 1] = ["$entropy"];
/// The Identifier fields in [`DocumentCreateTransition`]
pub use super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCreateTransition {
    /// Document Base Transition
    #[serde(flatten)]
    pub base: DocumentBaseTransition,

    /// Entropy used to create a Document ID.
    #[serde(rename = "$entropy")]
    pub entropy: [u8; 32],

    #[serde(rename = "$createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<TimestampMillis>,
    #[serde(rename = "$updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<TimestampMillis>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DocumentCreateTransition {
    pub fn get_revision(&self) -> Option<Revision> {
        //todo: fix this
        Some(INITIAL_REVISION)
    }

    pub fn bytes_to_strings(
        raw_create_document_transition: &mut JsonValue,
    ) -> Result<(), ProtocolError> {
        raw_create_document_transition.replace_binary_paths(BINARY_FIELDS, ReplaceWith::Base64)?;
        Ok(())
    }

    pub(crate) fn to_document(
        &self,
        owner_id: [u8;32],
    ) -> Result<Document, ProtocolError> {
        let properties = self.data.as_ref().map(|json_value| {
            let value : Value = json_value.clone().into();
            value.into_btree_map().map_err(ProtocolError::ValueError)
        })
            .transpose()?.unwrap_or_default();
        Ok(Document {
            id: self.base.id.to_buffer(),
            owner_id,
            properties,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            revision: self.get_revision(),
        })
    }


    pub(crate) fn into_document(
        self,
        owner_id: [u8;32],
    ) -> Result<Document, ProtocolError> {
        let id = self.base.id.to_buffer();
        let revision = self.get_revision();
        let created_at = self.created_at;
        let updated_at = self.updated_at;
        let properties = self.data.map(|json_value| {
            let value : Value = json_value.into();
            value.into_btree_map().map_err(ProtocolError::ValueError)
        }).transpose()?.unwrap_or_default();
        Ok(Document {
            id,
            owner_id,
            properties,
            created_at,
            updated_at,
            revision,
        })
    }
}

impl DocumentTransitionObjectLike for DocumentCreateTransition {
    fn from_json_object(
        mut json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let document_type = json_value.get_string("$type")?;

        let (identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths(document_type)?;

        json_value.replace_binary_paths(
            binary_paths.into_iter().chain(BINARY_FIELDS),
            ReplaceWith::Bytes,
        )?;
        // Only dynamic identifiers are being replaced with bytes. Static are Strings
        json_value.replace_identifier_paths(identifiers_paths, ReplaceWith::Bytes)?;
        let mut document: DocumentCreateTransition = serde_json::from_value(json_value)?;

        document.base.action = Action::Create;
        document.base.data_contract = data_contract;

        Ok(document)
    }

    fn from_raw_object(
        mut raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<DocumentCreateTransition, ProtocolError> {
        // Only static identifiers are replaced, as the dynamic ones are stored as Arrays
        raw_transition.replace_identifier_paths(
            document_base_transition::IDENTIFIER_FIELDS,
            ReplaceWith::Base58,
        )?;

        let mut document: DocumentCreateTransition = serde_json::from_value(raw_transition)?;
        document.base.action = Action::Create;
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

        value.replace_identifier_paths(identifier_paths, ReplaceWith::Base58)?;
        value.replace_binary_paths(
            binary_paths.into_iter().chain(BINARY_FIELDS).unique(),
            ReplaceWith::Base64,
        )?;

        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    fn data_contract_with_dynamic_properties() -> DataContract {
        let data_contract = json!({
            "protocolVersion" :0,
            "$id" : vec![0_u8;32],
            "$schema" : "schema",
            "version" : 0,
            "ownerId" : vec![0_u8;32],
            "documents" : {
                "test" : {
                    "properties" : {
                        "alphaIdentifier" :  {
                            "type": "array",
                            "byteArray": true,
                            "contentMediaType": "application/x.dash.dpp.identifier",
                        },
                        "alphaBinary" :  {
                            "type": "array",
                            "byteArray": true,
                        }
                    }
                }
            }
        });
        DataContract::from_raw_object(data_contract).unwrap()
    }

    #[test]
    fn convert_to_json_with_dynamic_binary_paths() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_value = vec![10_u8; 32];
        let id = vec![11_u8; 32];
        let data_contract_id = vec![13_u8; 32];
        let entropy = vec![14_u8; 32];

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

        let transition: DocumentCreateTransition =
            DocumentCreateTransition::from_raw_object(raw_document, data_contract).unwrap();

        let json_transition = transition.to_json().expect("no errors");
        assert_eq!(
            json_transition["$id"],
            JsonValue::String(bs58::encode(&id).into_string())
        );
        assert_eq!(
            json_transition["$dataContractId"],
            JsonValue::String(bs58::encode(&data_contract_id).into_string())
        );
        assert_eq!(
            json_transition["alphaBinary"],
            JsonValue::String(base64::encode(&alpha_value))
        );
        assert_eq!(
            json_transition["alphaIdentifier"],
            JsonValue::String(bs58::encode(&alpha_value).into_string())
        );
        assert_eq!(
            json_transition["$entropy"],
            JsonValue::String(base64::encode(&entropy))
        );
    }

    #[test]
    fn covert_to_object_with_dynamic_binary_paths() {
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
            DocumentCreateTransition::from_raw_object(raw_document, data_contract).unwrap();

        let object_transition = document.to_object().expect("no errors");
        assert_eq!(object_transition.get_bytes("$id").unwrap(), id);
        assert_eq!(
            object_transition.get_bytes("$dataContractId").unwrap(),
            data_contract_id
        );
        assert_eq!(
            object_transition.get_bytes("alphaBinary").unwrap(),
            alpha_value
        );
        assert_eq!(
            object_transition.get_bytes("alphaIdentifier").unwrap(),
            alpha_value
        );
    }
}
