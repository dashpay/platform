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
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
