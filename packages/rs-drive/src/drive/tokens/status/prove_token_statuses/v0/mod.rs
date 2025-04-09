use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_token_statuses_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_token_statuses_operations_v0(
            token_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_token_statuses_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::token_statuses_query(token_ids);

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
    use dpp::prelude::DataContract;
    use dpp::tokens::status::v0::TokenStatusV0;
    use dpp::tokens::status::TokenStatus;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_token_statuses_for_multiple_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

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
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Freeze the first token
        drive
            .token_apply_status(
                token_id_1.to_buffer(),
                TokenStatus::new(true, platform_version).expect("expected token status"),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to freeze token");

        let fetched_token_info_1 = drive
            .fetch_token_status(token_id_1.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");

        assert_eq!(
            fetched_token_info_1,
            Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
        );

        let fetched_token_info_2 = drive
            .fetch_token_status(token_id_2.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");

        assert_eq!(fetched_token_info_2, None);

        // Fetch token statuses before proving
        let fetched_token_statuses = drive
            .fetch_token_statuses(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                None,
                platform_version,
            )
            .expect("expected to fetch token statuses");

        // Verify fetched token statuses
        assert_eq!(
            fetched_token_statuses,
            BTreeMap::from([
                (
                    token_id_1.to_buffer(),
                    Some(TokenStatus::V0(TokenStatusV0 { paused: true })),
                ),
                (token_id_2.to_buffer(), None,),
            ]),
            "unexpected fetched token infos"
        );

        // Generate proof
        let proof = drive
            .prove_token_statuses(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                None,
                platform_version,
            )
            .expect("should not error when proving token statuses");

        // Verify proof
        let proved_token_statuses: BTreeMap<[u8; 32], Option<TokenStatus>> =
            Drive::verify_token_statuses(
                proof.as_slice(),
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert the token statuses match expected values
        assert_eq!(
            proved_token_statuses,
            BTreeMap::from([
                (
                    token_id_1.to_buffer(),
                    Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
                ),
                (token_id_2.to_buffer(), None),
            ]),
            "unexpected token statuses"
        );
    }

    #[test]
    fn should_prove_no_token_statuses_for_non_existent_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let non_existent_token_id = [0u8; 32]; // A token ID that doesn't exist in the database

        // Generate proof
        let proof = drive
            .prove_token_statuses_v0(&[non_existent_token_id], None, platform_version)
            .expect("should not error when proving token statuses for non-existent tokens");

        // Verify proof
        let proved_token_statuses: BTreeMap<[u8; 32], Option<TokenStatus>> =
            Drive::verify_token_statuses(
                proof.as_slice(),
                &[non_existent_token_id],
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert that the token status is `None` for the non-existent token
        assert_eq!(
            proved_token_statuses,
            BTreeMap::from([(non_existent_token_id, None)]),
            "unexpected token statuses for non-existent tokens"
        );
    }
}
