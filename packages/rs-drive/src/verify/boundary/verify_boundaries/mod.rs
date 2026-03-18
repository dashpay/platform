mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;

impl Drive {
    /// Returns all boundary keys at a given path in a GroveDB proof.
    ///
    /// Boundary elements (KVDigest / KVDigestCount) appear in proofs for
    /// exclusive range queries (e.g., `RangeAfter`, `RangeAfterTo`). They
    /// anchor the range but are not part of the result set.
    ///
    /// This is useful for **pagination verification**: after verifying a proof,
    /// the client can inspect all boundary keys at once to confirm that
    /// pagination cursors are still valid.
    ///
    /// # Parameters
    /// - `proof` - Raw proof bytes (serialized `GroveDBProof`)
    /// - `path` - GroveDB path to the subtree to inspect
    /// - `platform_version` - Protocol version for dispatch
    ///
    /// # Returns
    /// A `Vec<Vec<u8>>` of all boundary keys found at the given path.
    /// Empty if no boundaries exist at that path.
    pub fn verify_boundaries(
        proof: &[u8],
        path: &[&[u8]],
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        match platform_version
            .drive
            .methods
            .verify
            .boundary
            .verify_boundaries
        {
            0 => Self::verify_boundaries_v0(proof, path),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_boundaries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
