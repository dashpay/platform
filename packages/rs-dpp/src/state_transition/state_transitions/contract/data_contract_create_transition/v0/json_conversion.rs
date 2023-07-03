use std::convert::TryInto;
use crate::ProtocolError;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::documents_batch_transition::document_base_transition::JsonValue;
use crate::state_transition::StateTransitionJsonConvert;

impl StateTransitionJsonConvert for DataContractCreateTransitionV0 {}