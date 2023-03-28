use std::convert::TryInto;

use anyhow::Result;
use async_trait::async_trait;

use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionAction;
use crate::validation::ValidationResult;
use crate::{
    data_contract::{
        state_transition::data_contract_create_transition::DataContractCreateTransition,
        DataContract,
    },
    errors::StateError,
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
    validation::AsyncDataValidator,
    ProtocolError,
};

pub struct DataContractCreateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncDataValidator for DataContractCreateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = DataContractCreateTransition;
    type ResultItem = DataContractCreateTransitionAction;

    async fn validate(
        &self,
        data: &DataContractCreateTransition,
    ) -> Result<ValidationResult<Self::ResultItem>, ProtocolError> {
        validate_data_contract_create_transition_state(&self.state_repository, data).await
    }
}

impl<SR> DataContractCreateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> DataContractCreateTransitionStateValidator<SR>
    where
        SR: StateRepositoryLike,
    {
        DataContractCreateTransitionStateValidator { state_repository }
    }
}

pub async fn validate_data_contract_create_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DataContractCreateTransition,
) -> Result<ValidationResult<DataContractCreateTransitionAction>, ProtocolError> {
    // Data contract shouldn't exist
    let maybe_existing_data_contract: Option<DataContract> = state_repository
        .fetch_data_contract(
            &state_transition.data_contract.id,
            state_transition.get_execution_context(),
        )
        .await?
        .map(TryInto::try_into)
        .transpose()
        .map_err(Into::into)?;

    if maybe_existing_data_contract.is_none()
        || state_transition.get_execution_context().is_dry_run()
    {
        let action: DataContractCreateTransitionAction = state_transition.into();
        Ok(action.into())
    } else {
        Ok(ValidationResult::new_with_errors(vec![
            StateError::DataContractAlreadyPresentError {
                data_contract_id: state_transition.data_contract.id.to_owned(),
            }
            .into(),
        ]))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        state_repository::MockStateRepositoryLike, tests::fixtures::get_data_contract_fixture,
    };

    use super::*;

    #[tokio::test]
    async fn should_return_valid_result_on_dry_run() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let data_contract = get_data_contract_fixture(None);
        let state_transition = &DataContractCreateTransition {
            entropy: data_contract.entropy,
            data_contract,
            ..Default::default()
        };

        state_repository_mock
            .expect_fetch_data_contract()
            .return_once(|_, _| Ok(None));
        state_transition.execution_context.enable_dry_run();

        let result = validate_data_contract_create_transition_state(
            &state_repository_mock,
            state_transition,
        )
        .await
        .expect("should return validation result");

        assert!(result.is_valid());
    }
}
