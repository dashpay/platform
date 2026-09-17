use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_group_queries::identifier_from_request;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_group_members_request::get_contract_group_members_request_v0::Members;
use dapi_grpc::platform::v0::get_contract_group_members_request::GetContractGroupMembersRequestV0;
use dapi_grpc::platform::v0::get_contract_group_members_response::{
    get_contract_group_members_response_v0, ContractMembers, DocumentTypeMembers,
    GetContractGroupMembersResponseV0, TokenMembers,
};
use dapi_grpc::platform::v0::{ContractGroupDocumentTypeMember, ContractGroupTokenMember};
use dpp::check_validation_result_with_data;
use dpp::data_contract::TokenContractPosition;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract_groups::types::{ContractGroupMembersPage, ContractGroupMembersQuery};
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;

/// The members kind and cursor of a request as the query Drive reads pages with.
pub(crate) fn members_query_from_proto(
    members: Members,
) -> Result<ContractGroupMembersQuery, QueryError> {
    Ok(match members {
        Members::Contracts(query) => ContractGroupMembersQuery::Contracts {
            start_after: query
                .start_after
                .map(|contract_id| identifier_from_request(contract_id, "start_after"))
                .transpose()?,
        },
        Members::DocumentTypes(query) => ContractGroupMembersQuery::DocumentTypes {
            start_after: query
                .start_after
                .map(|cursor| {
                    let ContractGroupDocumentTypeMember {
                        contract_id,
                        document_type_name,
                    } = cursor;
                    Ok::<_, QueryError>((
                        identifier_from_request(contract_id, "start_after.contract_id")?,
                        document_type_name,
                    ))
                })
                .transpose()?,
        },
        Members::Tokens(query) => ContractGroupMembersQuery::Tokens {
            start_after: query
                .start_after
                .map(|cursor| {
                    let ContractGroupTokenMember {
                        contract_id,
                        token_position,
                    } = cursor;
                    Ok::<_, QueryError>((
                        identifier_from_request(contract_id, "start_after.contract_id")?,
                        token_position_from_request(token_position)?,
                    ))
                })
                .transpose()?,
        },
    })
}

fn token_position_from_request(token_position: u32) -> Result<TokenContractPosition, QueryError> {
    TokenContractPosition::try_from(token_position).map_err(|_| {
        QueryError::InvalidArgument(format!(
            "start_after.token_position {token_position} is out of bounds, it must be at most {}",
            TokenContractPosition::MAX
        ))
    })
}

/// The wire form of one members page.
pub(crate) fn members_page_to_proto(
    page: ContractGroupMembersPage,
) -> get_contract_group_members_response_v0::Result {
    match page {
        ContractGroupMembersPage::Contracts(contract_ids) => {
            get_contract_group_members_response_v0::Result::Contracts(ContractMembers {
                contract_ids: contract_ids.into_iter().map(|id| id.to_vec()).collect(),
            })
        }
        ContractGroupMembersPage::DocumentTypes(document_types) => {
            get_contract_group_members_response_v0::Result::DocumentTypes(DocumentTypeMembers {
                document_types: document_types
                    .into_iter()
                    .map(
                        |(contract_id, document_type_name)| ContractGroupDocumentTypeMember {
                            contract_id: contract_id.to_vec(),
                            document_type_name,
                        },
                    )
                    .collect(),
            })
        }
        ContractGroupMembersPage::Tokens(tokens) => {
            get_contract_group_members_response_v0::Result::Tokens(TokenMembers {
                tokens: tokens
                    .into_iter()
                    .map(|(contract_id, token_position)| ContractGroupTokenMember {
                        contract_id: contract_id.to_vec(),
                        token_position: token_position as u32,
                    })
                    .collect(),
            })
        }
    }
}

impl<C> Platform<C> {
    /// Returns one page of a contract group's members of one kind, in key order, continuing
    /// after the request's cursor. An absent group answers with an empty page, as does a
    /// group with no members of that kind after the cursor; the proved form proves the page.
    ///
    /// `limit` defaults to and is capped by `max_returned_elements`, a protocol constant
    /// rather than node configuration, because a client verifying the proof rebuilds the page
    /// query from the request it sent: an omitted limit has to mean the same page on every
    /// node.
    pub(super) fn query_contract_group_members_v0(
        &self,
        GetContractGroupMembersRequestV0 {
            contract_group_id,
            members,
            limit,
            prove,
        }: GetContractGroupMembersRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractGroupMembersResponseV0>, Error> {
        let contract_group_id = check_validation_result_with_data!(identifier_from_request(
            contract_group_id,
            "contract_group_id"
        ));

        let Some(members) = members else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "members must select contracts, document types or tokens".to_string(),
                ),
            ));
        };
        let query = check_validation_result_with_data!(members_query_from_proto(members));

        let max_returned_elements = platform_version.drive_abci.query.max_returned_elements;
        let limit = match limit {
            None => max_returned_elements,
            Some(requested) => match u16::try_from(requested) {
                Ok(limit) if limit >= 1 && limit <= max_returned_elements => limit,
                _ => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidLimit(format!(
                            "limit {requested} is out of bounds, it must be between 1 and {max_returned_elements}"
                        )),
                    )));
                }
            },
        };

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_contract_group_members(
                    contract_group_id,
                    &query,
                    limit,
                    None,
                    platform_version
                ));

            GetContractGroupMembersResponseV0 {
                result: Some(get_contract_group_members_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let page = check_validation_result_with_data!(self.drive.fetch_contract_group_members(
                contract_group_id,
                &query,
                limit,
                None,
                platform_version
            ));

            GetContractGroupMembersResponseV0 {
                result: Some(members_page_to_proto(page)),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::contract_group_queries::tests::{
        join_group, register_group, single_owner_info,
    };
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_contract_group_members_request::{
        ContractMembersQuery, DocumentTypeMembersQuery, TokenMembersQuery,
    };
    use dpp::contract_group::ContractGroupMember;
    use dpp::dashcore::Network;
    use dpp::identifier::Identifier;
    use drive::drive::Drive;

    fn request(
        contract_group_id: Vec<u8>,
        members: Option<Members>,
        limit: Option<u32>,
        prove: bool,
    ) -> GetContractGroupMembersRequestV0 {
        GetContractGroupMembersRequestV0 {
            contract_group_id,
            members,
            limit,
            prove,
        }
    }

    fn contracts(start_after: Option<Identifier>) -> Option<Members> {
        Some(Members::Contracts(ContractMembersQuery {
            start_after: start_after.map(|id| id.to_vec()),
        }))
    }

    fn page(
        result: QueryValidationResult<GetContractGroupMembersResponseV0>,
    ) -> get_contract_group_members_response_v0::Result {
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        match result.data.expect("expected data").result {
            Some(get_contract_group_members_response_v0::Result::Proof(_)) => {
                panic!("expected a page, got a proof")
            }
            Some(page) => page,
            None => panic!("expected a page"),
        }
    }

    fn proof(
        result: QueryValidationResult<GetContractGroupMembersResponseV0>,
    ) -> dapi_grpc::platform::v0::Proof {
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        match result.data.expect("expected data").result {
            Some(get_contract_group_members_response_v0::Result::Proof(proof)) => proof,
            other => panic!("expected a proof, got {other:?}"),
        }
    }

    /// One group joined by three contracts as a whole, two document types and two tokens.
    fn populated_group(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        version: &PlatformVersion,
    ) -> (Identifier, [Identifier; 3]) {
        let group = Identifier::from([10; 32]);
        let contracts = [
            Identifier::from([20; 32]),
            Identifier::from([21; 32]),
            Identifier::from([22; 32]),
        ];
        register_group(
            platform,
            group,
            &single_owner_info(Identifier::from([2; 32]), None, None),
            version,
        );
        for contract in contracts {
            join_group(
                platform,
                contract,
                &[(group, ContractGroupMember::Contract)],
                version,
            );
        }
        join_group(
            platform,
            Identifier::from([30; 32]),
            &[
                (group, ContractGroupMember::DocumentType("note".to_string())),
                (group, ContractGroupMember::DocumentType("post".to_string())),
                (group, ContractGroupMember::Token(1)),
            ],
            version,
        );
        join_group(
            platform,
            Identifier::from([31; 32]),
            &[(group, ContractGroupMember::Token(0))],
            version,
        );
        (group, contracts)
    }

    #[test]
    fn test_invalid_contract_group_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_group_members_v0(
                request(vec![0; 8], contracts(None), None, false),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_group_id")
        ));
    }

    #[test]
    fn test_rejects_missing_members_kind() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_group_members_v0(
                request(vec![0; 32], None, None, false),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("members")
        ));
    }

    #[test]
    fn test_rejects_out_of_bounds_limits() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let too_many = version.drive_abci.query.max_returned_elements as u32 + 1;

        for limit in [0, too_many, u32::MAX] {
            let result = platform
                .query_contract_group_members_v0(
                    request(vec![0; 32], contracts(None), Some(limit), false),
                    &state,
                    version,
                )
                .expect("expected query to succeed");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [QueryError::Query(QuerySyntaxError::InvalidLimit(_))]
                ),
                "limit {limit}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn test_rejects_invalid_cursors() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let bad_cursors = [
            Members::Contracts(ContractMembersQuery {
                start_after: Some(vec![1; 5]),
            }),
            Members::DocumentTypes(DocumentTypeMembersQuery {
                start_after: Some(ContractGroupDocumentTypeMember {
                    contract_id: vec![1; 5],
                    document_type_name: "note".to_string(),
                }),
            }),
            Members::Tokens(TokenMembersQuery {
                start_after: Some(ContractGroupTokenMember {
                    contract_id: vec![1; 32],
                    token_position: u16::MAX as u32 + 1,
                }),
            }),
        ];
        for members in bad_cursors {
            let result = platform
                .query_contract_group_members_v0(
                    request(vec![0; 32], Some(members.clone()), None, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [QueryError::InvalidArgument(msg)] if msg.contains("start_after")
                ),
                "{members:?}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn test_absent_group_has_an_empty_page() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let page = page(
            platform
                .query_contract_group_members_v0(
                    request(vec![0; 32], contracts(None), None, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        assert_eq!(
            page,
            get_contract_group_members_response_v0::Result::Contracts(ContractMembers::default())
        );
    }

    #[test]
    fn test_pages_through_contracts_with_a_cursor() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let (group, members) = populated_group(&platform, version);

        let first = page(
            platform
                .query_contract_group_members_v0(
                    request(group.to_vec(), contracts(None), Some(2), false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            first,
            get_contract_group_members_response_v0::Result::Contracts(ContractMembers {
                contract_ids: vec![members[0].to_vec(), members[1].to_vec()],
            })
        );

        let second = page(
            platform
                .query_contract_group_members_v0(
                    request(group.to_vec(), contracts(Some(members[1])), Some(2), false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            second,
            get_contract_group_members_response_v0::Result::Contracts(ContractMembers {
                contract_ids: vec![members[2].to_vec()],
            })
        );
    }

    #[test]
    fn test_returns_document_types_and_tokens() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let (group, _) = populated_group(&platform, version);

        let document_types = page(
            platform
                .query_contract_group_members_v0(
                    request(
                        group.to_vec(),
                        Some(Members::DocumentTypes(DocumentTypeMembersQuery {
                            start_after: None,
                        })),
                        None,
                        false,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            document_types,
            get_contract_group_members_response_v0::Result::DocumentTypes(DocumentTypeMembers {
                document_types: vec![
                    ContractGroupDocumentTypeMember {
                        contract_id: vec![30; 32],
                        document_type_name: "note".to_string(),
                    },
                    ContractGroupDocumentTypeMember {
                        contract_id: vec![30; 32],
                        document_type_name: "post".to_string(),
                    },
                ],
            })
        );

        let tokens_after_first = page(
            platform
                .query_contract_group_members_v0(
                    request(
                        group.to_vec(),
                        Some(Members::Tokens(TokenMembersQuery {
                            start_after: Some(ContractGroupTokenMember {
                                contract_id: vec![30; 32],
                                token_position: 1,
                            }),
                        })),
                        None,
                        false,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            tokens_after_first,
            get_contract_group_members_response_v0::Result::Tokens(TokenMembers {
                tokens: vec![ContractGroupTokenMember {
                    contract_id: vec![31; 32],
                    token_position: 0,
                }],
            })
        );
    }

    #[test]
    fn test_proof_verifies_the_same_page() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let (group, members) = populated_group(&platform, version);
        let query = ContractGroupMembersQuery::Contracts {
            start_after: Some(members[0]),
        };

        let proof = proof(
            platform
                .query_contract_group_members_v0(
                    request(group.to_vec(), contracts(Some(members[0])), Some(1), true),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        let (_root_hash, verified) =
            Drive::verify_contract_group_members(&proof.grovedb_proof, group, &query, 1, version)
                .expect("expected the proof to verify");
        assert_eq!(
            verified,
            ContractGroupMembersPage::Contracts(vec![members[1]])
        );
    }
}
