use anyhow::Result;

use crate::{state_repository::StateRepositoryLike, state_transition::StateTransitionLike};

use super::DataContractUpdateTransition;

pub struct ApplyDataContractUpdateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR: StateRepositoryLike> ApplyDataContractUpdateTransition<SR> {
    pub fn new(state_repository: SR) -> Self {
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
    ) -> Result<()> {
        self.state_repository
            .store_data_contract(
                state_transition.data_contract.clone(),
                state_transition.get_execution_context(),
            )
            .await
    }
}
