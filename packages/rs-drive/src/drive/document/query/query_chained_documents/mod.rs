mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_chained_document_query::{
    ChainedDocumentsResult, ChainedProofBundle, DriveChainedDocumentQuery,
};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

pub use v0::QueryChainedDocumentsOutcomeV0;

impl Drive {
    /// Executes a chained document query (provable semi-join) without
    /// proofs and returns the materialized halves plus the processing
    /// cost (when an epoch is given).
    pub fn query_chained_documents(
        &self,
        query: &DriveChainedDocumentQuery,
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

    /// Executes a chained document query AND generates its two proofs.
    ///
    /// Same-root contract: pass a transaction so both proofs are
    /// generated against one snapshot — see
    /// [`DriveChainedDocumentQuery::execute_with_proofs_internal`].
    /// Shares the `query_chained_documents` version slot with the
    /// no-proof path (one surface, one version).
    pub fn query_chained_documents_with_proofs(
        &self,
        query: &DriveChainedDocumentQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ChainedProofBundle, ChainedDocumentsResult), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .query_chained_documents
        {
            0 => self.query_chained_documents_with_proofs_v0(query, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "query_chained_documents_with_proofs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
