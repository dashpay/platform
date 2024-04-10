use grovedb::Element;
use grovedb_path::SubtreePath;
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
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
    /// Returns a `Result` with a tuple of `RootHash` and `Vec<Element>`. The `Vec<Element>`
    /// is the array of elements we get back.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub fn verify_elements<B: AsRef<[u8]>>(
        proof: &[u8],
        path: SubtreePath<B>,
        keys: Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Element>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_elements
        {
            0 => Drive::verify_elements_v0(proof, path, keys),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_elements".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}