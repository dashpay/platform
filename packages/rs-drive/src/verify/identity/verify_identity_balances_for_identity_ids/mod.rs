mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;
use dpp::fee::Credits;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

use std::iter::FromIterator;

impl Drive {
    /// Verifies the balances of multiple identities by their identity IDs.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `identity_ids`: A slice of 32-byte arrays representing the identity IDs of the users.
    /// - `platform_version`: The platform version against which to verify the balances.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// a generic collection `T` of tuples. Each tuple in `T` consists of a 32-byte array
    /// representing an identity ID and an `Option<Credits>`. The `Option<Credits>` represents
    /// the balance of the respective identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    ///
    pub fn verify_identity_balances_for_identity_ids<
        T: FromIterator<(I, Option<Credits>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        identity_ids: &[[u8; 32]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_balances_for_identity_ids
        {
            0 => Self::verify_identity_balances_for_identity_ids_v0::<T, I>(
                proof,
                is_proof_subset,
                identity_ids,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_balances_for_identity_ids".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use dpp::fee::Credits;

    #[test]
    fn test_verify_identity_balances_for_identity_ids_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_balances_for_identity_ids = 255;

        let result = Drive::verify_identity_balances_for_identity_ids::<
            Vec<([u8; 32], Option<Credits>)>,
            [u8; 32],
        >(&[], false, &[], &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_identity_balances_for_identity_ids" && known_versions == vec![0] && received == 255
            )
        );
    }
}
