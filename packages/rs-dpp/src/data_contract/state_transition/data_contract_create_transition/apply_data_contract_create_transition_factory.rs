use anyhow::Result;

use crate::{state_repository::StateRepositoryLike, state_transition::StateTransitionLike};

use super::DataContractCreateTransition;

pub struct ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR: StateRepositoryLike> ApplyDataContractCreateTransition<SR> {
    pub fn new(state_repository: SR) -> Self {
        ApplyDataContractCreateTransition { state_repository }
    }
}

// ???
pub fn fetch_documents_factory<SR>(state_repository: SR) -> ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    ApplyDataContractCreateTransition::new(state_repository)
}

impl<SR> ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn apply_data_contract_create_transition(
        &self,
        state_transition: &DataContractCreateTransition,
    ) -> Result<()> {
        self.state_repository
            .store_data_contract(
                state_transition.data_contract.clone(),
                state_transition.get_execution_context(),
            )
            .await
    }
}
