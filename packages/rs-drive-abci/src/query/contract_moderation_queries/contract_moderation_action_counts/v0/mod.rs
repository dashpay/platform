use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::identifier_from_request;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_moderation_action_counts_request::GetContractModerationActionCountsRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_action_counts_response::{
    get_contract_moderation_action_counts_response_v0, ContractModerationActionCount,
    ContractModerationActionCounts, GetContractModerationActionCountsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns how many counted moderation actions each member of an elected contract's seated
    /// team signed since the moderators pot was last paid out, which resets every count: the
    /// share of the pot a payout by actions gives each. The contract must be elected: no other
    /// has a counts tree to read or prove.
    pub(super) fn query_contract_moderation_action_counts_v0(
        &self,
        GetContractModerationActionCountsRequestV0 { contract_id, prove }: GetContractModerationActionCountsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractModerationActionCountsResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        if let Err(error) =
            self.check_keeps_moderation_action_counts(contract_id, platform_version)?
        {
            return Ok(QueryValidationResult::new_with_error(error));
        }

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_contract_moderation_action_counts(contract_id, None, platform_version));

            GetContractModerationActionCountsResponseV0 {
                result: Some(
                    get_contract_moderation_action_counts_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                            .map(|(_, proof)| proof)?,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            // All of them, as the proof reads them: the tree holds at most the team
            let counts = check_validation_result_with_data!(self
                .drive
                .fetch_contract_moderation_action_counts(
                    contract_id,
                    u16::MAX,
                    None,
                    platform_version
                ));

            GetContractModerationActionCountsResponseV0 {
                result: Some(
                    get_contract_moderation_action_counts_response_v0::Result::Counts(
                        ContractModerationActionCounts {
                            counts: counts
                                .into_iter()
                                .map(|(identity_id, count)| ContractModerationActionCount {
                                    identity_id: identity_id.to_vec(),
                                    count,
                                })
                                .collect(),
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::query::QueryError;
    use crate::query::contract_moderation_queries::tests::{
        contract_with_settled_posts, store_contract,
    };
    use crate::query::tests::{setup_platform, store_data_contract};
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use drive::drive::Drive;
    use drive::util::batch::{ContractModerationOperationType, DriveOperation};
    use std::collections::BTreeMap;

    fn request(contract_id: Identifier, prove: bool) -> GetContractModerationActionCountsRequestV0 {
        GetContractModerationActionCountsRequestV0 {
            contract_id: contract_id.to_vec(),
            prove,
        }
    }

    #[test]
    fn should_return_and_prove_the_counts_of_the_seated_team() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_settled_posts();
        store_data_contract(&platform, &contract, version);
        let counts = || {
            let result = platform
                .query_contract_moderation_action_counts_v0(
                    request(contract.id(), false),
                    &state,
                    version,
                )
                .expect("expected the query to run");
            assert!(result.is_valid(), "{:?}", result.errors);
            match result.into_data().expect("expected data").result {
                Some(get_contract_moderation_action_counts_response_v0::Result::Counts(counts)) => {
                    counts.counts
                }
                other => panic!("expected counts, got {other:?}"),
            }
        };
        // Nobody acted since the team was seated
        assert!(counts().is_empty());

        let set_count = |seed: u8, count: u32| {
            DriveOperation::ContractModerationOperation(
                ContractModerationOperationType::SetActionCount {
                    contract_id: contract.id(),
                    identity_id: Identifier::from([seed; 32]),
                    count,
                },
            )
        };
        platform
            .drive
            .apply_drive_operations(
                vec![set_count(2, 5), set_count(1, 3)],
                true,
                &BlockInfo::default(),
                None,
                version,
                None,
            )
            .expect("expected to count the actions");
        assert_eq!(
            counts(),
            vec![
                ContractModerationActionCount {
                    identity_id: vec![1; 32],
                    count: 3,
                },
                ContractModerationActionCount {
                    identity_id: vec![2; 32],
                    count: 5,
                },
            ]
        );

        let result = platform
            .query_contract_moderation_action_counts_v0(
                request(contract.id(), true),
                &state,
                version,
            )
            .expect("expected the query to run");
        assert!(result.is_valid(), "{:?}", result.errors);
        let Some(get_contract_moderation_action_counts_response_v0::Result::Proof(proof)) =
            result.into_data().expect("expected data").result
        else {
            panic!("expected a proof");
        };
        let (_, proved) = Drive::verify_contract_moderation_action_counts(
            &proof.grovedb_proof,
            contract.id(),
            false,
            version,
        )
        .expect("expected the proof to verify");
        assert_eq!(
            proved,
            BTreeMap::from([
                (Identifier::from([1; 32]), 3),
                (Identifier::from([2; 32]), 5)
            ])
        );
    }

    #[test]
    fn should_refuse_a_contract_without_an_elected_team() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let owned = store_contract(&platform, true, false, version);
        let refusal = |request| {
            platform
                .query_contract_moderation_action_counts_v0(request, &state, version)
                .expect("expected the query to run")
                .errors
        };
        // A contract its owner moderates keeps no counts: no team shares a pot
        assert!(matches!(
            refusal(request(owned.id(), true)).as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
        assert!(matches!(
            refusal(request(Identifier::from([0x5A; 32]), false)).as_slice(),
            [QueryError::NotFound(_)]
        ));
        let mut short = request(owned.id(), false);
        short.contract_id = vec![1; 31];
        assert!(matches!(
            refusal(short).as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
    }
}
