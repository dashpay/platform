mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::{DriveDocumentSumQuery, SumEntry};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a grovedb point-lookup sum proof and returns the
    /// per-branch entries. Sum analog of count's
    /// `verify_point_lookup_count_proof`.
    ///
    /// Single-terminator shape, kept in sync with
    /// [`Self::point_lookup_sum_path_query`]: the insertion side
    /// stores every `summable: "<prop>"` index's terminator value
    /// tree as a `SumTree` (with sibling continuations
    /// `NonCounted`-wrapped so they don't pollute the parent's sum),
    /// so proofs target the value tree directly via
    /// `Key(serialized_value)` and `sum_value_or_default()` on the
    /// verified element is the per-branch sum.
    ///
    /// ## Entry shape
    ///
    /// One entry per **present** queried key. Today's path query
    /// does not set `absence_proofs_for_non_existing_searched_keys:
    /// true`, so absent In values are silently omitted from the
    /// elements stream. Callers that need to distinguish "verified
    /// with sum N" from "queried but absent" diff their request's
    /// `In` array against the returned entries by `key`.
    ///
    /// The `Option<i64>` field's `None` variant is reserved for a
    /// future variant that flips
    /// `absence_proofs_for_non_existing_searched_keys`; the current
    /// path query won't produce it.
    pub fn verify_point_lookup_sum_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SumEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_point_lookup_sum_proof
        {
            0 => self.verify_point_lookup_sum_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_point_lookup_sum_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
