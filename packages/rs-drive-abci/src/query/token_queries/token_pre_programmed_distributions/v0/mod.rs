use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::get_token_pre_programmed_distributions_request_v0::StartAtInfo;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::GetTokenPreProgrammedDistributionsRequestV0;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::{get_token_pre_programmed_distributions_response_v0, GetTokenPreProgrammedDistributionsResponseV0};
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::get_token_pre_programmed_distributions_response_v0::{TokenDistributions, TokenDistributionEntry, TokenTimedDistributionEntry};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use drive::error::query::QuerySyntaxError;

impl<C> Platform<C> {
    pub(super) fn query_token_pre_programmed_distributions_v0(
        &self,
        GetTokenPreProgrammedDistributionsRequestV0 {
            token_id,
            start_at_info,
            limit,
            prove,
        }: GetTokenPreProgrammedDistributionsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenPreProgrammedDistributionsResponseV0>, Error> {
        let config = &self.config.drive;
        let token_id: Identifier =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = limit
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.default_query_limit
                {
                    None
                } else {
                    Some(limit_value as u16)
                }
            })
            .ok_or(drive::error::Error::Query(QuerySyntaxError::InvalidLimit(
                format!("limit greater than max limit {}", config.max_query_limit),
            )))?;

        let start_at = match start_at_info {
            None => None,
            Some(StartAtInfo {
                start_time_ms,
                start_recipient,
                start_recipient_included,
            }) => {
                let start_at_recipient = match start_recipient {
                    None => None,
                    Some(identifier) => {
                        let recipient_id: Identifier = check_validation_result_with_data!(
                            identifier.try_into().map_err(|_| {
                                QueryError::InvalidArgument(
                                    "start_recipient must be a valid identifier (32 bytes long)"
                                        .to_string(),
                                )
                            })
                        );
                        Some((recipient_id, start_recipient_included.unwrap_or(true)))
                    }
                };

                Some(QueryPreProgrammedDistributionStartAt {
                    start_at_time: start_time_ms,
                    start_at_recipient,
                })
            }
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_token_pre_programmed_distributions(
                    token_id.into_buffer(),
                    start_at,
                    Some(limit),
                    None,
                    platform_version,
                ));

            GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(
                    get_token_pre_programmed_distributions_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let token_distributions = self
                .drive
                .fetch_token_pre_programmed_distributions(
                    token_id.into_buffer(),
                    start_at,
                    Some(limit),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(timestamp, distributions_for_time)| {
                    let distributions = distributions_for_time
                        .into_iter()
                        .map(|(recipient, amount)| TokenDistributionEntry {
                            recipient_id: recipient.to_vec(),
                            amount,
                        })
                        .collect();
                    TokenTimedDistributionEntry {
                        timestamp,
                        distributions,
                    }
                })
                .collect();

            GetTokenPreProgrammedDistributionsResponseV0 {
                result: Some(
                    get_token_pre_programmed_distributions_response_v0::Result::TokenDistributions(
                        TokenDistributions {
                            token_distributions,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
