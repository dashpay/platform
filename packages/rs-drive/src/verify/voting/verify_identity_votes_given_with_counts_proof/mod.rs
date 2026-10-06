mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use crate::query::ContractLookupFn;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::votes::resource_vote::ResourceVote;

impl ContestedResourceVotesGivenByIdentityQuery {
    /// Verifies a proof of the votes an identity has given, and reads, next to each vote, how
    /// many times the identity has voted on that vote poll.
    ///
    /// The proof is the one [`Self::verify_identity_votes_given_proof`] verifies: the count is
    /// stored with the vote and proved with it, so no other request is needed to read it.
    ///
    /// # Parameters
    ///
    /// - `proof`: The serialized proof to verify.
    /// - `contract_lookup_fn`: Retrieves a data contract by its identifier.
    /// - `platform_version`: The platform version to verify against.
    ///
    /// # Returns
    ///
    /// The root hash and, for each vote poll the identity has a vote on in the queried range,
    /// the vote poll's unique id with the identity's current vote and the number of times the
    /// identity has voted on that poll. The count includes the current vote, so it is 1 after
    /// a first vote and grows by 1 with each changed vote. A vote poll the identity has not
    /// voted on has no entry. The collection type is chosen through `I`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof does not verify.
    /// - A stored vote can not be decoded, or its data contract is not found.
    /// - The platform version is unknown.
    pub fn verify_identity_votes_given_with_counts_proof<I>(
        &self,
        proof: &[u8],
        contract_lookup_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, I), Error>
    where
        I: FromIterator<(Identifier, (ResourceVote, u16))>,
    {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_identity_votes_given_with_counts_proof
        {
            0 => self.verify_identity_votes_given_with_counts_proof_v0(
                proof,
                contract_lookup_fn,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_votes_given_with_counts_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn should_refuse_an_unknown_verifier_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_identity_votes_given_with_counts_proof = 255;

        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id: Identifier::default(),
            offset: None,
            limit: None,
            start_at: None,
            order_ascending: true,
        };

        let contract_lookup_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = query
            .verify_identity_votes_given_with_counts_proof::<BTreeMap<Identifier, (ResourceVote, u16)>>(
                &[],
                contract_lookup_fn,
                &platform_version,
            );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_identity_votes_given_with_counts_proof"
                    && known_versions == vec![0]
                    && received == 255
        ));
    }
}
