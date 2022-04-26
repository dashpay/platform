use crate::{mocks, state_repository::StateRepositoryLike};
use anyhow::Result;

pub struct ApplyDataContractCreateTransitionFactory<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

pub fn fetch_documents_factory<SR>(
    state_repository: SR,
) -> ApplyDataContractCreateTransitionFactory<SR>
where
    SR: StateRepositoryLike,
{
    ApplyDataContractCreateTransitionFactory { state_repository }
}

impl<SR> ApplyDataContractCreateTransitionFactory<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn apply_data_contract_create_transition(
        &self,
        state_transition: mocks::StateTransition,
    ) -> Result<()> {
        self.state_repository
            .store_data_contract(state_transition.data_contract)
            .await
    }
}
