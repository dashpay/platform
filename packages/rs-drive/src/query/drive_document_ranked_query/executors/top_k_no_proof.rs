//! Ranked executor for `prove = false` — one page of `k` groups starting
//! at rank `offset`, in ranking order.
//!
//! The page is read from the axis secondary directly, with no proof
//! built. The `offset` is skipped by a counted descent rather than by
//! stepping through the skipped entries, so a deep offset costs
//! `O(log n)` — see
//! [`crate::query::DriveDocumentRankedQuery::execute_top_k_no_proof`].

use super::super::{DocumentRankedMode, RankedPage};
use super::ranked_query_for_mode;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::ResolvedTimeRange;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// One page of `k` groups on a ranked index, unproven.
    ///
    /// Entry order is the ranking order; callers must not re-sort.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_ranked_top_k_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentRankedMode,
        resolved_time_ranges: &[ResolvedTimeRange],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<RankedPage, Error> {
        let indexes = document_type.indexes();
        let ranked_query = ranked_query_for_mode(
            contract_id,
            document_type,
            document_type_name,
            indexes,
            mode,
            resolved_time_ranges,
            platform_version,
        )?;
        ranked_query.execute_top_k_no_proof(self, transaction, platform_version)
    }
}
