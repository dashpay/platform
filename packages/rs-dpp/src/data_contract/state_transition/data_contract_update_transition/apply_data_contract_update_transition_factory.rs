use anyhow::Result;
use std::sync::Arc;

use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;

use super::DataContractUpdateTransition;

#[derive(Clone)]
pub struct ApplyDataContractUpdateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
}

impl<SR: StateRepositoryLike> ApplyDataContractUpdateTransition<SR> {
    pub fn new(state_repository: Arc<SR>) -> Self {
        ApplyDataContractUpdateTransition { state_repository }
    }
}

impl<SR> ApplyDataContractUpdateTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn apply_data_contract_update_transition(
        &self,
        state_transition: &DataContractUpdateTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.state_repository
            .update_data_contract(
                state_transition.data_contract().clone(),
                Some(execution_context),
            )
            .await
    }
}
