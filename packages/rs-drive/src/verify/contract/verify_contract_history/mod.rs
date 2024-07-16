mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies that the contract's history is included in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `contract_id`: The contract's unique identifier.
    /// - `start_at_date`: The start date for the contract's history.
    /// - `limit`: An optional limit for the number of items to be retrieved.
    /// - `offset`: An optional offset for the items to be retrieved.
    /// - `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Option<BTreeMap<u64, DataContract>>`. The `Option<BTreeMap<u64, DataContract>>`
    /// represents a mapping from dates to contracts if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    /// - The contract serialization fails.
    pub fn verify_contract_history(
        proof: &[u8],
        contract_id: [u8; 32],
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<BTreeMap<u64, DataContract>>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract
            .verify_contract_history
        {
            0 => Drive::verify_contract_history_v0(
                proof,
                contract_id,
                start_at_date,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
