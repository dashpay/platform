use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn prove_token_total_supply_and_aggregated_identity_balances_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_token_total_supply_and_aggregated_identity_balances_add_operations_v0(
            token_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_token_total_supply_and_aggregated_identity_balances_add_operations_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let combined_path_query = Drive::token_total_supply_and_aggregated_identity_balances_query(
            token_id,
            platform_version,
        )?;
        self.grove_get_proved_path_query(
            &combined_path_query,
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
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_token_total_supply_and_aggregated_identity_balances() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        // Create a data contract with a token
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

        // Mint tokens for identity accounts
        let identity_1 = [1u8; 32];
        let identity_2 = [2u8; 32];

        drive
            .token_mint(
                token_id.to_buffer(),
                identity_1,
                50_000,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint tokens for identity 1");

        drive
            .token_mint(
                token_id.to_buffer(),
                identity_2,
                50_000,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint tokens for identity 2");

        // Prove token total supply and aggregated identity balances
        let proof = drive
            .prove_token_total_supply_and_aggregated_identity_balances(
                token_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("should not error when proving total supply and balances");

        // Verify proof
        let (root_hash, total_single_token_balance) =
            Drive::verify_token_total_supply_and_aggregated_identity_balance(
                proof.as_slice(),
                token_id.to_buffer(),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        // Assert root hash is not empty
        assert!(!root_hash.is_empty(), "expected a valid root hash");

        // Assert total supply matches aggregated identity balances
        // The contract was created with 100k, then 50k to each identity
        assert_eq!(
            total_single_token_balance.token_supply, 200_000,
            "unexpected token supply"
        );
        assert_eq!(
            total_single_token_balance.aggregated_token_account_balances, 200_000,
            "unexpected aggregated token account balances"
        );

        // Assert that the total balance is valid
        assert!(
            total_single_token_balance
                .ok()
                .expect("expected total balance to be valid"),
            "unexpected total balance validation failure"
        );
    }

    #[test]
    fn should_prove_token_total_supply_and_aggregated_identity_balances_for_empty_token() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        // Create a data contract with a token
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
                TokenConfiguration::V0(
                    TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
                ),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id = contract.token_id(0).expect("expected token at position 0");

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

        // Prove token total supply and aggregated identity balances for a token with no supply
        let proof = drive
            .prove_token_total_supply_and_aggregated_identity_balances_v0(
                token_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("should not error when proving total supply and balances");

        // Verify proof
        let (root_hash, total_single_token_balance) =
            Drive::verify_token_total_supply_and_aggregated_identity_balance(
                proof.as_slice(),
                token_id.to_buffer(),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        // Assert root hash is not empty
        assert!(!root_hash.is_empty(), "expected a valid root hash");

        // Assert total supply matches aggregated identity balances
        assert_eq!(
            total_single_token_balance.token_supply, 0,
            "unexpected token supply"
        );
        assert_eq!(
            total_single_token_balance.aggregated_token_account_balances, 0,
            "unexpected aggregated token account balances"
        );

        // Assert that the total balance is valid
        assert!(
            total_single_token_balance
                .ok()
                .expect("expected total balance to be valid"),
            "unexpected total balance validation failure"
        );
    }
}
