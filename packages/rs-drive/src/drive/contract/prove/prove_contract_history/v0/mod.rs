use crate::drive::Drive;
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
    #[inline(always)]
    pub(super) fn prove_contract_history_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let history_query =
            Self::fetch_contract_history_query(contract_id, start_at_date, limit, offset)?;

        self.grove_get_proved_path_query(
            &history_query,
            false,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
