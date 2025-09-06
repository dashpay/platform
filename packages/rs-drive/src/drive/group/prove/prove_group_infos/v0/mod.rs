use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_group_infos_v0(
        &self,
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_group_infos_operations_v0(
            contract_id,
            start_group_contract_position,
            limit,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_group_infos_operations_v0(
        &self,
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::group_infos_for_contract_id_query(
            contract_id.to_buffer(),
            start_group_contract_position,
            limit,
        );
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::GroupContractPosition;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_group_infos_with_multiple_groups() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let identity_2_id = identity_2.id();

        let identity_3 = Identity::random_identity(3, Some(9), platform_version)
            .expect("expected a platform identity");

        let identity_3_id = identity_3.id();

        // Create a data contract with groups
        let contract = DataContract::V1(DataContractV1 {
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
            groups: BTreeMap::from([
                (
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                        required_power: 2,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    }),
                ),
            ]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let contract_id = contract.id();

        // Insert contract into Drive
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Prove group infos
        let proof = drive
            .prove_group_infos_v0(
                contract_id,
                None,     // No start position
                Some(10), // Limit
                None,
                platform_version,
            )
            .expect("should not error when proving group infos");

        // Verify proof
        let proved_group_infos: BTreeMap<GroupContractPosition, Group> =
            Drive::verify_group_infos_in_contract(
                proof.as_slice(),
                contract_id,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert group infos match expected values
        assert_eq!(
            proved_group_infos,
            BTreeMap::from([
                (
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                        required_power: 2,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    }),
                ),
            ]),
            "unexpected group infos"
        );
    }

    #[test]
    fn should_prove_group_infos_with_start_position_and_limit() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let identity_2_id = identity_2.id();

        let identity_3 = Identity::random_identity(3, Some(9), platform_version)
            .expect("expected a platform identity");

        let identity_3_id = identity_3.id();

        // Create a data contract with groups
        let contract = DataContract::V1(DataContractV1 {
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
            groups: BTreeMap::from([
                (
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                        required_power: 2,
                    }),
                ),
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    }),
                ),
                (
                    2,
                    Group::V0(GroupV0 {
                        members: [(identity_2_id, 1), (identity_3_id, 2)].into(),
                        required_power: 3,
                    }),
                ),
                (
                    3,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 1), (identity_3_id, 1)].into(),
                        required_power: 2,
                    }),
                ),
            ]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let contract_id = contract.id();

        // Insert contract into Drive
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Prove group infos starting from position 1 with a limit of 2
        let proof = drive
            .prove_group_infos_v0(
                contract_id,
                Some((1, true)),
                Some(2),
                None,
                platform_version,
            )
            .expect("should not error when proving group infos");

        // Verify proof
        let proved_group_infos: BTreeMap<GroupContractPosition, Group> =
            Drive::verify_group_infos_in_contract(
                proof.as_slice(),
                contract_id,
                Some((1, true)),
                Some(2),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert group infos match expected values
        assert_eq!(
            proved_group_infos,
            BTreeMap::from([
                (
                    1,
                    Group::V0(GroupV0 {
                        members: [(identity_1_id, 2), (identity_2_id, 1), (identity_3_id, 1)]
                            .into(),
                        required_power: 2,
                    })
                ),
                (
                    2,
                    Group::V0(GroupV0 {
                        members: [(identity_2_id, 1), (identity_3_id, 2)].into(),
                        required_power: 3,
                    }),
                ),
            ]),
            "unexpected group infos"
        );
    }

    #[test]
    fn should_prove_no_group_infos_for_empty_contract() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let contract_id = Identifier::random();

        // Prove group infos for an empty contract
        let proof = drive
            .prove_group_infos_v0(contract_id, None, Some(10), None, platform_version)
            .expect("should not error when proving group infos for an empty contract");

        // Verify proof
        let proved_group_infos: BTreeMap<GroupContractPosition, Group> =
            Drive::verify_group_infos_in_contract(
                proof.as_slice(),
                contract_id,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert no group infos exist for the empty contract
        assert!(
            proved_group_infos.is_empty(),
            "expected no group infos, but got: {:?}",
            proved_group_infos
        );
    }
}
