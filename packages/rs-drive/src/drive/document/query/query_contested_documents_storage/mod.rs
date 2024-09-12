use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use derive_more::From;
use dpp::block::epoch::Epoch;
use dpp::document::Document;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use grovedb::TransactionArg;

mod v0;

use crate::query::drive_contested_document_query::DriveContestedDocumentQuery;
pub use v0::*;

/// Represents the outcome of a query to retrieve documents.
///
/// This enum provides versioning for the outcomes of querying documents.
/// As the system evolves, new versions of the outcome structure can be
/// added to this enum without breaking existing implementations.
#[derive(From, Debug)]
pub enum QueryContestedDocumentsOutcome {
    /// Version 0 of the `QueryDocumentsOutcome`.
    ///
    /// This version contains a list of documents retrieved, the number of
    /// skipped documents, and the cost associated with the query.
    V0(QueryContestedDocumentsOutcomeV0),
}

impl QueryContestedDocumentsOutcomeV0Methods for QueryContestedDocumentsOutcome {
    fn documents(&self) -> &Vec<Document> {
        match self {
            QueryContestedDocumentsOutcome::V0(outcome) => outcome.documents(),
        }
    }

    fn documents_owned(self) -> Vec<Document> {
        match self {
            QueryContestedDocumentsOutcome::V0(outcome) => outcome.documents_owned(),
        }
    }

    fn cost(&self) -> u64 {
        match self {
            QueryContestedDocumentsOutcome::V0(outcome) => outcome.cost(),
        }
    }
}

impl Drive {
    /// Performs a specified drive query and returns the result, along with any skipped items and the cost.
    ///
    /// This function is used to execute a given [DriveDocumentQuery]. It has options to operate in a dry-run mode
    /// and supports different protocol versions. In case an epoch is specified, it calculates the fee.
    ///
    /// # Arguments
    ///
    /// * `query` - The [DriveContestedDocumentQuery] being executed.
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
    pub fn query_contested_documents(
        &self,
        query: DriveContestedDocumentQuery,
        epoch: Option<&Epoch>,
        dry_run: bool,
        transaction: TransactionArg,
        protocol_version: Option<u32>,
    ) -> Result<QueryContestedDocumentsOutcome, Error> {
        let platform_version = PlatformVersion::get_version_or_current_or_latest(protocol_version)?;

        match platform_version
            .drive
            .methods
            .document
            .query
            .query_contested_documents
        {
            0 => self.query_contested_documents_v0(
                query,
                epoch,
                dry_run,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_contested_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
        .map(|outcome| outcome.into())
    }
}
