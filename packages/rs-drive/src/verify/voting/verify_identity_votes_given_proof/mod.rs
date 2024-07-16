mod v0;

use crate::error::drive::DriveError;
use crate::query::ContractLookupFn;
use crate::verify::RootHash;
use dpp::identifier::Identifier;

use crate::error::Error;

use crate::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use dpp::version::PlatformVersion;
use dpp::voting::votes::resource_vote::ResourceVote;

impl ContestedResourceVotesGivenByIdentityQuery {
    /// Verifies a proof for the vote poll vote state proof.
    ///
    /// This function verifies a given serialized proof and returns the root hash along with a collection of deserialized identifiers and their corresponding resource votes.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the serialized proof to be verified.
    /// * `contract_lookup_fn` - Function that retrieves data contract based on its identifier.
    /// * `platform_version` - A reference to the platform version to be used for verification.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a collection of `(Identifier, ResourceVote)` pairs if the proof is valid. The collection type is flexible and determined by the generic parameter `I`.
    /// * An `Error` variant if the proof verification fails or a deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` if:
    /// * The proof verification fails.
    /// * A deserialization error occurs when parsing the serialized document(s).
    pub fn verify_identity_votes_given_proof<I>(
        &self,
        proof: &[u8],
        contract_lookup_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, I), Error>
    where
        I: FromIterator<(Identifier, ResourceVote)>,
    {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_identity_votes_given_proof
        {
            0 => self.verify_identity_votes_given_proof_v0(
                proof,
                contract_lookup_fn,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_votes_given_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
