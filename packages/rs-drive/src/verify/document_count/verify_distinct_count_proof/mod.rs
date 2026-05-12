mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{DriveDocumentCountQuery, SplitCountEntry};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// Verifies a regular grovedb range proof against a
    /// `ProvableCountTree` and returns `(root_hash, entries)`. Each
    /// entry's `count` is bound to the merk root via
    /// `node_hash_with_count(kv_hash, l_hash, r_hash, count)`, so
    /// once this returns `Ok` every count is cryptographically
    /// committed to the same `root_hash` the caller can pass to a
    /// tenderdash signature check.
    ///
    /// Counterpart to the prover-side
    /// [`execute_distinct_count_with_proof`](Self::execute_distinct_count_with_proof):
    /// rebuilds the same `PathQuery` via
    /// [`distinct_count_path_query`](Self::distinct_count_path_query)
    /// and calls `GroveDb::verify_query`. Caller is responsible for
    /// combining the returned `root_hash` with the surrounding
    /// tenderdash signature — see `rs-drive-proof-verifier`'s
    /// `verify_distinct_count_proof` wrapper for the canonical
    /// composition.
    ///
    /// Entries are emitted unmerged: for compound (`In`-on-prefix)
    /// queries each entry retains its `in_key` (the In value for
    /// that fork) alongside the terminator `key`. See
    /// [`SplitCountEntry`]'s doc for the no-merge rationale.
    ///
    /// # Arguments
    /// * `proof` — raw grovedb proof bytes.
    /// * `limit` — the same limit the prover applied (also used to
    ///   reconstruct the matching path query).
    /// * `left_to_right` — same iteration direction the prover used.
    /// * `platform_version` — selects the method version.
    pub fn verify_distinct_count_proof(
        &self,
        proof: &[u8],
        limit: u16,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SplitCountEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_count
            .verify_distinct_count_proof
        {
            0 => self.verify_distinct_count_proof_v0(proof, limit, left_to_right, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentCountQuery::verify_distinct_count_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
