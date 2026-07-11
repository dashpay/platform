mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a grovedb proof of the document type's primary-key
    /// `CountSumTree` (or `ProvableCountSumTree` /
    /// `ProvableCountProvableSumTree`) element and returns the
    /// unfiltered `(count, sum)` pair. Average-aggregate analog of
    /// [`Self::verify_primary_key_sum_tree_proof`] / count's
    /// `verify_primary_key_count_tree_proof`.
    ///
    /// Used by the prove path's AVG fast path — when the where
    /// clauses are empty and the document type has both
    /// `documentsCountable: true` and `documentsSummable: "<prop>"`
    /// (which implies the primary key tree is one of the
    /// count-sum-bearing variants), the executor proves the
    /// primary-key element directly via
    /// [`Self::primary_key_sum_path_query`] — same single-key shape
    /// as the SumTree fast path, just decoded as a combined
    /// `(count, sum)` instead of `i64` alone.
    ///
    /// Returns `(0, 0)` when the element is absent.
    pub fn verify_primary_key_count_sum_tree_proof(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64, i64), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_primary_key_count_sum_tree_proof
        {
            0 => Self::verify_primary_key_count_sum_tree_proof_v0(
                proof,
                contract_id,
                document_type_name,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_primary_key_count_sum_tree_proof"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
