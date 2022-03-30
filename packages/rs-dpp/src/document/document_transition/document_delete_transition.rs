use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{data_contract::DataContract, errors::ProtocolError};

use super::{Action, DocumentBaseTransition, DocumentTransitionObjectLike};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentDeleteTransition {
    #[serde(flatten)]
    base: DocumentBaseTransition,
}

impl DocumentTransitionObjectLike for DocumentDeleteTransition {
    fn from_raw_document(
        raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut base = DocumentBaseTransition::from_raw_document(raw_transition, data_contract)?;
        base.action = Action::Delete;
        Ok(DocumentDeleteTransition { base })
    }

    fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        self.base.to_object()
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let value = serde_json::to_value(&self)?;
        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::document::document_transition::Action;
    use log::trace;

    #[test]
    fn test_deserialize_serialize_to_json() {
        let transition_json = r#"{
					"$id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
					"$type": "note",
					"$action": 3,
					"$dataContractId": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8"
				}"#;

        let cdt: DocumentDeleteTransition =
            serde_json::from_str(transition_json).expect("no error");
        trace!("the parsed Document Delete Transition is {:#?}", cdt);

        assert_eq!(cdt.base.action, Action::Delete);
        assert_eq!(cdt.base.document_type, "note");

        let mut json_no_whitespace = transition_json.to_string();
        json_no_whitespace.retain(|v| !v.is_whitespace());

        assert_eq!(cdt.to_json().unwrap().to_string(), json_no_whitespace);
    }
}
