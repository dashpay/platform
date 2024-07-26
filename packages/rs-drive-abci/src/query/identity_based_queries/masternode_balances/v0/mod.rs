use dapi_grpc::platform::v0::get_masternodes_balances_request::get_masternodes_balances_request_v0::StartAtProTxHash;
use dapi_grpc::platform::v0::get_masternodes_balances_request::GetMasternodesBalancesRequestV0;
use dapi_grpc::platform::v0::get_masternodes_balances_response::{get_masternodes_balances_response_v0, GetMasternodesBalancesResponseV0};
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dpp::check_validation_result_with_data;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;

impl<C> Platform<C> {
    pub(super) fn query_masternodes_balances_v0(
        &self,
        GetMasternodesBalancesRequestV0 {
            start_at, limit, offset, ascending,
            prove,
        }: GetMasternodesBalancesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetMasternodesBalancesResponseV0>, Error> {
        self.state.
                let config = &self.config.drive;
                let limit = check_validation_result_with_data!(limit
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
                    ))));
                let start_at =                             check_validation_result_with_data!(start_at.map(|StartAtProTxHash{start_pro_tx_hash,start_pro_tx_hash_included}| {
                    let start_pro_tx_hash: Identifier = start_pro_tx_hash.try_into().map_err(|_| {
                        QueryError::InvalidArgument(
                            "start_pro_tx_hash must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;
                    Ok::<([u8; 32], bool), QueryError>((start_pro_tx_hash.to_buffer(), start_pro_tx_hash_included))
                }).transpose());
                let response = if prove {
                    if offset.is_some() && offset.unwrap() != 0 {
                        return Ok(ValidationResult::new_with_errors(vec![QueryError::InvalidArgument(
                            "offset must not be set if asking for proof".to_string(),
                        )]));
                    }
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_many_identity_balances_by_range(
                            start_at,
                            ascending,
                            limit,
                            None,
                            &platform_version.drive
                        ));

                    GetMasternodesBalancesResponseV0 {
                        result: Some(get_masternodes_balances_response_v0::Result::Proof(
                            self.response_proof_v0(platform_state, proof),
                        )),
                        metadata: Some(self.response_metadata_v0(platform_state)),
                    }
                } else {
                    let maybe_balances : Vec<_> = self.drive.fetch_many_identity_balances_by_range(
                        start_at,
                        ascending,
                        limit,
                        None,
                        platform_version
                    )?;

                    let map = |(key, value) : ([u8;32], Credits)| get_masternodes_balances_response_v0::MasternodeBalance {
                        identity_id: key.to_vec(),
                        balance: Some(value),
                    };

                    let masternodes_balances = if ascending {
                        maybe_balances
                            .into_iter()
                            .map(
                                map
                            )
                            .collect()
                    } else {
                        maybe_balances
                            .into_iter().rev()
                            .map(
                                map
                            )
                            .collect()
                    };

                    GetMasternodesBalancesResponseV0 {
                        result: Some(
                            get_masternodes_balances_response_v0::Result::MasternodeBalances(
                                get_masternodes_balances_response_v0::MasternodesBalances {
                                    entries: masternodes_balances,
                                },
                            ),
                        ),
                        metadata: Some(self.response_metadata_v0(platform_state)),
                    }
                };

        Ok(QueryValidationResult::new_with_data(response))
    }
}