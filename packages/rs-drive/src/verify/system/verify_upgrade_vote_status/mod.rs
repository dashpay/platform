mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies a proof containing potentially multiple epoch infos.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `first_pro_tx_hash`: the first pro tx hash that we are querying for.
    /// - `count`: the amount of Evonodes that we want to retrieve.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `BTreeMap<[u8;32], ProtocolVersion>`. The `BTreeMap<[u8;32], ProtocolVersion>`
    /// represents a map of the version that each Evonode has voted for.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    pub fn verify_upgrade_vote_status(
        proof: &[u8],
        start_protx_hash: Option<[u8; 32]>,
        count: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<[u8; 32], ProtocolVersion>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .system
            .verify_upgrade_vote_status
        {
            0 => Drive::verify_upgrade_vote_status_v0(
                proof,
                start_protx_hash,
                count,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_upgrade_vote_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
