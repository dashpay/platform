use anyhow::Result;
use std::sync::Arc;

use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;

use super::DataContractCreateTransition;

#[derive(Clone)]
pub struct ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
}

impl<SR: StateRepositoryLike> ApplyDataContractCreateTransition<SR> {
    pub fn new(state_repository: Arc<SR>) -> Self {
        ApplyDataContractCreateTransition { state_repository }
    }
}

impl<SR> ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn apply_data_contract_create_transition(
        &self,
        state_transition: &DataContractCreateTransition,
        execution_context: Option<&StateTransitionExecutionContext>,
    ) -> Result<()> {
        self.state_repository
            .create_data_contract(state_transition.data_contract().clone(), execution_context)
            .await
    }
}
