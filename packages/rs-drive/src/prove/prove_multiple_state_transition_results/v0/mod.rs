use crate::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{IdentityBasedVoteDriveQuery, SingleDocumentDriveQuery};

use dpp::version::PlatformVersion;
use grovedb::{PathQuery, TransactionArg};
use itertools::{Either, Itertools};

impl Drive {
    /// This function query requested identities, documents and contracts and provide cryptographic proofs
    ///
    /// # Parameters
    /// - `identity_queries`: A list of [IdentityDriveQuery]. These specify the identities
    ///   to be proven.
    /// - `contract_ids`: A list of Data Contract IDs to prove
    /// - `document_queries`: A list of [SingleDocumentDriveQuery]. These specify the documents
    ///   to be proven.
    /// - `vote_queries`: A list of [IdentityBasedVoteDriveQuery]. These would be to figure out the
    ///   result of votes based on identities making them.
    /// - `transaction`: An optional grovedb transaction
    /// - `platform_version`: A reference to the [PlatformVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    #[inline(always)]
    pub(crate) fn prove_multiple_state_transition_results_v0(
        &self,
        identity_queries: &[IdentityDriveQuery],
        contract_ids: &[([u8; 32], Option<bool>)], //bool is history
        document_queries: &[SingleDocumentDriveQuery],
        vote_queries: &[IdentityBasedVoteDriveQuery],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut path_queries = vec![];
        if !identity_queries.is_empty() {
            for identity_query in identity_queries {
                match identity_query.prove_request_type {
                    IdentityProveRequestType::FullIdentity => {
                        path_queries.push(Self::full_identity_query(
                            &identity_query.identity_id,
                            &platform_version.drive.grove_version,
                        )?);
                    }
                    IdentityProveRequestType::Balance => {
                        path_queries.push(Self::balance_for_identity_id_query(
                            identity_query.identity_id,
                        ));
                    }
                    IdentityProveRequestType::Keys => {
                        path_queries.push(Self::identity_all_keys_query(
                            &identity_query.identity_id,
                            &platform_version.drive.grove_version,
                        )?);
                    }
                    IdentityProveRequestType::Revision => {
                        path_queries
                            .push(Self::identity_revision_query(&identity_query.identity_id));
                    }
                }
            }
        }

        let (contract_ids, historical_contract_ids): (Vec<_>, Vec<_>) = contract_ids
            .iter()
            .partition_map(|(contract_id, historical)| {
                // TODO: implement None
                let history = historical.unwrap_or(false);
                if !history {
                    Either::Left(*contract_id)
                } else {
                    Either::Right(*contract_id)
                }
            });

        if !contract_ids.is_empty() {
            let mut path_query =
                Self::fetch_non_historical_contracts_query(contract_ids.as_slice());
            path_query.query.limit = None;
            path_queries.push(path_query);
        }

        if !historical_contract_ids.is_empty() {
            let mut path_query =
                Self::fetch_historical_contracts_query(historical_contract_ids.as_slice());
            path_query.query.limit = None;
            path_queries.push(path_query);
        }
        if !document_queries.is_empty() {
            path_queries.extend(document_queries.iter().filter_map(|drive_query| {
                // The path query construction can only fail in extremely rare circumstances.
                let mut path_query = drive_query.construct_path_query(platform_version).ok()?;
                path_query.query.limit = None;
                Some(path_query)
            }));
        }

        if !vote_queries.is_empty() {
            path_queries.extend(vote_queries.iter().filter_map(|vote_query| {
                // The path query construction can only fail if the serialization fails.
                // Because the serialization will pretty much never fail, we can do this.
                let mut path_query = vote_query.construct_path_query().ok()?;
                path_query.query.limit = None;
                Some(path_query)
            }));
        }

        let path_query = PathQuery::merge(
            path_queries.iter().collect(),
            &platform_version.drive.grove_version,
        )
        .map_err(Error::GroveDB)?;

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
