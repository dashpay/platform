use futures::future::join_all;
use serde_json::json;

use crate::{
    document::{document_transition::DocumentTransition, Document},
    get_from_transition,
    state_repository::{SMLStoreLike, SimplifiedMNListLike, StateRepositoryLike},
    util::string_encoding::Encoding,
};

use std::{
    collections::hash_map::{Entry, HashMap},
    marker::PhantomData,
};

pub struct FetchDocumentsFactory<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    state_repository: SR,
    _phantom_l: PhantomData<L>,
    _phantom_s: PhantomData<S>,
}

pub fn fetch_documents_factory<SR, S, L>(state_repository: SR) -> FetchDocumentsFactory<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    FetchDocumentsFactory {
        state_repository,
        _phantom_l: PhantomData,
        _phantom_s: PhantomData,
    }
}

impl<SR, S, L> FetchDocumentsFactory<SR, S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
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
