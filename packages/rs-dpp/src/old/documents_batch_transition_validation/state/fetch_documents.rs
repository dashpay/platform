use std::{
    collections::hash_map::{Entry, HashMap},
    convert::TryInto,
};

use futures::future::join_all;
use itertools::Itertools;
use platform_value::platform_value;
use platform_value::string_encoding::Encoding;

use crate::document::{Document, ExtendedDocument};
use crate::{
    document::document_transition::DocumentTransition, get_from_transition,
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    ProtocolError,
};

pub async fn fetch_documents(
    state_repository: &impl StateRepositoryLike,
    document_transitions: &[&DocumentTransition],
    execution_context: &StateTransitionExecutionContext,
) -> Result<Vec<Document>, anyhow::Error> {
    let mut transitions_by_contracts_and_types: HashMap<String, Vec<&DocumentTransition>> =
        HashMap::new();

    for document_transition in document_transitions {
        let document_type = get_from_transition!(document_transition, document_type_name);
        let data_contract_id = get_from_transition!(document_transition, data_contract_id);
        let unique_key = format!("{}{}", data_contract_id, document_type);

        match transitions_by_contracts_and_types.entry(unique_key) {
            Entry::Vacant(v) => {
                v.insert(vec![document_transition]);
            }
            Entry::Occupied(mut o) => o.get_mut().push(document_transition),
        }
    }

    let mut fetch_documents_futures = vec![];
    for (_, dts) in transitions_by_contracts_and_types {
        let ids: Vec<[u8; 32]> = dts
            .iter()
            .map(|dt| get_from_transition!(dt, id).to_buffer())
            .collect();

        let options = platform_value!({
            "where" : [["$id", "in", ids ]],
            "orderBy" : [[ "$id", "asc"]],
        });

        let future = state_repository.fetch_documents(
            get_from_transition!(dts[0], data_contract_id),
            get_from_transition!(dts[0], document_type_name),
            options,
            Some(execution_context),
        );

        fetch_documents_futures.push(future);
    }
    let results = join_all(fetch_documents_futures).await;

    let mut documents = vec![];
    for result in results.into_iter() {
        let result = result?;
        let documents_from_fetch: Vec<Document> = result
            .into_iter()
            .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
            .try_collect()?;
        documents.extend(documents_from_fetch)
    }

    Ok(documents)
}

pub async fn fetch_extended_documents(
    state_repository: &impl StateRepositoryLike,
    document_transitions: &[&DocumentTransition],
    execution_context: &StateTransitionExecutionContext,
) -> Result<Vec<ExtendedDocument>, anyhow::Error> {
    let mut transitions_by_contracts_and_types: HashMap<String, Vec<&DocumentTransition>> =
        HashMap::new();

    for document_transition in document_transitions {
        let document_type = get_from_transition!(document_transition, document_type_name);
        let data_contract_id = get_from_transition!(document_transition, data_contract_id);
        let unique_key = format!("{}{}", data_contract_id, document_type);

        match transitions_by_contracts_and_types.entry(unique_key) {
            Entry::Vacant(v) => {
                v.insert(vec![document_transition]);
            }
            Entry::Occupied(mut o) => o.get_mut().push(document_transition),
        }
    }

    let mut fetch_documents_futures = vec![];
    for (_, dts) in transitions_by_contracts_and_types {
        let ids: Vec<String> = dts
            .iter()
            .map(|dt| get_from_transition!(dt, id).to_string(Encoding::Base58))
            .collect();

        let options = platform_value!({
            "where" : [["$id", "in", ids ]],
            "orderBy" : [[ "$id", "asc"]],
        });

        let future = state_repository.fetch_extended_documents(
            get_from_transition!(dts[0], data_contract_id),
            get_from_transition!(dts[0], document_type_name),
            options,
            Some(execution_context),
        );

        fetch_documents_futures.push(future);
    }
    let results = join_all(fetch_documents_futures).await;

    let mut documents = vec![];
    for result in results.into_iter() {
        let result = result?;
        let documents_from_fetch: Vec<ExtendedDocument> = result
            .into_iter()
            .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
            .try_collect()?;
        documents.extend(documents_from_fetch)
    }

    Ok(documents)
}
