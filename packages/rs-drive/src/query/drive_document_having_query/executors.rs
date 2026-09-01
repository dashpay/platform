//! Per-mode having-range executors on `impl Drive`. The dispatcher
//! ([`super::drive_dispatcher`]) picks between the two executors on the
//! request's `prove` flag.
//!
//! Mode-to-query resolution — covering-index pick + equality-pin
//! encoding — is [`resolve_having_query_for_mode`], shared with the
//! SDK's proof helpers: both surfaces and both sides read the same
//! indexed tree, and sharing the resolution is what guarantees a proof
//! and an unproven read are about the same subtree.

use super::super::drive_document_ranked_query::RankedEntry;
use super::{resolve_having_query_for_mode, DocumentHavingMode};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::ResolvedTimeRange;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// One page of groups matching a having bound, read without a proof.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_having_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentHavingMode,
        resolved_time_ranges: &[ResolvedTimeRange],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<RankedEntry>, Error> {
        let indexes = document_type.indexes();
        let having_query = resolve_having_query_for_mode(
            contract_id,
            document_type,
            document_type_name,
            indexes,
            mode,
            resolved_time_ranges,
            platform_version,
        )?;
        having_query.execute_range_no_proof(self, transaction, platform_version)
    }

    /// Proof of one page of groups matching a having bound.
    ///
    /// The client verifies it with
    /// [`DriveDocumentHavingQuery::verify_having_range_proof`](crate::query::DriveDocumentHavingQuery::verify_having_range_proof),
    /// reconstructing the same query from the same contract — which is
    /// why index resolution is shared with the no-proof executor.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_having_range_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentHavingMode,
        resolved_time_ranges: &[ResolvedTimeRange],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let indexes = document_type.indexes();
        let having_query = resolve_having_query_for_mode(
            contract_id,
            document_type,
            document_type_name,
            indexes,
            mode,
            resolved_time_ranges,
            platform_version,
        )?;
        having_query.execute_range_with_proof(self, transaction, platform_version)
    }
}
