use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::fee::Credits;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Verifies a proof containing the total credits in the platform.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `core_distribution_multiplier`: A multiplier for the core distribution. This is 1 for mainnet. And 10 for testnet.
    /// - `current_core_height`: The current core block height.
    /// - `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a tuple of `RootHash` and `Credits`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - The platform version is unknown.
    pub fn verify_total_credits_in_system(
        proof: &[u8],
        core_subsidy_halving_interval: u32,
        request_activation_core_height: impl Fn() -> Result<CoreBlockHeight, Error>,
        current_core_height: CoreBlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Credits), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_total_credits_in_system
        {
            0 => Drive::verify_total_credits_in_system_v0(
                proof,
                core_subsidy_halving_interval,
                request_activation_core_height,
                current_core_height,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_total_credits_in_system".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
