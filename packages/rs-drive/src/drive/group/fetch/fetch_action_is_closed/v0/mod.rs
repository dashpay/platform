use crate::drive::group::paths::group_closed_action_root_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// V0 implementation — checks for the presence of the action tree under the *closed* root
    /// first; if absent, checks the *active* root.  
    /// Fails if the action is missing from **both** roots.
    pub(super) fn fetch_action_is_closed_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let pos_bytes = group_contract_position.to_be_bytes();
        let key = action_id.as_slice();

        // 1. Does the action exist under the *closed* root?
        let closed_root =
            group_closed_action_root_path(contract_id.as_slice(), pos_bytes.as_slice());
        if self
            .grove_get_raw_optional(
                (&closed_root).into(),
                key,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
            .is_some()
        {
            return Ok(true);
        }

        Ok(false)
    }
}
