use dapi_grpc::platform::v0::get_identities_balances_request::get_identities_balances_request_v0::{GetIdentitiesBalancesByIdentityIdRange, GetIdentitiesBalancesByKnownIdentityIds, RequestType};
use dapi_grpc::platform::v0::get_identities_balances_request::get_identities_balances_request_v0::get_identities_balances_by_identity_id_range::StartAtIdentity;
use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_response::{get_identities_balances_response_v0, GetIdentitiesBalancesResponseV0};
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
    pub(super) fn query_identities_balances_v0(
        &self,
        GetIdentitiesBalancesRequestV0 {
            ascending,
            prove,
            request_type,
        }: GetIdentitiesBalancesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesBalancesResponseV0>, Error> {
        let request_type: RequestType = check_validation_result_with_data!(request_type
            .ok_or(QueryError::InvalidArgument("request type must be set".to_string())));
        let response = match request_type {
            RequestType::ByKnownIdentityIds(GetIdentitiesBalancesByKnownIdentityIds {
                identities_ids,
            }) => {
                let identifiers = check_validation_result_with_data!(identities_ids
                    .into_iter()
                    .map(|identity_id| {
                        let identifier: Identifier = identity_id.try_into().map_err(|_| {
                            QueryError::InvalidArgument(
                                "id must be a valid identifier (32 bytes long)".to_string(),
                            )
                        })?;
                        Ok(identifier.into_buffer())
                    })
                    .collect::<Result<Vec<_>, QueryError>>());
                if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_many_identity_balances(
                            identifiers.as_slice(),
                            None,
                            &platform_version.drive
                        ));

                    GetIdentitiesBalancesResponseV0 {
                        result: Some(get_identities_balances_response_v0::Result::Proof(
                            self.response_proof_v0(platform_state, proof),
                        )),
                        metadata: Some(self.response_metadata_v0(platform_state)),
                    }
                } else {
                    let maybe_balances = self.drive.fetch_optional_identities_balances(
                        &identifiers,
                        None,
                        platform_version,
                    )?;
                    
                    let map = |(key, value) : ([u8;32], Option<u64>)| get_identities_balances_response_v0::IdentityBalance {
                        identity_id: key.to_vec(),
                        balance: value,
                    };

                    let identities_balances = if ascending {
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

                    GetIdentitiesBalancesResponseV0 {
                        result: Some(
                            get_identities_balances_response_v0::Result::IdentitiesBalances(
                                get_identities_balances_response_v0::IdentitiesBalances {
                                    entries: identities_balances,
                                },
                            ),
                        ),
                        metadata: Some(self.response_metadata_v0(platform_state)),
                    }
                }
            }
            RequestType::ByRange(GetIdentitiesBalancesByIdentityIdRange{ start_at, limit, offset }) => {
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
                let start_at =                             check_validation_result_with_data!(start_at.map(|StartAtIdentity{start_identity_id,start_identity_id_included}| {
                    let start_identity_id: Identifier = start_identity_id.try_into().map_err(|_| {
                        QueryError::InvalidArgument(
                            "start_at must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;
                    Ok::<([u8; 32], bool), QueryError>((start_identity_id.to_buffer(), start_identity_id_included))
                }).transpose());
                if prove {
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

                    GetIdentitiesBalancesResponseV0 {
                        result: Some(get_identities_balances_response_v0::Result::Proof(
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

                    let map = |(key, value) : ([u8;32], Credits)| get_identities_balances_response_v0::IdentityBalance {
                        identity_id: key.to_vec(),
                        balance: Some(value),
                    };

                    let identities_balances = if ascending {
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

                    GetIdentitiesBalancesResponseV0 {
                        result: Some(
                            get_identities_balances_response_v0::Result::IdentitiesBalances(
                                get_identities_balances_response_v0::IdentitiesBalances {
                                    entries: identities_balances,
                                },
                            ),
                        ),
                        metadata: Some(self.response_metadata_v0(platform_state)),
                    }
                }
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}