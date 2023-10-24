use crate::drive::Drive;
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
    pub(super) fn prove_contract_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let contract_query = Self::fetch_contract_query(contract_id, true);
        tracing::trace!(?contract_query, "proving contract");
        let contract_proof = self.grove_get_proved_path_query(
            &contract_query,
            false,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        let result = Drive::verify_contract(
            contract_proof.as_slice(),
            Some(false),
            false,
            false,
            contract_id,
            platform_version,
        );
        match result {
            Ok(_) => Ok(contract_proof),
            Err(Error::GroveDB(grovedb::Error::WrongElementType(s))) if s == "expected an item" => {
                // In this case we are trying to prove a historical type contract
                let contract_query =
                    Self::fetch_contract_with_history_latest_query(contract_id, true);
                tracing::trace!(?contract_query, "proving historical contract");
                let historical_contract_proof = self.grove_get_proved_path_query(
                    &contract_query,
                    false,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )?;
                let (_, proof_returned_historical_contract) = Drive::verify_contract(
                    historical_contract_proof.as_slice(),
                    Some(true),
                    false,
                    false,
                    contract_id,
                    platform_version,
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
}
