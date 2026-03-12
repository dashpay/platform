mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;

use crate::verify::RootHash;

use dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dpp::identity::Purpose;

use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the identity keys of a user by their identity ID.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `is_proof_subset`: A boolean indicating whether the proof is a subset.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `platform_version`: The platform version against which to verify the identity keys.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option` of `PartialIdentity`. The `RootHash` represents the root hash of GroveDB,
    /// and the `Option<PartialIdentity>` represents the partial identity of the user if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    ///
    pub fn verify_identities_contract_keys(
        proof: &[u8],
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        is_proof_subset: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, IdentitiesContractKeys), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identities_contract_keys
        {
            0 => Self::verify_identities_contract_keys_v0(
                proof,
                identity_ids,
                contract_id,
                document_type_name,
                purposes,
                is_proof_subset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identities_contract_keys".to_string(),
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

    #[test]
    fn test_verify_identities_contract_keys_unknown_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identities_contract_keys = 255;

        let result = Drive::verify_identities_contract_keys(
            &[],
            &[],
            &[0u8; 32],
            None,
            vec![],
            false,
            &platform_version,
        );

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_identities_contract_keys" && known_versions == vec![0] && received == 255
            )
        );
    }
}
