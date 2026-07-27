use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonceKey;
use crate::drive::identity::identity_contract_info_group_path_vec;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the balance of an identity by their identity ID.
    ///
    /// `verify_subset_of_proof` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the balance and the revision, but here we are only interested
    /// in verifying the balance.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `contract_id`: A 32-byte array representing the contract ID that the nonce is for.
    /// - `verify_subset_of_proof`: A boolean indicating whether we are verifying a subset of a larger proof.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option<u64>`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<u64>` represents the balance of the user's identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid identity.
    ///
    pub(super) fn verify_identity_contract_nonce_v0(
        proof: &[u8],
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        let mut path_query = Self::identity_contract_nonce_query(identity_id, contract_id);
        path_query.query.limit = Some(1);
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            if path != identity_contract_info_group_path_vec(&identity_id, contract_id.as_slice()) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path for the identity contract nonce".to_string(),
                )));
            }
            if key != vec![IdentityContractNonceKey as u8] {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key for the identity contract nonce".to_string(),
                )));
            }

            let identity_contract_nonce = maybe_element
                .map(|element| {
                    let bytes: [u8; 8] = element
                        .into_item_bytes()
                        .map_err(Error::from)?
                        .try_into()
                        .map_err(|_| {
                            Error::Proof(ProofError::IncorrectValueSize("value size is incorrect"))
                        })?;

                    Ok::<u64, Error>(u64::from_be_bytes(bytes))
                })
                .transpose()?;
            Ok((root_hash, identity_contract_nonce))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one identity contract nonce",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_identity_contract_nonce() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::first();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a random identity");

        let identity_id = identity.id().to_buffer();
        let contract_id = [1u8; 32];

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        // Set a contract nonce value by merging
        let nonce_value: u64 = 1;
        let result = drive
            .merge_identity_contract_nonce(
                identity_id,
                contract_id,
                nonce_value,
                &BlockInfo::default(),
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");
        assert!(result.error_message().is_none());

        // Prove the contract nonce
        let proof = drive
            .prove_identity_contract_nonce(identity_id, contract_id, None, &platform_version.drive)
            .expect("should not error when proving identity contract nonce");

        // Verify the contract nonce
        let (_root_hash, proved_nonce) = Drive::verify_identity_contract_nonce(
            proof.as_slice(),
            identity_id,
            contract_id,
            false,
            platform_version,
        )
        .expect("expected to verify identity contract nonce");

        assert_eq!(proved_nonce, Some(nonce_value));
    }

    #[test]
    fn should_prove_and_verify_absent_identity_contract_nonce() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::first();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a random identity");

        let identity_id = identity.id().to_buffer();
        let contract_id = [2u8; 32];

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        // Prove without setting a contract nonce (should be absent)
        let proof = drive
            .prove_identity_contract_nonce(identity_id, contract_id, None, &platform_version.drive)
            .expect("should not error when proving identity contract nonce");

        let (_root_hash, proved_nonce) = Drive::verify_identity_contract_nonce(
            proof.as_slice(),
            identity_id,
            contract_id,
            false,
            platform_version,
        )
        .expect("expected to verify identity contract nonce");

        assert_eq!(proved_nonce, None);
    }
}
