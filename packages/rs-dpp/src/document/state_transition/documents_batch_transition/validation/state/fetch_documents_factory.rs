use futures::future::join_all;
use serde_json::json;

use crate::{
    document::{document_transition::DocumentTransition, Document},
    get_from_transition,
    state_repository::StateRepositoryLike,
    util::string_encoding::Encoding,
};

use std::collections::hash_map::{Entry, HashMap};

pub struct DocumentsRepository<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

pub fn fetch_documents_factory<SR>(state_repository: SR) -> DocumentsRepository<SR>
where
    SR: StateRepositoryLike,
{
    DocumentsRepository { state_repository }
}

impl<SR> DocumentsRepository<SR>
where
    SR: StateRepositoryLike,
{
    pub async fn fetch_documents(
        &self,
        document_transitions: &[DocumentTransition],
    ) -> Vec<Result<Vec<Document>, anyhow::Error>> {
        let mut transitions_by_contracts_and_types: HashMap<String, Vec<&DocumentTransition>> =
            HashMap::new();

        for dt in document_transitions {
            let document_type = get_from_transition!(dt, document_type);
            let data_contract_id = get_from_transition!(dt, data_contract_id);
            let unique_key = format!("{}{}", data_contract_id, document_type);

            match transitions_by_contracts_and_types.entry(unique_key) {
                Entry::Vacant(v) => {
                    v.insert(vec![dt]);
                }
                Entry::Occupied(mut o) => o.get_mut().push(dt),
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
            });

            let documents = self.state_repository.fetch_documents(
                get_from_transition!(dts[0], data_contract_id),
                get_from_transition!(dts[0], document_type),
                options,
            );

            fetch_documents_futures.push(documents);
        }

        join_all(fetch_documents_futures).await
    }
}
