mod v0;

use crate::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Generates separate index and identity proofs for the first identity whose
    /// non-unique public key hash matches, after an optional identity ID.
    ///
    /// These are the proofs a `getIdentityByNonUniquePublicKeyHash` V0 answer carries.
    /// The query handler is the only caller; block execution never reaches it.
    ///
    /// # Arguments
    ///
    /// - `public_key_hash` - A 20-byte array representing the hash of the public key
    ///   for which the identity should be fetched.
    /// - `after` - An optional identity ID specifying the starting point for retrieval.
    ///   If provided, the function will return the identity that appears after the given ID,
    ///   ensuring that the specified identity itself is not included.
    /// - `transaction` - A transaction argument used for database operations.
    /// - `platform_version` - A reference to the platform version, ensuring that the
    ///   correct version-specific function is used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an [`IdentityAndNonUniquePublicKeyHashDoubleProof`], which
    /// includes both the proof of the identity and the proof linking the public key hash to
    /// an identity ID. The identity proof is absent when no identity follows the cursor.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    /// - The identity retrieval operation fails.
    /// - The requested platform version is unknown or not supported.
    ///
    /// # Versioning
    ///
    /// - Only version `0` is implemented. If an unsupported version is provided, an
    ///   `UnknownVersionMismatch` error is returned.
    pub fn prove_full_identity_by_non_unique_public_key_hash_two_proofs(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityAndNonUniquePublicKeyHashDoubleProof, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .prove
            .prove_full_identity_by_non_unique_public_key_hash_two_proofs
        {
            0 => self.prove_full_identity_by_non_unique_public_key_hash_two_proofs_v0(
                public_key_hash,
                after,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_full_identity_by_non_unique_public_key_hash_two_proofs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn should_reject_unknown_two_proof_producer_version() {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .identity
            .prove
            .prove_full_identity_by_non_unique_public_key_hash_two_proofs = 255;

        let result = drive.prove_full_identity_by_non_unique_public_key_hash_two_proofs(
            [0; 20],
            None,
            None,
            &platform_version,
        );

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "prove_full_identity_by_non_unique_public_key_hash_two_proofs" && known_versions == vec![0] && received == 255
            )
        );
    }
}
