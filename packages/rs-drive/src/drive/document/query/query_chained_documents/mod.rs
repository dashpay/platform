mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::block::epoch::Epoch;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

pub use v0::QueryChainedDocumentsOutcomeV0;

impl Drive {
    /// Executes a chained document query — a [`DriveDocumentQuery`] inner
    /// half carrying a single by-id join in its
    /// [`sub_queries`](DriveDocumentQuery::sub_queries) (see
    /// [`DriveDocumentQuery::with_by_id_join`]) — without proofs and
    /// returns the materialized halves plus the processing cost (when an
    /// epoch is given).
    pub fn query_chained_documents(
        &self,
        query: &DriveDocumentQuery,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryChainedDocumentsOutcomeV0, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .query_chained_documents
        {
            0 => self.query_chained_documents_v0(query, epoch, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_chained_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Executes a chained document query AND generates its single
    /// merged proof (the limited inner page and the derived outer
    /// by-ids fetch merged by `prove_query_many` — one proof, one root
    /// by construction). Grovedb proves committed state only, so the
    /// materialize/prove sequence is bracketed by root-hash reads and
    /// retried when a block commit interleaves — see
    /// [`DriveDocumentQuery::execute_chained_with_proof_internal`].
    /// Shares the `query_chained_documents` version slot with the
    /// no-proof path (one surface, one version).
    /// Returns the merged proof plus the materialized INNER
    /// projections (join values / hint / cursor derive from them); the
    /// outer half is covered by the proof and not materialized.
    pub fn query_chained_documents_with_proof(
        &self,
        query: &DriveDocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, Vec<Document>), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .query_chained_documents
        {
            0 => self.query_chained_documents_with_proof_v0(query, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_chained_documents_with_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
