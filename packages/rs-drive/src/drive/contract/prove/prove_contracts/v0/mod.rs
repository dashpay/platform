use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

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
    #[inline(always)]
    pub(super) fn prove_contracts_v0(
        &self,
        contract_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let contracts_query = Self::fetch_non_historical_contracts_query(contract_ids);

        // we first need to fetch all contracts
        let contracts = self.grove_get_raw_path_query_with_optional(
            &contracts_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        // We have 3 options
        // If the contract is non existing -> treat it as non historical
        // If the contract is there as an item -> it is non historical
        // If the contract is there as a tree -> it is historical

        let mut historical_contracts: Vec<[u8; 32]> = Vec::new();
        let mut non_historical_contracts: Vec<[u8; 32]> = Vec::new();

        for (path, _key, element) in contracts.into_iter() {
            let contract_id: [u8; 32] = path
                .last()
                .ok_or(Error::Drive(DriveError::CorruptedContractPath(
                    "the path should always have a last",
                )))?
                .clone()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedContractPath(
                        "the path last component should always be 32 bytes",
                    ))
                })?;

            if let Some(element) = element {
                match element {
                    Element::Item(_, _) => {
                        non_historical_contracts.push(contract_id);
                    }
                    Element::Tree(_, _) => {
                        historical_contracts.push(contract_id);
                    }
                    _ => {
                        return Err(Error::Drive(DriveError::CorruptedContractPath("")));
                    }
                }
            } else {
                non_historical_contracts.push(contract_id);
            }
        }

        let contracts_query = Self::fetch_contracts_query(
            non_historical_contracts.as_slice(),
            historical_contracts.as_slice(),
        )?;

        self.grove_get_proved_path_query(
            &contracts_query,
            true,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
