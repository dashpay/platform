use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_response::{
    get_identities_balances_response_v0, GetIdentitiesBalancesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identities_balances_v0(
        &self,
        GetIdentitiesBalancesRequestV0 { ids, prove }: GetIdentitiesBalancesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesBalancesResponseV0>, Error> {
        let identifiers = check_validation_result_with_data!(ids
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
        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_many_identity_balances(
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
            let map = |(key, value): ([u8; 32], Option<u64>)| {
                get_identities_balances_response_v0::IdentityBalance {
                    identity_id: key.to_vec(),
                    balance: value,
                }
            };

            let identities_balances = self
                .drive
                .fetch_optional_identities_balances(&identifiers, None, platform_version)?
                .into_iter()
                .map(map)
                .collect();

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
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
