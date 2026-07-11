use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;

use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_request::{ContractTokenInfo, GetTokenPerpetualDistributionLastClaimRequestV0};
use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::{
    get_token_perpetual_distribution_last_claim_response_v0,
    GetTokenPerpetualDistributionLastClaimResponseV0,
};
use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0::{last_claim_info, LastClaimInfo};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_token_perpetual_distribution_last_claim_v0(
        &self,
        GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id,
            contract_info,
            identity_id,
            prove,
        }: GetTokenPerpetualDistributionLastClaimRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenPerpetualDistributionLastClaimResponseV0>, Error>
    {
        // ── Basic argument validation ──────────────────────────────────────────
        let token_id: Identifier =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let identity_id: Identifier =
            check_validation_result_with_data!(identity_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "identity_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_perpetual_distribution_last_paid_moment(
                    token_id.into_buffer(),
                    identity_id,
                    None,
                    platform_version,
                ));

            GetTokenPerpetualDistributionLastClaimResponseV0 {
                result: Some(
                    get_token_perpetual_distribution_last_claim_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                            .map(|(_, proof)| proof)?,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else if let Some(ContractTokenInfo {
            contract_id,
            token_contract_position,
        }) = contract_info
        {
            let contract_id: Identifier =
                check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "contract_id must be a valid identifier (32 bytes long)".to_string(),
                    )
                }));
            let Some(contract) = check_validation_result_with_data!(self
                .drive
                .get_contract_with_fetch_info(
                    contract_id.into_buffer(),
                    false,
                    None,
                    platform_version
                )
                .map_err(QueryError::Drive))
            else {
                return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                    format!("contract with identifier {} not found", contract_id),
                )));
            };

            if token_contract_position > u16::MAX as u32 {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "token_contract_position must be less than u16::MAX".to_string(),
                    ),
                ));
            }

            let token = check_validation_result_with_data!(contract
                .contract
                .expected_token_configuration(token_contract_position as u16)
                .map_err(QueryError::Protocol));
            let token_distribution_rules = token.distribution_rules();
            let Some(token_perpetual_distribution_rules) =
                token_distribution_rules.perpetual_distribution()
            else {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!(
                        "contract with identifier {} does not have perpetual distribution rules",
                        contract_id
                    )),
                ));
            };

            let paid_at = self
                .drive
                .fetch_perpetual_distribution_last_paid_moment(
                    token_id.into_buffer(),
                    identity_id,
                    token_perpetual_distribution_rules.distribution_type(),
                    None,
                    platform_version,
                )?
                .map(|moment| match moment {
                    RewardDistributionMoment::BlockBasedMoment(height) => {
                        last_claim_info::PaidAt::BlockHeight(height)
                    }
                    RewardDistributionMoment::TimeBasedMoment(timestamp) => {
                        last_claim_info::PaidAt::TimestampMs(timestamp)
                    }
                    RewardDistributionMoment::EpochBasedMoment(epoch) => {
                        last_claim_info::PaidAt::Epoch(epoch as u32)
                    }
                });

            GetTokenPerpetualDistributionLastClaimResponseV0 {
                result: Some(
                    get_token_perpetual_distribution_last_claim_response_v0::Result::LastClaim(
                        LastClaimInfo { paid_at },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let paid_at = self
                .drive
                .fetch_perpetual_distribution_last_paid_moment_raw(
                    token_id.into_buffer(),
                    identity_id,
                    None,
                    platform_version,
                )?
                .map(last_claim_info::PaidAt::RawBytes);

            GetTokenPerpetualDistributionLastClaimResponseV0 {
                result: Some(
                    get_token_perpetual_distribution_last_claim_response_v0::Result::LastClaim(
                        LastClaimInfo { paid_at },
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
    use crate::query::tests::setup_platform;
    use crate::query::tests::setup_platform_with_token_state;
    use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::get_token_perpetual_distribution_last_claim_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: vec![0; 8],
            contract_info: None,
            identity_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: vec![0; 32],
            contract_info: None,
            identity_id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: vec![0; 32],
            contract_info: None,
            identity_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenPerpetualDistributionLastClaimResponseV0 {
                result: Some(
                    get_token_perpetual_distribution_last_claim_response_v0::Result::Proof(_)
                ),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_without_contract_info_raw() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        // Query without contract_info - should return raw bytes
        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: None,
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_perpetual_distribution_last_claim_response_v0::Result::LastClaim(
                claim,
            )) => {
                // paid_at might be None if no distribution has happened yet
                // The important thing is the query succeeds
                let _ = claim.paid_at;
            }
            _ => panic!("expected LastClaim result"),
        }
    }

    #[test]
    fn test_query_with_contract_info() {
        let (platform, state, version, contract_id, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: contract_id.to_vec(),
                token_contract_position: 0,
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_perpetual_distribution_last_claim_response_v0::Result::LastClaim(
                claim,
            )) => {
                // No distribution has been executed yet, so paid_at should be None
                let _ = claim.paid_at;
            }
            _ => panic!("expected LastClaim result"),
        }
    }

    #[test]
    fn test_query_with_invalid_contract_id() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: vec![0; 8], // invalid
                token_contract_position: 0,
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id")
        ));
    }

    #[test]
    fn test_query_with_contract_not_found() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: vec![99; 32], // nonexistent
                token_contract_position: 0,
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(msg)] if msg.contains("contract")
        ));
    }

    #[test]
    fn test_query_with_token_position_too_large() {
        let (platform, state, version, contract_id, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: contract_id.to_vec(),
                token_contract_position: u32::MAX, // too large
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_contract_position")
        ));
    }

    #[test]
    fn test_query_with_data_proof() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: None,
            identity_id: identity_ids[0].to_vec(),
            prove: true,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenPerpetualDistributionLastClaimResponseV0 {
                result: Some(
                    get_token_perpetual_distribution_last_claim_response_v0::Result::Proof(_)
                ),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_invalid_identity_id_with_contract_info() {
        // Identity id validation fires before contract_info is touched.
        let (platform, state, version, contract_id, token_ids, _) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: contract_id.to_vec(),
                token_contract_position: 0,
            }),
            identity_id: vec![0; 7],
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_invalid_token_id_with_proof() {
        // Even with prove=true the token_id check fires first.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: vec![0; 31],
            contract_info: None,
            identity_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_token_position_at_u16_boundary_not_rejected_for_size() {
        // u16::MAX is the *boundary*; only `> u16::MAX as u32` is rejected.
        // Passing exactly u16::MAX should *not* hit the
        // "token_contract_position must be less than u16::MAX" error. It may
        // hit some other InvalidArgument variant (bad position), but not the
        // size rejection.
        let (platform, state, version, contract_id, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: contract_id.to_vec(),
                token_contract_position: u16::MAX as u32,
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        // There must NOT be a "token_contract_position must be less than
        // u16::MAX" validation error at the boundary.
        let has_size_err = result.errors.iter().any(|e| {
            matches!(e,
                QueryError::InvalidArgument(msg)
                    if msg.contains("token_contract_position must be less than")
            )
        });
        assert!(
            !has_size_err,
            "u16::MAX should not trigger the size validation: errors={:?}",
            result.errors
        );
    }

    #[test]
    fn test_nonexistent_token_position_on_existing_contract() {
        // Contract exists but requested position does not → handler currently
        // surfaces a protocol error via `QueryError::Protocol`.
        let (platform, state, version, contract_id, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: contract_id.to_vec(),
                token_contract_position: 999,
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            !result.errors.is_empty(),
            "expected some error for unknown token position"
        );
    }

    #[test]
    fn test_query_with_contract_info_returns_block_height_paid_at() {
        // Happy path with contract_info that *does* have perpetual
        // distribution rules: the handler takes the typed-moment branch.
        let (platform, state, version, contract_id, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: contract_id.to_vec(),
                token_contract_position: 0,
            }),
            identity_id: identity_ids[1].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        assert!(matches!(
            data.result,
            Some(get_token_perpetual_distribution_last_claim_response_v0::Result::LastClaim(_))
        ));
        assert!(data.metadata.is_some());
    }

    #[test]
    fn test_invalid_contract_id_bytes_length_zero() {
        // contract_id with zero length should fail the 32-byte identifier
        // check inside the contract_info branch.
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetTokenPerpetualDistributionLastClaimRequestV0 {
            token_id: token_ids[0].to_vec(),
            contract_info: Some(ContractTokenInfo {
                contract_id: vec![],
                token_contract_position: 0,
            }),
            identity_id: identity_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_perpetual_distribution_last_claim_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id")
        ));
    }
}
