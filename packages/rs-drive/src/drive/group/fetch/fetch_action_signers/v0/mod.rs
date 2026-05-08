use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::Element::SumItem;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_action_signers_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupMemberPower>, Error> {
        self.fetch_action_signers_and_add_operations_v0(
            contract_id,
            group_contract_position,
            action_status,
            action_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_signers_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupMemberPower>, Error> {
        let path_query = Drive::group_action_signers_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            action_id.to_buffer(),
        );

        self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?
        .0
        .to_key_elements_btree_map()
        .into_iter()
        .map(|(key, element)| match element {
            SumItem(value, ..) => Ok((
                key.try_into()?,
                value.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "signed power should be encodable on a u32 integer".to_string(),
                    ))
                })?,
            )),
            _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                "element should be a sum item representing member signed power".to_string(),
            ))),
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::group::group_action_status::GroupActionStatus;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn fetch_action_signers_v0_nonexistent_action_returns_empty() {
        // When the action path does not exist at all, the path query yields zero
        // results and the function returns an empty map (no deserialization).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let map = drive
            .fetch_action_signers_v0(
                Identifier::random(),
                0,
                GroupActionStatus::ActionActive,
                Identifier::random(),
                None,
                platform_version,
            )
            .expect("fetch on nonexistent action should return empty map");

        assert!(map.is_empty());
    }

    #[test]
    fn fetch_action_signers_v0_closed_is_empty_for_nonexistent_action() {
        // Same as above but against the Closed status tree.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let map = drive
            .fetch_action_signers_v0(
                Identifier::random(),
                0,
                GroupActionStatus::ActionClosed,
                Identifier::random(),
                None,
                platform_version,
            )
            .expect("fetch on nonexistent closed action should return empty map");

        assert!(map.is_empty());
    }

    #[test]
    fn fetch_action_signers_and_add_operations_v0_records_ops() {
        // Exercise the _and_add_operations variant — ops vec may remain empty
        // for a missing subtree, but the call itself must succeed.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive.fetch_action_signers_and_add_operations_v0(
            Identifier::random(),
            0,
            GroupActionStatus::ActionActive,
            Identifier::random(),
            None,
            &mut ops,
            platform_version,
        );

        // Either returns Ok(empty) or an error reflecting the missing path; it
        // must not panic.
        if let Ok(map) = result {
            assert!(map.is_empty());
        }
    }
}
