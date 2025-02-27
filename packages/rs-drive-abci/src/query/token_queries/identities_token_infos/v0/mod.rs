use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_token_infos_request::GetIdentitiesTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_identities_token_infos_response::{get_identities_token_infos_response_v0, GetIdentitiesTokenInfosResponseV0};
use dapi_grpc::platform::v0::get_identities_token_infos_response::get_identities_token_infos_response_v0::{IdentityTokenInfos, TokenIdentityInfoEntry, TokenInfoEntry};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identities_token_infos_v0(
        &self,
        GetIdentitiesTokenInfosRequestV0 {
            token_id,
            identity_ids,
            prove,
        }: GetIdentitiesTokenInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesTokenInfosResponseV0>, Error> {
        let token_id: Identifier =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let identity_ids: Vec<[u8; 32]> = check_validation_result_with_data!(identity_ids
            .into_iter()
            .map(|identity_id| {
                identity_id.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "identity_id must be a valid identifier (32 bytes long)".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_identities_token_infos(
                    token_id.into_buffer(),
                    identity_ids.as_slice(),
                    None,
                    platform_version,
                ));

            GetIdentitiesTokenInfosResponseV0 {
                result: Some(get_identities_token_infos_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let identity_token_infos = self
                .drive
                .fetch_identities_token_infos(
                    token_id.into_buffer(),
                    identity_ids.as_slice(),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(identity_id, info)| {
                    let info = info.map(|identity_token_info| TokenIdentityInfoEntry {
                        frozen: identity_token_info.frozen(),
                    });
                    TokenInfoEntry {
                        identity_id: identity_id.to_vec(),
                        info,
                    }
                })
                .collect();

            GetIdentitiesTokenInfosResponseV0 {
                result: Some(
                    get_identities_token_infos_response_v0::Result::IdentityTokenInfos(
                        IdentityTokenInfos {
                            token_infos: identity_token_infos,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
