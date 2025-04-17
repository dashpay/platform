use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_identity_token_balances_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_identity_token_balances_operations_v0(
            token_ids,
            identity_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_identity_token_balances_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::token_balances_for_identity_id_query(token_ids, identity_id);

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
    use dpp::balances::credits::TokenAmount;
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
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_multiple_token_balances_for_single_identity() {
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

        drive
            .token_mint(
                token_id_1.to_buffer(),
                identity.id().to_buffer(),
                5000,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint token 1");

        let token_balance_1 = drive
            .fetch_identity_token_balance(
                token_id_1.to_buffer(),
                identity_id,
                None,
                platform_version,
            )
            .expect("should not error when fetching token balance");

        assert_eq!(token_balance_1, Some(5000));

        drive
            .token_mint(
                token_id_2.to_buffer(),
                identity.id().to_buffer(),
                10000,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint token 2");

        let token_balance_2 = drive
            .fetch_identity_token_balance(
                token_id_2.to_buffer(),
                identity_id,
                None,
                platform_version,
            )
            .expect("should not error when fetching token balance");

        assert_eq!(token_balance_2, Some(10000));

        let token_balances = drive
            .fetch_identity_token_balances(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                identity_id,
                None,
                platform_version,
            )
            .expect("should not error when fetching token balances");

        assert_eq!(
            token_balances,
            BTreeMap::from([
                (token_id_1.to_buffer(), Some(5000)),
                (token_id_2.to_buffer(), Some(10000)),
            ])
        );

        let proof = drive
            .prove_identity_token_balances_v0(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                identity_id,
                None,
                platform_version,
            )
            .expect("should not error when proving token balances");

        let proved_identity_balance: BTreeMap<[u8; 32], Option<TokenAmount>> =
            Drive::verify_token_balances_for_identity_id(
                proof.as_slice(),
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                identity.id().to_buffer(),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        assert_eq!(
            proved_identity_balance,
            BTreeMap::from([
                (token_id_1.to_buffer(), Some(5000)),
                (token_id_2.to_buffer(), Some(10000)),
            ])
        );
    }

    #[test]
    fn should_prove_no_token_balances_for_single_identity() {
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
            .prove_identity_token_balances_v0(
                &[token_id.to_buffer()],
                identity_id,
                None,
                platform_version,
            )
            .expect("should not error when proving token balances");

        let proved_identity_balance: BTreeMap<[u8; 32], Option<TokenAmount>> =
            Drive::verify_token_balances_for_identity_id(
                proof.as_slice(),
                &[token_id.to_buffer()],
                identity.id().to_buffer(),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        assert_eq!(
            proved_identity_balance,
            BTreeMap::from([(token_id.to_buffer(), None)])
        );
    }

    #[test]
    fn should_prove_no_token_balances_for_non_existent_identity() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        // Create a random identity ID that is not added to the drive
        let non_existent_identity_id = [1u8; 32]; // An example of a non-existent identity ID

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

        // Insert the contract but no identities
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Attempt to prove token balances for a non-existent identity
        let proof_result = drive.prove_identity_token_balances_v0(
            &[token_id.to_buffer()],
            non_existent_identity_id,
            None,
            platform_version,
        );

        // Ensure proof generation succeeds even for non-existent identities
        assert!(proof_result.is_ok(), "proof generation should succeed");

        let proof = proof_result.expect("expected valid proof");

        // Verify the proof
        let proved_identity_balance: BTreeMap<[u8; 32], Option<TokenAmount>> =
            Drive::verify_token_balances_for_identity_id(
                proof.as_slice(),
                &[token_id.to_buffer()],
                non_existent_identity_id,
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Verify the balance is `None` for the non-existent identity
        assert_eq!(
            proved_identity_balance,
            BTreeMap::from([(token_id.to_buffer(), None)]),
            "expected no balances for non-existent identity"
        );
    }
}
