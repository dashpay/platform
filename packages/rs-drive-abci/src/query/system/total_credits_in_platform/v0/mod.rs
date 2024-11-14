use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_total_credits_in_platform_request::GetTotalCreditsInPlatformRequestV0;
use dapi_grpc::platform::v0::get_total_credits_in_platform_response::{
    get_total_credits_in_platform_response_v0, GetTotalCreditsInPlatformResponseV0,
};
use dpp::block::epoch::Epoch;
use dpp::check_validation_result_with_data;
use dpp::core_subsidy::epoch_core_reward_credits_for_distribution::epoch_core_reward_credits_for_distribution;
use dpp::core_subsidy::NetworkCoreSubsidy;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::balances::{
    total_credits_on_platform_path_query, TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
};
use drive::drive::credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_CORE_HEIGHT;
use drive::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use drive::drive::credit_pools::epochs::paths::EpochProposers;
use drive::drive::system::misc_path;
use drive::drive::RootTree;
use drive::error::proof::ProofError;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::DirectQueryType;

impl<C> Platform<C> {
    pub(super) fn query_total_credits_in_platform_v0(
        &self,
        GetTotalCreditsInPlatformRequestV0 { prove }: GetTotalCreditsInPlatformRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTotalCreditsInPlatformResponseV0>, Error> {
        let response = if prove {
            let mut total_credits_on_platform_path_query = total_credits_on_platform_path_query();

            total_credits_on_platform_path_query.query.limit = None;

            let unpaid_epoch = self.drive.get_unpaid_epoch_index(None, platform_version)?;

            // we also need the path_query for the start_core_height of this unpaid epoch
            let unpaid_epoch_index_path_query = PathQuery {
                path: vec![vec![RootTree::Pools as u8]],
                query: SizedQuery {
                    query: Query::new_single_key(KEY_UNPAID_EPOCH_INDEX.to_vec()),
                    limit: None,
                    offset: None,
                },
            };

            let epoch = Epoch::new(unpaid_epoch)?;

            let start_core_height_query = PathQuery {
                path: epoch.get_path_vec(),
                query: SizedQuery {
                    query: Query::new_single_key(KEY_START_BLOCK_CORE_HEIGHT.to_vec()),
                    limit: None,
                    offset: None,
                },
            };

            let path_query = PathQuery::merge(
                vec![
                    &total_credits_on_platform_path_query,
                    &unpaid_epoch_index_path_query,
                    &start_core_height_query,
                ],
                &platform_version.drive.grove_version,
            )?;

            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            GetTotalCreditsInPlatformResponseV0 {
                result: Some(get_total_credits_in_platform_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let path_holding_total_credits = misc_path();
            let total_credits_in_platform = self
                .drive
                .grove_get_raw_value_u64_from_encoded_var_vec(
                    (&path_holding_total_credits).into(),
                    TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )?
                .unwrap_or_default(); // 0  would mean we haven't initialized yet

            let unpaid_epoch_index = self.drive.get_unpaid_epoch_index(None, platform_version)?;

            let unpaid_epoch = Epoch::new(unpaid_epoch_index)?;

            let start_block_core_height = if unpaid_epoch.index == 0 {
                self.drive
                    .fetch_genesis_core_height(None, platform_version)?
            } else {
                self.drive.get_epoch_start_block_core_height(
                    &unpaid_epoch,
                    None,
                    platform_version,
                )? + 1
            };

            let reward_credits_accumulated_during_current_epoch =
                epoch_core_reward_credits_for_distribution(
                    start_block_core_height,
                    platform_state.last_committed_core_height(),
                    self.config.network.core_subsidy_halving_interval(),
                    platform_version,
                )?;

            let total_credits_with_rewards = total_credits_in_platform.checked_add(reward_credits_accumulated_during_current_epoch).ok_or(drive::error::Error::Proof(ProofError::CorruptedProof("overflow while adding platform credits with reward credits accumulated during current epoch".to_string())))?;

            GetTotalCreditsInPlatformResponseV0 {
                result: Some(get_total_credits_in_platform_response_v0::Result::Credits(
                    total_credits_with_rewards,
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dashcore_rpc::dashcore::Network;
    use dpp::block::epoch::EpochIndex;
    use dpp::fee::Credits;
    use dpp::prelude::CoreBlockHeight;
    use drive::drive::Drive;

    fn test_query_total_system_credits(
        epoch_index: EpochIndex,
        activation_core_height: CoreBlockHeight,
        epoch_core_start_height: CoreBlockHeight,
        current_core_height: CoreBlockHeight,
    ) -> Credits {
        let (platform, _state, platform_version) =
            setup_platform(Some((1, activation_core_height)), Network::Regtest, None);

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to insert identity");

        fast_forward_to_block(
            &platform,
            5000 * epoch_index as u64,
            100 * epoch_index as u64,
            epoch_core_start_height,
            epoch_index,
            true,
        );

        if current_core_height > epoch_core_start_height {
            fast_forward_to_block(
                &platform,
                5000 * epoch_index as u64 + 10000,
                100 * epoch_index as u64 + 50,
                current_core_height,
                epoch_index,
                false,
            );
        }

        let request = GetTotalCreditsInPlatformRequestV0 { prove: false };

        let state = platform.state.load();

        let response = platform
            .query_total_credits_in_platform_v0(request, &state, platform_version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_total_credits_in_platform_response_v0::Result::Credits(credits) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected credits")
        };

        let rewards = epoch_core_reward_credits_for_distribution(
            if epoch_index == 0 {
                activation_core_height
            } else {
                epoch_core_start_height + 1
            },
            current_core_height,
            Network::Regtest.core_subsidy_halving_interval(),
            platform_version,
        )
        .expect("expected to get rewards");

        assert_eq!(credits, 100 + rewards);

        credits
    }

    fn test_proved_query_total_system_credits(
        epoch_index: EpochIndex,
        activation_core_height: CoreBlockHeight,
        epoch_core_start_height: CoreBlockHeight,
        current_core_height: CoreBlockHeight,
    ) -> Credits {
        let (platform, _state, platform_version) =
            setup_platform(Some((1, activation_core_height)), Network::Regtest, None);

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to insert identity");

        fast_forward_to_block(
            &platform,
            5000 * epoch_index as u64,
            100 * epoch_index as u64,
            epoch_core_start_height,
            epoch_index,
            true,
        );

        if current_core_height > epoch_core_start_height {
            fast_forward_to_block(
                &platform,
                5000 * epoch_index as u64 + 10000,
                100 * epoch_index as u64 + 50,
                current_core_height,
                epoch_index,
                false,
            );
        }

        let request = GetTotalCreditsInPlatformRequestV0 { prove: true };

        let state = platform.state.load();

        let response = platform
            .query_total_credits_in_platform_v0(request, &state, platform_version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_total_credits_in_platform_response_v0::Result::Proof(proof) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected proof")
        };

        let network = Network::Regtest;

        let core_subsidy_halving_interval = network.core_subsidy_halving_interval();

        let (_, credits) = Drive::verify_total_credits_in_system(
            &proof.grovedb_proof,
            core_subsidy_halving_interval,
            || Ok(activation_core_height),
            current_core_height,
            platform_version,
        )
        .expect("expected to verify total credits in platform");

        let rewards = epoch_core_reward_credits_for_distribution(
            if epoch_index == 0 {
                activation_core_height
            } else {
                epoch_core_start_height + 1
            },
            current_core_height,
            core_subsidy_halving_interval,
            platform_version,
        )
        .expect("expected to get rewards");

        assert_eq!(credits, 100 + rewards);

        credits
    }

    #[test]
    fn test_query_total_system_credits_at_genesis_platform_immediate_start() {
        // the fork height is 1500, the genesis core height is 1500 and we are asking for credits after this first block was committed
        let non_proved = test_query_total_system_credits(0, 1500, 1500, 1500);
        let proved = test_proved_query_total_system_credits(0, 1500, 1500, 1500);
        assert_eq!(non_proved, proved);
    }

    #[test]
    fn test_query_total_system_credits_at_genesis_platform_later_start() {
        // the fork height was 1320, the genesis core height is 1500 and we are asking for credits after this first block was committed
        let non_proved = test_query_total_system_credits(0, 1320, 1500, 1500);
        let proved = test_proved_query_total_system_credits(0, 1320, 1500, 1500);
        assert_eq!(non_proved, proved);
    }

    #[test]
    fn test_query_total_system_credits_on_first_epoch_not_genesis_immediate_start() {
        // the fork height is 1500, the genesis core height is 1500 and we are at height 1550
        let non_proved = test_query_total_system_credits(0, 1500, 1500, 1550);
        let proved = test_proved_query_total_system_credits(0, 1500, 1500, 1550);
        assert_eq!(non_proved, proved);
    }

    #[test]
    fn test_query_total_system_credits_on_first_epoch_not_genesis_later_start() {
        // the fork height was 1320, the genesis core height is 1500 and we are at height 1550
        let non_proved = test_query_total_system_credits(0, 1320, 1500, 1550);
        let proved = test_proved_query_total_system_credits(0, 1320, 1500, 1550);
        assert_eq!(non_proved, proved);
    }

    #[test]
    fn test_query_total_system_credits_not_genesis_epoch() {
        // the fork height was 1500, the genesis core height is 1500 and we are at height 2500
        let non_proved = test_query_total_system_credits(1, 1500, 2000, 2500);
        let proved = test_proved_query_total_system_credits(1, 1500, 2000, 2500);
        assert_eq!(non_proved, proved);
    }
}
