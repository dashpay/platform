mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the given fee pots of a contract and the epochs they were last claimed in.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the pots belong to.
    /// * `pots`: The pots to prove, at least one.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the proof.
    /// * `Err(Error)` when the version is unknown, `pots` is empty, or proving fails.
    pub fn prove_contract_fee_pots(
        &self,
        contract_id: Identifier,
        pots: &[ContractFeePot],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .prove_contract_fee_pots
        {
            0 => self.prove_contract_fee_pots_v0(contract_id, pots, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_fee_pots".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
