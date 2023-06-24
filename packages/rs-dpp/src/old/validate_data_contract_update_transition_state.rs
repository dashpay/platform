use std::convert::TryInto;

use anyhow::Result;
use async_trait::async_trait;

use crate::consensus::basic::data_contract::InvalidDataContractVersionError;
use crate::consensus::basic::document::DataContractNotPresentError;
use crate::data_contract::state_transition::data_contract_update_transition::action::DataContractUpdateTransitionAction;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::{AsyncDataValidator, ConsensusValidationResult};
use crate::{
    data_contract::{
        state_transition::data_contract_update_transition::DataContractUpdateTransition,
        DataContract,
    },
    errors::consensus::basic::BasicError,
    state_repository::StateRepositoryLike,
    ProtocolError,
};

pub struct DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncDataValidator for DataContractUpdateTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = DataContractUpdateTransition;
    type ResultItem = DataContractUpdateTransitionAction;
    async fn validate(
        &self,
        data: &Self::Item,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<Self::ResultItem>, ProtocolError> {
        validate_data_contract_update_transition_state(
            &self.state_repository,
            data,
            execution_context,
        )
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
    execution_context: &StateTransitionExecutionContext,
) -> Result<ConsensusValidationResult<DataContractUpdateTransitionAction>, ProtocolError> {
    let mut result = ConsensusValidationResult::<DataContractUpdateTransitionAction>::default();

    // Data contract should exist
    let maybe_existing_data_contract: Option<DataContract> = state_repository
        .fetch_data_contract(
            &state_transition.data_contract().id,
            Some(execution_context),
        )
        .await?
        .map(TryInto::try_into)
        .transpose()
        .map_err(Into::into)?;

    if execution_context.is_dry_run() {
        let action: DataContractUpdateTransitionAction = state_transition.into();
        return Ok(action.into());
    }

    let existing_data_contract: DataContract = match maybe_existing_data_contract {
        None => {
            let err = DataContractNotPresentError::new(state_transition.data_contract().id);
            result.add_error(err);
            return Ok(result);
        }
        Some(dc) => dc,
    };

    // Version difference should be exactly 1
    let old_version = existing_data_contract.version;
    let new_version = state_transition.data_contract().version;

    if new_version < old_version || new_version - old_version != 1 {
        let err = BasicError::InvalidDataContractVersionError(
            InvalidDataContractVersionError::new(old_version + 1, new_version),
        );
        result.add_error(err);
        Ok(result)
    } else {
        let action: DataContractUpdateTransitionAction = state_transition.into();
        Ok(action.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data_contract::state_transition::data_contract_update_transition::v0::DataContractUpdateTransitionV0;
    use crate::{
        data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition,
        state_repository::MockStateRepositoryLike, tests::fixtures::get_data_contract_fixture,
    };

    #[tokio::test]
    async fn should_return_valid_result_on_dry_run() {
        let data_contract = get_data_contract_fixture(None).data_contract;
        let state_transition = DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
            data_contract,
            ..Default::default()
        });
        let mut mock_state_repository = MockStateRepositoryLike::new();

        mock_state_repository
            .expect_fetch_data_contract()
            .return_once(|_, _| Ok(None));

        let execution_context = StateTransitionExecutionContext::default();
        execution_context.enable_dry_run();

        let result = validate_data_contract_update_transition_state(
            &mock_state_repository,
            &state_transition,
            &execution_context,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid());
    }

    // TODO! - rest of test
}
