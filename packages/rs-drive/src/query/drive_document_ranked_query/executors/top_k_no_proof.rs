//! Ranked executor for `prove = false` — reads one page of `k` groups
//! straight out of the axis secondary, starting at rank `offset`, and
//! returns them in ranking order.

use super::super::{DocumentRankedMode, RankedPage};
use super::ranked_query_for_mode;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// One page of `k` groups on a ranked index, unproven.
    ///
    /// Entry order is the ranking order; callers must not re-sort.
    pub fn execute_document_ranked_top_k_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentRankedMode,
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
            platform_version,
        )?;
        ranked_query.execute_top_k_no_proof(self, transaction, platform_version)
    }
}
