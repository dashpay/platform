use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
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
use drive::util::grove_operations::GroveDBToUse;

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
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
    use dapi_grpc::platform::v0::get_token_statuses_response::get_token_statuses_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_statuses_response_v0::Result::TokenStatuses(statuses)) => {
                assert_eq!(statuses.token_statuses.len(), 1);
                // Status should be None for nonexistent token
                assert!(statuses.token_statuses[0].paused.is_none());
            }
            _ => panic!("expected TokenStatuses result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenStatusesResponseV0 {
                result: Some(get_token_statuses_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_token_not_paused() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 0 is NOT paused - status may be None or Some(false) depending on
        // whether initial status was written
        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![token_ids[0].to_vec()],
            prove: false,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_statuses_response_v0::Result::TokenStatuses(statuses)) => {
                assert_eq!(statuses.token_statuses.len(), 1);
                // Token 0 was not explicitly paused, so paused is either None or Some(false)
                assert_ne!(statuses.token_statuses[0].paused, Some(true));
            }
            _ => panic!("expected TokenStatuses result"),
        }
    }

    #[test]
    fn test_query_token_paused() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 1 IS paused
        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![token_ids[1].to_vec()],
            prove: false,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_statuses_response_v0::Result::TokenStatuses(statuses)) => {
                assert_eq!(statuses.token_statuses.len(), 1);
                assert_eq!(statuses.token_statuses[0].paused, Some(true));
            }
            _ => panic!("expected TokenStatuses result"),
        }
    }

    #[test]
    fn test_query_multiple_tokens() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![
                token_ids[0].to_vec(),
                token_ids[1].to_vec(),
                token_ids[2].to_vec(),
            ],
            prove: false,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_statuses_response_v0::Result::TokenStatuses(statuses)) => {
                assert_eq!(statuses.token_statuses.len(), 3);
                // Exactly one should be paused (token_1)
                let paused_count = statuses
                    .token_statuses
                    .iter()
                    .filter(|s| s.paused == Some(true))
                    .count();
                assert_eq!(paused_count, 1);
            }
            _ => panic!("expected TokenStatuses result"),
        }
    }

    #[test]
    fn test_query_with_data_proof() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenStatusesRequestV0 {
            token_ids: vec![token_ids[0].to_vec(), token_ids[1].to_vec()],
            prove: true,
        };

        let result = platform
            .query_token_statuses_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenStatusesResponseV0 {
                result: Some(get_token_statuses_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
