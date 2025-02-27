use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_statuses_request::GetTokenStatusesRequestV0;
use dapi_grpc::platform::v0::get_token_statuses_response::get_token_statuses_response_v0::{
    TokenStatusEntry, TokenStatuses,
};
use dapi_grpc::platform::v0::get_token_statuses_response::{
    get_token_statuses_response_v0, GetTokenStatusesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_token_statuses_v0(
        &self,
        GetTokenStatusesRequestV0 { token_ids, prove }: GetTokenStatusesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenStatusesResponseV0>, Error> {
        let token_ids: Vec<[u8; 32]> = check_validation_result_with_data!(token_ids
            .into_iter()
            .map(|token_id| {
                token_id.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "token_id must be a valid identifier (32 bytes long)".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_token_statuses(
                token_ids.as_slice(),
                None,
                platform_version,
            ));

            GetTokenStatusesResponseV0 {
                result: Some(get_token_statuses_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let token_statuses = self
                .drive
                .fetch_token_statuses(token_ids.as_slice(), None, platform_version)?
                .into_iter()
                .map(|(token_id, status)| {
                    let paused = status.map(|token_status| token_status.paused());
                    TokenStatusEntry {
                        token_id: token_id.to_vec(),
                        paused,
                    }
                })
                .collect();

            GetTokenStatusesResponseV0 {
                result: Some(get_token_statuses_response_v0::Result::TokenStatuses(
                    TokenStatuses { token_statuses },
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
