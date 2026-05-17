use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use grovedb::GroveDb;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;

impl Drive {
    pub(super) fn verify_token_perpetual_distribution_last_paid_time_v0(
        proof: &[u8],
        token_id: [u8; 32],
        identity_id: [u8; 32],
        distribution_type: &RewardDistributionType,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<RewardDistributionMoment>), Error> {
        let path_query =
            Drive::perpetual_distribution_last_paid_moment_query(token_id, identity_id);
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
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
        if proved_key_values.len() == 1 {
            let proved_key_value = proved_key_values.remove(0);
            match proved_key_value.2 {
                Some(Item(value, ..)) => {
                    let moment = distribution_type.moment_from_bytes(&value).map_err(|e| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "Moment should be specific amount of bytes: {}",
                            e
                        )))
                    })?;
                    Ok((root_hash, Some(moment)))
                }
                None => Ok((root_hash, None)),
                _ => Err(Error::Proof(ProofError::IncorrectValueSize(
                    "proof did not point to an item",
                ))),
            }
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: 1,
                got: proved_key_values.len(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::tokens::paths::token_perpetual_distributions_identity_last_claimed_time_path_vec;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::{TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters};
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use grovedb::Element;
    use std::collections::BTreeMap;

    /// Helper to build a token configuration that includes a block-based perpetual distribution
    /// so that `insert_contract` creates the necessary GroveDB subtree structure.
    fn token_config_with_perpetual_distribution() -> TokenConfigurationV0 {
        let distribution_type = RewardDistributionType::BlockBasedDistribution {
            interval: 100,
            function: DistributionFunction::FixedAmount { amount: 50 },
        };
        let perpetual = TokenPerpetualDistribution::V0(TokenPerpetualDistributionV0 {
            distribution_type,
            distribution_recipient: TokenDistributionRecipient::ContractOwner,
        });
        let mut config = TokenConfigurationV0::default_most_restrictive();
        config
            .distribution_rules_mut()
            .set_perpetual_distribution(Some(perpetual));
        config
    }

    #[test]
    fn should_prove_and_verify_perpetual_distribution_last_paid_time() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id();

        let token_config = token_config_with_perpetual_distribution();
        let distribution_type = token_config
            .distribution_rules()
            .perpetual_distribution()
            .unwrap()
            .distribution_type()
            .clone();

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
            tokens: BTreeMap::from([(0, TokenConfiguration::V0(token_config))]),
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

        let moment = RewardDistributionMoment::BlockBasedMoment(42);
        let moment_bytes = moment.to_be_bytes_vec();

        // Insert the last-paid-moment directly into GroveDB
        let path =
            token_perpetual_distributions_identity_last_claimed_time_path_vec(token_id.to_buffer());
        let path_refs: Vec<&[u8]> = path.iter().map(|p| p.as_slice()).collect();
        drive
            .grove
            .insert(
                path_refs.as_slice(),
                identity_id.as_slice(),
                Element::new_item(moment_bytes),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert moment into grove");

        // Generate proof
        let proof = drive
            .prove_perpetual_distribution_last_paid_moment(
                token_id.to_buffer(),
                identity_id,
                None,
                platform_version,
            )
            .expect("expected to prove perpetual distribution last paid moment");

        // Verify proof
        let (_, verified_moment) = Drive::verify_token_perpetual_distribution_last_paid_time(
            proof.as_slice(),
            token_id.to_buffer(),
            identity_id.to_buffer(),
            &distribution_type,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(verified_moment, Some(moment));
    }

    #[test]
    fn should_prove_and_verify_absent_perpetual_distribution_last_paid_time() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_id = identity.id();

        let token_config = token_config_with_perpetual_distribution();
        let distribution_type = token_config
            .distribution_rules()
            .perpetual_distribution()
            .unwrap()
            .distribution_type()
            .clone();

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
            tokens: BTreeMap::from([(0, TokenConfiguration::V0(token_config))]),
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

        // No claimed moment set -- should be absent
        let proof = drive
            .prove_perpetual_distribution_last_paid_moment(
                token_id.to_buffer(),
                identity_id,
                None,
                platform_version,
            )
            .expect("expected to prove perpetual distribution last paid moment");

        let (_, verified_moment) = Drive::verify_token_perpetual_distribution_last_paid_time(
            proof.as_slice(),
            token_id.to_buffer(),
            identity_id.to_buffer(),
            &distribution_type,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(verified_moment, None);
    }
}
