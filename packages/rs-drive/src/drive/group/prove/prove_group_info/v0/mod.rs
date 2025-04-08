use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_group_info_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_group_info_operations_v0(
            contract_id,
            group_contract_position,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_group_info_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::group_info_for_contract_id_and_group_contract_position_query(
            contract_id.to_buffer(),
            group_contract_position,
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
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_group_info_for_existing_group() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Create identities for group members
        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id();
        let identity_2_id = identity_2.id();

        // Create a data contract with a single group
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
            groups: BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(identity_1_id, 1), (identity_2_id, 2)].into(),
                    required_power: 3,
                }),
            )]),
            tokens: Default::default(),
            keywords: Vec::new(),
            description: None,
        });

        let contract_id = contract.id();

        // Insert the contract into Drive
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Prove group info for group at position 0
        let proof = drive
            .prove_group_info_v0(contract_id, 0, None, platform_version)
            .expect("should not error when proving group info");

        // Verify the proof
        let (root_hash, group) =
            Drive::verify_group_info(proof.as_slice(), contract_id, 0, false, platform_version)
                .expect("expected proof verification to succeed");

        // Assert root hash is valid
        assert!(!root_hash.is_empty(), "root hash should not be empty");

        // Assert group matches the expected value
        assert_eq!(
            group,
            Some(Group::V0(GroupV0 {
                members: [(identity_1_id, 1), (identity_2_id, 2)].into(),
                required_power: 3,
            })),
            "unexpected group info"
        );
    }

    #[test]
    fn should_prove_no_group_info_for_non_existent_group() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Create a data contract without groups
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
            groups: Default::default(),
            tokens: Default::default(),
            keywords: Vec::new(),
            description: None,
        });

        let contract_id = contract.id();

        // Insert the contract into Drive
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Prove group info for a non-existent group at position 0
        let proof = drive
            .prove_group_info_v0(contract_id, 0, None, platform_version)
            .expect("should not error when proving group info for a non-existent group");

        // Verify the proof
        let (root_hash, group) =
            Drive::verify_group_info(proof.as_slice(), contract_id, 0, false, platform_version)
                .expect("expected proof verification to succeed");

        // Assert root hash is valid
        assert!(!root_hash.is_empty(), "root hash should not be empty");

        // Assert group is None
        assert!(group.is_none(), "expected no group info, but got some");
    }
}
