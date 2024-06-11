use crate::drive::Drive;
use grovedb::{GroveDb, PathQuery, SizedQuery};

use crate::error::Error;

use crate::drive::verify::RootHash;

use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
use crate::error::proof::ProofError;
use crate::query::Query;
use dpp::voting::votes::Vote;

impl Drive {
    /// Verifies the masternode vote.
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
    ///
    pub(crate) fn verify_masternode_vote_v0(
        proof: &[u8],
        masternode_pro_tx_hash: [u8; 32],
        vote: &Vote,
        verify_subset_of_proof: bool,
    ) -> Result<(RootHash, Option<Vote>), Error> {
        // First we should get the overall document_type_path
        let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            &masternode_pro_tx_hash,
        );

        let vote_id = vote.unique_id()?;

        let mut query = Query::new();
        query.insert_key(vote_id.to_vec());

        let path_query = PathQuery::new(path, SizedQuery::new(query, Some(1), None));

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(proof, &path_query)?
        } else {
            GroveDb::verify_query_with_absence_proof(proof, &path_query)?
        };
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = &proved_key_values.remove(0);
            todo!()
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one masternode vote",
            )))
        }
    }
}
