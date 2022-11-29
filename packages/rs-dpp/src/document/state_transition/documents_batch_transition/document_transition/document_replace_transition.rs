use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    errors::ProtocolError,
    util::json_value::{self, ReplaceWith},
};

use super::{Action, DocumentBaseTransition, DocumentTransitionObjectLike};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReplaceTransition {
    #[serde(flatten)]
    pub base: DocumentBaseTransition,
    #[serde(rename = "$revision")]
    pub revision: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

impl DocumentTransitionObjectLike for DocumentReplaceTransition {
    fn from_json_str(json_str: &str, data_contract: DataContract) -> Result<Self, ProtocolError> {
        let mut document: DocumentReplaceTransition = serde_json::from_str(json_str)?;
        document.base.action = Action::Replace;
        document.base.data_contract_id = data_contract.id.clone();
        document.base.data_contract = data_contract;
        Ok(document)
    }

    fn from_raw_document(
        mut raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<DocumentReplaceTransition, ProtocolError> {
        DocumentBaseTransition::identifiers_to_strings(&mut raw_transition)?;

        let mut document: DocumentReplaceTransition = serde_json::from_value(raw_transition)?;
        document.base.action = Action::Replace;
        document.base.data_contract = data_contract;

        if let Some(ref mut dynamic_data) = document.data {
            json_value::identifiers_to(
                document
                    .base
                    .data_contract
                    .get_binary_properties(&document.base.document_type)?,
                dynamic_data,
                ReplaceWith::Base58,
            )?;
        }

        Ok(document)
    }

    fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        let object_base = self.base.to_object()?;
        let mut object = serde_json::to_value(&self)?;
        let object_base_map = object_base.as_object().unwrap().to_owned();

        json_value::identifiers_to(
            self.base
                .data_contract
                .get_binary_properties(&self.base.document_type)?,
            &mut object,
            ReplaceWith::Bytes,
        )?;

        match object {
            JsonValue::Object(ref mut o) => o.extend(object_base_map),
            _ => return Err("The Document Base Transaction isn't an Object".into()),
        }

        Ok(object)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = serde_json::to_value(&self)?;
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
