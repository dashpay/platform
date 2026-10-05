mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the version items of the specified contracts: for each requested id, either
    /// the four-byte version item stored beside the contract or its absence.
    ///
    /// This is the proof `getDataContractsLatestVersions` returns when the contracts
    /// themselves are not requested, from protocol version 14 (state before that carries no
    /// version items). It is a few hundred bytes of hash path per contract instead of the
    /// serialized contract the multi-contract proof carries. `Drive::verify_contracts_versions`
    /// verifies it.
    ///
    /// # Arguments
    ///
    /// * `contract_ids` - The contract ids whose versions to prove, at least one; duplicates
    ///   are folded.
    /// * `transaction` - The transaction to prove against, or `None` for the committed state.
    /// * `platform_version` - The platform version for version dispatch.
    ///
    /// # Returns
    ///
    /// The GroveDB proof bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if no ids were given, the proof generation fails, or the method
    /// version is unknown.
    pub fn prove_contracts_versions(
        &self,
        contract_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .prove
            .prove_contracts_versions
        {
            0 => self.prove_contracts_versions_v0(contract_ids, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contracts_versions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
