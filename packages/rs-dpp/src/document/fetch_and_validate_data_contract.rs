use std::{convert::TryInto, sync::Arc};

use serde_json::Value as JsonValue;
use platform_value::Value;

use crate::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use crate::data_contract::state_transition::errors::MissingDataContractIdError;
use crate::{
    consensus::{basic::BasicError, ConsensusError},
    data_contract::DataContract,
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    util::json_value::JsonValueExt,
    validation::ValidationResult,
    ProtocolError,
};

use crate::document::extended_document::property_names;

pub struct DataContractFetcherAndValidator<ST> {
    state_repository: Arc<ST>,
}

impl<ST> Clone for DataContractFetcherAndValidator<ST> {
    fn clone(&self) -> Self {
        Self {
            state_repository: self.state_repository.clone(),
        }
    }
}

impl<ST> DataContractFetcherAndValidator<ST>
where
    ST: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<ST>) -> Self {
        Self { state_repository }
    }

    pub async fn validate_extended(
        &self,
        raw_extended_document: &Value,
    ) -> Result<ValidationResult<DataContract>, ProtocolError> {
        // TODO - stateTransitionExecutionContext shouldn't be created because it should be optional for
        // TODO all StateRepository queries
        let ctx = StateTransitionExecutionContext::default();
        fetch_and_validate_data_contract(
            self.state_repository.as_ref(),
            raw_extended_document,
            &ctx,
        )
        .await
    }
}

pub async fn fetch_and_validate_data_contract(
    state_repository: &impl StateRepositoryLike,
    raw_extended_document: &Value,
    execution_context: &StateTransitionExecutionContext,
) -> Result<ValidationResult<DataContract>, ProtocolError> {
    let mut validation_result = ValidationResult::<DataContract>::default();

    let id_bytes = if let Some(id_bytes) = raw_extended_document.get_optional_hash256(property_names::DATA_CONTRACT_ID).map_err(ProtocolError::ValueError)? {
        id_bytes
    } else {
        validation_result.add_error(ConsensusError::BasicError(Box::new(
            BasicError::MissingDataContractIdError(MissingDataContractIdError::new(
                raw_extended_document.clone(),
            )),
        )));
        return Ok(validation_result);
    };

    let data_contract_id = Identifier::from(id_bytes);

    let maybe_data_contract = state_repository
        .fetch_data_contract(&data_contract_id, execution_context)
        .await?
        .map(TryInto::try_into)
        .transpose()
        .map_err(Into::into)?;

    if let Some(data_contract) = maybe_data_contract {
        validation_result.set_data(data_contract);
    } else {
        let consensus_error =
            ConsensusError::BasicError(Box::new(BasicError::DataContractNotPresent {
                data_contract_id,
            }));
        validation_result.add_error(consensus_error);
    }

    Ok(validation_result)
}
