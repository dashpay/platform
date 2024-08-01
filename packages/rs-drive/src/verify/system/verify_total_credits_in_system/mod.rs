use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use dpp::fee::Credits;

mod v0;

impl Drive {
    /// Verifies a proof containing the total credits in platform.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Credits`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub fn verify_total_credits_in_system(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Credits), Error> {
        match platform_version.drive.methods.verify.system.verify_total_credits_in_system {
            0 => Drive::verify_total_credits_in_system_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_total_credits_in_system".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
