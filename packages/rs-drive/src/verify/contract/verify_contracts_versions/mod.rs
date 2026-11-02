mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// The verified versions of the requested contracts, keyed by contract id: `None` for an id
/// no contract has.
pub type DataContractsVersions = BTreeMap<[u8; 32], Option<u32>>;

impl Drive {
    /// Verifies a proof of the version items of several contracts, the proof
    /// `getDataContractsLatestVersions` returns without the contracts from protocol version
    /// 14 (`Drive::prove_contracts_versions`).
    ///
    /// The proved query is rebuilt from the requested ids, so the caller must pass exactly
    /// the ids it requested; duplicates are folded the way the prover folds them.
    ///
    /// # Parameters
    ///
    /// - `proof`: The GroveDB proof bytes.
    /// - `contract_ids`: The requested contract ids, at least one.
    /// - `platform_version`: The platform version for version dispatch.
    ///
    /// # Returns
    ///
    /// The root hash and one entry per distinct requested id: `Some(version)` for a contract
    /// in state, `None` for an id no contract has.
    ///
    /// # Errors
    ///
    /// Returns an error if the proof is corrupted, does not cover exactly the requested ids,
    /// or the method version is unknown.
    pub fn verify_contracts_versions(
        proof: &[u8],
        contract_ids: &[[u8; 32]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DataContractsVersions), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract
            .verify_contracts_versions
        {
            0 => Drive::verify_contracts_versions_v0(proof, contract_ids, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contracts_versions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
