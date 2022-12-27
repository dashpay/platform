use std::{convert::TryInto, sync::Arc};

use serde_json::Value;

use crate::{
    consensus::{basic::BasicError, ConsensusError},
    data_contract::DataContract,
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::{self, StateTransitionExecutionContext},
    util::json_value::JsonValueExt,
    validation::ValidationResult,
    ProtocolError,
};

use super::property_names;

pub struct DataContractFetcherAndValidator<ST> {
    state_repository: Arc<ST>,
}

impl<ST> DataContractFetcherAndValidator<ST>
where
    ST: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<ST>) -> Self {
        Self { state_repository }
    }

    pub async fn validate(
        &self,
        raw_document: &Value,
    ) -> Result<ValidationResult<DataContract>, ProtocolError> {
        // TODO - where the state transition context should be created?
        let ctx = StateTransitionExecutionContext::default();
        fetch_and_validate_data_contract(self.state_repository.as_ref(), raw_document, &ctx).await
    }
}

pub async fn fetch_and_validate_data_contract(
    state_repository: &impl StateRepositoryLike,
    raw_document: &Value,
    execution_context: &StateTransitionExecutionContext,
) -> Result<ValidationResult<DataContract>, ProtocolError> {
    let mut validation_result = ValidationResult::<DataContract>::default();

    let id_bytes = if let Ok(id_bytes) = raw_document.get_bytes(property_names::DATA_CONTRACT_ID) {
        id_bytes
    } else {
        validation_result.add_error(ConsensusError::BasicError(Box::new(
            BasicError::MissingDataContractIdError,
        )));
        return Ok(validation_result);
    };

    let data_contract_id = match Identifier::from_bytes(&id_bytes) {
        Ok(id) => id,

        Err(e) => {
            let id_base58 = bs58::encode(id_bytes).into_string();
            let consensus_error =
                ConsensusError::BasicError(Box::new(BasicError::InvalidIdentifierError {
                    identifier_name: id_base58,
                    error: e.to_string(),
                }));
            validation_result.add_error(consensus_error);
            return Ok(validation_result);
        }
    };

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
