use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;

use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use nohash_hasher::IntMap;

mod v0;

impl Drive {
    /// Verifies a proof containing the current upgrade state.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
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
    pub fn verify_upgrade_state(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, IntMap<ProtocolVersion, u64>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_upgrade_state
        {
            0 => Drive::verify_upgrade_state_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_upgrade_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
