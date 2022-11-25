use anyhow::Result;
use async_trait::async_trait;

use crate::{
    data_contract::{state_transition::DataContractUpdateTransition, DataContract},
    errors::consensus::basic::BasicError,
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
    validation::{AsyncDataValidator, SimpleValidationResult, ValidationResult},
    ProtocolError,
};

pub struct DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait]
impl<SR> AsyncDataValidator for DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = DataContractUpdateTransition;
    async fn validate(
        &self,
        state_transition: &DataContractUpdateTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        validate_data_contract_update_transition_state(&self.state_repository, state_transition)
            .await
    }
}

impl<SR> DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> Self {
        DataContractUpdateTransitionStateValidator { state_repository }
    }
}

pub async fn validate_data_contract_update_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DataContractUpdateTransition,
) -> Result<ValidationResult<()>, ProtocolError> {
    let mut result = ValidationResult::default();

    // Data contract should exist
    let maybe_existing_data_contract: Option<DataContract> = state_repository
        .fetch_data_contract(
            &state_transition.data_contract.id,
            state_transition.get_execution_context(),
        )
        .await?;

    if state_transition.execution_context.is_dry_run() {
        return Ok(result);
    }

    let existing_data_contract: DataContract = match maybe_existing_data_contract {
        None => {
            let err = BasicError::DataContractNotPresent {
                data_contract_id: state_transition.data_contract.id.clone(),
            };
            result.add_error(err);
            return Ok(result);
        }
        Some(dc) => dc,
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

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        data_contract::state_transition::DataContractUpdateTransition,
        state_repository::MockStateRepositoryLike, state_transition::StateTransitionLike,
        tests::fixtures::get_data_contract_fixture,
    };

    #[tokio::test]
    async fn should_return_valid_result_on_dry_run() {
        let data_contract = get_data_contract_fixture(None);
        let state_transition = DataContractUpdateTransition {
            data_contract,
            ..Default::default()
        };
        let mut mock_state_repository = MockStateRepositoryLike::new();

        mock_state_repository
            .expect_fetch_data_contract::<Option<DataContract>>()
            .return_once(|_, _| Ok(None));
        state_transition.get_execution_context().enable_dry_run();

        let result = validate_data_contract_update_transition_state(
            &mock_state_repository,
            &state_transition,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid());
    }

    // TODO! - rest of test
}
