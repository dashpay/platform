use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_token_contract_info_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::token_contract_info_query(token_id);

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
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
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::tokens::contract_info::v0::TokenContractInfoV0;
    use dpp::tokens::contract_info::TokenContractInfo;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_token_contract_info() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        // Create a random identity
        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

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

        let fetched_token_contract_info_1 = drive
            .fetch_token_contract_info(token_id_1.to_buffer(), None, platform_version)
            .expect("expected to fetch token info");

        assert_eq!(
            fetched_token_contract_info_1,
            Some(TokenContractInfo::V0(TokenContractInfoV0 {
                contract_id: contract.id(),
                token_contract_position: 0
            }))
        );

        let fetched_token_contract_info_2 = drive
            .fetch_token_contract_info(token_id_2.to_buffer(), None, platform_version)
            .expect("expected to fetch token info");

        assert_eq!(
            fetched_token_contract_info_2,
            Some(TokenContractInfo::V0(TokenContractInfoV0 {
                contract_id: contract.id(),
                token_contract_position: 1
            }))
        );

        // Generate proof
        let proof = drive
            .prove_token_contract_info(token_id_1.to_buffer(), None, platform_version)
            .expect("should not error when proving token infos");

        // Verify proof
        let proved_token_contract_info: Option<TokenContractInfo> =
            Drive::verify_token_contract_info(
                proof.as_slice(),
                token_id_1.to_buffer(),
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed")
            .1;

        // Assert proved token infos match the fetched ones
        assert_eq!(
            proved_token_contract_info,
            Some(TokenContractInfo::V0(TokenContractInfoV0 {
                contract_id: contract.id(),
                token_contract_position: 0
            })),
            "unexpected fetched token contract info"
        );
    }
}
