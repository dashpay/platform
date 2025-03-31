use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_infos_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupAction>, Error> {
        self.fetch_action_infos_and_add_operations_v0(
            contract_id,
            group_contract_position,
            action_status,
            start_action_id,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_infos_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupAction>, Error> {
        let path_query = Drive::group_action_infos_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            start_action_id.map(|(s, i)| (s.to_buffer(), i)),
            limit,
        );

        self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        )?
        .0
        .to_path_key_elements()
        .into_iter()
        .map(|(path, _, element)| {
            let Some(last_path_component) = path.last() else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "we should always have a path not be empty".to_string(),
                )));
            };
            let action_id = Identifier::from_bytes(last_path_component)?;

            match element {
                Item(value, ..) => Ok((action_id, GroupAction::deserialize_from_bytes(&value)?)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "element should be an item representing the group action".to_string(),
                ))),
            }
        })
        .collect()
    }
}
