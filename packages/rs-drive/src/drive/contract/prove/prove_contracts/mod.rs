mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the existence of the specified contracts.
    ///
    /// This function creates a path query for each contract ID provided, and then proves
    /// the existence of the contracts in the context of the provided database `transaction`.
    ///
    /// # Arguments
    ///
    /// * `contract_ids` - A slice of contract IDs as 32-byte arrays. Each contract ID is used to
    ///   create a path query for proving its existence.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for proving the existence of the contracts. This is either None or Some(&Transaction).
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - If successful, returns a `Vec<u8>` containing the proof data.
    ///   If an error occurs during the proof generation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the path query generation or proof generation fails.
    pub fn prove_contracts(
        &self,
        contract_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version.methods.contract.prove.prove_contracts {
            0 => self.prove_contracts_v0(contract_ids, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contracts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
