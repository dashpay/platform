use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::block::epoch::EpochIndex;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Verifies a proof containing finalized epoch information for a given range.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `start_epoch_index`: The starting epoch index for the query.
    /// - `start_epoch_index_included`: If `true`, the epoch at `start_epoch_index` is included.
    /// - `end_epoch_index`: The ending epoch index for the query.
    /// - `end_epoch_index_included`: If `true`, the epoch at `end_epoch_index` is included.
    /// - `platform_version`: The platform version to use for method dispatch.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Vec<(EpochIndex, FinalizedEpochInfo)>`.
    /// The vector contains verified finalized epoch information.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - An epoch index plus the offset overflows.
    pub fn verify_finalized_epoch_infos(
        proof: &[u8],
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(EpochIndex, FinalizedEpochInfo)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_finalized_epoch_infos
        {
            0 => Drive::verify_finalized_epoch_infos_v0(
                proof,
                start_epoch_index,
                start_epoch_index_included,
                end_epoch_index,
                end_epoch_index_included,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_finalized_epoch_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
