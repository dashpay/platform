use crate::drive::Drive;
use crate::error::Error;

use crate::drive::contract::paths::contract_root_path;
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
    #[inline(always)]
    pub(super) fn prove_contract_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.grove_get_proved_path_query_with_conditional(
            (&contract_root_path(&contract_id)).into(),
            &[0],
            &|element| {
                if let Some(element) = element {
                    if element.is_tree() {
                        // this is a contract that keeps history
                        Self::fetch_contract_with_history_latest_query(contract_id, true)
                    } else {
                        // this is a normal contract
                        Self::fetch_contract_query(contract_id, true)
                    }
                } else {
                    // we will just get the proof that the contract doesn't exist, either way
                    Self::fetch_contract_query(contract_id, true)
                }
            },
            false,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
