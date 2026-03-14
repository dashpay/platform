use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_group_action_signers_request::GetGroupActionSignersRequestV0;
use dapi_grpc::platform::v0::get_group_action_signers_response::{
    get_group_action_signers_response_v0, GetGroupActionSignersResponseV0,
};
use dapi_grpc::platform::v0::get_group_action_signers_response::get_group_action_signers_response_v0::{GroupActionSigner, GroupActionSigners};
use dpp::check_validation_result_with_data;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_group_action_signers_v0(
        &self,
        GetGroupActionSignersRequestV0 {
            contract_id,
            group_contract_position,
            status,
            action_id,
            prove,
        }: GetGroupActionSignersRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetGroupActionSignersResponseV0>, Error> {
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let action_id: Identifier =
            check_validation_result_with_data!(action_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "action id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        if group_contract_position > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidParameter(format!(
                    "group contract position {} can not be over u16::MAX",
                    group_contract_position
                )),
            )));
        }

        let group_status: GroupActionStatus =
            check_validation_result_with_data!(status.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "group action status must be Active or Closed".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_action_signers(
                contract_id,
                group_contract_position as GroupContractPosition,
                group_status,
                action_id,
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetGroupActionSignersResponseV0 {
                result: Some(get_group_action_signers_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let group_action_signers = self
                .drive
                .fetch_action_signers(
                    contract_id,
                    group_contract_position as GroupContractPosition,
                    group_status,
                    action_id,
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(signer_id, power)| GroupActionSigner {
                    signer_id: signer_id.to_vec(),
                    power,
                })
                .collect();
            GetGroupActionSignersResponseV0 {
                result: Some(
                    get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners {
                            signers: group_action_signers,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 8],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_action_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("action id must be a valid identifier")
        ));
    }

    #[test]
    fn test_invalid_action_id_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            action_id: vec![],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("action id must be a valid identifier")
        ));
    }

    #[test]
    fn test_group_contract_position_over_u16_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32 + 1,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }

    #[test]
    fn test_group_contract_position_over_u16_max_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32 + 1,
            status: 0,
            action_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }

    #[test]
    fn test_invalid_status_value() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 99,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("group action status must be Active or Closed")
        ));
    }

    #[test]
    fn test_query_group_action_signers_no_prove_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0, // ActionActive
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(
                    get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners { signers }
                    )
                ),
                metadata: Some(_),
            }) if signers.is_empty()
        ));
    }

    #[test]
    fn test_query_group_action_signers_prove_returns_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0, // ActionActive
            action_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(get_group_action_signers_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_action_signers_status_closed_no_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 1, // ActionClosed
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(
                    get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners { signers }
                    )
                ),
                metadata: Some(_),
            }) if signers.is_empty()
        ));
    }

    #[test]
    fn test_group_contract_position_at_u16_max_is_valid() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        // Should not have a validation error for position being over u16::MAX
        assert!(result.errors.is_empty());
    }
}
