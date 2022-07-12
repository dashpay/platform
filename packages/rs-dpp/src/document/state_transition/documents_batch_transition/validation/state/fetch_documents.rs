use std::collections::hash_map::{Entry, HashMap};

use futures::future::join_all;
use serde_json::json;

use crate::{
    document::{document_transition::DocumentTransition, Document},
    get_from_transition,
    state_repository::StateRepositoryLike,
    util::string_encoding::Encoding,
};

pub struct DocumentsFetcher<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

pub fn fetch_documents_factory<SR>(state_repository: SR) -> DocumentsFetcher<SR>
where
    SR: StateRepositoryLike,
{
    DocumentsFetcher { state_repository }
}

impl<SR> DocumentsFetcher<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn fetch_documents(
        &self,
        document_transitions: &[DocumentTransition],
    ) -> Result<Vec<Document>, anyhow::Error> {
        fetch_documents(&self.state_repository, document_transitions).await
    }
}

pub async fn fetch_documents(
    state_repository: &impl StateRepositoryLike,
    document_transitions: impl IntoIterator<Item = impl AsRef<DocumentTransition>>,
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

    let mut fetch_documents_futures = vec![];
    for (_, dts) in transitions_by_contracts_and_types {
        let ids: Vec<String> = dts
            .iter()
            .map(|dt| get_from_transition!(dt, id).to_string(Encoding::Base58))
            .collect();

        let options = json!({
            "where" : [["$id", "in", ids ]],
            "orderBy" : [[ "$id", "asc"]],
        });

        let documents = state_repository.fetch_documents(
            get_from_transition!(dts[0], data_contract_id),
            get_from_transition!(dts[0], document_type),
            options,
        );
        fetch_documents_futures.push(documents);
    }

    let results: Result<Vec<Vec<Document>>, anyhow::Error> = join_all(fetch_documents_futures)
        .await
        .into_iter()
        .collect();

    let documents = results?.into_iter().flatten().collect();
    Ok(documents)
}

//  TODO spec for fetchDocumentsFactory
