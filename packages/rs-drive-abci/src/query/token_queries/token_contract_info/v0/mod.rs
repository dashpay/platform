use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_contract_info_request::GetTokenContractInfoRequestV0;
use dapi_grpc::platform::v0::get_token_contract_info_response::{
    get_token_contract_info_response_v0, GetTokenContractInfoResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

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
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_token_contract_info(
                token_id,
                None,
                platform_version
            ));

            GetTokenContractInfoResponseV0 {
                result: Some(get_token_contract_info_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let result = check_validation_result_with_data!(self.drive.fetch_token_contract_info(
                token_id,
                None,
                platform_version
            ))
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
    use dapi_grpc::platform::v0::get_token_contract_info_response::get_token_contract_info_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenContractInfoRequestV0 {
            token_id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_token_contract_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_token_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenContractInfoRequestV0 {
            token_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_token_contract_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        // When token doesn't exist, result should be None (no data)
        let data = result.data.unwrap();
        assert!(data.result.is_none());
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenContractInfoRequestV0 {
            token_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_token_contract_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenContractInfoResponseV0 {
                result: Some(get_token_contract_info_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_token_data() {
        let (platform, state, version, contract_id, token_ids, _) =
            setup_platform_with_token_state();

        let request = GetTokenContractInfoRequestV0 {
            token_id: token_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_contract_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_contract_info_response_v0::Result::Data(info)) => {
                assert_eq!(info.contract_id, contract_id.to_vec());
                assert_eq!(info.token_contract_position, 0);
            }
            _ => panic!("expected Data result"),
        }
    }

    #[test]
    fn test_query_with_token_data_position_1() {
        let (platform, state, version, contract_id, token_ids, _) =
            setup_platform_with_token_state();

        let request = GetTokenContractInfoRequestV0 {
            token_id: token_ids[1].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_contract_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_contract_info_response_v0::Result::Data(info)) => {
                assert_eq!(info.contract_id, contract_id.to_vec());
                assert_eq!(info.token_contract_position, 1);
            }
            _ => panic!("expected Data result"),
        }
    }

    #[test]
    fn test_query_with_token_data_proof() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenContractInfoRequestV0 {
            token_id: token_ids[0].to_vec(),
            prove: true,
        };

        let result = platform
            .query_token_contract_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenContractInfoResponseV0 {
                result: Some(get_token_contract_info_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
