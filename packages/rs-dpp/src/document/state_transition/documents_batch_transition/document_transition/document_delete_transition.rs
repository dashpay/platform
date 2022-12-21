use crate::{data_contract::DataContract, errors::ProtocolError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{document_base_transition::DocumentBaseTransition, DocumentTransitionObjectLike};

/// Identifier fields in [`DocumentDeleteTransition`]
pub use super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentDeleteTransition {
    #[serde(flatten)]
    pub base: DocumentBaseTransition,
}

impl DocumentTransitionObjectLike for DocumentDeleteTransition {
    fn from_json_object(
        json_value: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut document: DocumentDeleteTransition = serde_json::from_value(json_value)?;
        document.base.data_contract = data_contract;

        Ok(document)
    }

    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut base = DocumentBaseTransition::from_raw_object(raw_transition, data_contract)?;

        Ok(DocumentDeleteTransition { base })
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        self.base.to_object()
    }

    fn to_json(&self) -> Result<Value, ProtocolError> {
        self.base.to_json()
    }
}

#[cfg(test)]
mod test {
    use crate::document::document_transition::Action;

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
					"$action": 3,
					"$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8"
				}"#;

        let cdt: DocumentDeleteTransition =
            serde_json::from_str(transition_json).expect("no error");

        assert_eq!(cdt.base.action, Action::Delete);
        assert_eq!(cdt.base.document_type, "note");

        let mut json_no_whitespace = transition_json.to_string();
        json_no_whitespace.retain(|v| !v.is_whitespace());

        assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
    }
}
