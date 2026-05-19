mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_average_query::AverageEntry;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a grovedb point-lookup proof against an index whose
    /// terminator value tree is a count-sum-bearing variant
    /// (`CountSumTree` / `ProvableCountSumTree` /
    /// `ProvableCountProvableSumTree`) and returns per-branch
    /// `(count, sum)` entries. AVG analog of
    /// [`Self::verify_point_lookup_sum_proof`].
    ///
    /// Walks the verified `(path, key, Option<Element>)` triples
    /// emitted by [`Self::point_lookup_sum_path_query`] and extracts
    /// `count_sum_value_or_default()` from each present terminator
    /// element — `(count, sum)` come from the same merk hash so
    /// there's no way for the server to splice a count from one
    /// branch with a sum from another.
    ///
    /// Today's path query does not set
    /// `absence_proofs_for_non_existing_searched_keys: true`, so
    /// absent In values are silently omitted from the result.
    /// Callers that need to distinguish "verified with (c, s)" from
    /// "queried but absent" diff their In array against the
    /// returned entries by `key`.
    pub fn verify_point_lookup_count_and_sum_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<AverageEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_point_lookup_count_and_sum_proof
        {
            0 => self.verify_point_lookup_count_and_sum_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_point_lookup_count_and_sum_proof"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
