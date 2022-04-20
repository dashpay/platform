use crate::{
    data_contract::DataContract,
    document::document_transition::Action,
    errors::ProtocolError,
    util::deserializer::{self, parse_bytes},
    util::json_value::{self, ReplaceWith},
};

use super::{DocumentBaseTransition, DocumentTransitionObjectLike};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub const INITIAL_REVISION: usize = 1;

/**
 * @typedef {RawDocumentTransition & Object} RawDocumentCreateTransition
 * @property {Buffer} $entropy
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

/**
 * @typedef {JsonDocumentTransition & Object} JsonDocumentCreateTransition
 * @property {string} $entropy
 * @property {number} [$createdAt]
 * @property {number} [$updatedAt]
 */

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCreateTransition {
    #[serde(flatten)]
    /// Document Base Transition
    pub base: DocumentBaseTransition,
    #[serde(rename = "$entropy", with = "entropy_serde")]
    /// Entropy ised in creating the Document ID.
    pub entropy: [u8; 32],
    #[serde(rename = "$createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(rename = "$updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DocumentCreateTransition {
    pub fn bytes_to_strings(
        raw_create_document_transition: &mut JsonValue,
    ) -> Result<(), ProtocolError> {
        if let JsonValue::Object(ref mut o) = raw_create_document_transition {
            parse_bytes(o, &["$entropy"])?;
        } else {
            return Err("The raw_transition isn't an Object".into());
        }
        Ok(())
    }
}

impl DocumentTransitionObjectLike for DocumentCreateTransition {
    fn from_json_str(json_str: &str, data_contract: DataContract) -> Result<Self, ProtocolError> {
        let mut document: DocumentCreateTransition = serde_json::from_str(json_str)?;
        document.base.action = Action::Create;
        document.base.data_contract_id = data_contract.id.clone();
        document.base.data_contract = data_contract;
        Ok(document)
    }

    fn from_raw_document(
        mut raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<DocumentCreateTransition, ProtocolError> {
        DocumentBaseTransition::identifiers_to_strings(&mut raw_transition)?;
        Self::bytes_to_strings(&mut raw_transition)?;

        let mut document: DocumentCreateTransition = serde_json::from_value(raw_transition)?;
        document.base.action = Action::Create;
        document.base.data_contract_id = data_contract.id.clone();
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
        let entropy: Vec<JsonValue> = self.entropy.iter().map(|v| JsonValue::from(*v)).collect();

        json_value::identifiers_to(
            self.base
                .data_contract
                .get_binary_properties(&self.base.document_type)?,
            &mut object,
            ReplaceWith::Bytes,
        )?;

        match object {
            JsonValue::Object(ref mut o) => {
                o.insert(String::from("$entropy"), JsonValue::Array(entropy));
                o.extend(object_base_map)
            }
            _ => return Err("The Document Base Transaction isn't an Object".into()),
        }

        Ok(object)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = serde_json::to_value(&self)?;
        Ok(value)
    }
}

mod entropy_serde {
    use std::convert::TryInto;

    use serde::{Deserialize, Deserializer, Serializer};
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let data: String = Deserialize::deserialize(d)?;
        base64::decode(&data)
            .map_err(|e| {
                serde::de::Error::custom(format!("Unable to decode {}' with base64 - {}", data, e))
            })?
            .try_into()
            .map_err(|_| {
                serde::de::Error::custom(format!(
                    "Unable to convert the '{:?}' into 32 bytes array",
                    data
                ))
            })
    }

    pub fn serialize<S>(buffer: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&base64::encode(&buffer))
    }
}

#[cfg(test)]
mod test {
    use log::trace;
    use std::convert::TryInto;

    use super::*;

    #[test]
    fn test_deserialize_serialize_to_json() {
        let transition_json = r#"{
					"$id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
					"$type": "note",
					"$action": 0,
					"$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
					"$entropy": "WdkGW1qg2eM8FhldN6OmAyGzhfTsZR8grEUENlgBfH0=",
					"message": "example_message"
				}"#;

        let expect_entropy: [u8; 32] =
            base64::decode("WdkGW1qg2eM8FhldN6OmAyGzhfTsZR8grEUENlgBfH0=")
                .unwrap()
                .try_into()
                .unwrap();

        let cdt: DocumentCreateTransition =
            serde_json::from_str(transition_json).expect("no error");
        trace!("the parsed Document Create Transition is {:#?}", cdt);

        assert_eq!(cdt.base.action, Action::Create);
        assert_eq!(cdt.base.document_type, "note");
        assert_eq!(cdt.entropy, expect_entropy);
        assert_eq!(cdt.data.as_ref().unwrap()["message"], "example_message");

        let mut json_no_whitespace = transition_json.to_string();
        json_no_whitespace.retain(|v| !v.is_whitespace());

        assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
    }
}
