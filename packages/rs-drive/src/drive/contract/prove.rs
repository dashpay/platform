use crate::drive::Drive;
use crate::error::Error;
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
    ) -> Result<Vec<u8>, Error> {
        let contract_query = Self::fetch_contract_query(contract_id);
        self.grove_get_proved_path_query(&contract_query, false, transaction, &mut vec![])
    }

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
    ) -> Result<Vec<u8>, Error> {
        let contracts_query = Self::fetch_contracts_query(contract_ids)?;
        self.grove_get_proved_path_query(&contracts_query, false, transaction, &mut vec![])
    }
}
