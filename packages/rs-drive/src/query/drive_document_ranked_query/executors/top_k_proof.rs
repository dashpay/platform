//! Ranked executor for `prove = true` — generates the grovedb
//! indexed-axis top-k proof envelope for the same `(index, axis,
//! descending, k)` the no-proof executor would read.

use super::super::DocumentRankedMode;
use super::ranked_query_for_mode;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proof of the top / bottom `k` groups on a ranked index.
    ///
    /// The client verifies it with
    /// [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`](crate::query::DriveDocumentRankedQuery::verify_ranked_top_k_proof),
    /// reconstructing the same query from the same contract — which is
    /// why index resolution is shared with the no-proof executor.
    pub fn execute_document_ranked_top_k_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        mode: &DocumentRankedMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let indexes = document_type.indexes();
        let ranked_query = ranked_query_for_mode(
            contract_id,
            document_type,
            document_type_name,
            indexes,
            mode,
        )?;
        ranked_query.execute_top_k_with_proof(self, transaction, platform_version)
    }
}
