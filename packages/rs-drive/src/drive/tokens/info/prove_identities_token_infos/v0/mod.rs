use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_identities_token_infos_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_identities_token_infos_operations_v0(
            token_id,
            identity_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_identities_token_infos_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::token_infos_for_identity_ids_query(token_id, identity_ids);

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
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::identity::Identity;
    use std::collections::BTreeMap;

    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::prelude::DataContract;
    use dpp::tokens::info::IdentityTokenInfo;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_a_single_identity_token_info() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_id = identity.id().to_buffer();

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
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });
        let token_id = contract.token_id(0).expect("expected token at position 0");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        drive
            .token_freeze(
                token_id,
                identity.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint token");
        let proof = drive
            .prove_identities_token_infos_v0(
                token_id.to_buffer(),
                &[identity.id().to_buffer()],
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let proved_identity_info: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            Drive::verify_token_infos_for_identity_ids(
                proof.as_slice(),
                token_id.to_buffer(),
                &[identity.id().to_buffer()],
                false,
                platform_version,
            )
            .expect("expect that this be verified")
            .1;

        assert_eq!(
            proved_identity_info,
            BTreeMap::from([(
                identity_id,
                Some(IdentityTokenInfo::new(true, platform_version).expect("expected token info"))
            )])
        );
    }

    #[test]
    fn should_prove_a_single_identity_token_info_does_not_exist() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_id = identity.id().to_buffer();

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
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });
        let token_id = contract.token_id(0).expect("expected token at position 0");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let proof = drive
            .prove_identities_token_infos_v0(
                token_id.to_buffer(),
                &[identity.id().to_buffer()],
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let proved_identity_info: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            Drive::verify_token_infos_for_identity_ids(
                proof.as_slice(),
                token_id.to_buffer(),
                &[identity.id().to_buffer()],
                false,
                platform_version,
            )
            .expect("expect that this be verified")
            .1;

        assert_eq!(proved_identity_info, BTreeMap::from([(identity_id, None)]));
    }

    #[test]
    fn should_prove_multiple_identity_single_token_infos() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_1_id = identity_1.id().to_buffer();

        let identity_2 = Identity::random_identity(3, Some(15), platform_version)
            .expect("expected a platform identity");

        let identity_2_id = identity_2.id().to_buffer();

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
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });
        let token_id = contract.token_id(0).expect("expected token at position 0");
        drive
            .add_new_identity(
                identity_1.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .add_new_identity(
                identity_2.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        drive
            .token_freeze(
                token_id,
                identity_1.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint token");

        let proof = drive
            .prove_identities_token_infos_v0(
                token_id.to_buffer(),
                &[identity_1.id().to_buffer(), identity_2.id().to_buffer()],
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let proved_identity_info: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            Drive::verify_token_infos_for_identity_ids(
                proof.as_slice(),
                token_id.to_buffer(),
                &[identity_1.id().to_buffer(), identity_2.id().to_buffer()],
                false,
                platform_version,
            )
            .expect("expect that this be verified")
            .1;

        assert_eq!(
            proved_identity_info,
            BTreeMap::from([
                (
                    identity_1_id,
                    Some(
                        IdentityTokenInfo::new(true, platform_version)
                            .expect("expected token info")
                    )
                ),
                (identity_2_id, None)
            ])
        );
    }
}
