mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// Verifies a grovedb proof of the document type's primary-key
    /// `CountTree` element and returns `(root_hash, count)`. Used by
    /// the SDK to verify the response from the prove path's
    /// `documents_countable: true` fast path — unfiltered total
    /// counts on a doctype whose primary-key tree is itself a
    /// CountTree.
    ///
    /// Free-function on the type rather than `&self` because the
    /// documents_countable case isn't tied to any index — it
    /// operates on the doctype primary-key tree directly. The
    /// `contract_id` + `document_type_name` are all the verifier
    /// needs to reconstruct the same `PathQuery` the prover used
    /// via [`Self::primary_key_count_tree_path_query`].
    ///
    /// The verified count is cryptographically bound to the merk
    /// root via `node_hash_with_count(kv_hash, l_hash, r_hash,
    /// count)` — same forge-resistance guarantee the other count-
    /// proof verifiers rely on. Once this returns `Ok`, the count is
    /// committed to the `root_hash` the caller passes to the
    /// tenderdash signature check.
    ///
    /// Returns `count = 0` when the CountTree element is absent
    /// (fresh doctype with no documents inserted). The
    /// documents_countable storage layout creates the type-level
    /// CountTree at contract apply time, so absence really does mean
    /// "zero docs"; callers can rely on it.
    pub fn verify_primary_key_count_tree_proof(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_count
            .verify_primary_key_count_tree_proof
        {
            0 => Self::verify_primary_key_count_tree_proof_v0(
                proof,
                contract_id,
                document_type_name,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentCountQuery::verify_primary_key_count_tree_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
