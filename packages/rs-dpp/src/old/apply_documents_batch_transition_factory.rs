use std::collections::HashMap;
use std::sync::Arc;

use crate::document::{Document, ExtendedDocument};
use crate::prelude::TimestampMillis;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::{
    document::errors::DocumentError, prelude::Identifier, state_repository::StateRepositoryLike,
    ProtocolError,
};

use super::{
    document_transition::{DocumentReplaceTransition, DocumentTransition},
    validation::state::fetch_documents::fetch_extended_documents,
    DocumentsBatchTransition,
};

#[derive(Clone)]
pub struct ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
}

impl<SR> ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>) -> ApplyDocumentsBatchTransition<SR>
    where
        SR: StateRepositoryLike,
    {
        ApplyDocumentsBatchTransition { state_repository }
    }

    pub async fn apply(
        &self,
        state_transition: &DocumentsBatchTransition,
        execution_context: StateTransitionExecutionContext,
    ) -> Result<(), ProtocolError> {
        apply_documents_batch_transition(
            self.state_repository.as_ref(),
            state_transition,
            &execution_context,
        )
        .await
    }
}

pub async fn apply_documents_batch_transition(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DocumentsBatchTransition,
    execution_context: &StateTransitionExecutionContext,
) -> Result<(), ProtocolError> {
    let replace_transitions: Vec<_> = state_transition
        .transitions_slice()
        .iter()
        .filter(|dt| dt.base().action == Action::Replace)
        .collect();

    let fetched_documents = fetch_extended_documents(
        state_repository,
        replace_transitions.as_slice(),
        execution_context,
    )
    .await?;

    let mut fetched_documents_by_id: HashMap<Identifier, ExtendedDocument> = fetched_documents
        .into_iter()
        .map(|dt| (dt.id(), dt))
        .collect();

    // since groveDB doesn't support parallel inserts, we need to make them sequential

    for document_transition in state_transition.transitions() {
        match document_transition {
            DocumentTransition::Create(document_create_transition) => {
                let document =
                    document_create_transition.to_extended_document(state_transition.owner_id)?;
                //todo: eventually we should use Cow instead
                state_repository
                    .create_document(&document, Some(execution_context))
                    .await?;
            }
            DocumentTransition::Replace(document_replace_transition) => {
                if execution_context.is_dry_run() {
                    let document = document_replace_transition
                        .to_extended_document_for_dry_run(state_transition.owner_id)?;
                    state_repository
                        .update_document(&document, Some(execution_context))
                        .await?;
                } else {
                    let document = fetched_documents_by_id
                        .get_mut(&document_replace_transition.base.id)
                        .ok_or(DocumentError::DocumentNotProvidedError {
                            document_transition: document_transition.clone(),
                        })?;
                    document_replace_transition.replace_extended_document(document)?;
                    state_repository
                        .update_document(document, Some(execution_context))
                        .await?;
                };
            }
            DocumentTransition::Delete(document_delete_transition) => {
                state_repository
                    .remove_document(
                        &document_delete_transition.base.data_contract,
                        &document_delete_transition.base.document_type_name,
                        &document_delete_transition.base.id,
                        Some(execution_context),
                    )
                    .await?;
            }
        };
    }
    Ok(())
}

fn document_from_transition_replace(
    document_replace_transition: &DocumentReplaceTransition,
    state_transition: &DocumentsBatchTransition,
    created_at: TimestampMillis,
) -> Result<ExtendedDocument, ProtocolError> {
    // TODO cloning is costly. Probably the [`Document`] should have properties of type `Cow<'a, K>`
    Ok(ExtendedDocument {
        feature_version: state_transition.feature_version,
        document_type_name: document_replace_transition.base.document_type_name.clone(),
        data_contract_id: document_replace_transition.base.data_contract_id,
        metadata: None,

        //? In the JS implementation the `data_contract` and `entropy` properties are completely omitted, what suggest we should make
        //? them optional. On the other end the `data_contract` seems obligatory as it's used by methods like `get_binary_properties()`,
        //? Also, the `getEntropy()` in JS API always returns `Buffer`
        data_contract: Default::default(),
        entropy: Default::default(),
        document: Document {
            id: document_replace_transition.base.id,
            owner_id: state_transition.owner_id,
            properties: document_replace_transition.data.clone().unwrap_or_default(),
            revision: Some(document_replace_transition.revision),
            created_at: Some(created_at),
            updated_at: document_replace_transition.updated_at,
        },
    })
}

#[cfg(test)]
mod test {
    use platform_value::Value;

    use std::collections::BTreeMap;

    use crate::tests::fixtures::get_extended_documents_fixture;

    use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::{
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            DocumentsBatchTransition,
        },
        state_repository::MockStateRepositoryLike,
        tests::{
            fixtures::{get_data_contract_fixture, get_document_transitions_fixture},
            utils::generate_random_identifier_struct,
        },
    };

    use super::apply_documents_batch_transition;

    #[tokio::test]
    async fn should_fetch_latest_block_when_replace_and_dry_run_enabled() {
        let mut state_repository = MockStateRepositoryLike::new();

        let owner_id = generate_random_identifier_struct();
        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents = get_extended_documents_fixture(data_contract.clone()).unwrap();
        let documents_transitions = get_document_transitions_fixture([
            (DocumentTransitionActionType::Replace, documents),
            (DocumentTransitionActionType::Create, vec![]),
        ]);
        let raw_document_transitions: Vec<Value> = documents_transitions
            .iter()
            .map(|dt| dt.to_value_map().unwrap().into())
            .collect::<Vec<Value>>();
        let owner_id_bytes = owner_id.to_buffer();
        let mut map = BTreeMap::new();
        map.insert("ownerId".to_string(), Value::Identifier(owner_id_bytes));
        map.insert(
            "transitions".to_string(),
            Value::Array(raw_document_transitions),
        );
        let state_transition =
            DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
                .expect("documents batch state transition should be created");

        let execution_context = StateTransitionExecutionContext::default();
        execution_context.enable_dry_run();
        state_repository
            .expect_fetch_extended_documents()
            .returning(|_, _, _, _| Ok(vec![]));
        state_repository
            .expect_update_document()
            .returning(|_, _| Ok(()));
        state_repository
            .expect_fetch_latest_platform_block_time()
            .returning(|| Ok(0));

        let result = apply_documents_batch_transition(
            &state_repository,
            &state_transition,
            &execution_context,
        )
        .await;
        assert!(result.is_ok());
    }
}
