mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a grovedb proof of the document type's primary-key
    /// `SumTree` element and returns the unfiltered total sum. Sum
    /// analog of count's `verify_primary_key_count_tree_proof`.
    ///
    /// Used by the prove path's `documentsSummable: "<prop>"` fast
    /// path — when the where clauses are empty and the document type
    /// has a matching `documents_summable`, the executor proves the
    /// primary-key SumTree element directly via
    /// [`Self::primary_key_sum_path_query`] (a single-key
    /// `verify_query` shape), avoiding the per-index covering walk.
    ///
    /// Returns 0 when the element is absent (the proof's element
    /// stream is empty or carries `None`). At contract apply time
    /// the SumTree element is created unconditionally for
    /// `documents_summable` doctypes, so absence here means "no
    /// documents inserted yet", not a misconfiguration.
    pub fn verify_primary_key_sum_tree_proof(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, i64), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_primary_key_sum_tree_proof
        {
            0 => Self::verify_primary_key_sum_tree_proof_v0(
                proof,
                contract_id,
                document_type_name,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_primary_key_sum_tree_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
