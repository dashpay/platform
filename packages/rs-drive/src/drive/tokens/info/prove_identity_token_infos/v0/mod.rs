use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_identity_token_infos_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_identity_token_infos_operations_v0(
            token_ids,
            identity_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_identity_token_infos_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::token_infos_for_identity_id_query(token_ids, identity_id);

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
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::tokens::info::v0::IdentityTokenInfoV0;
    use dpp::tokens::info::IdentityTokenInfo;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_token_infos_with_one_token_frozen() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        // Create a random identity
        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

        let identity_id = identity.id().to_buffer();

        // Create a data contract with multiple tokens
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
            tokens: BTreeMap::from([
                (
                    0,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
                (
                    1,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
            ]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id_1 = contract.token_id(0).expect("expected token at position 0");
        let token_id_2 = contract.token_id(1).expect("expected token at position 1");

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

        // Freeze one token for the identity
        drive
            .token_freeze(
                token_id_1,
                identity.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token");

        let fetched_token_info_1 = drive
            .fetch_identity_token_info(token_id_1.to_buffer(), identity_id, None, platform_version)
            .expect("expected to fetch token info");

        assert_eq!(
            fetched_token_info_1,
            Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true }))
        );

        let fetched_token_info_2 = drive
            .fetch_identity_token_info(token_id_2.to_buffer(), identity_id, None, platform_version)
            .expect("expected to fetch token info");

        assert_eq!(fetched_token_info_2, None);

        // Fetch identity token infos before proving
        let fetched_token_infos = drive
            .fetch_identity_token_infos(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                identity_id,
                None,
                platform_version,
            )
            .expect("expected to fetch token infos");

        // Verify fetched token infos
        assert_eq!(
            fetched_token_infos,
            BTreeMap::from([
                (
                    token_id_1.to_buffer(),
                    Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })),
                ),
                (token_id_2.to_buffer(), None,),
            ]),
            "unexpected fetched token infos"
        );

        // Generate proof
        let proof = drive
            .prove_identity_token_infos_v0(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                identity_id,
                None,
                platform_version,
            )
            .expect("should not error when proving token infos");

        // Verify proof
        let proved_token_infos: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            Drive::verify_token_infos_for_identity_id(
                proof.as_slice(),
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                identity_id,
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert proved token infos match the fetched ones
        assert_eq!(
            proved_token_infos,
            BTreeMap::from([
                (
                    token_id_1.to_buffer(),
                    Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })),
                ),
                (token_id_2.to_buffer(), None,),
            ]),
            "unexpected fetched token infos"
        );
    }

    #[test]
    fn should_prove_no_token_infos_for_non_existent_identity() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let non_existent_identity_id = [0u8; 32]; // An identity that doesn't exist in the database

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
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let proof = drive
            .prove_identity_token_infos_v0(
                &[token_id.to_buffer()],
                non_existent_identity_id,
                None,
                platform_version,
            )
            .expect("should not error when proving token infos for non-existent identity");

        let proved_token_infos: BTreeMap<[u8; 32], Option<IdentityTokenInfo>> =
            Drive::verify_token_infos_for_identity_id(
                proof.as_slice(),
                &[token_id.to_buffer()],
                non_existent_identity_id,
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert that no token infos exist for the non-existent identity
        assert_eq!(
            proved_token_infos,
            BTreeMap::from([(token_id.to_buffer(), None)])
        );
    }
}
