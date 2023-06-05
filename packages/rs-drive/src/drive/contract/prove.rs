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
        let contract_proof =
            self.grove_get_proved_path_query(&contract_query, false, transaction, &mut vec![])?;
        let result =
            Drive::verify_contract(contract_proof.as_slice(), Some(false), false, contract_id);
        match result {
            Ok(_) => Ok(contract_proof),
            Err(Error::GroveDB(grovedb::Error::WrongElementType(s))) if s == "expected an item" => {
                // In this case we are trying to prove a historical type contract
                let contract_query = Self::fetch_contract_with_history_latest_query(contract_id);
                let historical_contract_proof = self.grove_get_proved_path_query(
                    &contract_query,
                    false,
                    transaction,
                    &mut vec![],
                )?;
                let (_, proof_returned_historical_contract) = Drive::verify_contract(
                    historical_contract_proof.as_slice(),
                    Some(true),
                    false,
                    contract_id,
                )
                .expect("expected to get contract from proof");
                // Only return the historical proof if an element was found
                if proof_returned_historical_contract.is_some() {
                    Ok(historical_contract_proof)
                } else {
                    Ok(contract_proof)
                }
            }
            Err(e) => Err(e),
        }
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
    ) -> Result<Vec<u8>, Error> {
        let history_query =
            Self::fetch_contract_history_query(contract_id, start_at_date, limit, offset)?;

        self.grove_get_proved_path_query(&history_query, false, transaction, &mut vec![])
    }
}
