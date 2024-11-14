use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::verify::RootHash;
use crate::{
    drive::{identity::identity_path_vec, Drive},
    error::{proof::ProofError, Error},
};
use dpp::prelude::Revision;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the revision of an identity by their identity ID.
    ///
    /// `verify_subset_of_proof` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the balance and the revision, but here we are only interested
    /// in verifying the balance.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `identity_id`: A 32-byte array representing the identity ID of the user.
    /// - `verify_subset_of_proof`: A boolean indicating whether we are verifying a subset of a larger proof.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// an `Option<u64>`. The `RootHash` represents the root hash of GroveDB, and the
    /// `Option<u64>` represents the revision of the user's identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid balance.
    /// - The proved key value is not for the correct path or key in balances.
    /// - More than one balance is found.
    ///
    #[inline(always)]
    pub(super) fn verify_identity_revision_for_identity_id_v0(
        proof: &[u8],
        identity_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<u64>), Error> {
        let mut path_query = Self::identity_revision_query(&identity_id);
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
            let (path, key, maybe_element) = &proved_key_values.remove(0);
            if path != &identity_path_vec(&identity_id) {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct path in revision".to_string(),
                )));
            }
            if key != &vec![IdentityTreeRevision as u8] {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "we did not get back an element for the correct key in revision".to_string(),
                )));
            }

            let revision = maybe_element
                .as_ref()
                .map(|element| {
                    let encoded_revision = element.as_item_bytes()?;
                    Ok::<u64, Error>(Revision::from_be_bytes(
                        encoded_revision.try_into().map_err(|_| {
                            Error::Proof(ProofError::CorruptedProof(
                                "identity revision was not 8 bytes as expected".to_string(),
                            ))
                        })?,
                    ))
                })
                .transpose()?;
            Ok((root_hash, revision))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one identity revision",
            )))
        }
    }
}
