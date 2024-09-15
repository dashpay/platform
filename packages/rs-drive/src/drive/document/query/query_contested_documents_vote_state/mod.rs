use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use derive_more::From;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use grovedb::TransactionArg;

mod v0;

use crate::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
pub use v0::*;

/// Represents the outcome of a query to retrieve documents.
///
/// This enum provides versioning for the outcomes of querying documents.
/// As the system evolves, new versions of the outcome structure can be
/// added to this enum without breaking existing implementations.
#[derive(From, Debug)]
pub enum QueryContestedDocumentsVoteStateOutcome {
    /// Version 0 of the `QueryDocumentsOutcome`.
    ///
    /// This version contains a list of documents retrieved, the number of
    /// skipped documents, and the cost associated with the query.
    V0(QueryContestedDocumentsVoteStateOutcomeV0),
}

impl QueryContestedDocumentsVoteStateOutcomeV0Methods for QueryContestedDocumentsVoteStateOutcome {
    fn contenders(&self) -> &Vec<ContenderWithSerializedDocument> {
        match self {
            QueryContestedDocumentsVoteStateOutcome::V0(outcome) => outcome.contenders(),
        }
    }

    fn contenders_owned(self) -> Vec<ContenderWithSerializedDocument> {
        match self {
            QueryContestedDocumentsVoteStateOutcome::V0(outcome) => outcome.contenders_owned(),
        }
    }

    fn cost(&self) -> u64 {
        match self {
            QueryContestedDocumentsVoteStateOutcome::V0(outcome) => outcome.cost(),
        }
    }
}

impl Drive {
    /// Performs a specified drive query and returns the result, along with any skipped items and the cost.
    ///
    /// This function is used to execute a given [DriveQuery]. It has options to operate in a dry-run mode
    /// and supports different protocol versions. In case an epoch is specified, it calculates the fee.
    ///
    /// # Arguments
    ///
    /// * `query` - The [DriveQuery] being executed.
    /// * `epoch` - An `Option<&Epoch>`. If provided, it will be used to calculate the processing fee.
    /// * `dry_run` - If true, the function will not perform any actual operation and return a default `QueryDocumentsOutcome`.
    /// * `transaction` - The `TransactionArg` holding the transaction data.
    /// * `protocol_version` - An `Option<u32>` representing the protocol version. If not provided, the function falls back
    ///    to current or latest version.
    ///
    /// # Returns
    ///
    /// * `Result<QueryDocumentsOutcome, Error>` - Returns `QueryDocumentsOutcome` on success with the list of documents,
    ///    number of skipped items, and cost. If the operation fails, it returns an `Error`.
    pub fn query_contested_documents_vote_state(
        &self,
        query: ContestedDocumentVotePollDriveQuery,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryContestedDocumentsVoteStateOutcome, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .query_contested_documents_vote_state
        {
            0 => self.query_contested_documents_vote_state_v0(
                query,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_contested_documents_vote_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
        .map(|outcome| outcome.into())
    }
}
