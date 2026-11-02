use crate::drive::contract::paths::contract_team_action_status_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Whether the action exists with that status, which the read of its approvals, a range under
    /// the action, is kept to. A point read: a contract that keeps no team actions, or an id
    /// nobody has, reads as having none.
    pub(super) fn contract_team_action_exists_v0(
        &self,
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        self.grove_has_raw(
            (&contract_team_action_status_path(contract_id.as_slice(), status)).into(),
            action_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }

    /// The approvals, read the way the transform of a moderation reads state: the operations
    /// of the read are added to `drive_operations` for billing.
    pub(super) fn fetch_contract_team_action_signers_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identifier>, Error> {
        let path_query = Drive::contract_team_action_signers_query(
            contract_id.to_buffer(),
            status,
            action_id.to_buffer(),
        );
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                let malformed = || {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "contract {} team action {} approval {:?} is malformed",
                        contract_id, action_id, key
                    )))
                };
                if !matches!(element, Element::SumItem(..)) {
                    return Err(malformed());
                }
                Identifier::from_bytes(&key).map_err(|_| malformed())
            })
            .collect()
    }
}
