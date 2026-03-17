mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;

impl Drive {
    /// Checks whether a key exists as a boundary element in a GroveDB proof.
    ///
    /// Boundary elements appear in proofs for exclusive range queries (e.g.,
    /// `RangeAfter(cursor_key)`) where the boundary key anchors the range
    /// but is not part of the result set.
    ///
    /// This is useful for **pagination verification**: after verifying a proof
    /// for "next N documents after cursor X", the client can check that cursor X
    /// still exists in the tree by calling this function.
    ///
    /// # Parameters
    /// - `proof` - Raw proof bytes (serialized `GroveDBProof`)
    /// - `path` - GroveDB path to the subtree where the range query was executed
    /// - `key` - The boundary key to check for (e.g., the pagination cursor)
    /// - `platform_version` - Protocol version for dispatch
    ///
    /// # Returns
    /// `true` if the key exists as a KVDigest or KVDigestCount boundary in the
    /// proof at the given path, `false` otherwise.
    pub fn verify_key_exists_as_boundary(
        proof: &[u8],
        path: &[&[u8]],
        key: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version.drive.methods.verify.document.verify_proof {
            0 => Self::verify_key_exists_as_boundary_v0(proof, path, key),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_key_exists_as_boundary".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
