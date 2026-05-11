mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{DriveDocumentCountQuery, SplitCountEntry};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// Verifies a grovedb proof of CountTree elements produced by the
    /// point-lookup count proof path and returns `(root_hash, entries)`.
    ///
    /// Counterpart to the prover-side
    /// [`execute_point_lookup_count_with_proof`](Self::execute_point_lookup_count_with_proof):
    /// rebuilds the same `PathQuery` via
    /// [`point_lookup_count_path_query`](Self::point_lookup_count_path_query)
    /// and calls `GroveDb::verify_query`. Each verified element's
    /// `count_value` is cryptographically bound to the merk root via
    /// `node_hash_with_count(kv_hash, l_hash, r_hash, count)`, so once
    /// this returns `Ok` every count is committed to the same
    /// `root_hash` the caller can pass to a tenderdash signature check.
    /// Caller is responsible for combining the returned `root_hash`
    /// with the surrounding tenderdash signature — see
    /// `rs-drive-proof-verifier`'s `verify_point_lookup_count_proof`
    /// wrapper for the canonical composition.
    ///
    /// Entry shape:
    /// - **Equal-only, fully covered**: a single entry with
    ///   `in_key: None`, `key: vec![]`, and `count` equal to the
    ///   covered branch's CountTree `count_value`.
    /// - **Equal prefix + `In` on last or before-last property**: one
    ///   entry per In value, with `in_key: None`,
    ///   `key: <serialized_in_value>`, and `count` equal to that In
    ///   branch's CountTree `count_value`. For the In-on-before-last
    ///   shape the trailing Equal is part of the descent (so each
    ///   branch's count is "docs with `in_field == in_value AND
    ///   trailing_field == trailing_value`"); the entry's `key`
    ///   still records the In value because the trailing Equal is
    ///   fixed across all entries. Matches the no-proof `PerInValue`
    ///   shape (`in_key` is reserved for the range-distinct compound
    ///   case where In sits on a prefix of a range index).
    ///
    /// Branches with no documents at the covered path don't appear in
    /// the result (CountTree element is absent → no entry emitted).
    pub fn verify_point_lookup_count_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SplitCountEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_count
            .verify_point_lookup_count_proof
        {
            0 => self.verify_point_lookup_count_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentCountQuery::verify_point_lookup_count_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
