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
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        )?
        .0
        .to_path_key_elements()
        .into_iter()
        .map(|(path, _key, element)| {
            // The query uses a subquery key (GROUP_INFO_KEY), so the returned key is the
            // subquery key, not the group contract position. The group contract position is
            // the last component of the path (the parent key that matched the range query).
            let Some(group_position_bytes) = path.last() else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "expected path to not be empty when fetching group infos".to_string(),
                )));
            };
            let group_contract_position: GroupContractPosition =
                GroupContractPosition::from_be_bytes(
                    group_position_bytes.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "group contract position not encoded on 2 bytes as expected"
                                .to_string(),
                        ))
                    })?,
                );
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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    fn contract_with_groups(identity_id: Identifier, positions: &[u16]) -> DataContract {
        let groups = positions
            .iter()
            .map(|p| {
                (
                    *p,
                    Group::V0(GroupV0 {
                        members: [(identity_id, 1)].into(),
                        required_power: 1,
                    }),
                )
            })
            .collect();
        DataContract::V1(DataContractV1 {
            id: Default::default(),
            version: 0,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups,
            tokens: Default::default(),
            keywords: Vec::new(),
            description: None,
        })
    }

    #[test]
    fn fetch_group_infos_v0_nonexistent_contract_returns_empty() {
        // Range query against a contract that has no entries in the group-actions
        // subtree should exit the to_path_key_elements collect loop immediately
        // with an empty map.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_group_infos_v0(Identifier::random(), None, Some(10), None, platform_version)
            .expect("expected empty fetch to succeed");

        assert!(result.is_empty());
    }

    #[test]
    fn fetch_group_infos_v0_zero_limit_returns_empty() {
        // limit of 0 should short-circuit the query and yield nothing, even when
        // groups exist.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let contract = contract_with_groups(identity.id(), &[0, 1, 2]);
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let result = drive
            .fetch_group_infos_v0(contract_id, None, Some(0), None, platform_version)
            .expect("expected zero-limit fetch to succeed");

        assert!(result.is_empty());
    }

    #[test]
    fn fetch_group_infos_v0_start_past_last_position_returns_empty() {
        // When start is strictly after the highest existing position, the
        // range yields no entries.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let contract = contract_with_groups(identity.id(), &[0, 1]);
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let result = drive
            .fetch_group_infos_v0(
                contract_id,
                Some((500, true)),
                Some(10),
                None,
                platform_version,
            )
            .expect("expected out-of-range start fetch to succeed");

        assert!(result.is_empty());
    }

    #[test]
    fn fetch_group_infos_operations_v0_records_read_operations() {
        // The _operations variant must append read ops to the supplied vec.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let contract = contract_with_groups(identity.id(), &[0]);
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let mut ops = vec![];
        let result = drive
            .fetch_group_infos_operations_v0(
                contract_id,
                None,
                Some(10),
                None,
                &mut ops,
                platform_version,
            )
            .expect("expected fetch_operations to succeed");

        assert_eq!(result.len(), 1);
        assert!(
            !ops.is_empty(),
            "operations should be recorded for a stateful path query"
        );
    }

    #[test]
    fn fetch_group_infos_v0_limit_enforced() {
        // limit=1 should truncate results even when multiple groups exist.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let contract = contract_with_groups(identity.id(), &[0, 1, 2, 3]);
        let contract_id = contract.id();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let result = drive
            .fetch_group_infos_v0(contract_id, None, Some(1), None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(result.len(), 1, "limit=1 should return exactly one entry");
    }
}
