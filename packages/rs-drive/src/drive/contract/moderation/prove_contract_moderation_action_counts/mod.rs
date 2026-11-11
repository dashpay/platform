mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The proof of the moderation action counts of the elected contract `contract_id`: how
    /// many counted moderation actions each member of its seated team signed since the
    /// moderators pot was last settled. What `verify_contract_moderation_action_counts` checks.
    /// Only an elected contract has the counts tree.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the proof.
    /// * `Err(Error)` when the method version is unknown or the proof fails.
    pub fn prove_contract_moderation_action_counts(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .prove_contract_moderation_action_counts
        {
            0 => self.prove_contract_moderation_action_counts_v0(
                contract_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_moderation_action_counts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
