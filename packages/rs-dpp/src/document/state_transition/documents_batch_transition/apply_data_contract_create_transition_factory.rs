use crate::{
    mocks,
    state_repository::{SMLStoreLike, SimplifiedMNListLike, StateRepositoryLike},
};
use anyhow::Result;
use std::marker::PhantomData;

pub struct ApplyDataContractCreateTransitionFactory<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    state_repository: SR,
    _phantom_l: PhantomData<L>,
    _phantom_s: PhantomData<S>,
}

pub fn fetch_documents_factory<SR, S, L>(
    state_repository: SR,
) -> ApplyDataContractCreateTransitionFactory<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    ApplyDataContractCreateTransitionFactory {
        state_repository,
        _phantom_l: PhantomData,
        _phantom_s: PhantomData,
    }
}

impl<SR, S, L> ApplyDataContractCreateTransitionFactory<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
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
