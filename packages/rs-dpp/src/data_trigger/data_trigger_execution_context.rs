use std::marker::PhantomData;

use crate::{
    prelude::{DataContract, Identifier},
    state_repository::{SMLStoreLike, SimplifiedMNListLike, StateRepositoryLike},
};

// TODO consider using the reference to the StateRepository instance
#[derive(Clone, Debug)]
pub struct DataTriggerExecutionContext<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    pub state_repository: SR,
    pub owner_id: Identifier,
    pub data_contract: DataContract,

    _phantom_s: std::marker::PhantomData<S>,
    _phantom_l: std::marker::PhantomData<L>,
}

impl<SR, S, L> DataTriggerExecutionContext<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    /// Creates a new instance of DataTriggerExecutionContext
    pub fn new(state_repository: SR, owner_id: Identifier, data_contract: DataContract) -> Self {
        DataTriggerExecutionContext {
            state_repository,
            owner_id,
            data_contract,

            _phantom_l: PhantomData,
            _phantom_s: PhantomData,
        }
    }
}
