mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The proof of who approved one of a contract's team actions, active or closed as `status`
    /// says: what `verify_contract_team_action_signers` checks.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `status`: Whether the action is active or closed.
    /// * `action_id`: The action.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the proof.
    /// * `Err(Error)` when the method version is unknown or the proof fails.
    pub fn prove_contract_team_action_signers(
        &self,
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .prove_contract_team_action_signers
        {
            0 => self.prove_contract_team_action_signers_v0(
                contract_id,
                status,
                action_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_team_action_signers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
