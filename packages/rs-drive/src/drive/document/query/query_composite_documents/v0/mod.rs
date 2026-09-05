use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::drive_composite_document_query::{
    CompositeDocumentsResult, DriveCompositeDocumentQuery,
};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// The outcome of a composite document query: the materialized results
/// and the processing cost.
#[derive(Debug, Default)]
pub struct QueryCompositeDocumentsOutcomeV0 {
    /// The materialized page and sub-query results.
    pub result: CompositeDocumentsResult,
    /// The processing cost, when an epoch was given.
    pub cost: u64,
}

impl Drive {
    #[inline(always)]
    pub(super) fn query_composite_documents_v0(
        &self,
        query: &DriveCompositeDocumentQuery,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<QueryCompositeDocumentsOutcomeV0, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let result = query.execute_no_proof_internal(
            self,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(epoch) = epoch {
            Drive::calculate_fee(
                None,
                Some(drive_operations),
                epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?
            .processing_fee
        } else {
            0
        };
        Ok(QueryCompositeDocumentsOutcomeV0 { result, cost })
    }

    #[inline(always)]
    pub(super) fn query_composite_documents_with_proof_v0(
        &self,
        query: &DriveCompositeDocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, Vec<dpp::document::Document>), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        query.execute_with_proof_internal(self, &mut drive_operations, platform_version)
    }
}
