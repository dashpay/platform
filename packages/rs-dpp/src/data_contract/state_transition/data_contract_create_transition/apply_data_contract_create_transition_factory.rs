use anyhow::Result;

use crate::state_repository::StateRepositoryLike;

use super::DataContractCreateTransition;

pub struct ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

pub fn fetch_documents_factory<SR>(state_repository: SR) -> ApplyDataContractCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    ApplyDataContractCreateTransition { state_repository }
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
            .store_data_contract(state_transition.data_contract.clone())
            .await
    }
}
