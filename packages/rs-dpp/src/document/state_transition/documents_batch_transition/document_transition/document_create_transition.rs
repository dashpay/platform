use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    document::document_transition::Action,
    errors::ProtocolError,
    util::json_value::JsonValueExt,
    util::json_value::{self, ReplaceWith},
};

use super::{
    document_base_transition, merge_serde_json_values, DocumentBaseTransition,
    DocumentTransitionObjectLike,
};

pub const INITIAL_REVISION: u32 = 1;
pub const BINARY_FIELDS: [&str; 1] = ["$entropy"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCreateTransition {
    #[serde(flatten)]
    /// Document Base Transition
    pub base: DocumentBaseTransition,
    #[serde(rename = "$entropy")]
    /// Entropy used in creating the Document ID.
    pub entropy: [u8; 32],
    #[serde(rename = "$createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(rename = "$updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DocumentCreateTransition {
    pub fn get_revision(&self) -> u32 {
        INITIAL_REVISION
    }

    pub fn bytes_to_strings(
        raw_create_document_transition: &mut JsonValue,
    ) -> Result<(), ProtocolError> {
        raw_create_document_transition.replace_binary_paths(BINARY_FIELDS, ReplaceWith::Base64)?;
        Ok(())
    }
}

impl DocumentTransitionObjectLike for DocumentCreateTransition {
    fn from_json_object(
        json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut json_value = json_value;
        let document_type = json_value.get_string("$type")?;

        let (identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths(document_type);

        json_value.replace_binary_paths(
            binary_paths.into_iter().chain(BINARY_FIELDS),
            ReplaceWith::Bytes,
        )?;
        // only dynamic identifiers are being replaced with bytes.
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
        // only static identifiers are replaced, as the dynamic ones are stored as Arrays
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
            .get_identifiers_and_binary_paths(&self.base.document_type);

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
