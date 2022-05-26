use crate::{mocks, state_repository::StateRepositoryLike};

use super::validation::state::fetch_documents_factory::DocumentsRepository;

pub struct ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
    fetch_documents: DocumentsRepository<SR>,
}

pub fn apply_documents_batch_transition_factory<SR>(
    state_repository: SR,
    fetch_documents: DocumentsRepository<SR>,
) -> ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    ApplyDocumentsBatchTransition {
        state_repository,
        fetch_documents,
    }
}

impl<SR> ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn apply_documents_batch_transition(
        &self,
        _state_transition: mocks::DocumentsBatchTransition,
    ) {
    }
}
