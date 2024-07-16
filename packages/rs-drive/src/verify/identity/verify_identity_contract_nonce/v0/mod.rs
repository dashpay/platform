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
    pub(crate) fn verify_identity_contract_nonce_v0(
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
                        .map_err(Error::GroveDB)?
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
