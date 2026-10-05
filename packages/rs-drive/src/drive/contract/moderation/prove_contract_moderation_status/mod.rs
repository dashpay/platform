mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves one identity's status on a moderated contract: its entry, present or absent, in
    /// every list of `lists`, which are the lists the contract's config declares.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity.
    /// * `lists`: The lists the contract keeps; at least one.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the proof.
    /// * `Err(Error)` when `lists` is empty, the version is unknown or proving fails.
    pub fn prove_contract_moderation_status(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .prove_contract_moderation_status
        {
            0 => self.prove_contract_moderation_status_v0(
                contract_id,
                identity_id,
                lists,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_moderation_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
