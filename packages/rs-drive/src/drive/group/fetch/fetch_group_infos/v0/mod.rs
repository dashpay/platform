use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_group_infos_v0(
        &self,
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<GroupContractPosition, Group>, Error> {
        self.fetch_group_infos_operations_v0(
            contract_id,
            start_group_contract_position,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_group_infos_operations_v0(
        &self,
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<GroupContractPosition, Group>, Error> {
        let path_query = Self::group_infos_for_contract_id_query(
            contract_id.to_buffer(),
            start_group_contract_position,
            limit,
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
        .map(|(key, element)| {
            let group_contract_position: GroupContractPosition =
                GroupContractPosition::from_be_bytes(key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "group contract position not encoded on 2 bytes as expected".to_string(),
                    ))
                })?);
            match element {
                Item(value, ..) => Ok((
                    group_contract_position,
                    Group::deserialize_from_bytes(&value)?,
                )),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for infos should contain only items".to_string(),
                ))),
            }
        })
        .collect()
    }
}
