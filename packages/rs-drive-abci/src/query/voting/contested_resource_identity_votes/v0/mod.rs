use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_response::{
    get_contested_resource_identity_votes_response_v0, GetContestedResourceIdentityVotesResponseV0,
};
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::{check_validation_result_with_data, platform_value, ProtocolError};
use drive::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use drive::error::query::QuerySyntaxError;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_contested_resource_identity_votes_v0(
        &self,
        GetContestedResourceIdentityVotesRequestV0 {
            identity_id,
            limit,
            offset,
            order_ascending,
            start_at_vote_poll_id_info,
            prove,
        }: GetContestedResourceIdentityVotesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourceIdentityVotesResponseV0>, Error> {
        let config = &self.config.drive;
        let identity_id: Identifier =
            check_validation_result_with_data!(identity_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "identity_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = limit
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
            )))?;

        let offset = check_validation_result_with_data!(offset
            .map(|offset| {
                u16::try_from(offset)
                    .map_err(|_| QueryError::InvalidArgument("offset out of bounds".to_string()))
            })
            .transpose());

        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset,
            limit: Some(limit),
            start_at: start_at_vote_poll_id_info
                .map(|start_at_vote_poll_id_info| {
                    Ok::<([u8; 32], bool), platform_value::Error>((
                        Identifier::from_vec(start_at_vote_poll_id_info.start_at_poll_identifier)?
                            .to_buffer(),
                        start_at_vote_poll_id_info.start_poll_identifier_included,
                    ))
                })
                .transpose()?,
            order_ascending,
        };

        let response = if prove {
            let proof = match query.execute_with_proof(&self.drive, None, None, platform_version) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Proof(proof),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let votes =
                match query.execute_no_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let finished_results = if votes.len() == limit as usize && limit > 0 {
                let start_at = if order_ascending {
                    votes
                        .iter()
                        .next_back()
                        .map(|(id, _)| (id.to_buffer(), false))
                } else {
                    votes.iter().next().map(|(id, _)| (id.to_buffer(), false))
                };
                let extra_query = ContestedResourceVotesGivenByIdentityQuery {
                    identity_id,
                    offset: None,
                    limit: Some(1),
                    start_at,
                    order_ascending,
                };
                let another_result = match extra_query.execute_no_proof(
                    &self.drive,
                    None,
                    &mut vec![],
                    platform_version,
                ) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };
                another_result.is_empty()
            } else {
                true
            };

            let contested_resource_identity_votes = votes
                .into_values()
                .map(|resource_vote| {
                    let ContestedDocumentResourceVoteStorageForm {
                        contract_id, document_type_name, index_values, resource_vote_choice
                    } = resource_vote;
                    let vote_choice = match resource_vote_choice {
                        ResourceVoteChoice::TowardsIdentity(towards_identity) => {
                            get_contested_resource_identity_votes_response_v0::ResourceVoteChoice {
                                vote_choice_type: 0,
                                identity_id: Some(towards_identity.to_vec()),
                            }
                        }
                        ResourceVoteChoice::Abstain => {
                            get_contested_resource_identity_votes_response_v0::ResourceVoteChoice {
                                vote_choice_type: 1,
                                identity_id: None,
                            }
                        }
                        ResourceVoteChoice::Lock => {
                            get_contested_resource_identity_votes_response_v0::ResourceVoteChoice {
                                vote_choice_type: 2,
                                identity_id: None,
                            }
                        }
                    };
                    Ok(get_contested_resource_identity_votes_response_v0::ContestedResourceIdentityVote {
                        contract_id: contract_id.to_vec(),
                        document_type_name,
                        serialized_index_storage_values: index_values,
                        vote_choice: Some(vote_choice),
                    })
                })
                .collect::<Result<
                    Vec<get_contested_resource_identity_votes_response_v0::ContestedResourceIdentityVote>,
                    ProtocolError,
                >>()?;

            GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Votes(
                        get_contested_resource_identity_votes_response_v0::ContestedResourceIdentityVotes {
                            contested_resource_identity_votes,
                            finished_results,
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
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_contested_resource_identity_votes_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 8],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id must be a valid identifier")
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_offset_out_of_bounds() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // offset > u16::MAX triggers InvalidArgument("offset out of bounds")
        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: Some((u16::MAX as u32) + 1),
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("offset out of bounds")
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        // empty platform should return the Votes variant with no entries and finished_results=true
        assert!(matches!(
            result.data,
            Some(GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Votes(
                        get_contested_resource_identity_votes_response_v0::ContestedResourceIdentityVotes {
                            contested_resource_identity_votes,
                            finished_results,
                        },
                    ),
                ),
                metadata: Some(_),
            }) if contested_resource_identity_votes.is_empty() && finished_results
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_empty_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: true,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetContestedResourceIdentityVotesResponseV0 {
                result: Some(get_contested_resource_identity_votes_response_v0::Result::Proof(_),),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_limit_above_max_is_error() {
        // `limit` that overflows u16 takes the `.ok_or(...)?` path and surfaces
        // a Drive(Query(InvalidLimit)) error via `Err(_)` rather than as a
        // validation error in the response.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: Some((u16::MAX as u32) + 1),
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let err = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect_err("oversized limit should propagate an Err");

        let msg = format!("{:?}", err);
        assert!(
            msg.contains("InvalidLimit") || msg.contains("limit"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_query_contested_resource_identity_votes_limit_zero_is_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: Some(0),
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let err = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect_err("limit=0 should propagate an Err");

        let msg = format!("{:?}", err);
        assert!(
            msg.contains("InvalidLimit") || msg.contains("limit"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_query_contested_resource_identity_votes_descending_empty() {
        // Exercise the descending branch of the `start_at` computation and the
        // no-data path where `finished_results` short-circuits to true.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: Some(1),
            offset: None,
            order_ascending: false,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Votes(
                        get_contested_resource_identity_votes_response_v0::ContestedResourceIdentityVotes {
                            contested_resource_identity_votes,
                            finished_results,
                        },
                    ),
                ),
                metadata: Some(_),
            }) if contested_resource_identity_votes.is_empty() && finished_results
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_invalid_identity_id_zero_length() {
        // Zero-length identifier hits the same validation as 8-byte.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_offset_at_u16_max_accepted() {
        // offset == u16::MAX is within u16 range; the size guard rejects only
        // `> u16::MAX`. This covers the exact boundary.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: Some(u16::MAX as u32),
            order_ascending: true,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        // `u16::MAX` is the exact upper boundary accepted by the offset check
        // — it must neither surface any validation error nor drop the
        // response payload. Tightening beyond "no offset error" prevents the
        // test from silently passing if a different validation error starts
        // firing at this boundary.
        assert!(
            result.errors.is_empty(),
            "u16::MAX offset must be accepted without validation errors: {:?}",
            result.errors
        );
        assert!(matches!(
            result.data,
            Some(GetContestedResourceIdentityVotesResponseV0 {
                result: Some(get_contested_resource_identity_votes_response_v0::Result::Votes(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_start_at_invalid_identifier_errors() {
        // Invalid bytes for the start_at poll identifier should surface as an
        // outer Err because the `?` propagates through a platform_value::Error.
        use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: Some(StartAtVotePollIdInfo {
                start_at_poll_identifier: vec![0; 7],
                start_poll_identifier_included: true,
            }),
            prove: false,
        };

        let err = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect_err("invalid start_at_poll_identifier should propagate an Err");

        let msg = format!("{:?}", err);
        assert!(
            msg.contains("bytes") || msg.to_lowercase().contains("identifier"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_query_contested_resource_identity_votes_valid_start_at_empty_state() {
        // Valid 32-byte start_at poll identifier + empty state → success with
        // an *empty* result set. Assert the inner fields so the test does not
        // silently pass if the query started returning entries on empty state.
        use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: Some(StartAtVotePollIdInfo {
                start_at_poll_identifier: vec![0xAB; 32],
                start_poll_identifier_included: false,
            }),
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(matches!(
            result.data,
            Some(GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Votes(
                        get_contested_resource_identity_votes_response_v0::ContestedResourceIdentityVotes {
                            contested_resource_identity_votes,
                            finished_results,
                        },
                    ),
                ),
                metadata: Some(_),
            }) if contested_resource_identity_votes.is_empty() && finished_results
        ));
    }

    #[test]
    fn test_query_contested_resource_identity_votes_with_offset_and_descending() {
        // offset + descending should still succeed on empty state.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: Some(5),
            offset: Some(10),
            order_ascending: false,
            start_at_vote_poll_id_info: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn test_query_contested_resource_identity_votes_start_at_proof() {
        // Prove path with a valid start_at identifier exercises
        // `execute_with_proof`'s happy path on empty state.
        use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceIdentityVotesRequestV0 {
            identity_id: vec![0; 32],
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: Some(StartAtVotePollIdInfo {
                start_at_poll_identifier: vec![0x55; 32],
                start_poll_identifier_included: true,
            }),
            prove: true,
        };

        let result = platform
            .query_contested_resource_identity_votes_v0(request, &state, version)
            .expect("expected query to succeed");

        // An empty vote state can still yield a Proof (absence proof).
        assert!(matches!(
            result.data,
            Some(GetContestedResourceIdentityVotesResponseV0 {
                result: Some(get_contested_resource_identity_votes_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
