use crate::{
    prelude::{DataContract, Identifier},
    state_repository::StateRepositoryLike,
};

// TODO consider using the reference to the StateRepository instance
#[derive(Clone, Debug)]
pub struct DataTriggerExecutionContext<SR>
where
    SR: StateRepositoryLike,
{
    pub state_repository: SR,
    pub owner_id: Identifier,
    pub data_contract: DataContract,
}

impl<SR> DataTriggerExecutionContext<SR>
where
    SR: StateRepositoryLike,
{
    /// Creates a new instance of DataTriggerExecutionContext
    pub fn new(state_repository: SR, owner_id: Identifier, data_contract: DataContract) -> Self {
        DataTriggerExecutionContext {
            state_repository,
            owner_id,
            data_contract,
        }
    }
}
