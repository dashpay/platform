use serde::Serialize;
use crate::ProtocolError;
use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::documents_batch_transition::document_base_transition::JsonValue;
use crate::state_transition::state_transition_helpers;


/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionJsonConvert: Serialize + Signable + PlatformSerializable {
    /// Returns the [`serde_json::Value`] instance that encodes:
    ///  - Identifiers  - with base58
    ///  - Binary data  - with base64
    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let skip_signature_paths = if skip_signature {
            Self::signature_property_paths()
        } else {
            vec![]
        };
        state_transition_helpers::to_json(self, skip_signature_paths)
    }
}