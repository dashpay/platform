use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dpp::prelude::Identifier;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_total_supply_and_aggregated_identity_balance_v0(
        proof: &[u8],
        token_id: [u8; 32],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, TotalSingleTokenBalance), Error> {
        let path_query = Self::token_total_supply_and_aggregated_identity_balances_query(
            token_id,
            platform_version,
        )?;
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() == 2 {
            let (
                _aggregated_identity_balances_path,
                _aggregated_identity_balances_key,
                Some(aggregated_identity_balances_element),
            ) = proved_key_values.first().unwrap()
            else {
                return Err(Error::Proof(ProofError::UnexpectedResultProof(format!(
                    "Token {} most likely does not exist",
                    Identifier::new(token_id)
                ))));
            };
            let (_total_supply_path, _total_supply_key, Some(total_supply_element)) =
                proved_key_values.get(1).unwrap()
            else {
                return Err(Error::Proof(ProofError::UnexpectedResultProof(format!(
                    "Token {} has no known supply",
                    Identifier::new(token_id)
                ))));
            };

            Ok((
                root_hash,
                TotalSingleTokenBalance {
                    token_supply: total_supply_element.as_sum_item_value()?,
                    aggregated_token_account_balances: aggregated_identity_balances_element
                        .as_sum_tree_value()?,
                },
            ))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: 2,
                got: proved_key_values.len(),
            }))
        }
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
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn create_token_contract() -> DataContract {
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
            groups: Default::default(),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        })
    }

    #[test]
    fn should_prove_and_verify_token_total_supply_and_aggregated_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id().to_buffer();

        let contract = create_token_contract();
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

        let mint_amount = 1000u64;
        drive
            .token_mint(
                token_id.to_buffer(),
                identity_id,
                mint_amount,
                true,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint tokens");

        let proof = drive
            .prove_token_total_supply_and_aggregated_identity_balances(
                token_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to get proof");

        let (_, total_balance) = Drive::verify_token_total_supply_and_aggregated_identity_balance(
            proof.as_slice(),
            token_id.to_buffer(),
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        // base_supply from default_most_restrictive is 100000, plus our mint of 1000
        let expected_supply = (100000 + mint_amount) as i64;
        assert_eq!(total_balance.token_supply, expected_supply);
        assert_eq!(
            total_balance.aggregated_token_account_balances,
            expected_supply
        );
    }
}
