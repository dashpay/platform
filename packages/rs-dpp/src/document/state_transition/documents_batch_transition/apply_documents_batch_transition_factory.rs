use platform_value::Value;
use std::collections::HashMap;

use crate::document::{Document, ExtendedDocument};
use crate::prelude::TimestampMillis;
use crate::{
    document::errors::DocumentError, prelude::Identifier, state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike, ProtocolError,
};

use super::{
    document_transition::{Action, DocumentReplaceTransition, DocumentTransition},
    validation::state::fetch_documents::fetch_extended_documents,
    DocumentsBatchTransition,
};

pub struct ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR> ApplyDocumentsBatchTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> ApplyDocumentsBatchTransition<SR>
    where
        SR: StateRepositoryLike,
    {
        ApplyDocumentsBatchTransition { state_repository }
    }

    pub async fn apply(
        &self,
        state_transition: &DocumentsBatchTransition,
    ) -> Result<(), ProtocolError> {
        apply_documents_batch_transition(&self.state_repository, state_transition).await
    }
}

pub async fn apply_documents_batch_transition(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DocumentsBatchTransition,
) -> Result<(), ProtocolError> {
    let replace_transitions = state_transition
        .get_transitions()
        .iter()
        .filter(|dt| dt.base().action == Action::Replace);

    let fetched_documents = fetch_extended_documents(
        state_repository,
        replace_transitions,
        &state_transition.execution_context,
    )
    .await?;

    let mut fetched_documents_by_id: HashMap<Identifier, ExtendedDocument> = fetched_documents
        .into_iter()
        .map(|dt| (dt.id(), dt))
        .collect();

    // since groveDB doesn't support parallel inserts, we need to make them sequential

    for document_transition in state_transition.get_transitions() {
        match document_transition {
            DocumentTransition::Create(document_create_transition) => {
                let document = document_create_transition
                    .to_document(state_transition.owner_id.to_buffer())?;
                //todo: eventually we should use Cow instead
                state_repository
                    .create_document(&document, state_transition.get_execution_context())
                    .await?;
            }
            DocumentTransition::Replace(document_replace_transition) => {
                if state_transition.execution_context.is_dry_run() {
                    let document = document_replace_transition.to_document_for_dry_run()?;
                    state_repository
                        .update_document(&document, state_transition.get_execution_context())
                        .await?;
                } else {
                    let document = fetched_documents_by_id
                        .get_mut(&document_replace_transition.base.id)
                        .ok_or(DocumentError::DocumentNotProvidedError {
                            document_transition: document_transition.clone(),
                        })?;
                    document_replace_transition.replace_extended_document(document)?;
                    state_repository
                        .update_document(document, state_transition.get_execution_context())
                        .await?;
                };
            }
            DocumentTransition::Delete(document_delete_transition) => {
                state_repository
                    .remove_document(
                        &document_delete_transition.base.data_contract,
                        &document_delete_transition.base.document_type,
                        &document_delete_transition.base.id,
                        state_transition.get_execution_context(),
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
    let property_value: Value = document_replace_transition
        .data
        .as_ref()
        .unwrap_or(&serde_json::Value::Null)
        .clone()
        .into();
    Ok(ExtendedDocument {
        protocol_version: state_transition.protocol_version,
        document_type_name: document_replace_transition.base.document_type.clone(),
        data_contract_id: document_replace_transition.base.data_contract_id,
        metadata: None,

        //? In the JS implementation the `data_contract` and `entropy` properties are completely omitted, what suggest we should make
        //? them optional. On the other end the `data_contract` seems obligatory as it's used by methods like `get_binary_properties()`,
        //? Also, the `getEntropy()` in JS API always returns `Buffer`
        data_contract: Default::default(),
        entropy: Default::default(),
        document: Document {
            id: document_replace_transition.base.id.buffer,
            owner_id: state_transition.owner_id.buffer,
            properties: property_value
                .into_btree_map()
                .map_err(ProtocolError::ValueError)?,
            revision: Some(document_replace_transition.revision),
            created_at: Some(created_at),
            updated_at: document_replace_transition.updated_at,
        },
    })
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use crate::tests::fixtures::get_extended_documents_fixture;

    use crate::{
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            DocumentsBatchTransition,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::StateTransitionLike,
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
        let data_contract = get_data_contract_fixture(None);
        let documents = get_extended_documents_fixture(data_contract.clone()).unwrap();
        let documents_transitions = get_document_transitions_fixture([
            (Action::Replace, documents),
            (Action::Create, vec![]),
        ]);
        let raw_document_transitions: Vec<Value> = documents_transitions
            .iter()
            .map(|dt| dt.to_object().unwrap())
            .collect();
        let owner_id_bytes = owner_id.to_buffer();
        let state_transition = DocumentsBatchTransition::from_raw_object(
            json!({
                "ownerId" : owner_id_bytes,
                "transitions" : raw_document_transitions,
            }),
            vec![data_contract.clone()],
        )
        .expect("documents batch state transition should be created");

        state_transition.get_execution_context().enable_dry_run();
        state_repository
            .expect_fetch_documents()
            .returning(|_, _, _, _| Ok(vec![]));
        state_repository
            .expect_update_document()
            .returning(|_, _| Ok(()));
        state_repository
            .expect_fetch_latest_platform_block_time()
            .returning(|| Ok(0));

        let result = apply_documents_batch_transition(&state_repository, &state_transition).await;
        assert!(result.is_ok());
    }
}
