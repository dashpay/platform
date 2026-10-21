//! Unsupported proof modes must fail locally while supported request encodings stay intact.

use dash_sdk::platform::query::{
    ProposerBlockCountByIdsQuery, RecentAddressBalanceChangesQuery,
    RecentCompactedAddressBalanceChangesQuery, TokenLastClaimQuery, VoteQuery,
};
use dash_sdk::platform::tokens::token_pre_programmed_distributions::TokenPreProgrammedDistributionsQuery;
use dash_sdk::platform::types::{
    epoch::EpochQuery,
    finalized_epoch::FinalizedEpochQuery,
    identity::{IdentityRequest, NonUniquePublicKeyHashQuery, PublicKeyHash},
};
use dash_sdk::platform::{
    proto, DocumentHistoryQuery, Identifier, IdentityKeysQuery, LimitQuery, Query, QuerySettings,
    QueryStartInfo,
};
use dash_sdk::Error;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::{hashes::Hash, ProTxHash};
use dpp::data_contract::DataContractFactory;
use dpp::platform_value::platform_value;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::query::{
    contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery,
    vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery,
    vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    },
    vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery,
    VotePollsByEndDateDriveQuery,
};
use drive_proof_verifier::types::{KeysInPath, NoParamQuery};
use rs_dapi_client::{CanRetry, RequestSettings};
use std::collections::BTreeSet;

const UNPROVED_MESSAGE: &str = "unproved queries are not supported for this query; enable proofs";
const QUORUM_MESSAGE: &str = "GetCurrentQuorumsInfo does not support proofs; use FetchUnproved";

fn id() -> Identifier {
    Identifier::new([1; 32])
}
fn other_id() -> Identifier {
    Identifier::new([2; 32])
}
fn hash() -> ProTxHash {
    ProTxHash::from_byte_array([3; 32])
}
fn limited<Q>(query: Q) -> LimitQuery<Q> {
    LimitQuery {
        query,
        limit: Some(2),
        start_info: None,
    }
}
fn cursor() -> QueryStartInfo {
    QueryStartInfo {
        start_key: vec![2; 32],
        start_included: false,
    }
}
fn poll() -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: id(),
        document_type_name: "domain".into(),
        index_name: "label".into(),
        index_values: vec!["alice".into()],
    }
}
fn vote_state() -> ContestedDocumentVotePollDriveQuery {
    ContestedDocumentVotePollDriveQuery {
        vote_poll: poll(),
        result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
        offset: None,
        limit: Some(2),
        start_at: None,
        allow_include_locked_and_abstaining_vote_tally: true,
    }
}
fn assert_config(error: Error, message: &str) {
    assert!(
        !error.can_retry(),
        "local configuration errors must not retry or penalize nodes"
    );
    match error {
        Error::Config(actual) => assert_eq!(actual, message),
        other => panic!("expected a local Config refusal, got {other:?}"),
    }
}

macro_rules! proof_only {
    ($name:ident, $input:expr, $request:ty, $check:expr) => {
        mod $name {
            use super::*;
            #[test]
            fn should_reject_unproved_before_transport() {
                let settings = RequestSettings::default();
                let settings = QuerySettings {
                    request_settings: &settings,
                    protocol_version: PlatformVersion::latest(),
                    prove: false,
                };
                let input = $input;
                assert_config(
                    <_ as Query<$request>>::query(&input, &settings).unwrap_err(),
                    UNPROVED_MESSAGE,
                );
            }
            #[test]
            fn should_preserve_proved_request() {
                let settings = RequestSettings::default();
                let settings = QuerySettings {
                    request_settings: &settings,
                    protocol_version: PlatformVersion::latest(),
                    prove: true,
                };
                let input = $input;
                let request = <_ as Query<$request>>::query(&input, &settings).unwrap();
                ($check)(request);
            }
        }
    };
}

proof_only!(
    contract,
    id(),
    proto::GetDataContractRequest,
    |request: proto::GetDataContractRequest| {
        let Some(proto::get_data_contract_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.id, id().to_vec());
    }
);

proof_only!(
    contracts,
    vec![id(), other_id()],
    proto::GetDataContractsRequest,
    |request: proto::GetDataContractsRequest| {
        let Some(proto::get_data_contracts_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.ids, vec![id().to_vec(), other_id().to_vec()]);
    }
);

proof_only!(
    contract_history,
    limited((id(), 123_u64)),
    proto::GetDataContractHistoryRequest,
    |request: proto::GetDataContractHistoryRequest| {
        let Some(proto::get_data_contract_history_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.start_at_ms, 123);
    }
);

proof_only!(
    document_history,
    DocumentHistoryQuery {
        data_contract_id: id(),
        document_type_name: "domain".into(),
        document_id: other_id(),
        start_at_ms: 123,
        limit: Some(2),
        offset: None
    },
    proto::GetDocumentHistoryRequest,
    |request: proto::GetDocumentHistoryRequest| {
        let Some(proto::get_document_history_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.document_id, other_id().to_vec());
    }
);

proof_only!(
    identity_all_keys,
    id(),
    proto::GetIdentityKeysRequest,
    |request: proto::GetIdentityKeysRequest| {
        let Some(proto::get_identity_keys_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.identity_id, id().to_vec());
    }
);

proof_only!(
    identity_specific_keys,
    IdentityKeysQuery::new(id(), vec![0, 2]).with_limit(2),
    proto::GetIdentityKeysRequest,
    |request: proto::GetIdentityKeysRequest| {
        let Some(proto::get_identity_keys_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.limit, Some(2));
    }
);

proof_only!(
    address,
    PlatformAddress::P2pkh([4; 20]),
    proto::GetAddressInfoRequest,
    |request: proto::GetAddressInfoRequest| {
        let Some(proto::get_address_info_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.address, PlatformAddress::P2pkh([4; 20]).to_bytes());
    }
);

proof_only!(
    addresses,
    BTreeSet::from([PlatformAddress::P2pkh([4; 20])]),
    proto::GetAddressesInfosRequest,
    |request: proto::GetAddressesInfosRequest| {
        let Some(proto::get_addresses_infos_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(
            v0.addresses,
            vec![PlatformAddress::P2pkh([4; 20]).to_bytes()]
        );
    }
);

proof_only!(
    address_trunk,
    (),
    proto::GetAddressesTrunkStateRequest,
    |request: proto::GetAddressesTrunkStateRequest| {
        let Some(proto::get_addresses_trunk_state_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert_eq!(
            v0,
            proto::get_addresses_trunk_state_request::GetAddressesTrunkStateRequestV0 {}
        );
    }
);

proof_only!(
    epochs,
    limited(EpochQuery {
        start: Some(1),
        ascending: false
    }),
    proto::GetEpochsInfoRequest,
    |request: proto::GetEpochsInfoRequest| {
        let Some(proto::get_epochs_info_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.count, 2);
    }
);

proof_only!(
    upgrade_state,
    (),
    proto::GetProtocolVersionUpgradeStateRequest,
    |request: proto::GetProtocolVersionUpgradeStateRequest| {
        let Some(proto::get_protocol_version_upgrade_state_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
    }
);

proof_only!(
    upgrade_votes,
    limited(Some(hash())),
    proto::GetProtocolVersionUpgradeVoteStatusRequest,
    |request: proto::GetProtocolVersionUpgradeVoteStatusRequest| {
        let Some(proto::get_protocol_version_upgrade_vote_status_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.start_pro_tx_hash, vec![3; 32]);
    }
);

proof_only!(
    contested_resources,
    VotePollsByDocumentTypeQuery {
        contract_id: id(),
        document_type_name: "domain".into(),
        index_name: "label".into(),
        start_index_values: vec!["alice".into()],
        end_index_values: vec![],
        start_at_value: None,
        limit: Some(2),
        order_ascending: false
    },
    proto::GetContestedResourcesRequest,
    |request: proto::GetContestedResourcesRequest| {
        let Some(proto::get_contested_resources_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.document_type_name, "domain");
    }
);

proof_only!(
    contested_vote_state,
    vote_state(),
    proto::GetContestedResourceVoteStateRequest,
    |request: proto::GetContestedResourceVoteStateRequest| {
        let Some(proto::get_contested_resource_vote_state_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.contract_id, id().to_vec());
    }
);

proof_only!(
    limited_contested_vote_state,
    LimitQuery {
        query: vote_state(),
        limit: Some(2),
        start_info: Some(cursor())
    },
    proto::GetContestedResourceVoteStateRequest,
    |request: proto::GetContestedResourceVoteStateRequest| {
        let Some(proto::get_contested_resource_vote_state_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(
            v0.start_at_identifier_info.unwrap().start_identifier,
            vec![2; 32]
        );
    }
);

proof_only!(
    contested_voters,
    ContestedDocumentVotePollVotesDriveQuery {
        vote_poll: poll(),
        contestant_id: other_id(),
        offset: None,
        limit: Some(2),
        start_at: None,
        order_ascending: false
    },
    proto::GetContestedResourceVotersForIdentityRequest,
    |request: proto::GetContestedResourceVotersForIdentityRequest| {
        let Some(proto::get_contested_resource_voters_for_identity_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.contestant_id, other_id().to_vec());
    }
);

proof_only!(
    identity_votes,
    ContestedResourceVotesGivenByIdentityQuery {
        identity_id: id(),
        offset: None,
        limit: Some(2),
        start_at: Some(([2; 32], false)),
        order_ascending: false
    },
    proto::GetContestedResourceIdentityVotesRequest,
    |request: proto::GetContestedResourceIdentityVotesRequest| {
        let Some(proto::get_contested_resource_identity_votes_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.identity_id, id().to_vec());
    }
);

proof_only!(
    masternode_votes,
    hash(),
    proto::GetContestedResourceIdentityVotesRequest,
    |request: proto::GetContestedResourceIdentityVotesRequest| {
        let Some(proto::get_contested_resource_identity_votes_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.identity_id, vec![3; 32]);
    }
);

proof_only!(
    vote_polls_by_end,
    VotePollsByEndDateDriveQuery {
        start_time: Some((123, false)),
        end_time: Some((456, true)),
        limit: Some(2),
        offset: None,
        order_ascending: false
    },
    proto::GetVotePollsByEndDateRequest,
    |request: proto::GetVotePollsByEndDateRequest| {
        let Some(proto::get_vote_polls_by_end_date_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.limit, Some(2));
    }
);

proof_only!(
    prefunded_balance,
    id(),
    proto::GetPrefundedSpecializedBalanceRequest,
    |request: proto::GetPrefundedSpecializedBalanceRequest| {
        let Some(proto::get_prefunded_specialized_balance_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.id, id().to_vec());
    }
);

proof_only!(
    single_vote,
    VoteQuery::new(id(), other_id()),
    proto::GetContestedResourceIdentityVotesRequest,
    |request: proto::GetContestedResourceIdentityVotesRequest| {
        let Some(proto::get_contested_resource_identity_votes_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.limit, Some(1));
    }
);

proof_only!(
    limited_vote,
    LimitQuery {
        query: VoteQuery::new(id(), other_id()),
        limit: Some(2),
        start_info: Some(cursor())
    },
    proto::GetContestedResourceIdentityVotesRequest,
    |request: proto::GetContestedResourceIdentityVotesRequest| {
        let Some(proto::get_contested_resource_identity_votes_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(
            v0.start_at_vote_poll_id_info
                .unwrap()
                .start_at_poll_identifier,
            vec![2; 32]
        );
    }
);

proof_only!(
    path_elements,
    KeysInPath {
        path: vec![vec![1]],
        keys: vec![vec![2]]
    },
    proto::GetPathElementsRequest,
    |request: proto::GetPathElementsRequest| {
        let Some(proto::get_path_elements_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.keys, vec![vec![2]]);
    }
);

proof_only!(
    total_credits,
    NoParamQuery {},
    proto::GetTotalCreditsInPlatformRequest,
    |request: proto::GetTotalCreditsInPlatformRequest| {
        let Some(proto::get_total_credits_in_platform_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
    }
);

proof_only!(
    proposed_blocks_range,
    limited(Some(1_u16)),
    proto::GetEvonodesProposedEpochBlocksByRangeRequest,
    |request: proto::GetEvonodesProposedEpochBlocksByRangeRequest| {
        let Some(proto::get_evonodes_proposed_epoch_blocks_by_range_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.epoch, Some(1));
    }
);

proof_only!(
    token_purchase_prices,
    &[id(), other_id()][..],
    proto::GetTokenDirectPurchasePricesRequest,
    |request: proto::GetTokenDirectPurchasePricesRequest| {
        let Some(proto::get_token_direct_purchase_prices_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.token_ids, vec![id().to_vec(), other_id().to_vec()]);
    }
);

proof_only!(
    token_last_claim,
    TokenLastClaimQuery {
        token_id: id(),
        identity_id: other_id()
    },
    proto::GetTokenPerpetualDistributionLastClaimRequest,
    |request: proto::GetTokenPerpetualDistributionLastClaimRequest| {
        let Some(proto::get_token_perpetual_distribution_last_claim_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.identity_id, other_id().to_vec());
    }
);

proof_only!(
    proposed_blocks_ids,
    ProposerBlockCountByIdsQuery {
        epoch: Some(1),
        pro_tx_hashes: vec![hash()]
    },
    proto::GetEvonodesProposedEpochBlocksByIdsRequest,
    |request: proto::GetEvonodesProposedEpochBlocksByIdsRequest| {
        let Some(proto::get_evonodes_proposed_epoch_blocks_by_ids_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.ids, vec![vec![3; 32]]);
    }
);

proof_only!(
    recent_address_changes,
    RecentAddressBalanceChangesQuery::new(123),
    proto::GetRecentAddressBalanceChangesRequest,
    |request: proto::GetRecentAddressBalanceChangesRequest| {
        let Some(proto::get_recent_address_balance_changes_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.start_height, 123);
    }
);

proof_only!(
    compacted_address_changes,
    RecentCompactedAddressBalanceChangesQuery::new(123),
    proto::GetRecentCompactedAddressBalanceChangesRequest,
    |request: proto::GetRecentCompactedAddressBalanceChangesRequest| {
        let Some(proto::get_recent_compacted_address_balance_changes_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.start_block_height, 123);
    }
);

proof_only!(
    identity_balance,
    id(),
    proto::GetIdentityBalanceRequest,
    |request: proto::GetIdentityBalanceRequest| {
        let Some(proto::get_identity_balance_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.id, id().to_vec());
    }
);

proof_only!(
    identity_nonce,
    id(),
    proto::GetIdentityNonceRequest,
    |request: proto::GetIdentityNonceRequest| {
        let Some(proto::get_identity_nonce_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.identity_id, id().to_vec());
    }
);

proof_only!(
    identity_contract_nonce,
    (id(), other_id()),
    proto::GetIdentityContractNonceRequest,
    |request: proto::GetIdentityContractNonceRequest| {
        let Some(proto::get_identity_contract_nonce_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.contract_id, other_id().to_vec());
    }
);

proof_only!(
    identity_balance_revision,
    id(),
    proto::GetIdentityBalanceAndRevisionRequest,
    |request: proto::GetIdentityBalanceAndRevisionRequest| {
        let Some(proto::get_identity_balance_and_revision_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.id, id().to_vec());
    }
);

proof_only!(
    identities_balances,
    vec![id(), other_id()],
    proto::GetIdentitiesBalancesRequest,
    |request: proto::GetIdentitiesBalancesRequest| {
        let Some(proto::get_identities_balances_request::Version::V0(v0)) = request.version else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.ids, vec![id().to_vec(), other_id().to_vec()]);
    }
);

proof_only!(
    finalized_epochs,
    FinalizedEpochQuery::from((1, 2)),
    proto::GetFinalizedEpochInfosRequest,
    |request: proto::GetFinalizedEpochInfosRequest| {
        let Some(proto::get_finalized_epoch_infos_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.end_epoch_index, 2);
    }
);

proof_only!(
    token_distributions,
    TokenPreProgrammedDistributionsQuery {
        token_id: id(),
        start_at_info: None,
        limit: Some(2)
    },
    proto::GetTokenPreProgrammedDistributionsRequest,
    |request: proto::GetTokenPreProgrammedDistributionsRequest| {
        let Some(proto::get_token_pre_programmed_distributions_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.token_id, id().to_vec());
    }
);

proof_only!(identity, id(), IdentityRequest, |request| {
    let IdentityRequest::GetIdentity(request) = request else {
        panic!("expected identity request variant")
    };
    let Some(proto::get_identity_request::Version::V0(v0)) = request.version else {
        panic!("expected V0 request")
    };
    assert!(v0.prove);
    assert_eq!(v0.id, id().to_vec());
});

proof_only!(
    identity_unique_key,
    PublicKeyHash([3; 20]),
    IdentityRequest,
    |request| {
        let IdentityRequest::GetIdentityByPublicKeyHash(request) = request else {
            panic!("expected identity request variant")
        };
        let Some(proto::get_identity_by_public_key_hash_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.public_key_hash, vec![3; 20]);
    }
);

proof_only!(
    identity_non_unique_key,
    NonUniquePublicKeyHashQuery {
        key_hash: [3; 20],
        after: Some([2; 32])
    },
    IdentityRequest,
    |request| {
        let IdentityRequest::GetIdentityByNonUniquePublicKeyHash(request) = request else {
            panic!("expected identity request variant")
        };
        let Some(proto::get_identity_by_non_unique_public_key_hash_request::Version::V0(v0)) =
            request.version
        else {
            panic!("expected V0 request")
        };
        assert!(v0.prove);
        assert_eq!(v0.start_after, Some(vec![2; 32]));
    }
);

#[test]
fn should_reject_proved_quorum_info_before_transport() {
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::latest(),
        prove: true,
    };
    assert_config(
        <NoParamQuery as Query<proto::GetCurrentQuorumsInfoRequest>>::query(
            &NoParamQuery {},
            &settings,
        )
        .unwrap_err(),
        QUORUM_MESSAGE,
    );
}

#[test]
fn should_preserve_unproved_quorum_info() {
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::latest(),
        prove: false,
    };
    let raw = <NoParamQuery as Query<proto::GetCurrentQuorumsInfoRequest>>::query(
        &NoParamQuery {},
        &settings.without_proofs(),
    )
    .unwrap();
    assert!(matches!(
        raw.version,
        Some(proto::get_current_quorums_info_request::Version::V0(_))
    ));
}

#[test]
fn should_reject_unsupported_modes_for_older_protocol_versions() {
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::get(1).unwrap(),
        prove: false,
    };
    assert_config(
        <Identifier as Query<IdentityRequest>>::query(&id(), &settings).unwrap_err(),
        UNPROVED_MESSAGE,
    );
    assert_config(
        FinalizedEpochQuery::from((1, 2))
            .query(&settings)
            .unwrap_err(),
        UNPROVED_MESSAGE,
    );
}

#[test]
fn should_reject_unproved_epoch_conveniences() {
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::latest(),
        prove: false,
    };
    assert_config(
        <u16 as Query<proto::GetEpochsInfoRequest>>::query(&1, &settings).unwrap_err(),
        UNPROVED_MESSAGE,
    );
    assert_config(
        <EpochQuery as Query<proto::GetEpochsInfoRequest>>::query(
            &EpochQuery {
                start: Some(1),
                ascending: false,
            },
            &settings,
        )
        .unwrap_err(),
        UNPROVED_MESSAGE,
    );
    assert_config(
        <(u16, u16) as Query<proto::GetFinalizedEpochInfosRequest>>::query(&(1, 2), &settings)
            .unwrap_err(),
        UNPROVED_MESSAGE,
    );
}

#[test]
fn should_preserve_raw_wire_requests_under_both_settings() {
    let request_settings = RequestSettings::default();
    for prove in [false, true] {
        let settings = QuerySettings {
            request_settings: &request_settings,
            protocol_version: PlatformVersion::latest(),
            prove,
        };
        let raw = proto::GetIdentityRequest {
            version: Some(proto::get_identity_request::Version::V0(
                proto::get_identity_request::GetIdentityRequestV0 {
                    id: id().to_vec(),
                    prove: false,
                },
            )),
        };
        assert_eq!(raw.query(&settings).unwrap(), raw);
    }
}

#[test]
fn should_preserve_forced_proofs_for_rich_document_queries() {
    let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .unwrap()
        .create_with_value_config(
            id(),
            0,
            platform_value!({
                "post": {
                    "type": "object",
                    "properties": {
                        "label": { "type": "string", "maxLength": 63, "position": 0 }
                    },
                    "required": ["label"],
                    "additionalProperties": false
                }
            }),
            None,
            None,
        )
        .unwrap()
        .data_contract_owned();
    let query = dash_sdk::platform::DocumentQuery::new(contract, "post")
        .unwrap()
        .with_limit(2);
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::latest(),
        prove: false,
    };
    let retained: dash_sdk::platform::DocumentQuery = query.query(&settings).unwrap();
    let request = retained
        .try_into_request_for_version(settings.protocol_version)
        .unwrap();
    let Some(proto::get_documents_request::Version::V1(v1)) = request.version else {
        panic!("expected latest document wire version")
    };
    assert!(v1.prove);
    assert_eq!(v1.document_type, "post");
    assert_eq!(v1.limit, Some(2));
}

#[test]
fn should_preserve_proved_requests_for_older_protocol_versions() {
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::get(1).unwrap(),
        prove: true,
    };
    let identity = <Identifier as Query<IdentityRequest>>::query(&id(), &settings).unwrap();
    let IdentityRequest::GetIdentity(request) = identity else {
        panic!("expected identity by id")
    };
    let Some(proto::get_identity_request::Version::V0(v0)) = request.version else {
        panic!("expected V0")
    };
    assert!(v0.prove);
    assert_eq!(v0.id, id().to_vec());
    let epochs = FinalizedEpochQuery::from((1, 2)).query(&settings).unwrap();
    let Some(proto::get_finalized_epoch_infos_request::Version::V0(v0)) = epochs.version else {
        panic!("expected V0")
    };
    assert!(v0.prove);
    assert_eq!((v0.start_epoch_index, v0.end_epoch_index), (1, 2));
}

#[cfg(feature = "mocks")]
mod fetch {
    use super::*;
    use dash_sdk::platform::{DataContract, Fetch, FetchMany, Identity};
    use dash_sdk::SdkBuilder;
    fn sdk() -> dash_sdk::Sdk {
        SdkBuilder::new_mock()
            .with_proofs(false)
            .with_version(PlatformVersion::latest())
            .build()
            .unwrap()
    }
    #[tokio::test]
    async fn should_reject_identity_fetch_before_transport() {
        assert_config(
            Identity::fetch(&sdk(), id()).await.unwrap_err(),
            UNPROVED_MESSAGE,
        );
    }
    #[tokio::test]
    async fn should_reject_contract_fetch_before_transport() {
        assert_config(
            DataContract::fetch(&sdk(), id()).await.unwrap_err(),
            UNPROVED_MESSAGE,
        );
    }
    #[tokio::test]
    async fn should_reject_contract_fetch_many_before_transport() {
        assert_config(
            DataContract::fetch_many(&sdk(), vec![id(), other_id()])
                .await
                .unwrap_err(),
            UNPROVED_MESSAGE,
        );
    }
    #[tokio::test]
    async fn should_reject_nonce_fetch_before_transport() {
        assert_config(
            sdk()
                .get_identity_nonce(id(), false, None)
                .await
                .unwrap_err(),
            UNPROVED_MESSAGE,
        );
    }
    #[tokio::test]
    async fn should_preserve_proof_disabled_protocol_refresh() {
        assert_eq!(
            sdk().refresh_protocol_version().await.unwrap(),
            PlatformVersion::latest().protocol_version
        );
    }
}
