use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_contract_info_request::GetTokenContractInfoRequestV0;
use dapi_grpc::platform::v0::get_token_contract_info_response::{
    get_token_contract_info_response_v0, GetTokenContractInfoResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_token_contract_info_v0(
        &self,
        GetTokenContractInfoRequestV0 { token_id, prove }: GetTokenContractInfoRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenContractInfoResponseV0>, Error> {
        let token_id: [u8; 32] =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_ids must be a list of valid identifiers (32 bytes long)".to_string(),
                )
            }));

        let response =
            if prove {
                let proof = check_validation_result_with_data!(self
                    .drive
                    .prove_token_contract_info(token_id, None, platform_version));

                GetTokenContractInfoResponseV0 {
                    result: Some(get_token_contract_info_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    )),
                    metadata: Some(self.response_metadata_v0(platform_state)),
                }
            } else {
                let result = check_validation_result_with_data!(self
                    .drive
                    .fetch_token_contract_info(token_id, None, platform_version))
                .map(|token_contract_info| {
                    get_token_contract_info_response_v0::Result::Data(
                        get_token_contract_info_response_v0::TokenContractInfoData {
                            contract_id: token_contract_info.contract_id().to_vec(),
                            token_contract_position: token_contract_info.token_contract_position()
                                as u32,
                        },
                    )
                });

                GetTokenContractInfoResponseV0 {
                    result,
                    metadata: Some(self.response_metadata_v0(platform_state)),
                }
            };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
