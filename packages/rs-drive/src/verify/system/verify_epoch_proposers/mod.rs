use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::proposer_block_count_query::ProposerQueryType;
use crate::verify::RootHash;
use dpp::block::epoch::EpochIndex;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Verifies a proof containing a single epochs proposers.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `epoch_index`: The epoch index, can be acquired from metadata.
    /// - `limit`: The amount of proposers to get.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Vec<ExtendedEpochInfo>`. The `Vec<ExtendedEpochInfo>`
    /// represents verified epoch information if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub fn verify_epoch_proposers<I, P, E>(
        proof: &[u8],
        epoch_index: EpochIndex,
        proposer_query_type: ProposerQueryType,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, I), Error>
    where
        I: FromIterator<(P, u64)>,
        P: TryFrom<Vec<u8>, Error = E>,
    {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_epoch_infos
        {
            0 => Drive::verify_epoch_proposers_v0(
                proof,
                epoch_index,
                proposer_query_type,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_epoch_proposers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
