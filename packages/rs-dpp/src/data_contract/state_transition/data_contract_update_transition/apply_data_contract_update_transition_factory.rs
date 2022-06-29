use crate::state_repository::StateRepositoryLike;
use anyhow::Result;

use super::DataContractUpdateTransition;

pub struct ApplyDataContractUpdateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

pub fn fetch_documents_factory<SR>(state_repository: SR) -> ApplyDataContractUpdateTransition<SR>
where
    SR: StateRepositoryLike,
{
    ApplyDataContractUpdateTransition { state_repository }
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
            .store_data_contract(state_transition.data_contract.clone())
            .await
    }
}
