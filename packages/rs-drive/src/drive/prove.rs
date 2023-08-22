use crate::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// Given public key hashes, fetches full identities as proofs.
    pub fn prove_multiple(
        &self,
        identity_queries: &Vec<IdentityDriveQuery>,
        contract_ids: &[[u8; 32]],
        document_queries: &Vec<SingleDocumentDriveQuery>,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut path_queries = vec![];
        let mut count = 0;
        if !identity_queries.is_empty() {
            for identity_query in identity_queries {
                match identity_query.prove_request_type {
                    IdentityProveRequestType::FullIdentity => {
                        path_queries.push(Self::full_identity_query(&identity_query.identity_id)?);
                    }
                    IdentityProveRequestType::Balance => {
                        path_queries.push(Self::balance_for_identity_id_query(
                            identity_query.identity_id,
                        ));
                    }
                    IdentityProveRequestType::Keys => {
                        path_queries
                            .push(Self::identity_all_keys_query(&identity_query.identity_id)?);
                    }
                }
            }
            count += identity_queries.len();
        }
        if !contract_ids.is_empty() {
            let mut path_query = Self::fetch_non_historical_contracts_query(contract_ids);
            path_query.query.limit = None;
            path_queries.push(path_query);
            count += contract_ids.len();
        }
        if !document_queries.is_empty() {
            path_queries.extend(
                document_queries
                    .iter()
                    .map(|drive_query| drive_query.construct_path_query()),
            );
            count += document_queries.len();
        }
        let verbose = match count {
            0 => {
                return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                    "we are asking to prove nothing",
                )))
            }
            1 => false,
            _ => true,
        };
        let path_query = PathQuery::merge(path_queries.iter().collect()).map_err(Error::GroveDB)?;
        self.grove_get_proved_path_query(&path_query, verbose, transaction, &mut vec![])
    }
}
