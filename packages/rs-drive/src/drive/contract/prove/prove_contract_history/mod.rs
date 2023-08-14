mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the existence of the specified contract's history.
    ///
    /// This function creates a path query for each for the given contract id and limit and offset
    ///
    /// # Arguments
    ///
    /// * `contract_id` - A contract IDs as 32-byte array.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for proving the existence of the contracts. This is either None or Some(&Transaction).
    /// * `start_at_date` - The date to start the history query from.
    /// * `limit` - The maximum number of items to return.
    /// * `offset` - The number of items to skip before returning results.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - If successful, returns a `Vec<u8>` containing the proof data.
    ///   If an error occurs during the proof generation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the path query generation or proof generation fails.
    pub fn prove_contract_history(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .prove
            .prove_contract_history
        {
            0 => self.prove_contract_history_v0(
                contract_id,
                transaction,
                start_at_date,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
