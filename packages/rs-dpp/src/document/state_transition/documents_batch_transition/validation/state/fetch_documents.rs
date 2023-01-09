use std::{
    collections::hash_map::{Entry, HashMap},
    convert::TryInto,
};

use futures::future::join_all;
use itertools::Itertools;
use serde_json::json;

use crate::{
    document::{document_transition::DocumentTransition, Document},
    get_from_transition,
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    util::string_encoding::Encoding,
    ProtocolError,
};

pub async fn fetch_documents(
    state_repository: &impl StateRepositoryLike,
    document_transitions: impl IntoIterator<Item = impl AsRef<DocumentTransition>>,
    execution_context: &StateTransitionExecutionContext,
) -> Result<Vec<Document>, anyhow::Error> {
    let mut transitions_by_contracts_and_types: HashMap<String, Vec<&DocumentTransition>> =
        HashMap::new();
    let collected_transitions: Vec<_> = document_transitions.into_iter().collect();

    for dt in collected_transitions.iter() {
        let document_transition = dt.as_ref();
        let document_type = get_from_transition!(document_transition, document_type);
        let data_contract_id = get_from_transition!(document_transition, data_contract_id);
        let unique_key = format!("{}{}", data_contract_id, document_type);

        match transitions_by_contracts_and_types.entry(unique_key) {
            Entry::Vacant(v) => {
                v.insert(vec![document_transition]);
            }
            Entry::Occupied(mut o) => o.get_mut().push(document_transition),
        }
    }

    // let mut fetch_documents_futures = vec![];
    let mut documents = vec![];
    for (_, dts) in transitions_by_contracts_and_types {
        let ids: Vec<String> = dts
            .iter()
            .map(|dt| get_from_transition!(dt, id).to_string(Encoding::Base58))
            .collect();

        let options = json!({
            "where" : [["$id", "in", ids ]],
            "orderBy" : [[ "$id", "asc"]],
        });

        // let future = state_repository.fetch_documents(
        //     get_from_transition!(dts[0], data_contract_id),
        //     get_from_transition!(dts[0], document_type),
        //     options,
        //     execution_context,
        // );
        // fetch_documents_futures.push(future);
        let current_documents_data = state_repository
            .fetch_documents(
                get_from_transition!(dts[0], data_contract_id),
                get_from_transition!(dts[0], document_type),
                options,
                execution_context,
            )
            .await?;
        let current_documents: Vec<Document> = current_documents_data
            .into_iter()
            .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
            .try_collect()?;

        documents.extend(current_documents);
        // fetch_documents_futures.push(future);
    }

    // let results: Result<Vec<Vec<Document>>, anyhow::Error> = join_all(fetch_documents_futures)
    //     .await
    //     .into_iter()
    //     .collect();

    // let documents = results?.into_iter().flatten().collect();
    Ok(documents)
}

//  TODO spec for fetchDocumentsFactory
