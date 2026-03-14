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
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

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
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
    use dapi_grpc::platform::v0::get_identity_token_infos_response::get_identity_token_infos_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenInfosRequestV0 {
            identity_id: vec![0; 8],
            token_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identity_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenInfosRequestV0 {
            identity_id: vec![0; 32],
            token_ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_identity_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenInfosRequestV0 {
            identity_id: vec![0; 32],
            token_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identity_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identity_token_infos_response_v0::Result::TokenInfos(infos)) => {
                assert_eq!(infos.token_infos.len(), 1);
                assert!(infos.token_infos[0].info.is_none());
            }
            _ => panic!("expected TokenInfos result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenInfosRequestV0 {
            identity_id: vec![0; 32],
            token_ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_identity_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentityTokenInfosResponseV0 {
                result: Some(get_identity_token_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_token_data() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        // Identity 2: frozen on token_0, no info on token_1, has balance on token_2
        let request = GetIdentityTokenInfosRequestV0 {
            identity_id: identity_ids[1].to_vec(),
            token_ids: vec![token_ids[0].to_vec(), token_ids[2].to_vec()],
            prove: false,
        };

        let result = platform
            .query_identity_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identity_token_infos_response_v0::Result::TokenInfos(infos)) => {
                assert_eq!(infos.token_infos.len(), 2);
                // One of them should be frozen (token_0 for identity_2)
                let frozen_count = infos
                    .token_infos
                    .iter()
                    .filter(|e| e.info.as_ref().map_or(false, |i| i.frozen))
                    .count();
                assert_eq!(frozen_count, 1);
            }
            _ => panic!("expected TokenInfos result"),
        }
    }

    #[test]
    fn test_query_with_token_data_proof() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentityTokenInfosRequestV0 {
            identity_id: identity_ids[0].to_vec(),
            token_ids: vec![token_ids[0].to_vec()],
            prove: true,
        };

        let result = platform
            .query_identity_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentityTokenInfosResponseV0 {
                result: Some(get_identity_token_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
