//! Masternode vote proof verification

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use dpp::voting::votes::Vote;

impl Drive {
    /// Verifies the authenticity of a masternode vote using the provided proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the grovedb proof of authenticity for the vote.
    /// - `masternode_pro_tx_hash`: A 32-byte array representing the masternode identifier.
    /// - `vote`: A reference to the vote being verified.
    /// - `verify_subset_of_proof`: A boolean indicating whether a subset of a larger proof is being verified.
    /// - `platform_version`: A reference to the platform version against which to verify the vote.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple containing `RootHash` and an `Option<Vote>`. The `RootHash` represents
    /// the root hash of GroveDB, and the `Option<Vote>` contains the vote if the proved vote differs from the
    /// one provided; otherwise, it returns `None`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is invalid or does not authenticate the vote.
    /// - The `masternode_pro_tx_hash` does not correspond to a valid masternode.
    /// - The vote details are incorrect or manipulated.
    /// - An unknown or unsupported platform version is provided.
    ///
    pub fn verify_masternode_vote(
        proof: &[u8],
        masternode_pro_tx_hash: [u8; 32],
        vote: &Vote,
        data_contract: &DataContract,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Vote>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_masternode_vote
        {
            0 => Self::verify_masternode_vote_v0(
                proof,
                masternode_pro_tx_hash,
                vote,
                data_contract,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_masternode_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
