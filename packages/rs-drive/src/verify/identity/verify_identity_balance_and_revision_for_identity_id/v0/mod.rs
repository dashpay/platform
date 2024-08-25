use crate::verify::RootHash;
use crate::{
    drive::Drive,
    error::{proof::ProofError, Error},
};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the balance and the reviof an identity by their identity ID.
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
    /// `Option<u64>` represents the balance of the user's identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid balance.
    /// - The proved key value is not for the correct path or key in balances.
    /// - More than one balance is found.
    pub fn verify_identity_balance_and_revision_for_identity_id(
        proof: &[u8],
        identity_id: [u8; 32],
        _verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<(u64, u64)>), Error> {
        let (root_hash_0, signed_balance) = Self::verify_identity_balance_for_identity_id_v0(
            proof,
            identity_id,
            true,
            platform_version,
        )?;

        let (root_hash_1, revision) = Self::verify_identity_revision_for_identity_id_v0(
            proof,
            identity_id,
            true,
            platform_version,
        )?;

        if root_hash_0 != root_hash_1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "root hash of balance and root hash for revision do not match".to_string(),
            )));
        }

        if signed_balance.is_some() && revision.is_none() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we got back a balance but no revision".to_string(),
            )));
        }

        if revision.is_some() && signed_balance.is_none() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we got back a revision but no balance".to_string(),
            )));
        }

        if let Some(signed_balance) = signed_balance {
            Ok((root_hash_0, Some((signed_balance, revision.unwrap()))))
        } else {
            Ok((root_hash_0, None))
        }
    }
}
