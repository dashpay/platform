use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Verifies a proof containing potentially multiple epoch infos.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `current_epoch`: The current epoch index, can be acquired from metadata.
    /// - `start_epoch`: The first epoch index.
    /// - `count`: The amount of epochs to get.
    /// - `ascending`: True if we want to get epochs from oldest to newest.
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
    pub fn verify_epoch_infos(
        proof: &[u8],
        current_epoch: EpochIndex,
        start_epoch: Option<EpochIndex>,
        count: u16,
        ascending: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ExtendedEpochInfo>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_epoch_infos
        {
            0 => Drive::verify_epoch_infos_v0(
                proof,
                current_epoch,
                start_epoch,
                count,
                ascending,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_epoch_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
