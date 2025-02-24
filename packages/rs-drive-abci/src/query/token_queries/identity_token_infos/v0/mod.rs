use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_token_infos_request::GetIdentityTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_identity_token_infos_response::{get_identity_token_infos_response_v0, GetIdentityTokenInfosResponseV0};
use dapi_grpc::platform::v0::get_identity_token_infos_response::get_identity_token_infos_response_v0::{TokenIdentityInfoEntry, TokenInfoEntry, TokenInfos};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_token_infos_v0(
        &self,
        GetIdentityTokenInfosRequestV0 {
            identity_id,
            token_ids,
            prove,
        }: GetIdentityTokenInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityTokenInfosResponseV0>, Error> {
        let identity_id: Identifier =
            check_validation_result_with_data!(identity_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "identity_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

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
            let proof = check_validation_result_with_data!(self.drive.prove_identity_token_infos(
                token_ids.as_slice(),
                identity_id.into_buffer(),
                None,
                platform_version,
            ));

            GetIdentityTokenInfosResponseV0 {
                result: Some(get_identity_token_infos_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let token_infos = self
                .drive
                .fetch_identity_token_infos(
                    token_ids.as_slice(),
                    identity_id.into_buffer(),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(token_id, info)| {
                    let info = info.map(|identity_token_info| TokenIdentityInfoEntry {
                        frozen: identity_token_info.frozen(),
                    });
                    TokenInfoEntry {
                        token_id: token_id.to_vec(),
                        info,
                    }
                })
                .collect();

            GetIdentityTokenInfosResponseV0 {
                result: Some(get_identity_token_infos_response_v0::Result::TokenInfos(
                    TokenInfos { token_infos },
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
