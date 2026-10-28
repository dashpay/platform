//! Queries for features a protocol version does not have yet. A 5.0 node at protocol 13 serves
//! the methods 5.0 added, but some of them read state that only exists from protocol 14. Every
//! node at a height runs the same protocol version, so another node cannot answer those either:
//! the refusal must be a status clients do not retry, or each call bans the nodes it reaches.
//! A query method a later protocol version adds belongs in the sweep below.

use super::offsets::{assert_no_node_bans, node_query};
use super::*;
use dapi_grpc::platform::v0::get_contract_document_removals_request::{
    get_contract_document_removals_request_v0, GetContractDocumentRemovalsRequestV0, Page,
};
use dapi_grpc::platform::v0::get_contract_fee_pots_request::GetContractFeePotsRequestV0;
use dapi_grpc::platform::v0::get_contract_group_info_request::GetContractGroupInfoRequestV0;
use dapi_grpc::platform::v0::get_contract_group_members_request::{
    get_contract_group_members_request_v0, ContractMembersQuery, DocumentTypeMembersQuery,
    GetContractGroupMembersRequestV0, TokenMembersQuery,
};
use dapi_grpc::platform::v0::get_contract_groups_for_contract_request::GetContractGroupsForContractRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_action_counts_request::GetContractModerationActionCountsRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_entries_request::GetContractModerationEntriesRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
use dapi_grpc::platform::v0::get_contract_team_action_signers_request::GetContractTeamActionSignersRequestV0;
use dapi_grpc::platform::v0::get_contract_team_actions_request::GetContractTeamActionsRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_request::GetIdentityKeysRemainingBudgetsRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_response::{
    get_identity_keys_remaining_budgets_response_v0, Version as KeysRemainingBudgetsVersion,
};
use dapi_grpc::platform::v0::ContractModerationList;
use rs_dapi_client::CanRetry;

node_query!(
    KeysRemainingBudgetsQuery,
    wire::GetIdentityKeysRemainingBudgetsRequest,
    wire::GetIdentityKeysRemainingBudgetsResponse,
    get_identity_keys_remaining_budgets
);

fn keys_remaining_budgets(prove: bool) -> wire::GetIdentityKeysRemainingBudgetsRequest {
    GetIdentityKeysRemainingBudgetsRequestV0 {
        identity_id: vec![1; 32],
        key_ids: vec![0],
        prove,
    }
    .into()
}

/// What a protocol 13 node answers each method 5.0 added, with and without a proof. None of
/// them may be a status the client retries on another node, and each refusal must be the one
/// the row is about, not an unrelated complaint about the request.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_answer_every_query_added_in_5_0_without_a_retryable_status_at_protocol_13() {
    let fixture = QueryFixture::new_with_version(PlatformVersion::get(13).expect("protocol 13"));
    let (contract_id, document_type) = fixture.store_contract();
    let contract_id = contract_id.to_vec();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();

    for prove in [false, true] {
        macro_rules! check {
            ($name:expr, $expected:expr, $method:ident, $request:expr) => {
                check!($name, $expected, "", $method, $request)
            };
            ($name:expr, $expected:expr, $reason:expr, $method:ident, $request:expr) => {{
                let result = client.$method($request).await;
                if let Err(status) = &result {
                    if status.can_retry() {
                        failures.push(format!(
                            "{} prove={prove}: retryable {:?}: {}",
                            $name,
                            status.code(),
                            status.message()
                        ));
                    }
                    if !status.message().contains($reason) {
                        failures.push(format!(
                            "{} prove={prove}: refused for another reason than {:?}: {}",
                            $name,
                            $reason,
                            status.message()
                        ));
                    }
                }
                record($name, prove, result, $expected, &mut failures);
            }};
        }

        check!(
            "identity keys remaining budgets",
            Code::InvalidArgument,
            "remaining_budgets not active",
            get_identity_keys_remaining_budgets,
            keys_remaining_budgets(prove)
        );
        for include_contracts in [false, true] {
            check!(
                "data contracts latest versions",
                Code::Ok,
                get_data_contracts_latest_versions,
                wire::GetDataContractsLatestVersionsRequest::from(
                    GetDataContractsLatestVersionsRequestV0 {
                        ids: vec![contract_id.clone()],
                        include_contracts,
                        prove,
                    }
                )
            );
        }
        for ids_only in [false, true] {
            check!(
                "data contracts by range",
                Code::Ok,
                get_data_contracts_by_range,
                wire::GetDataContractsByRangeRequest::from(GetDataContractsByRangeRequestV0 {
                    limit: Some(10),
                    start: None,
                    ids_only,
                    prove,
                })
            );
        }
        check!(
            "contract group info",
            Code::Ok,
            get_contract_group_info,
            wire::GetContractGroupInfoRequest::from(GetContractGroupInfoRequestV0 {
                contract_group_id: vec![5; 32],
                prove,
            })
        );
        for members in [
            get_contract_group_members_request_v0::Members::Contracts(ContractMembersQuery {
                start_after: None,
            }),
            get_contract_group_members_request_v0::Members::DocumentTypes(
                DocumentTypeMembersQuery { start_after: None },
            ),
            get_contract_group_members_request_v0::Members::Tokens(TokenMembersQuery {
                start_after: None,
            }),
        ] {
            check!(
                "contract group members",
                Code::Ok,
                get_contract_group_members,
                wire::GetContractGroupMembersRequest::from(GetContractGroupMembersRequestV0 {
                    contract_group_id: vec![5; 32],
                    members: Some(members),
                    limit: Some(10),
                    prove,
                })
            );
        }
        check!(
            "contract groups for contract",
            Code::Ok,
            get_contract_groups_for_contract,
            wire::GetContractGroupsForContractRequest::from(
                GetContractGroupsForContractRequestV0 {
                    contract_id: contract_id.clone(),
                    prove,
                }
            )
        );
        check!(
            "contract moderation status",
            Code::InvalidArgument,
            "is not moderated",
            get_contract_moderation_status,
            wire::GetContractModerationStatusRequest::from(GetContractModerationStatusRequestV0 {
                contract_id: contract_id.clone(),
                identity_id: vec![1; 32],
                lists: vec![ContractModerationList::Banlist as i32],
                prove,
            })
        );
        check!(
            "contract moderation entries",
            Code::InvalidArgument,
            "is not moderated",
            get_contract_moderation_entries,
            wire::GetContractModerationEntriesRequest::from(
                GetContractModerationEntriesRequestV0 {
                    contract_id: contract_id.clone(),
                    list: ContractModerationList::Banlist as i32,
                    start_after: None,
                    limit: Some(10),
                    prove,
                }
            )
        );
        check!(
            "contract document removals",
            Code::InvalidArgument,
            "deletions keep records",
            get_contract_document_removals,
            wire::GetContractDocumentRemovalsRequest::from(GetContractDocumentRemovalsRequestV0 {
                contract_id: contract_id.clone(),
                document_type_name: document_type.clone(),
                selection: Some(get_contract_document_removals_request_v0::Selection::Page(
                    Page {
                        start_after: None,
                        limit: Some(10),
                    }
                )),
                prove,
            })
        );
        check!(
            "contract team actions",
            Code::InvalidArgument,
            "keeps no team actions",
            get_contract_team_actions,
            wire::GetContractTeamActionsRequest::from(GetContractTeamActionsRequestV0 {
                contract_id: contract_id.clone(),
                status: 0,
                start_at_action_id: None,
                count: Some(10),
                prove,
            })
        );
        check!(
            "contract team action signers",
            Code::InvalidArgument,
            "keeps no team actions",
            get_contract_team_action_signers,
            wire::GetContractTeamActionSignersRequest::from(
                GetContractTeamActionSignersRequestV0 {
                    contract_id: contract_id.clone(),
                    status: 0,
                    action_id: vec![3; 32],
                    prove,
                }
            )
        );
        check!(
            "contract moderation action counts",
            Code::InvalidArgument,
            "keeps no moderation action counts",
            get_contract_moderation_action_counts,
            wire::GetContractModerationActionCountsRequest::from(
                GetContractModerationActionCountsRequestV0 {
                    contract_id: contract_id.clone(),
                    prove,
                }
            )
        );
        check!(
            "contract fee pots",
            Code::Ok,
            get_contract_fee_pots,
            wire::GetContractFeePotsRequest::from(GetContractFeePotsRequestV0 {
                contract_id: contract_id.clone(),
                prove,
            })
        );
    }
    server.abort();
    assert!(
        failures.is_empty(),
        "queries added in 5.0 answered at protocol 13 with the wrong status:\n{}",
        failures.join("\n")
    );
}

/// Three calls to a method protocol 13 does not have, sent to 13 protocol 13 nodes: each call
/// fails on the first node it reaches and leaves every node available.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_keep_all_thirteen_nodes_available_after_three_key_budget_requests_at_protocol_13() {
    let fixture = Arc::new(QueryFixture::new_with_version(
        PlatformVersion::get(13).expect("protocol 13"),
    ));
    for prove in [false, true] {
        let attempts = Arc::default();
        assert_no_node_bans(
            KeysRemainingBudgetsQuery {
                fixture: Arc::clone(&fixture),
                request: keys_remaining_budgets(prove),
                attempts: Arc::clone(&attempts),
            },
            &attempts,
        )
        .await;
    }
}

/// From protocol 14 the same request is served, with and without a proof.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_serve_identity_key_budgets_from_protocol_14() {
    for version in [
        PlatformVersion::get(14).expect("protocol 14"),
        PlatformVersion::latest(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let (mut client, server) = fixture.client();
        for prove in [false, true] {
            let response = client
                .get_identity_keys_remaining_budgets(keys_remaining_budgets(prove))
                .await
                .unwrap_or_else(|status| {
                    panic!(
                        "protocol {} prove={prove}: {status}",
                        version.protocol_version
                    )
                })
                .into_inner();
            let Some(KeysRemainingBudgetsVersion::V0(v0)) = response.version else {
                panic!("expected the V0 response");
            };
            match (prove, v0.result) {
                (
                    false,
                    Some(get_identity_keys_remaining_budgets_response_v0::Result::KeysRemainingBudgets(
                        budgets,
                    )),
                ) => assert_eq!(
                    budgets.entries.len(),
                    1,
                    "one entry per requested key"
                ),
                (true, Some(get_identity_keys_remaining_budgets_response_v0::Result::Proof(proof))) => {
                    assert!(!proof.grovedb_proof.is_empty())
                }
                (prove, result) => panic!("prove={prove}: unexpected result {result:?}"),
            }
        }
        server.abort();
    }
}
