use std::collections::HashMap;

use dashcore::Block;
use serde_json::Value;

use crate::{
    document::Document, prelude::Identifier, state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike, ProtocolError,
};

use super::{
    document_transition::{
        Action, DocumentCreateTransition, DocumentReplaceTransition, DocumentTransition,
    },
    validation::state::fetch_documents::fetch_documents,
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

    let fetched_documents = fetch_documents(
        state_repository,
        replace_transitions,
        &state_transition.execution_context,
    )
    .await?;

    let fetched_documents_by_id: HashMap<&Identifier, &Document> =
        fetched_documents.iter().map(|dt| (&dt.id, dt)).collect();

    // since groveDB doesn't support parallel inserts, wee need to make them sequential

    for document_transition in state_transition.get_transitions() {
        match document_transition {
            DocumentTransition::Create(dt) => {
                let new_document = document_from_transition_create(dt, state_transition);
                state_repository
                    .create_document(&new_document, state_transition.get_execution_context())
                    .await?;
            }
            DocumentTransition::Replace(dt) => {
                let document = if state_transition.execution_context.is_dry_run() {
                    let latest_platform_block: Block = state_repository
                        .fetch_latest_platform_block_header()
                        .await?;
                    let timestamp_millis = (latest_platform_block.header.time * 1000) as i64;
                    document_from_transition_replace(dt, state_transition, timestamp_millis)
                } else {
                    let mut document = fetched_documents_by_id
                        .get(&dt.base.id)
                        .ok_or(ProtocolError::DocumentNotProvided {
                            document_transition: document_transition.clone(),
                        })?
                        .to_owned()
                        .clone();

                    document.revision = dt.revision;
                    document.data = dt.data.as_ref().unwrap_or(&Value::Null).clone();
                    document.updated_at = dt.updated_at;
                    document
                };
                state_repository
                    .update_document(&document, state_transition.get_execution_context())
                    .await?;
            }
            DocumentTransition::Delete(dt) => {
                state_repository
                    .remove_document(
                        &dt.base.data_contract,
                        &dt.base.document_type,
                        &dt.base.id,
                        state_transition.get_execution_context(),
                    )
                    .await?;
            }
        };
    }
    Ok(())
}
fn document_from_transition_create(
    document_create_transition: &DocumentCreateTransition,
    state_transition: &DocumentsBatchTransition,
) -> Document {
    // TODO cloning is costly. Probably the [`Document`] should have properties of type `Cov<'a, K>`
    Document {
        protocol_version: state_transition.protocol_version,
        id: document_create_transition.base.id.clone(),
        document_type: document_create_transition.base.document_type.clone(),
        data_contract_id: document_create_transition.base.data_contract_id.clone(),
        owner_id: state_transition.owner_id.clone(),
        data: document_create_transition
            .data
            .as_ref()
            .unwrap_or(&serde_json::Value::Null)
            .clone(),
        created_at: document_create_transition.created_at,
        updated_at: document_create_transition.updated_at,
        entropy: document_create_transition.entropy,
        revision: document_create_transition.get_revision(),
        metadata: None,

        //? In the JS implementation the `data_contract` property is completely omitted, what suggest we should make
        //? it optional. On the other end the `data_contract` seems obligatory as it's used by methods like `get_binary_properties()`
        data_contract: Default::default(),
    }
}

fn document_from_transition_replace(
    document_replace_transition: &DocumentReplaceTransition,
    state_transition: &DocumentsBatchTransition,
    created_at: i64,
) -> Document {
    // TODO cloning is costly. Probably the [`Document`] should have properties of type `Cov<'a, K>`
    Document {
        protocol_version: state_transition.protocol_version,
        id: document_replace_transition.base.id.clone(),
        document_type: document_replace_transition.base.document_type.clone(),
        data_contract_id: document_replace_transition.base.data_contract_id.clone(),
        owner_id: state_transition.owner_id.clone(),
        data: document_replace_transition
            .data
            .as_ref()
            .unwrap_or(&serde_json::Value::Null)
            .clone(),
        updated_at: document_replace_transition.updated_at,
        revision: document_replace_transition.revision,
        created_at: Some(created_at),
        metadata: None,

        //? In the JS implementation the `data_contract` and `entropy` properties are completely omitted, what suggest we should make
        //? them optional. On the other end the `data_contract` seems obligatory as it's used by methods like `get_binary_properties()`,
        //? Also, the `getEntropy()` in JS API always returns `Buffer`
        data_contract: Default::default(),
        entropy: Default::default(),
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use crate::{
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            Document, DocumentsBatchTransition,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::StateTransitionLike,
        tests::{
            fixtures::{
                get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture,
            },
            utils::{create_empty_block, generate_random_identifier_struct},
        },
    };

    use super::apply_documents_batch_transition;

    #[tokio::test]
    async fn should_fetch_latest_block_when_replace_and_dry_run_enabled() {
        let mut state_repository = MockStateRepositoryLike::new();

        let owner_id = generate_random_identifier_struct();
        let data_contract = get_data_contract_fixture(None);
        let documents = get_documents_fixture(data_contract.clone()).unwrap();
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
            .expect_fetch_documents::<Document>()
            .returning(|_, _, _, _| Ok(vec![]));
        state_repository
            .expect_update_document()
            .returning(|_, _| Ok(()));
        state_repository
            .expect_fetch_latest_platform_block_header()
            .returning(|| Ok(create_empty_block(None)));

        let result = apply_documents_batch_transition(&state_repository, &state_transition).await;
        assert!(result.is_ok());
    }
}
