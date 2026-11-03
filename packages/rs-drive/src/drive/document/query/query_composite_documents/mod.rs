mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::block::epoch::Epoch;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

pub use v0::QueryCompositeDocumentsOutcomeV0;

impl Drive {
    /// Executes a composite document query — a [`DriveDocumentQuery`]
    /// page carrying derived [`sub_queries`](DriveDocumentQuery::sub_queries)
    /// — without proofs and returns the materialized results plus the
    /// processing cost (when an epoch is given).
    ///
    /// # Parameters
    ///
    /// * `query`: The page query, carrying the derived sub-queries.
    /// * `epoch`: The epoch to price the reads in; `None` leaves the cost at zero.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(QueryCompositeDocumentsOutcomeV0)` with the page, one result per sub-query, the
    ///   ids each by-id join found no document for, and the processing cost (zero without an
    ///   epoch).
    /// * `Err(Error)` when the method version is unknown, the query is not a valid composite
    ///   query, a read or a document deserialization fails, or the fee cannot be calculated.
    pub fn query_composite_documents(
        &self,
        query: &DriveDocumentQuery,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryCompositeDocumentsOutcomeV0, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .query_composite_documents
        {
            0 => self.query_composite_documents_v0(query, epoch, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_composite_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Executes a composite document query AND generates its single
    /// merged proof: the page and every derived sub-query merged by
    /// `prove_query_many` — one proof, one root by construction. Grovedb
    /// proves committed state only, so the materialize/prove sequence is
    /// bracketed by root-hash reads and retried when a block commit
    /// interleaves — see
    /// [`DriveDocumentQuery::execute_composite_with_proof_internal`].
    /// Shares the `query_composite_documents` version slot with the
    /// no-proof path (one surface, one version).
    ///
    /// Returns the merged proof plus the materialized page (the
    /// caller's pagination cursor derives from it); the sub-query
    /// results are covered by the proof and not materialized twice.
    ///
    /// # Parameters
    ///
    /// * `query`: The page query, carrying the derived sub-queries.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((Vec<u8>, Vec<Document>))` with the merged proof and the page.
    /// * `Err(Error)` when the method version is unknown, the query is not a valid composite
    ///   query, a read or proving fails, or every attempt raced a block commit.
    pub fn query_composite_documents_with_proof(
        &self,
        query: &DriveDocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, Vec<Document>), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .query_composite_documents
        {
            0 => self.query_composite_documents_with_proof_v0(query, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_composite_documents_with_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
