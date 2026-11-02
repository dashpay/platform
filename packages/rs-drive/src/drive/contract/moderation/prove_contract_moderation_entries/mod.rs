mod v0;

use crate::drive::contract::moderation::types::ContractModerationEntriesQuery;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves one page of one moderation list of a contract.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `query`: The list, the cursor and the limit.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the proof.
    /// * `Err(Error)` when the limit is out of bounds, the version is unknown or proving fails.
    pub fn prove_contract_moderation_entries(
        &self,
        contract_id: Identifier,
        query: &ContractModerationEntriesQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .prove_contract_moderation_entries
        {
            0 => self.prove_contract_moderation_entries_v0(
                contract_id,
                query,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_moderation_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
