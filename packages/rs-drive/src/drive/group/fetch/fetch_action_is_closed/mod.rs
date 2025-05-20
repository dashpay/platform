mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Returns **`true`** if the action has already been closed, **`false`** if it is still
    /// active.  
    ///
    /// *Errors* if the action is missing or if the platform‑version number is unknown.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_action_is_closed(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_is_closed
        {
            0 => self.fetch_action_is_closed_v0(
                contract_id,
                group_contract_position,
                action_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_is_closed".to_owned(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
