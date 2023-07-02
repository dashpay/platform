use std::convert::TryInto;
use crate::ProtocolError;
use crate::state_transition::abstract_state_transition::{StateTransitionJsonConvert, StateTransitionValueConvert};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::documents_batch_transition::document_base_transition::JsonValue;

impl StateTransitionJsonConvert for DataContractCreateTransitionV0 {
    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    }
}