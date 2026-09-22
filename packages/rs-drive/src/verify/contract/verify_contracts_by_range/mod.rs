mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

/// One verified page of the contract enumeration: `(contract id, contract)` rows in ascending
/// contract id order. The contract is `None` on an ids-only page.
pub type DataContractsByRangePage = Vec<(Identifier, Option<DataContract>)>;

impl Drive {
    /// Verifies a proof of one page of the contract enumeration (`getDataContractsByRange`).
    ///
    /// The page query is rebuilt from the request parameters the server proved against, so
    /// the caller must pass exactly the cursor, limit and mode it requested.
    ///
    /// # Parameters
    ///
    /// - `proof`: The GroveDB proof bytes.
    /// - `start_at`: Optional cursor: the contract id to start from and whether it is
    ///   included (`true` = `startAt`, `false` = `startAfter`). `None` for the first page.
    /// - `limit`: Maximum number of contracts in the page.
    /// - `ids_only`: Whether the proof carries contract ids only.
    /// - `platform_version`: The platform version for version dispatch.
    ///
    /// # Returns
    ///
    /// The root hash and the page rows in ascending contract id order. Every row is
    /// `(contract id, Some(contract))`, or `(contract id, None)` when `ids_only`. An empty
    /// page means there are no contracts at or after the cursor.
    ///
    /// # Errors
    ///
    /// Returns an error if the proof is corrupted, does not match the requested page, or the
    /// method version is unknown.
    pub fn verify_contracts_by_range(
        proof: &[u8],
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        ids_only: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DataContractsByRangePage), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract
            .verify_contracts_by_range
        {
            0 => Drive::verify_contracts_by_range_v0(
                proof,
                start_at,
                limit,
                ids_only,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contracts_by_range".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_unknown_verify_contracts_by_range_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .contract
            .verify_contracts_by_range = 255;

        let result = Drive::verify_contracts_by_range(&[], None, 10, false, &platform_version);

        assert!(
            matches!(result, Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                if method == "verify_contracts_by_range" && known_versions == vec![0] && received == 255
            )
        );
    }
}
