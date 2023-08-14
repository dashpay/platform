mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the existence of a specified contract.
    ///
    /// This function creates a path query for the provided contract ID and then proves
    /// the existence of the contract in the context of the provided database `transaction`.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract ID as a 32-byte array. The contract ID is used to
    ///   create a path query for proving its existence.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for proving the existence of the contract. This is either None or Some(&Transaction).
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - If successful, returns a `Vec<u8>` containing the proof data.
    ///   If an error occurs during the proof generation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the path query generation or proof generation fails.
    pub fn prove_contract(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.contract.prove.prove_contract {
            0 => self.prove_contract_v0(contract_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
