use anyhow::Result;

use crate::{
    data_contract::{state_transition::DataContractUpdateTransition, DataContract},
    errors::consensus::basic::BasicError,
    state_repository::StateRepositoryLike,
    validation::ValidationResult,
};

pub struct DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR> DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> Self {
        DataContractUpdateTransitionStateValidator { state_repository }
    }

    pub async fn validate_data_contract_update_transition_state(
        &self,
        state_transition: &DataContractUpdateTransition,
    ) -> ValidationResult<()> {
        let mut result = ValidationResult::default();

        // Data contract should exist
        let maybe_existing_data_contract: Result<Option<DataContract>> = self
            .state_repository
            .fetch_data_contract(&state_transition.data_contract.id)
            .await;

        let existing_data_contract: DataContract = match maybe_existing_data_contract {
            // assumed the conservativeness for the validation. TBD: in the case of
            // general error we want to add the same result
            Ok(None) | Err(_) => {
                let err = BasicError::DataContractContPresent {
                    data_contract_id: state_transition.data_contract.id.clone(),
                };
                result.add_error(err);
                return result;
            }
            Ok(Some(dt)) => dt,
        };

        // Version difference should be exactly 1
        let old_version = existing_data_contract.version;
        let new_version = state_transition.data_contract.version;
        let version_diff = new_version - old_version;

        if version_diff != 1 {
            let err = BasicError::InvalidDataContractVersionError {
                expected_version: old_version + 1,
                version: old_version + version_diff,
            };
            result.add_error(err);
        }

        result
    }
}
