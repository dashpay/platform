use crate::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::SingleDocumentDriveQuery;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// This function query requested identities, documents and contracts and provide cryptographic proofs
    ///
    /// # Parameters
    /// - `identity_queries`: A list of [IdentityDriveQuery]. These specify the identities
    ///   to be proven.
    /// - `contract_ids`: A list of Data Contract IDs to prove
    /// - `document_queries`: A list of [SingleDocumentDriveQuery]. These specify the documents
    ///   to be proven.
    /// - `transaction`: An optional grovedb transaction
    /// - `drive_version`: A reference to the [DriveVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    pub(super) fn prove_multiple_v0(
        &self,
        identity_queries: &Vec<IdentityDriveQuery>,
        contract_ids: &[[u8; 32]],
        document_queries: &Vec<SingleDocumentDriveQuery>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
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
            path_queries.push(Self::fetch_contracts_query(contract_ids)?);
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
        self.grove_get_proved_path_query(
            &path_query,
            verbose,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
