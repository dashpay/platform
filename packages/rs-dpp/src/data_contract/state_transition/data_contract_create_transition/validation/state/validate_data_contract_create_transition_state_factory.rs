use crate::{
    errors::StateError, mocks, state_repository::StateRepositoryLike, validation::ValidationResult,
};
use anyhow::Result;

pub struct ValidateDataContractCreateTransitionStateFactory<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

pub fn fetch_documents_factory<SR>(
    state_repository: SR,
) -> ValidateDataContractCreateTransitionStateFactory<SR>
where
    SR: StateRepositoryLike,
{
    ValidateDataContractCreateTransitionStateFactory { state_repository }
}

impl<SR> ValidateDataContractCreateTransitionStateFactory<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn validate_data_contract_create_transition_state(
        &self,
        state_transition: mocks::StateTransition,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Data contract shouldn't exist
        let maybe_existing_data_contract: Result<Option<Vec<u8>>> = self
            .state_repository
            .fetch_data_contract(&state_transition.data_contract.id)
            .await;

        match maybe_existing_data_contract {
            Err(_) | Ok(None) => result.add_error(StateError::DataContractAlreadyPresentError {
                data_contract_id: state_transition.data_contract.id.clone(),
            }),
            _ => {}
        }

        result
    }
}
